use rolldown_utils::{
  pattern_filter::{FilterResult, StringOrRegex, filter},
  url::clean_url,
};
use rollipop_react_native_transform::ReactCompilerFilter;

/// Host-side filename patterns for the React Native compiler.
#[derive(Debug, Default, Clone)]
pub struct ReactCompilerFilenameFilter {
  pub include: Vec<StringOrRegex>,
  pub exclude: Vec<StringOrRegex>,
}

impl ReactCompilerFilter for ReactCompilerFilenameFilter {
  fn matches(&self, id: &str, cwd: &str) -> bool {
    if self.include.is_empty() && self.exclude.is_empty() {
      return true;
    }

    let exclude = (!self.exclude.is_empty()).then_some(self.exclude.as_slice());
    let include = (!self.include.is_empty()).then_some(self.include.as_slice());

    let original = filter(exclude, include, id, cwd);
    if original == FilterResult::Match(false) {
      return false;
    }
    if exclude.is_none() && original.inner() {
      return true;
    }

    let cleaned_id = clean_url(id);
    if cleaned_id == id {
      return original.inner();
    }

    // Exclusion of either form wins, even when the other form matches an include.
    let cleaned = filter(exclude, include, cleaned_id, cwd);
    cleaned != FilterResult::Match(false) && (original.inner() || cleaned.inner())
  }
}

#[cfg(test)]
mod tests {
  use rolldown_utils::js_regex::HybridRegex;

  use super::*;

  #[test]
  fn excludes_cleaned_filenames_with_queries_or_fragments() {
    for exclude in [
      StringOrRegex::String("**/Probe.jsx".into()),
      StringOrRegex::Regex(HybridRegex::new(r"/Probe\.jsx$").unwrap()),
    ] {
      for include in [Vec::new(), vec![StringOrRegex::String("**/*".into())]] {
        let options = ReactCompilerFilenameFilter { include, exclude: vec![exclude.clone()] };
        for id in ["/app/Probe.jsx", "/app/Probe.jsx?platform=ios", "/app/Probe.jsx#component"] {
          assert!(!options.matches(id, "/app"), "{id}");
        }
        assert!(options.matches("/app/Other.jsx?platform=ios", "/app"));
      }
    }
  }

  #[test]
  fn retains_query_exclusions_when_cleaned_filename_is_included() {
    let options = ReactCompilerFilenameFilter {
      include: vec![StringOrRegex::String("**/Probe.jsx".into())],
      exclude: vec![StringOrRegex::Regex(HybridRegex::new(r"\?platform=ios$").unwrap())],
    };
    assert!(!options.matches("/app/Probe.jsx?platform=ios", "/app"));
    assert!(options.matches("/app/Probe.jsx?platform=android", "/app"));
  }

  #[test]
  fn includes_original_or_cleaned_filenames() {
    let options = ReactCompilerFilenameFilter {
      include: vec![
        StringOrRegex::Regex(HybridRegex::new(r"\?platform=ios$").unwrap()),
        StringOrRegex::String("src/Probe.jsx".into()),
      ],
      exclude: vec![StringOrRegex::String("**/vendor/**".into())],
    };
    assert!(options.matches("/app/lib/Other.jsx?platform=ios", "/app"));
    assert!(options.matches("/app/src/Probe.jsx?platform=android", "/app"));
    assert!(!options.matches("/app/lib/Other.jsx?platform=android", "/app"));
    assert!(!options.matches("/app/vendor/Probe.jsx?platform=ios", "/app"));
  }
}
