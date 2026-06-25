use json_escape_simd::escape;
use rolldown_common::{AddonRenderContext, ExportsKind, NormalModule};
use rolldown_sourcemap::{Source, SourceJoiner};
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
    "\tvar ",
    ROLLIPOP_MODULES_NAME,
    " = ",
    ROLLIPOP_REQUIRE_NAME,
    ".m = {"
  ));
  let mut modules = module_sources
    .iter()
    .filter_map(|RenderedModuleSource { module_idx, sources, .. }| {
      if *module_idx == ctx.link_output.runtime.id() {
        return None;
      }
      let sources = sources.as_ref()?;
      let module = ctx.link_output.module_table[*module_idx].as_normal()?;
      Some((module, sources))
    })
    .peekable();

  while let Some((module, sources)) = modules.next() {
    source_joiner.append_source(concat_string!(
      "\t\t",
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
    append_module_sources(sources.as_ref(), source_joiner);
    source_joiner.append_source(if modules.peek().is_some() { "\t\t}," } else { "\t\t}" });
  }
  source_joiner.append_source("\t};");
}

fn render_entry_execution(ctx: &GenerateContext<'_>) -> String {
  if let Some(entry_module) = ctx.chunk.entry_module(&ctx.link_output.module_table)
    && matches!(
      entry_module.exports_kind,
      ExportsKind::Esm | ExportsKind::CommonJs | ExportsKind::None
    )
  {
    return concat_string!(
      "\t",
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
  source_joiner.append_source("\t//#region \\0rollipop/runtime");
  for line in ROLLIPOP_RUNTIME.trim_end_matches('\n').lines() {
    source_joiner.append_source(concat_string!("\t", line));
  }
  source_joiner.append_source("\t//#endregion");
}

fn append_module_sources<'code>(
  sources: &'code [Box<dyn Source + Send + Sync>],
  source_joiner: &mut SourceJoiner<'code>,
) {
  for source in sources {
    if source.sourcemap().is_some() {
      source_joiner.append_source(source);
    } else {
      source_joiner.append_source(indent_module_source(source.content()));
    }
  }
}

fn indent_module_source(source: &str) -> String {
  let mut ret = String::with_capacity(source.len() + source.lines().count() * 2);
  for line in source.split_inclusive('\n') {
    if line.trim_end_matches('\n').is_empty() {
      ret.push_str(line);
    } else {
      let trimmed = line.trim_start();
      if trimmed.starts_with("//#region") || trimmed.starts_with("//#endregion") {
        ret.push_str("\t\t\t");
      } else {
        ret.push_str("\t\t");
      }
      ret.push_str(line);
    }
  }
  ret
}
