//! Compatibility hooks grouped around the legacy class-properties pass.

use swc_common::{Mark, SyntaxContext};
use swc_ecma_ast::Pass;
use swc_ecma_hooks::CompositeHook;
use swc_ecma_transformer::{Options, hook_pass, transform_hook};

use crate::{
  RuntimeTarget,
  visitors::{hermes_v1_fixes, template_literal_caching},
};

/// Static blocks must become fields before the legacy class-properties pass.
pub fn before_class_properties() -> impl Pass {
  let mut options = Options::default();
  options.env.es2022.class_static_block = true;
  options.env.regexp.named_capturing_groups_regex = true;
  options.env.regexp.unicode_property_regex = true;
  options.env.regexp.unicode_regex = true;

  hook_pass(CompositeHook { first: transform_hook(options), second: template_literal_caching() })
}

pub fn runtime(runtime_target: RuntimeTarget, unresolved_mark: Mark) -> impl Pass {
  let mut options = Options::default();
  options.unresolved_ctxt = SyntaxContext::empty().apply_mark(unresolved_mark);
  options.env.es2017.async_to_generator = true;
  let legacy = matches!(runtime_target, RuntimeTarget::Hermes);
  options.env.es2022.private_property_in_object = legacy;
  options.env.es2018.object_rest_spread = legacy;

  // CompositeHook runs exit hooks in reverse order. Cache tagged templates
  // before the async hook moves their enclosing function bodies.
  hook_pass(CompositeHook {
    first: (!legacy).then(hermes_v1_fixes),
    second: CompositeHook {
      first: transform_hook(options),
      second: (!legacy).then(template_literal_caching),
    },
  })
}
