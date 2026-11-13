//! Which exports of a module the generated Rollipop code provably reads through that module.
//!
//! A lazy import binding is only worth keeping lazy when something reads it. For a
//! re-export (`export { x } from './y'`, or `import { x }` plus `export { x }`), the
//! read is a call of the re-exporting module's getter, which happens only when another
//! module reads `x` *through* it. The linker binds importers straight to the canonical
//! symbol, so symbol usage can't answer this. Instead this walks the import graph the
//! way Rollipop codegen routes it: an import of an included module goes through it.
//!
//! Only reads that are visible statically count. When a namespace object escapes (a bare
//! `ns`, `import()`, `require()`, `export * as ns`), nothing is recorded, so re-exported
//! modules behind it stay eager. Eager is always correct; it only costs laziness.
//!
//! See internal-docs/rollipop-lazy-imports/implementation.md

use oxc_str::CompactStr;
use rolldown_common::{
  ImportKind, ImportRecordMeta, ModuleIdx, Specifier, SymbolOrMemberExprRef, SymbolRef,
};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::stages::link_stage::LinkStageOutput;

/// Per module, the exports that emitted code reads through that module.
pub type ExportReadsMap = FxHashMap<ModuleIdx, FxHashSet<CompactStr>>;

#[derive(Default)]
struct Graph {
  reads: ExportReadsMap,
  /// `(exported name, importee, imported name)`: reading this module's export reads
  /// the importee's export.
  re_exports: FxHashMap<ModuleIdx, Vec<(CompactStr, ModuleIdx, CompactStr)>>,
  /// `export * from` targets: names this module doesn't declare come from them.
  star_exports: FxHashMap<ModuleIdx, Vec<ModuleIdx>>,
  queue: Vec<(ModuleIdx, CompactStr)>,
}

impl Graph {
  fn mark(&mut self, module: ModuleIdx, name: CompactStr) {
    if self.reads.entry(module).or_default().insert(name.clone()) {
      self.queue.push((module, name));
    }
  }

  fn propagate(&mut self, link_output: &LinkStageOutput) {
    while let Some((module, name)) = self.queue.pop() {
      let mut next = vec![];
      if let Some(edges) = self.re_exports.get(&module) {
        next.extend(
          edges
            .iter()
            .filter(|(exported, ..)| *exported == name)
            .map(|(_, importee, imported)| (*importee, imported.clone())),
        );
      }
      if let Some(star_targets) = self.star_exports.get(&module) {
        let declares = link_output.module_table[module]
          .as_normal()
          .is_some_and(|module| module.named_exports.contains_key(&name));
        if !declares {
          next.extend(star_targets.iter().map(|target| (*target, name.clone())));
        }
      }
      for (module, name) in next {
        self.mark(module, name);
      }
    }
  }
}

pub fn compute_export_reads(link_output: &LinkStageOutput) -> ExportReadsMap {
  let mut graph = Graph::default();
  let is_included = |idx: ModuleIdx| {
    link_output.module_table[idx].as_normal().is_some() && link_output.metas[idx].is_included
  };
  // Modules loaded for their side effects or whose namespace object escapes. Their
  // re-exported modules must run with them, as they do eagerly.
  let mut pinned_roots = vec![];

  for module in link_output.module_table.modules.iter().filter_map(|module| module.as_normal()) {
    if module.idx == link_output.runtime.id() || !is_included(module.idx) {
      continue;
    }

    for importee in module.star_export_module_ids() {
      if is_included(importee) {
        graph.star_exports.entry(module.idx).or_default().push(importee);
      }
    }

    // `import './x'` (also when tree-shaking merged it into a named import),
    // `require('./x')` and `import('./x')`.
    let named_records =
      module.named_imports.values().map(|named| named.record_idx).collect::<FxHashSet<_>>();
    for (record_idx, record) in module.import_records.iter_enumerated() {
      let pins = match record.kind {
        ImportKind::Import => {
          !record.meta.contains(ImportRecordMeta::IsExportStar)
            && (record.meta.contains(ImportRecordMeta::HasSideEffectImport)
              || !named_records.contains(&record_idx))
        }
        ImportKind::Require | ImportKind::DynamicImport => true,
        _ => false,
      };
      if pins && let Some(importee) = record.resolved_module {
        pinned_roots.push(importee);
      }
    }

    if module.named_imports.is_empty() {
      continue;
    }
    let meta = &link_output.metas[module.idx];
    // Symbols read by the statements that end up in the output, and for member
    // expressions (`ns.x`), the first property read from each object.
    let mut referenced = FxHashSet::<SymbolRef>::default();
    let mut bare_references = FxHashSet::<SymbolRef>::default();
    let mut member_reads = FxHashMap::<SymbolRef, FxHashSet<&CompactStr>>::default();
    for (_, info) in link_output.stmt_infos[module.idx]
      .iter_enumerated_without_namespace_stmt()
      .filter(|(idx, _)| meta.stmt_info_included.has_bit(*idx))
    {
      for symbol in &info.referenced_symbols {
        referenced.insert(*symbol.symbol_ref());
        match symbol {
          SymbolOrMemberExprRef::Symbol(symbol) => {
            bare_references.insert(*symbol);
          }
          SymbolOrMemberExprRef::MemberExpr(member) => {
            if let Some(prop) = member.prop_and_span_list.first() {
              member_reads.entry(member.object_ref).or_default().insert(&prop.name);
            }
          }
        }
      }
    }
    let mut exported_as = FxHashMap::<SymbolRef, Vec<&CompactStr>>::default();
    for (exported, local) in &module.named_exports {
      exported_as.entry(local.referenced).or_default().push(exported);
    }

    for (local, named_import) in &module.named_imports {
      // Codegen goes through the importee only when it is included; otherwise
      // the binding targets the canonical owner and never calls its getters.
      let Some(importee) = module.import_records[named_import.record_idx].resolved_module else {
        continue;
      };
      if !is_included(importee) {
        continue;
      }
      match &named_import.imported {
        Specifier::Literal(imported) => {
          if let Some(exported) = exported_as.get(local) {
            let edges = graph.re_exports.entry(module.idx).or_default();
            edges.extend(
              exported.iter().map(|exported| ((*exported).clone(), importee, imported.clone())),
            );
          }
          if referenced.contains(local) {
            graph.mark(importee, imported.clone());
          }
        }
        // `ns.x` reads `x`. A bare `ns`, or re-exporting it (`export * as ns`),
        // lets the namespace escape, so any export may be read.
        Specifier::Star => {
          for prop in member_reads.get(local).into_iter().flatten() {
            graph.mark(importee, (*prop).clone());
          }
          if bare_references.contains(local) || exported_as.contains_key(local) {
            pinned_roots.push(importee);
          }
        }
      }
    }
  }

  graph.propagate(link_output);
  let mut reads = graph.reads;
  // `import 'pkg'`, `require('pkg')` and an escaping namespace run or expose pkg's
  // whole graph eagerly, including what its index re-exports, so re-export getters
  // of those barrels don't count as reads.
  for module in pinned_barrels(link_output, &is_included, pinned_roots) {
    reads.remove(&module);
  }
  reads
}

/// `roots` plus every barrel they reach through re-exports and `export *`.
fn pinned_barrels(
  link_output: &LinkStageOutput,
  is_included: &impl Fn(ModuleIdx) -> bool,
  roots: Vec<ModuleIdx>,
) -> FxHashSet<ModuleIdx> {
  let mut seen = FxHashSet::default();
  let mut stack = vec![];
  for root in roots {
    if is_included(root) && seen.insert(root) {
      stack.push(root);
    }
  }
  while let Some(module_idx) = stack.pop() {
    let Some(module) = link_output.module_table[module_idx].as_normal() else { continue };
    let re_exported =
      module.named_exports.values().map(|local| local.referenced).collect::<FxHashSet<_>>();
    let targets = module
      .named_imports
      .iter()
      .filter(|(local, _)| re_exported.contains(*local))
      .filter_map(|(_, named)| module.import_records[named.record_idx].resolved_module)
      .chain(module.star_export_module_ids());
    for target in targets {
      if is_included(target) && seen.insert(target) {
        stack.push(target);
      }
    }
  }
  seen
}
