//! Hook ports of swc_react_native_hermes_v1_fixes 0.3.2 (Apache-2.0).
//! Keep the rewrites aligned with that crate when upgrading SWC React Native.
// See internal-docs/native-swc-passes/implementation.md.

mod ast;
mod async_arrow_non_simple_params;
mod class_in_finally;
mod super_in_object_accessor;
#[cfg(test)]
mod tests;

use swc_ecma_hooks::{CompositeHook, VisitMutHook};
use swc_ecma_transformer::TraverseCtx;

pub fn hermes_v1_fixes() -> impl VisitMutHook<TraverseCtx> {
  CompositeHook {
    first: async_arrow_non_simple_params::AsyncArrowNonSimpleParamsVisitor,
    second: CompositeHook {
      first: super_in_object_accessor::SuperInObjectAccessor::default(),
      second: class_in_finally::ClassInFinally::default(),
    },
  }
}
