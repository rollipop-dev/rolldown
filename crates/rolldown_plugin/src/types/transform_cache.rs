use std::path::PathBuf;
use std::sync::Arc;

use arcstr::ArcStr;
use dashmap::DashMap;
use rolldown_common::{
  ModuleType, PluginIdx, SourcemapChainElement, side_effects::HookSideEffects,
};
use rolldown_sourcemap::OwnedSourceMap;
use serde::{Deserialize, Serialize};
use tokio::sync::Semaphore;

const MAX_CONCURRENT_WRITES: usize = 20;

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

pub struct TransformCache {
  /// In-memory cache for fast lookup within the current build
  entries: DashMap<u128, TransformCacheEntry>,
  /// Pending entries to be flushed to disk
  pending: DashMap<u128, Vec<u8>>,
  /// Directory for filesystem cache
  cache_dir: PathBuf,
}

impl TransformCache {
  pub fn new(cache_dir: PathBuf) -> Self {
    if !cache_dir.exists() {
      std::fs::create_dir_all(&cache_dir).ok();
    }
    Self { entries: DashMap::default(), pending: DashMap::default(), cache_dir }
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
    self.entries.clear();
    self.pending.clear();
    std::fs::remove_dir_all(&self.cache_dir).ok();
  }
}
