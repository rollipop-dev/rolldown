//! Flow text-level utilities — directive detection and pragma stripping.
//! Pure helpers that don't touch transformer state.

use swc_common::comments::SingleThreadedComments;

/// Mirrors Babel's `@babel/plugin-transform-flow-strip-types` directive
/// regex `/@flow(?:\s+(?:strict(?:-local)?|weak))?|@noflow/` — both
/// `@flow` and `@noflow` count as Flow markers for build-pipeline
/// purposes (Flow's type-checker semantics are unrelated).
pub fn has_directive(code: &str) -> bool {
  let bytes = code.as_bytes();
  memchr::memmem::find(bytes, b"@flow").is_some()
    || memchr::memmem::find(bytes, b"@noflow").is_some()
}

/// Strip `@flow` pragma lines from leading/trailing comments. Run after
/// the SWC pipeline so they don't leak into the downstream parser — a
/// stray `@flow` makes plain-JS parsers reject the otherwise valid output.
pub fn strip_pragma_comments(comments: &SingleThreadedComments) {
  let (mut leading, mut trailing) = comments.borrow_all_mut();
  let retain = |c: &swc_common::comments::Comment| !is_pragma(&c.text);
  for list in leading.values_mut() {
    list.retain(retain);
  }
  for list in trailing.values_mut() {
    list.retain(retain);
  }
}

fn is_pragma(text: &str) -> bool {
  text
    .lines()
    .any(|line| line.trim_start().trim_start_matches('*').trim_start().starts_with("@flow"))
}
