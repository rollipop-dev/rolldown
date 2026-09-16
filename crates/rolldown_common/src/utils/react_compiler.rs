use oxc::{
  allocator::Allocator,
  ast::ast::Program,
  diagnostics::{Diagnostics, Severity},
  semantic::Scoping,
};
use rolldown_ecmascript::semantic_builder_for_transform;

/// Run OXC React Compiler before the regular OXC transformer.
pub fn run_react_compiler<'a>(
  allocator: &'a Allocator,
  program: &mut Program<'a>,
  options: Option<oxc_react_compiler::PluginOptions>,
) -> (Option<Scoping>, Diagnostics) {
  let Some(options) = options else {
    return (None, Diagnostics::new());
  };

  let result = {
    let semantic = semantic_builder_for_transform().with_build_nodes(true).build(program).semantic;
    oxc_react_compiler::compile(program, &semantic, allocator, options)
  };

  match result {
    oxc_react_compiler::CompileResult::Success { output, mut diagnostics } => {
      // Compiler bailouts can have error severity without being fatal under panic_threshold.
      // Only CompileResult::Fatal should fail the surrounding transform or bundle.
      for diagnostic in diagnostics.iter_mut() {
        if diagnostic.severity == Severity::Error {
          diagnostic.severity = Severity::Warning;
        }
      }
      if let Some(output) = output {
        output.transform(program);
        let scoping = semantic_builder_for_transform().build(program).semantic.into_scoping();
        (Some(scoping), diagnostics)
      } else {
        (None, diagnostics)
      }
    }
    oxc_react_compiler::CompileResult::Fatal { diagnostics } => (None, diagnostics),
  }
}
