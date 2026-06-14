use oxc_traverse::traverse_mut;
use rolldown_common::OutputFormat;
use rolldown_ecmascript::EcmaAst;
use rolldown_ecmascript_utils::AstFactory;
use rolldown_utils::index_vec_ext::IndexVecExt;
use rolldown_utils::rayon::ParallelIterator as _;
use tracing::debug_span;

use crate::{
  chunk_graph::ChunkGraph,
  module_finalizers::rollipop::{RollipopAstFinalizer, RollipopAstFinalizerParams},
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
      ast_table
        .par_iter_mut_enumerated()
        .filter(|(idx, _ast)| {
          self.link_output.module_table[*idx]
            .as_normal()
            .is_some_and(|m| self.link_output.metas[m.idx].is_included)
        })
        .for_each(|(idx, ast)| {
          let Some(ast) = ast.as_mut() else { return };
          let module = self.link_output.module_table[idx].as_normal().unwrap();
          let Some(_chunk_idx) = chunk_graph.module_to_chunk[idx] else { return };
          let unique_index = idx.raw() as usize;
          let is_dev_mode = self.options.is_dev_mode_enabled();
          let is_runtime_module = self.link_output.runtime.id() == idx;
          ast.program.with_mut(|fields| {
            let scoping = EcmaAst::make_semantic(fields.program, /*with_cfg*/ false).into_scoping();
            let mut finalizer = RollipopAstFinalizer::new(RollipopAstFinalizerParams {
              ast_factory: AstFactory::new(fields.allocator),
              modules: &self.link_output.module_table.modules,
              module,
              linking_info: &self.link_output.metas[module.idx],
              stmt_infos: &self.link_output.stmt_infos[idx],
              symbol_db: &self.link_output.symbol_db,
              unique_index,
              is_dev_mode,
              is_runtime_module,
            });
            traverse_mut(&mut finalizer, fields.allocator, fields.program, scoping, ());
          });
        });
    });
  }
}
