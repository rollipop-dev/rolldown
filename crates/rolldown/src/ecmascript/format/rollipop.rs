use json_escape_simd::escape;
use rolldown_common::{AddonRenderContext, ExportsKind, NormalModule};
use rolldown_sourcemap::SourceJoiner;
use rolldown_utils::concat_string;

use crate::{
  ecmascript::ecma_generator::{RenderedModuleSource, RenderedModuleSources},
  types::generator::GenerateContext,
};

use super::utils::{is_use_strict_directive, render_chunk_directives};

use crate::rollipop::{
  ROLLIPOP_EXPORTS_NAME, ROLLIPOP_GLOBAL_NAME, ROLLIPOP_MODULE_NAME, ROLLIPOP_MODULES_NAME,
  ROLLIPOP_REQUIRE_NAME, ROLLIPOP_RUNTIME,
};

pub fn render_rollipop<'code>(
  ctx: &GenerateContext<'_>,
  addon_render_context: &AddonRenderContext<'code>,
  module_sources: &'code RenderedModuleSources,
) -> SourceJoiner<'code> {
  let hashbang = addon_render_context.hashbang;
  let banner = addon_render_context.banner;
  let intro = addon_render_context.intro;
  let outro = addon_render_context.outro;
  let footer = addon_render_context.footer;
  let directives = addon_render_context.directives;
  let mut source_joiner = SourceJoiner::default();

  if let Some(hashbang) = hashbang {
    source_joiner.append_source(hashbang);
  }
  if let Some(banner) = banner {
    source_joiner.append_source(banner);
  }
  if !directives.is_empty() {
    let rendered_chunk_directives =
      render_chunk_directives(directives.iter().filter(|d| !is_use_strict_directive(d)));
    if !rendered_chunk_directives.is_empty() {
      source_joiner.append_source(rendered_chunk_directives);
    }
  }
  if let Some(intro) = intro {
    source_joiner.append_source(intro);
  }

  source_joiner.append_source(concat_string!("(function(", ROLLIPOP_GLOBAL_NAME, ") {"));
  render_runtime_module(ctx, module_sources, &mut source_joiner);
  render_rollipop_runtime(&mut source_joiner);
  render_module_factories(ctx, module_sources, &mut source_joiner);
  source_joiner.append_source(render_entry_execution(ctx));
  source_joiner.append_source(concat!(
    "})(typeof globalThis !== 'undefined' ? globalThis",
    " : typeof global !== 'undefined' ? global",
    " : typeof window !== 'undefined' ? window",
    " : this",
    ");"
  ));

  if let Some(outro) = outro {
    source_joiner.append_source(outro);
  }
  if let Some(footer) = footer {
    source_joiner.append_source(footer);
  }

  source_joiner
}

fn render_module_factories<'code>(
  ctx: &GenerateContext<'_>,
  module_sources: &'code [RenderedModuleSource],
  source_joiner: &mut SourceJoiner<'code>,
) {
  source_joiner.append_source(concat_string!(
    "var ",
    ROLLIPOP_MODULES_NAME,
    " = ",
    ROLLIPOP_REQUIRE_NAME,
    ".m = {"
  ));
  let mut is_first_module = true;
  for RenderedModuleSource { module_idx, sources, .. } in module_sources {
    if *module_idx == ctx.link_output.runtime.id() {
      continue;
    }
    let Some(sources) = sources else { continue };
    let Some(module) = ctx.link_output.module_table[*module_idx].as_normal() else { continue };

    if is_first_module {
      is_first_module = false;
    } else {
      source_joiner.append_source(",");
    }

    source_joiner.append_source(concat_string!(
      render_module_runtime_id(ctx, module),
      ": function(",
      ROLLIPOP_GLOBAL_NAME,
      ", ",
      ROLLIPOP_MODULE_NAME,
      ", ",
      ROLLIPOP_EXPORTS_NAME,
      ", ",
      ROLLIPOP_REQUIRE_NAME,
      ") {"
    ));
    for source in sources.as_ref() {
      source_joiner.append_source(source);
    }
    source_joiner.append_source("}");
  }
  source_joiner.append_source("};");
}

fn render_entry_execution(ctx: &GenerateContext<'_>) -> String {
  if let Some(entry_module) = ctx.chunk.entry_module(&ctx.link_output.module_table)
    && matches!(
      entry_module.exports_kind,
      ExportsKind::Esm | ExportsKind::CommonJs | ExportsKind::None
    )
  {
    return concat_string!(
      ROLLIPOP_REQUIRE_NAME,
      "(",
      render_module_runtime_id(ctx, entry_module),
      ");"
    );
  }
  String::new()
}

fn render_module_runtime_id(ctx: &GenerateContext<'_>, module: &NormalModule) -> String {
  if ctx.options.profiler_names {
    escape(module.stable_id.as_str())
  } else {
    module.idx.raw().to_string()
  }
}

fn render_runtime_module<'code>(
  ctx: &GenerateContext<'_>,
  module_sources: &'code [RenderedModuleSource],
  source_joiner: &mut SourceJoiner<'code>,
) {
  let Some(RenderedModuleSource { sources: Some(sources), .. }) =
    module_sources.iter().find(|source| source.module_idx == ctx.link_output.runtime.id())
  else {
    return;
  };
  for source in sources.as_ref() {
    source_joiner.append_source(source);
  }
}

fn render_rollipop_runtime(source_joiner: &mut SourceJoiner<'_>) {
  source_joiner.append_source("//#region \\0rollipop/runtime");
  for line in ROLLIPOP_RUNTIME.trim_end_matches('\n').lines() {
    source_joiner.append_source(line);
  }
  source_joiner.append_source("//#endregion");
}
