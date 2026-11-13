//! Which exports of a module the generated Rollipop code reads through that module.
//!
//! A lazy import binding is only worth keeping lazy when something reads it. For a
//! re-export (`export { x } from './y'`, or `import { x }` plus `export { x }`), the
//! read is a call of the re-exporting module's getter, which happens only when another
//! module imports `x` *through* it. The linker binds importers straight to the
//! canonical symbol, so symbol usage can't answer this. Instead this walks the
//! import graph the way Rollipop codegen routes it: an import of an included module
//! goes through that module.
//!
//! See internal-docs/rollipop-lazy-imports/implementation.md

use oxc_str::CompactStr;
use rolldown_common::{ImportKind, ModuleIdx, Specifier, StmtInfos, SymbolRef};
use rustc_hash::{FxHashMap, FxHashSet};

use crate::stages::link_stage::LinkStageOutput;

/// Exports of one module that some emitted code can read through it.
pub enum ExportReads {
  /// The module's namespace object is reachable, so any export can be read.
  All,
  Names(FxHashSet<CompactStr>),
}

impl ExportReads {
  pub fn contains(&self, name: &str) -> bool {
    match self {
      ExportReads::All => true,
      ExportReads::Names(names) => names.contains(name),
    }
  }
}

pub type ExportReadsMap = FxHashMap<ModuleIdx, ExportReads>;

#[derive(Clone)]
enum Target {
  Named(ModuleIdx, CompactStr),
  All(ModuleIdx),
}

#[derive(Default)]
struct Graph {
  reads: ExportReadsMap,
  /// `(exported name, target)`: reading this module's export reads the target.
  re_exports: FxHashMap<ModuleIdx, Vec<(CompactStr, Target)>>,
  /// `export * from` targets: names this module doesn't declare come from them.
  star_exports: FxHashMap<ModuleIdx, Vec<ModuleIdx>>,
  queue: Vec<Target>,
}

impl Graph {
  fn mark(&mut self, target: Target) {
    let changed = match &target {
      Target::All(module) => {
        let reads = self.reads.entry(*module).or_insert(ExportReads::Names(FxHashSet::default()));
        let changed = !matches!(reads, ExportReads::All);
        *reads = ExportReads::All;
        changed
      }
      Target::Named(module, name) => {
        match self.reads.entry(*module).or_insert(ExportReads::Names(FxHashSet::default())) {
          ExportReads::All => false,
          ExportReads::Names(names) => names.insert(name.clone()),
        }
      }
    };
    if changed {
      self.queue.push(target);
    }
  }

  fn propagate(&mut self, link_output: &LinkStageOutput) {
    while let Some(target) = self.queue.pop() {
      let module = match &target {
        Target::All(module) | Target::Named(module, _) => *module,
      };
      let mut next = vec![];
      if let Some(edges) = self.re_exports.get(&module) {
        for (exported, edge_target) in edges {
          if matches!(&target, Target::All(_))
            || matches!(&target, Target::Named(_, name) if name == exported)
          {
            next.push(edge_target.clone());
          }
        }
      }
      if let Some(star_targets) = self.star_exports.get(&module) {
        let declares = |name: &str| {
          link_output.module_table[module]
            .as_normal()
            .is_some_and(|module| module.named_exports.contains_key(name))
        };
        for star_target in star_targets {
          match &target {
            Target::All(_) => next.push(Target::All(*star_target)),
            Target::Named(_, name) if !declares(name) => {
              next.push(Target::Named(*star_target, name.clone()));
            }
            Target::Named(..) => {}
          }
        }
      }
      for target in next {
        self.mark(target);
      }
    }
  }
}

pub fn compute_export_reads(link_output: &LinkStageOutput) -> ExportReadsMap {
  let mut graph = Graph::default();
  let is_included = |idx: ModuleIdx| {
    link_output.module_table[idx].as_normal().is_some() && link_output.metas[idx].is_included
  };

  for module in link_output.module_table.modules.iter().filter_map(|module| module.as_normal()) {
    if module.idx == link_output.runtime.id() || !is_included(module.idx) {
      continue;
    }
    let meta = &link_output.metas[module.idx];
    let stmt_infos = &link_output.stmt_infos[module.idx];

    // An included namespace statement means the namespace object is built.
    if meta.stmt_info_included.has_bit(StmtInfos::NAMESPACE_STMT_IDX) {
      graph.mark(Target::All(module.idx));
    }

    // `import()` and `require()` hand out the whole exports object.
    for record in &module.import_records {
      if matches!(record.kind, ImportKind::DynamicImport | ImportKind::Require)
        && let Some(importee) = record.resolved_module
        && is_included(importee)
      {
        graph.mark(Target::All(importee));
      }
    }

    for importee in module.star_export_module_ids() {
      if is_included(importee) {
        graph.star_exports.entry(module.idx).or_default().push(importee);
      }
    }

    if module.named_imports.is_empty() {
      continue;
    }
    // Symbols read by the statements that end up in the output.
    let referenced = stmt_infos
      .iter_enumerated_without_namespace_stmt()
      .filter(|(idx, _)| meta.stmt_info_included.has_bit(*idx))
      .flat_map(|(_, info)| info.referenced_symbols.iter().map(|symbol| *symbol.symbol_ref()))
      .collect::<FxHashSet<SymbolRef>>();
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
      let target = match &named_import.imported {
        Specifier::Star => Target::All(importee),
        Specifier::Literal(name) => Target::Named(importee, name.clone()),
      };
      if let Some(exported) = exported_as.get(local) {
        let edges = graph.re_exports.entry(module.idx).or_default();
        edges.extend(exported.iter().map(|exported| ((*exported).clone(), target.clone())));
      }
      if referenced.contains(local) {
        graph.mark(target);
      }
    }
  }

  graph.propagate(link_output);
  graph.reads
}
