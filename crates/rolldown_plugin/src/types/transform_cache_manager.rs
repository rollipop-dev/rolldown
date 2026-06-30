use std::{
  io,
  path::{Path, PathBuf},
  sync::{Arc, LazyLock, Mutex, MutexGuard, Weak},
};

use arcstr::ArcStr;
use dashmap::DashMap;
use rolldown_common::{
  ModuleType, PluginIdx, SourcemapChainElement, side_effects::HookSideEffects,
};
use rolldown_sourcemap::OwnedSourceMap;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

const MAX_CONCURRENT_WRITES: usize = 20;
pub const ROLLIPOP_PATH: &str = ".rollipop";
pub const ROLLIPOP_CACHE_PATH: &str = "cache";

static TRANSFORM_CACHE_MANAGER_REGISTRY: LazyLock<Mutex<Vec<Weak<TransformCacheManager>>>> =
  LazyLock::new(|| Mutex::new(Vec::new()));

pub struct TransformCacheEntry {
  pub code: String,
  pub sourcemap_chain: Vec<SourcemapChainElement>,
  pub side_effects: Option<HookSideEffects>,
  pub module_type: ModuleType,
}

#[derive(Serialize, Deserialize)]
struct SerializableEntry {
  code: String,
  sourcemap_chain: Vec<SerializableSourcemapChainElement>,
  /// 0=True, 1=False, 2=NoTreeshake
  side_effects: Option<u8>,
  module_type: String,
}

#[derive(Serialize, Deserialize)]
enum SerializableSourcemapChainElement {
  Transform { plugin_idx: u32, map: String },
  Omitted { plugin_idx: u32, plugin_name: String },
  Null { plugin_idx: u32, original_content: String },
}

impl SerializableEntry {
  fn from_entry(entry: &TransformCacheEntry) -> Self {
    Self {
      code: entry.code.clone(),
      sourcemap_chain: entry
        .sourcemap_chain
        .iter()
        .filter_map(|e| match e {
          SourcemapChainElement::Transform((idx, map)) => {
            Some(SerializableSourcemapChainElement::Transform {
              plugin_idx: idx.raw(),
              map: map.to_json_string(),
            })
          }
          SourcemapChainElement::Omitted { plugin_idx, plugin_name } => {
            Some(SerializableSourcemapChainElement::Omitted {
              plugin_idx: plugin_idx.raw(),
              plugin_name: plugin_name.to_string(),
            })
          }
          SourcemapChainElement::Null { plugin_idx, original_content } => {
            Some(SerializableSourcemapChainElement::Null {
              plugin_idx: plugin_idx.raw(),
              original_content: original_content.to_string(),
            })
          }
          SourcemapChainElement::Load(_) => None,
        })
        .collect(),
      side_effects: entry.side_effects.map(|s| match s {
        HookSideEffects::True => 0,
        HookSideEffects::False => 1,
        HookSideEffects::NoTreeshake => 2,
      }),
      module_type: entry.module_type.to_string(),
    }
  }

  fn into_entry(self) -> Option<TransformCacheEntry> {
    let sourcemap_chain = self
      .sourcemap_chain
      .into_iter()
      .map(|element| match element {
        SerializableSourcemapChainElement::Transform { plugin_idx, map } => {
          OwnedSourceMap::from_json_string(&map)
            .map(OwnedSourceMap::into_inner)
            .ok()
            .map(|map| SourcemapChainElement::Transform((PluginIdx::from_raw(plugin_idx), map)))
        }
        SerializableSourcemapChainElement::Omitted { plugin_idx, plugin_name } => {
          Some(SourcemapChainElement::Omitted {
            plugin_idx: PluginIdx::from_raw(plugin_idx),
            plugin_name: ArcStr::from(plugin_name),
          })
        }
        SerializableSourcemapChainElement::Null { plugin_idx, original_content } => {
          Some(SourcemapChainElement::Null {
            plugin_idx: PluginIdx::from_raw(plugin_idx),
            original_content: ArcStr::from(original_content),
          })
        }
      })
      .collect::<Option<Vec<_>>>()?;

    let side_effects = self.side_effects.map(|s| match s {
      0 => HookSideEffects::True,
      1 => HookSideEffects::False,
      _ => HookSideEffects::NoTreeshake,
    });
    let module_type = ModuleType::from_known_str(&self.module_type).unwrap_or(ModuleType::Js);

    Some(TransformCacheEntry { code: self.code, sourcemap_chain, side_effects, module_type })
  }
}

/// Manager for persistent transform cache entries for one Rolldown `options.id`.
///
/// This is not a single cache entry. It owns the in-memory entry map, the pending
/// disk-write queue, and the filesystem cache directory for that build id.
pub struct TransformCacheManager {
  id: String,
  /// In-memory cache for fast lookup within the current build
  entries: DashMap<u128, TransformCacheEntry>,
  /// Pending entries to be flushed to disk
  pending: DashMap<u128, Vec<u8>>,
  /// Directory for filesystem cache
  cache_dir: PathBuf,
}

impl TransformCacheManager {
  fn new(id: String, cache_dir: PathBuf) -> Self {
    if !cache_dir.exists() {
      std::fs::create_dir_all(&cache_dir).ok();
    }
    Self { id, entries: DashMap::default(), pending: DashMap::default(), cache_dir }
  }

  pub fn shared(id: String, cache_dir: PathBuf) -> Arc<Self> {
    let cache = Arc::new(Self::new(id, cache_dir));
    transform_cache_manager_registry().push(Arc::downgrade(&cache));
    cache
  }

  fn key_to_filename(key: u128) -> String {
    format!("{key:032x}")
  }

  fn cache_file_path(&self, key: u128) -> PathBuf {
    self.cache_dir.join(Self::key_to_filename(key))
  }

  /// Look up cache: in-memory first, then filesystem.
  pub fn get(&self, key: u128) -> Option<TransformCacheEntry> {
    // Check in-memory cache
    if let Some(entry) = self.entries.get(&key) {
      return Some(TransformCacheEntry {
        code: entry.code.clone(),
        sourcemap_chain: entry.sourcemap_chain.clone(),
        side_effects: entry.side_effects,
        module_type: entry.module_type.clone(),
      });
    }

    // Check filesystem cache
    let path = self.cache_file_path(key);
    let data = std::fs::read(&path).ok()?;
    let serializable: SerializableEntry = serde_json::from_slice(&data).ok()?;
    let entry = serializable.into_entry()?;

    // Promote to in-memory cache
    let result = TransformCacheEntry {
      code: entry.code.clone(),
      sourcemap_chain: entry.sourcemap_chain.clone(),
      side_effects: entry.side_effects,
      module_type: entry.module_type.clone(),
    };
    self.entries.insert(key, entry);

    Some(result)
  }

  /// Store entry in in-memory cache and queue for disk flush.
  pub fn insert(&self, key: u128, entry: TransformCacheEntry) {
    let serialized = serde_json::to_vec(&SerializableEntry::from_entry(&entry)).ok();
    self.entries.insert(key, entry);
    if let Some(data) = serialized {
      self.pending.insert(key, data);
    }
  }

  /// Flush all pending entries to disk asynchronously with concurrency limit.
  pub async fn flush(&self) {
    if !self.cache_dir.exists() {
      tokio::fs::create_dir_all(&self.cache_dir).await.ok();
    }

    // Drain pending entries
    let writes: Vec<_> = self
      .pending
      .iter()
      .map(|r| *r.key())
      .collect::<Vec<_>>()
      .into_iter()
      .filter_map(|key| {
        let (_, data) = self.pending.remove(&key)?;
        Some((self.cache_file_path(key), data))
      })
      .collect();

    let semaphore = Arc::new(Semaphore::new(MAX_CONCURRENT_WRITES));
    let mut handles = Vec::with_capacity(writes.len());

    for (path, data) in writes {
      let permit = Arc::clone(&semaphore);
      handles.push(tokio::spawn(async move {
        let _permit = permit.acquire().await;
        tokio::fs::write(&path, &data).await.ok();
      }));
    }

    for handle in handles {
      handle.await.ok();
    }
  }

  pub fn clear(&self) {
    self.clear_inner().ok();
  }

  fn clear_inner(&self) -> io::Result<()> {
    self.entries.clear();
    self.pending.clear();
    remove_cache_dir(&self.cache_dir)
  }
}

pub fn clear_transform_cache(cwd: &Path) -> io::Result<()> {
  let registered_result = clear_registered_transform_caches(None);
  let root_result = remove_cache_dir(&cwd.join(ROLLIPOP_PATH).join(ROLLIPOP_CACHE_PATH));
  registered_result.and(root_result)
}

pub fn clear_transform_cache_by_id(cwd: &Path, cache_id: &str) -> io::Result<()> {
  if cache_id.is_empty() {
    return Err(io::Error::new(io::ErrorKind::InvalidInput, "cache id must not be empty"));
  }

  let registered_result = clear_registered_transform_caches(Some(cache_id));
  let root_result =
    remove_cache_dir(&cwd.join(ROLLIPOP_PATH).join(ROLLIPOP_CACHE_PATH).join(cache_id));
  registered_result.and(root_result)
}

fn clear_registered_transform_caches(cache_id: Option<&str>) -> io::Result<()> {
  let mut caches = Vec::new();
  transform_cache_manager_registry().retain(|cache_ref| {
    let Some(cache) = cache_ref.upgrade() else {
      return false;
    };
    let should_clear = match cache_id {
      Some(cache_id) => cache.id == cache_id,
      None => true,
    };
    if should_clear {
      caches.push(cache);
    }
    true
  });

  let mut first_error = None;

  for cache in caches {
    if let Err(error) = cache.clear_inner() {
      if first_error.is_none() {
        first_error = Some(error);
      }
    }
  }

  if let Some(error) = first_error { Err(error) } else { Ok(()) }
}

fn transform_cache_manager_registry() -> MutexGuard<'static, Vec<Weak<TransformCacheManager>>> {
  TRANSFORM_CACHE_MANAGER_REGISTRY.lock().unwrap_or_else(std::sync::PoisonError::into_inner)
}

fn remove_cache_dir(path: &Path) -> io::Result<()> {
  match std::fs::remove_dir_all(path) {
    Ok(()) => Ok(()),
    Err(error) if error.kind() == io::ErrorKind::NotFound => Ok(()),
    Err(error) => Err(error),
  }
}

#[cfg(test)]
mod tests {
  use super::*;
  use std::sync::{
    Mutex,
    atomic::{AtomicU64, Ordering},
  };

  static TEST_LOCK: Mutex<()> = Mutex::new(());
  static NEXT_TEST_ID: AtomicU64 = AtomicU64::new(1);

  fn test_root(label: &str) -> PathBuf {
    let id = NEXT_TEST_ID.fetch_add(1, Ordering::Relaxed);
    std::env::temp_dir()
      .join(format!("rolldown-transform-cache-{label}-{}-{id}", std::process::id()))
  }

  fn test_entry(code: &str) -> TransformCacheEntry {
    TransformCacheEntry {
      code: code.to_string(),
      sourcemap_chain: Vec::new(),
      side_effects: None,
      module_type: ModuleType::Js,
    }
  }

  #[test]
  fn clear_by_id_clears_registered_memory_and_filesystem_cache() {
    let _guard = TEST_LOCK.lock().unwrap();
    let root = test_root("by-id");
    let cache_dir = root.join(ROLLIPOP_PATH).join(ROLLIPOP_CACHE_PATH).join("app");
    let cache = TransformCacheManager::shared("app".to_string(), cache_dir.clone());

    cache.insert(1, test_entry("cached"));
    std::fs::create_dir_all(&cache_dir).unwrap();
    std::fs::write(cache_dir.join("manual"), b"cached").unwrap();

    clear_transform_cache_by_id(&root, "app").unwrap();

    assert!(cache.get(1).is_none());
    assert!(!cache_dir.exists());

    std::fs::remove_dir_all(root).ok();
  }

  #[test]
  fn clear_by_id_removes_filesystem_cache_before_any_cache_is_registered() {
    let _guard = TEST_LOCK.lock().unwrap();
    let root = test_root("by-id-no-register");
    let cache_dir = root.join(ROLLIPOP_PATH).join(ROLLIPOP_CACHE_PATH).join("app");
    std::fs::create_dir_all(&cache_dir).unwrap();
    std::fs::write(cache_dir.join("manual"), b"cached").unwrap();

    clear_transform_cache_by_id(&root, "app").unwrap();

    assert!(!cache_dir.exists());

    std::fs::remove_dir_all(root).ok();
  }

  #[test]
  fn clear_cache_removes_current_workspace_cache_root() {
    let _guard = TEST_LOCK.lock().unwrap();
    let root = test_root("all");
    let cache_dir = root.join(ROLLIPOP_PATH).join(ROLLIPOP_CACHE_PATH).join("app");
    std::fs::create_dir_all(&cache_dir).unwrap();
    std::fs::write(cache_dir.join("manual"), b"cached").unwrap();

    clear_transform_cache(&root).unwrap();

    assert!(!root.join(ROLLIPOP_PATH).join(ROLLIPOP_CACHE_PATH).exists());

    std::fs::remove_dir_all(root).ok();
  }
}
