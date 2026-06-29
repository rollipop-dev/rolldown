use std::{
  ops::{Deref, DerefMut},
  path::{Path, PathBuf},
  sync::Arc,
};

use dashmap::Entry;
use oxc::transformer::{ESFeature, EngineTargets, TransformOptions as OxcTransformOptions};
use oxc_resolver::{ResolveOptions, Resolver, TsconfigDiscovery, TsconfigOptions};
use rolldown_error::{BuildDiagnostic, BuildResult};
use rolldown_utils::dashmap::FxDashMap;

use super::tsconfig_merge::merge_transform_options_with_tsconfig as merge_tsconfig;
use crate::{BundlerTransformOptions, ReactCompilerOptions, TsConfig};

#[derive(Debug, Default, Clone)]
pub enum JsxPreset {
  /// Enable JSX transformer
  #[default]
  Enable,
  /// Disable JSX parser - syntax error if JSX is encountered
  Disable,
  /// Parse JSX but preserve it in output
  Preserve,
}

/// Transform options with auto tsconfig discovery and caching
#[derive(Debug, Clone)]
pub struct RawTransformOptions {
  pub base_options: Arc<BundlerTransformOptions>,
  /// Cache key: tsconfig path, or empty PathBuf for files without tsconfig
  pub cache: FxDashMap<PathBuf, Arc<ResolvedTransformOptions>>,
  resolver: Arc<Resolver>,
}

impl RawTransformOptions {
  pub fn new(base_options: BundlerTransformOptions, tsconfig: TsConfig, yarn_pnp: bool) -> Self {
    Self {
      base_options: Arc::new(base_options),
      cache: FxDashMap::default(),
      resolver: Arc::new(Resolver::new(ResolveOptions {
        tsconfig: match tsconfig {
          TsConfig::Auto(v) => v.then_some(TsconfigDiscovery::Auto),
          TsConfig::Manual(config_file) => Some(TsconfigDiscovery::Manual(TsconfigOptions {
            config_file,
            references: oxc_resolver::TsconfigReferences::Auto,
          })),
        },
        yarn_pnp,
        ..Default::default()
      })),
    }
  }

  pub fn get_or_create_for_tsconfig(
    &self,
    tsconfig: Option<&oxc_resolver::TsConfig>,
    warnings: &mut Vec<BuildDiagnostic>,
  ) -> BuildResult<Arc<ResolvedTransformOptions>> {
    let cache_key = tsconfig.map(|t| t.path.clone()).unwrap_or_default();
    match self.cache.entry(cache_key) {
      Entry::Occupied(entry) => Ok(Arc::clone(entry.get())),
      Entry::Vacant(vacant_entry) => {
        let merged_options = Arc::new(merge_transform_options_with_tsconfig(
          self.base_options.as_ref().clone(),
          tsconfig,
          warnings,
        )?);
        vacant_entry.insert(Arc::clone(&merged_options));
        Ok(merged_options)
      }
    }
  }
}

#[derive(Debug, Clone)]
pub struct ResolvedTransformOptions {
  options: Arc<OxcTransformOptions>,
  react_compiler: Option<ReactCompilerOptions>,
}

impl ResolvedTransformOptions {
  fn new(options: OxcTransformOptions, react_compiler: Option<ReactCompilerOptions>) -> Self {
    Self { options: Arc::new(options), react_compiler }
  }

  fn has_react_compiler(&self) -> bool {
    self.react_compiler.is_some()
  }

  fn options_for_file(&self, file_path: &str, cwd: &str) -> TransformOptionsForFile {
    let Some(react_compiler) = &self.react_compiler else {
      return TransformOptionsForFile { options: Arc::clone(&self.options) };
    };

    if react_compiler.should_transform(file_path, cwd) {
      return TransformOptionsForFile { options: Arc::clone(&self.options) };
    }

    let mut options = self.options.as_ref().clone();
    // MARK: - Rollipop
    options.react_compiler = None;
    TransformOptionsForFile { options: Arc::new(options) }
  }
}

#[derive(Debug, Clone)]
pub struct TransformOptionsForFile {
  options: Arc<OxcTransformOptions>,
}

impl Deref for TransformOptionsForFile {
  type Target = OxcTransformOptions;

  fn deref(&self) -> &Self::Target {
    &self.options
  }
}

impl AsRef<OxcTransformOptions> for TransformOptionsForFile {
  fn as_ref(&self) -> &OxcTransformOptions {
    &self.options
  }
}

#[derive(Debug, Clone)]
pub enum TransformOptionsInner {
  /// Auto tsconfig discovery - each file uses its nearest tsconfig
  Raw(RawTransformOptions),
  /// Pre-resolved options - all files use the same options
  Normal(Arc<ResolvedTransformOptions>),
}

#[derive(Debug, Clone)]
pub struct TransformOptions {
  inner: TransformOptionsInner,
  pub target: EngineTargets,
  pub jsx_preset: JsxPreset,
  cwd: String,
}

impl Deref for TransformOptions {
  type Target = TransformOptionsInner;

  fn deref(&self) -> &Self::Target {
    &self.inner
  }
}

impl DerefMut for TransformOptions {
  fn deref_mut(&mut self) -> &mut Self::Target {
    &mut self.inner
  }
}

impl TransformOptions {
  #[inline]
  pub fn new(
    options: ResolvedTransformOptions,
    target: EngineTargets,
    jsx_preset: JsxPreset,
    cwd: &Path,
  ) -> Self {
    Self {
      inner: TransformOptionsInner::Normal(Arc::new(options)),
      target,
      jsx_preset,
      cwd: cwd.to_string_lossy().into_owned(),
    }
  }

  #[inline]
  pub fn new_raw(
    raw: RawTransformOptions,
    target: EngineTargets,
    jsx_preset: JsxPreset,
    cwd: &Path,
  ) -> Self {
    Self {
      inner: TransformOptionsInner::Raw(raw),
      target,
      jsx_preset,
      cwd: cwd.to_string_lossy().into_owned(),
    }
  }

  #[inline]
  pub fn is_jsx_disabled(&self) -> bool {
    matches!(self.jsx_preset, JsxPreset::Disable)
  }

  #[inline]
  pub fn is_jsx_preserve(&self) -> bool {
    matches!(self.jsx_preset, JsxPreset::Preserve)
  }

  pub fn should_transform_js(&self) -> bool {
    match &self.inner {
      TransformOptionsInner::Normal(opts) => {
        opts.options.env.regexp.set_notation || opts.has_react_compiler()
      }
      TransformOptionsInner::Raw(raw) => {
        self.target.has_feature(ESFeature::ES2024UnicodeSetsRegex)
          || raw.base_options.react_compiler.is_some()
      }
    }
  }

  pub fn options_for_file(
    &self,
    id: &str,
    file_path: Option<&Path>,
    warnings: &mut Vec<BuildDiagnostic>,
  ) -> BuildResult<TransformOptionsForFile> {
    match &self.inner {
      TransformOptionsInner::Normal(opts) => Ok(opts.options_for_file(id, &self.cwd)),
      TransformOptionsInner::Raw(raw) => {
        let tsconfig = match file_path {
          Some(path) => raw
            .resolver
            .find_tsconfig(path)
            .map_err(|err| BuildDiagnostic::tsconfig_error(path.display().to_string(), err))?,
          None => None,
        };
        Ok(
          raw
            .get_or_create_for_tsconfig(tsconfig.as_deref(), warnings)?
            .options_for_file(id, &self.cwd),
        )
      }
    }
  }
}

impl Default for TransformOptions {
  fn default() -> Self {
    Self {
      inner: TransformOptionsInner::Normal(Arc::new(ResolvedTransformOptions::new(
        OxcTransformOptions::default(),
        None,
      ))),
      target: EngineTargets::default(),
      jsx_preset: JsxPreset::default(),
      cwd: String::new(),
    }
  }
}

pub fn merge_transform_options_with_tsconfig(
  transform_options: BundlerTransformOptions,
  tsconfig: Option<&oxc_resolver::TsConfig>,
  warnings: &mut Vec<BuildDiagnostic>,
) -> BuildResult<ResolvedTransformOptions> {
  let merged_options = if let Some(tsconfig) = tsconfig {
    let (merged, merge_warnings) = merge_tsconfig(transform_options, tsconfig, true);
    warnings.extend(merge_warnings);
    merged
  } else {
    transform_options
  };
  let react_compiler = merged_options.react_compiler.clone();

  let options = merged_options.try_into().map_err(|message: String| {
    let hint = message
      .contains("Invalid target")
      .then(|| "Rolldown only supports ES2015 (ES6) and later.".to_owned());
    BuildDiagnostic::bundler_initialize_error(message, hint)
  })?;

  Ok(ResolvedTransformOptions::new(options, react_compiler))
}
