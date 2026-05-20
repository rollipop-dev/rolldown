//! Remove Flow type-only class field declarations (no initializer).
//!
//! In Flow, a class property without an initializer — both `+name: T;` and
//! plain `name: T;` — is a type annotation with no runtime field.
//! babel's `@babel/preset-flow` drops them entirely.
//!
//! swc's Flow strip leaves them as ordinary `ClassProp`s,
//! so the downstream `class_properties` pass emits `this.name = void 0;` in the constructor.

use swc_ecma_ast::{Class, ClassMember};
use swc_ecma_visit::{VisitMut, VisitMutWith};

pub struct RemoveFlowTypeOnlyFields;

impl VisitMut for RemoveFlowTypeOnlyFields {
  fn visit_mut_class(&mut self, class: &mut Class) {
    class
      .body
      .retain(|member| !matches!(member, ClassMember::ClassProp(prop) if prop.value.is_none()));
    class.visit_mut_children_with(self);
  }
}
