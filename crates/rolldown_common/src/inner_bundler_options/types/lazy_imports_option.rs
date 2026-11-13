// MARK: - Rollipop
// See internal-docs/rollipop-lazy-imports/design.md
use rolldown_utils::pattern_filter::{FilterResult, StringOrRegex, filter};
#[cfg(feature = "deserialize_bundler_options")]
use schemars::JsonSchema;
#[cfg(feature = "deserialize_bundler_options")]
use serde::{Deserialize, Deserializer};

/// `output.lazyImports`: `true` or an options object enables lazy imports.
#[derive(Debug, Clone)]
#[cfg_attr(
  feature = "deserialize_bundler_options",
  derive(Deserialize, JsonSchema),
  serde(untagged)
)]
pub enum LazyImportsOption {
  Bool(bool),
  Options(LazyImportsOptions),
}

#[derive(Debug, Default, Clone)]
#[cfg_attr(
  feature = "deserialize_bundler_options",
  derive(Deserialize, JsonSchema),
  serde(rename_all = "camelCase", deny_unknown_fields)
)]
pub struct LazyImportsOptions {
  /// Modules that are always evaluated eagerly when imported, matched against module ids.
  #[cfg_attr(
    feature = "deserialize_bundler_options",
    serde(default, deserialize_with = "deserialize_string_or_regex"),
    schemars(with = "Option<Vec<String>>")
  )]
  pub exclude: Option<Vec<StringOrRegex>>,
}

impl LazyImportsOption {
  pub fn normalize(self) -> Option<NormalizedLazyImportsOptions> {
    match self {
      Self::Bool(false) => None,
      Self::Bool(true) => Some(NormalizedLazyImportsOptions::default()),
      Self::Options(options) => {
        Some(NormalizedLazyImportsOptions { exclude: options.exclude.unwrap_or_default() })
      }
    }
  }
}

#[derive(Debug, Default, Clone)]
pub struct NormalizedLazyImportsOptions {
  pub exclude: Vec<StringOrRegex>,
}

impl NormalizedLazyImportsOptions {
  /// Whether imports of the module with this id must stay eager.
  /// String patterns are globs resolved against `cwd`, like other module filters.
  pub fn is_excluded(&self, id: &str, cwd: &str) -> bool {
    matches!(filter(Some(&self.exclude), None, id, cwd), FilterResult::Match(false))
  }
}

#[cfg(feature = "deserialize_bundler_options")]
fn deserialize_string_or_regex<'de, D>(
  deserializer: D,
) -> Result<Option<Vec<StringOrRegex>>, D::Error>
where
  D: Deserializer<'de>,
{
  let deserialized = Option::<Vec<String>>::deserialize(deserializer)?;
  Ok(deserialized.map(|v| v.into_iter().map(StringOrRegex::String).collect::<Vec<_>>()))
}
