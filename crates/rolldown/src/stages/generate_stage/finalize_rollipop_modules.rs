use oxc::ast::builder::AstBuilder;
use oxc_traverse::traverse_mut;
use rolldown_common::OutputFormat;
use rolldown_ecmascript::EcmaAst;
use rolldown_utils::index_vec_ext::IndexVecExt;
use rolldown_utils::rayon::ParallelIterator as _;
use tracing::debug_span;

use crate::{
  chunk_graph::ChunkGraph,
  module_finalizers::rollipop::{RollipopAstFinalizer, RollipopAstFinalizerContext},
  type_alias::IndexEcmaAst,
};

use super::GenerateStage;

impl GenerateStage<'_> {
  #[tracing::instrument(level = "debug", skip_all)]
  pub(super) fn finalize_rollipop_modules(
    &self,
    chunk_graph: &ChunkGraph,
    ast_table: &mut IndexEcmaAst,
  ) {
    if !matches!(self.options.format, OutputFormat::Rollipop) {
      return;
    }

    debug_span!("finalize_rollipop_modules").in_scope(|| {
      let link_output = &*self.link_output;
      let options = self.options;
      ast_table
        .par_iter_mut_enumerated()
        .filter(|(idx, _ast)| {
          link_output.module_table[*idx].as_normal().is_some_and(|m| {
            // The Rollipop renderer also prints a runtime retained by chunk planning.
            m.idx == link_output.runtime.id() || link_output.metas[m.idx].is_included
          })
        })
        .for_each(|(idx, ast)| {
          let Some(ast) = ast.as_mut() else { return };
          let module = link_output.module_table[idx].as_normal().unwrap();
          let Some(_chunk_idx) = chunk_graph.module_to_chunk[idx] else { return };
          let unique_index = idx.raw() as usize;
          ast.program.with_mut(|fields| {
            let scoping = EcmaAst::make_semantic(fields.program).into_scoping();
            let mut finalizer = RollipopAstFinalizer::new(
              AstBuilder::new(fields.allocator),
              RollipopAstFinalizerContext { link_output, options, module, unique_index },
            );
            traverse_mut(&mut finalizer, fields.allocator, fields.program, scoping, ());
          });
        });
    });
  }
}
