use oxc::{
  allocator::{Allocator, Box as ArenaBox, GetAllocator, TakeIn},
  ast::{
    ast::{self, ExportDefaultDeclarationKind, Expression, ObjectPropertyKind, Statement},
    builder::{AstBuilder, GetAstBuilder},
  },
  semantic::{IsGlobalReference, Scoping, SymbolId},
  span::{GetSpanMut, SPAN, Span},
};
use oxc_traverse::{Ancestor, Traverse};
use rolldown_common::{
  ExternalModule, ImportRecordIdx, IndexModules, Interop, Module, ModuleIdx, ModuleType,
  NormalModule, Specifier, StmtInfoIdx, StmtInfos, StrictMode, SymbolRef, SymbolRefDb,
  ThisExprReplaceKind,
};
use rolldown_ecmascript::CJS_REQUIRE_REF_STR;
use rolldown_ecmascript_utils::{
  BindingIdentifierFactoryExt, ExpressionExt, ExpressionFactoryExt, IdentifierNameFactoryExt,
  ObjectPropertyKindFactoryExt, StatementFactoryExt,
};
use rolldown_utils::ecmascript::is_validate_identifier_name;
use rustc_hash::{FxHashMap, FxHashSet};
use std::ops::Deref;

use crate::{
  SharedOptions,
  hmr::utils::HmrAstBuilder,
  rollipop::{
    HMR_RUNTIME_NAME, ROLLIPOP_EXPORTS_NAME, ROLLIPOP_GLOBAL_NAME, ROLLIPOP_MODULE_NAME,
    ROLLIPOP_REQUIRE_NAME,
  },
  stages::link_stage::LinkStageOutput,
  types::linking_metadata::{LinkingMetadata, LinkingMetadataVec},
};

const FACTORY_PARAM_NAMES: [&str; 5] = [
  ROLLIPOP_GLOBAL_NAME,
  ROLLIPOP_MODULE_NAME,
  ROLLIPOP_EXPORTS_NAME,
  ROLLIPOP_REQUIRE_NAME,
  HMR_RUNTIME_NAME,
];

struct AstFactory<'ast> {
  ast_builder: AstBuilder<'ast>,
}

impl<'ast> AstFactory<'ast> {
  fn make_id_ref_expr(&self, span: Span, name: &str) -> Expression<'ast> {
    Expression::new_id_ref_expr(span, name, self)
  }

  fn make_member_access_expr(&self, object: &str, property: &str) -> Expression<'ast> {
    Expression::new_member_access_expr(object, property, self)
  }

  fn make_call_with_arg(
    &self,
    callee: Expression<'ast>,
    arg: Expression<'ast>,
    pure: bool,
  ) -> Expression<'ast> {
    Expression::new_call_with_arg(callee, arg, pure, self)
  }

  fn make_to_esm_call_with_interop(
    &self,
    to_esm_fn_name: &str,
    call_expr: Expression<'ast>,
    interop: Option<Interop>,
  ) -> Expression<'ast> {
    Expression::new_to_esm_call_with_interop(to_esm_fn_name, call_expr, interop, self)
  }

  fn make_member_expr_or_ident_ref(
    &self,
    object: Expression<'ast>,
    props: &[rolldown_common::MemberExprProp],
    span: Span,
  ) -> Expression<'ast> {
    Expression::new_member_expr_or_ident_ref(object, props, span, self)
  }

  fn make_member_expr_with_void_zero_object(
    &self,
    props: &[rolldown_common::MemberExprProp],
    span: Span,
  ) -> Expression<'ast> {
    Expression::new_member_expr_with_void_zero_object(props, span, self)
  }

  fn make_promise_resolve_then(&self, expr: Expression<'ast>) -> Expression<'ast> {
    Expression::new_promise_resolve_then(expr, self)
  }

  fn make_id(&self, span: Span, name: &str) -> ast::BindingIdentifier<'ast> {
    ast::BindingIdentifier::new_id(span, name, self)
  }

  fn make_id_name(&self, span: Span, name: &str) -> ast::IdentifierName<'ast> {
    ast::IdentifierName::new_id_name(span, name, self)
  }

  fn make_lazy_export_property(
    &self,
    key: &str,
    expr: Expression<'ast>,
    computed: bool,
  ) -> ObjectPropertyKind<'ast> {
    ObjectPropertyKind::new_lazy_export_property(key, expr, computed, self)
  }

  fn make_var_decl(&self, name: &str, init: Expression<'ast>) -> Statement<'ast> {
    Statement::new_var_decl(name, init, self)
  }
}

impl<'ast> GetAstBuilder<'ast> for AstFactory<'ast> {
  type Builder = AstBuilder<'ast>;

  fn builder(&self) -> &AstBuilder<'ast> {
    &self.ast_builder
  }
}

impl<'ast> GetAllocator<'ast> for AstFactory<'ast> {
  fn allocator(&self) -> &'ast Allocator {
    self.ast_builder.allocator()
  }
}

impl Deref for AstFactory<'_> {
  type Target = Allocator;

  fn deref(&self) -> &Allocator {
    self.ast_builder.allocator()
  }
}

#[derive(Clone, Copy)]
pub enum RollipopRuntimeIdMode {
  Numeric,
  StableId,
}

#[derive(Clone, Copy)]
pub struct RollipopAstFinalizerContext<'me> {
  pub link_output: &'me LinkStageOutput,
  pub options: &'me SharedOptions,
  pub module: &'me NormalModule,
  pub unique_index: usize,
}

pub struct RollipopAstFinalizer<'me, 'ast> {
  ast_factory: AstFactory<'ast>,
  pub ctx: RollipopAstFinalizerContext<'me>,
  pub runtime_id_mode: RollipopRuntimeIdMode,

  import_bindings: FxHashMap<SymbolId, ImportBinding>,
  generated_static_import_infos: FxHashMap<ModuleIdx, String>,
  generated_imports: FxHashSet<ModuleIdx>,
  exports: oxc::allocator::Vec<'ast, ObjectPropertyKind<'ast>>,
  named_exports: FxHashMap<oxc::ast::ast::Str<'ast>, NamedExport>,
  uses_import_meta_hot: bool,
  is_dev_mode: bool,
  is_runtime_module: bool,
  renamed_factory_param_bindings: FxHashMap<SymbolId, String>,
}

impl<'me, 'ast> RollipopAstFinalizer<'me, 'ast> {
  pub fn new(ast_builder: AstBuilder<'ast>, ctx: RollipopAstFinalizerContext<'me>) -> Self {
    let runtime_id_mode = if ctx.options.profiler_names {
      RollipopRuntimeIdMode::StableId
    } else {
      RollipopRuntimeIdMode::Numeric
    };
    let is_dev_mode = ctx.options.is_dev_mode_enabled();
    let is_runtime_module = ctx.link_output.runtime.id() == ctx.module.idx;
    let exports = oxc::allocator::Vec::new_in(&ast_builder);
    let ast_factory = AstFactory { ast_builder };
    Self {
      ast_factory,
      ctx,
      runtime_id_mode,
      import_bindings: FxHashMap::default(),
      generated_static_import_infos: FxHashMap::default(),
      generated_imports: FxHashSet::default(),
      exports,
      named_exports: FxHashMap::default(),
      uses_import_meta_hot: false,
      is_dev_mode,
      is_runtime_module,
      renamed_factory_param_bindings: FxHashMap::default(),
    }
  }

  fn modules(&self) -> &'me IndexModules {
    &self.ctx.link_output.module_table.modules
  }

  fn metas(&self) -> &'me LinkingMetadataVec {
    &self.ctx.link_output.metas
  }

  fn linking_info(&self) -> &'me LinkingMetadata {
    &self.ctx.link_output.metas[self.ctx.module.idx]
  }

  fn stmt_infos(&self) -> &'me StmtInfos {
    &self.ctx.link_output.stmt_infos[self.ctx.module.idx]
  }

  fn symbol_db(&self) -> &'me SymbolRefDb {
    &self.ctx.link_output.symbol_db
  }

  fn runtime_id_expr_for(&self, module: &Module) -> Expression<'ast> {
    if matches!(self.runtime_id_mode, RollipopRuntimeIdMode::StableId) {
      Expression::new_string_literal(
        SPAN,
        ast::Str::from_str_in(module.stable_id().as_str(), &self.ast_factory),
        None,
        &self.ast_factory,
      )
    } else {
      Expression::new_numeric_literal(
        SPAN,
        f64::from(module.idx().raw()),
        None,
        ast::NumberBase::Decimal,
        &self.ast_factory,
      )
    }
  }

  fn binding_name_for_import(&mut self, target_idx: ModuleIdx, rec_id: ImportRecordIdx) -> &str {
    let modules = self.modules();
    let unique_index = self.ctx.unique_index;
    self.generated_static_import_infos.entry(target_idx).or_insert_with(|| {
      let importee = &modules[target_idx];
      // A removed barrel can resolve one import record to multiple same-named modules.
      format!(
        "import_{}_{}_{}_{}",
        importee.repr_name(),
        unique_index,
        rec_id.raw(),
        target_idx.raw()
      )
    })
  }

  fn require_call_for_module(&self, importee: &Module) -> Expression<'ast> {
    let callee = self.ast_factory.make_id_ref_expr(SPAN, ROLLIPOP_REQUIRE_NAME);
    self.ast_factory.make_call_with_arg(callee, self.runtime_id_expr_for(importee), false)
  }

  fn require_call_for_external(&self, importee: &ExternalModule) -> Expression<'ast> {
    let callee = self.ast_factory.make_member_access_expr(ROLLIPOP_REQUIRE_NAME, "e");
    self.ast_factory.make_call_with_arg(
      callee,
      Expression::new_string_literal(
        SPAN,
        ast::Str::from_str_in(importee.id.as_str(), &self.ast_factory),
        None,
        &self.ast_factory,
      ),
      false,
    )
  }

  fn to_esm_expr(&self, expr: Expression<'ast>, interop: Option<Interop>) -> Expression<'ast> {
    self.ast_factory.make_to_esm_call_with_interop("__rollipop_require__.t", expr, interop)
  }

  fn create_import_binding_stmt(
    &mut self,
    importee: &Module,
    binding_name: &str,
  ) -> Option<Statement<'ast>> {
    if !self.generated_imports.insert(importee.idx()) {
      return None;
    }

    let (require_expr, interop) = match importee {
      Module::Normal(importee) => (
        self.require_call_for_module(&self.modules()[importee.idx]),
        self.ctx.module.interop(importee),
      ),
      Module::External(importee) => {
        (self.require_call_for_external(importee), Some(Interop::Babel))
      }
    };

    Some(self.ast_factory.make_var_decl(binding_name, self.to_esm_expr(require_expr, interop)))
  }

  fn import_binding_for_imported(&self, binding_name: &str, imported: &Specifier) -> ImportBinding {
    ImportBinding { binding_name: binding_name.to_string(), imported: imported.clone() }
  }

  fn should_require_importee(&self, importee_idx: ModuleIdx) -> bool {
    self.modules()[importee_idx].as_external().is_some() || self.metas()[importee_idx].is_included
  }

  fn export_name_for_canonical_ref(
    &self,
    module_idx: ModuleIdx,
    canonical_ref: SymbolRef,
    fallback: &Specifier,
  ) -> Specifier {
    if self.modules()[module_idx]
      .as_normal()
      .is_some_and(|module| module.namespace_object_ref == canonical_ref)
    {
      return Specifier::Star;
    }

    for (exported, resolved_export) in self.metas()[module_idx].canonical_exports(true) {
      if self.symbol_db().canonical_ref_for(resolved_export.symbol_ref) == canonical_ref {
        return Specifier::Literal(exported.clone());
      }
    }

    fallback.clone()
  }

  fn create_import_binding_for_module(
    &mut self,
    module_idx: ModuleIdx,
    rec_id: ImportRecordIdx,
    imported: &Specifier,
    span: Span,
  ) -> (ImportBinding, Option<Statement<'ast>>) {
    let binding_name = self.binding_name_for_import(module_idx, rec_id).to_string();
    let stmt = self.create_import_binding_stmt(&self.modules()[module_idx], &binding_name).map(
      |mut stmt| {
        *stmt.span_mut() = span;
        stmt
      },
    );
    (self.import_binding_for_imported(&binding_name, imported), stmt)
  }

  fn create_import_binding_for_import_specifier(
    &mut self,
    rec_id: ImportRecordIdx,
    local_symbol_id: SymbolId,
    imported: &Specifier,
    span: Span,
  ) -> Option<(ImportBinding, Option<Statement<'ast>>)> {
    let rec = &self.ctx.module.import_records[rec_id];
    let importee_idx = rec.resolved_module?;
    let (target_idx, target_imported) = if self.should_require_importee(importee_idx) {
      (importee_idx, imported.clone())
    } else {
      let local_ref = SymbolRef::from((self.ctx.module.idx, local_symbol_id));
      let canonical_ref = self.symbol_db().canonical_ref_for(local_ref);
      (
        canonical_ref.owner,
        self.export_name_for_canonical_ref(canonical_ref.owner, canonical_ref, imported),
      )
    };

    if !self.should_require_importee(target_idx) {
      return None;
    }
    Some(self.create_import_binding_for_module(target_idx, rec_id, &target_imported, span))
  }

  fn create_import_binding_for_re_export(
    &mut self,
    rec_id: ImportRecordIdx,
    imported: &Specifier,
    span: Span,
  ) -> Option<(ImportBinding, Option<Statement<'ast>>)> {
    let rec = &self.ctx.module.import_records[rec_id];
    let importee_idx = rec.resolved_module?;
    let (target_idx, target_imported) = if self.should_require_importee(importee_idx) {
      (importee_idx, imported.clone())
    } else {
      let Specifier::Literal(imported_name) = imported else {
        return None;
      };
      let resolved_export = self.metas()[importee_idx].resolved_exports.get(imported_name)?;
      let canonical_ref = self.symbol_db().canonical_ref_for(resolved_export.symbol_ref);
      (
        canonical_ref.owner,
        self.export_name_for_canonical_ref(canonical_ref.owner, canonical_ref, imported),
      )
    };

    if !self.should_require_importee(target_idx) {
      return None;
    }
    Some(self.create_import_binding_for_module(target_idx, rec_id, &target_imported, span))
  }

  fn inline_import_access_for_symbol_ref(
    &self,
    symbol_ref: SymbolRef,
    span: Span,
  ) -> Expression<'ast> {
    let canonical_ref = self.symbol_db().canonical_ref_for(symbol_ref);
    let module = &self.modules()[canonical_ref.owner];
    let fallback = Specifier::Literal(canonical_ref.name(self.symbol_db()).into());
    let imported =
      self.export_name_for_canonical_ref(canonical_ref.owner, canonical_ref, &fallback);
    let (require_expr, interop) = match module {
      Module::Normal(importee) => (
        self.require_call_for_module(&self.modules()[importee.idx]),
        self.ctx.module.interop(importee),
      ),
      Module::External(importee) => {
        (self.require_call_for_external(importee), Some(Interop::Babel))
      }
    };
    let mut expr = make_import_access_expr_for_object(
      &self.ast_factory,
      self.to_esm_expr(require_expr, interop),
      &imported,
    );
    *expr.span_mut() = span;
    expr
  }

  fn rewrite_resolved_member_expr(
    &self,
    node: &Expression<'ast>,
    scoping: &Scoping,
  ) -> Option<Expression<'ast>> {
    let (node_id, span) = match node {
      Expression::StaticMemberExpression(expr) => (expr.node_id(), expr.span),
      Expression::ComputedMemberExpression(expr) => (expr.node_id(), expr.span),
      _ => return None,
    };
    let resolution = self.linking_info().resolved_member_expr_refs.get(&node_id)?;
    // JSON default properties can resolve to individual exports after the default object is tree-shaken.
    // Those reads must use the linker's surviving symbol.
    let is_json_resolution = resolution.resolved.is_some_and(|symbol_ref| {
      self.modules()[symbol_ref.owner]
        .as_normal()
        .is_some_and(|module| matches!(module.module_type, ModuleType::Json))
    });
    if let Some(reference_id) = resolution.reference_id
      && let Some(symbol_id) = scoping.get_reference(reference_id).symbol_id()
      && self.import_bindings.contains_key(&symbol_id)
      && self.should_require_importee(
        self.symbol_db().canonical_ref_for((self.ctx.module.idx, symbol_id).into()).owner,
      )
      && !is_json_resolution
    {
      return None;
    }
    let Some(resolved_ref) = resolution.resolved else {
      return Some(
        self
          .ast_factory
          .make_member_expr_with_void_zero_object(&resolution.prop_and_related_span_list, span),
      );
    };
    let base = self.inline_import_access_for_symbol_ref(resolved_ref, span);
    Some(self.ast_factory.make_member_expr_or_ident_ref(
      base,
      &resolution.prop_and_related_span_list,
      span,
    ))
  }

  fn should_include_top_level_stmt(&self, stmt_info_idx: StmtInfoIdx) -> bool {
    self.is_runtime_module || self.linking_info().stmt_info_included.has_bit(stmt_info_idx)
  }

  fn collect_factory_param_binding_renames(&mut self, scoping: &Scoping) {
    let mut used_names =
      FACTORY_PARAM_NAMES.iter().map(std::string::ToString::to_string).collect::<FxHashSet<_>>();
    for (name, _) in scoping.iter_bindings().flat_map(|(_, bindings)| bindings) {
      used_names.insert(name.to_string());
    }

    for (name, symbol_id) in scoping.iter_bindings().flat_map(|(_, bindings)| bindings) {
      if self.is_runtime_module && name == HMR_RUNTIME_NAME {
        continue;
      }
      if !FACTORY_PARAM_NAMES.contains(&name.as_str()) {
        continue;
      }

      for count in 1u32.. {
        let candidate = format!("{name}${count}");
        if used_names.insert(candidate.clone()) {
          self.renamed_factory_param_bindings.insert(*symbol_id, candidate);
          break;
        }
      }
    }
  }

  fn local_binding_name(&self, symbol_id: SymbolId, original_name: &str) -> String {
    self
      .renamed_factory_param_bindings
      .get(&symbol_id)
      .cloned()
      .unwrap_or_else(|| original_name.to_string())
  }

  fn generated_binding_name(&self, scoping: &Scoping, base: &str) -> String {
    let mut used_names =
      FACTORY_PARAM_NAMES.iter().map(std::string::ToString::to_string).collect::<FxHashSet<_>>();
    for (name, _) in scoping.iter_bindings().flat_map(|(_, bindings)| bindings) {
      used_names.insert(name.to_string());
    }
    used_names.extend(self.renamed_factory_param_bindings.values().cloned());

    if !used_names.contains(base) {
      return base.to_string();
    }
    for count in 1u32.. {
      let candidate = format!("{base}${count}");
      if !used_names.contains(&candidate) {
        return candidate;
      }
    }
    unreachable!("generated binding name should always find a free suffix");
  }

  fn ensure_import_for_record(
    &mut self,
    rec_id: ImportRecordIdx,
    span: Span,
  ) -> Option<(ModuleIdx, String, Option<Statement<'ast>>)> {
    let rec = &self.ctx.module.import_records[rec_id];
    let importee_idx = rec.resolved_module?;
    if !self.should_require_importee(importee_idx) {
      return None;
    }
    let binding_name = self.binding_name_for_import(importee_idx, rec_id).to_string();
    let importee = &self.modules()[importee_idx];
    let stmt = self.create_import_binding_stmt(importee, &binding_name).map(|mut stmt| {
      *stmt.span_mut() = span;
      stmt
    });
    Some((importee_idx, binding_name, stmt))
  }

  #[expect(clippy::too_many_lines)]
  fn handle_top_level_stmt(
    &mut self,
    program_body: &mut oxc::allocator::Vec<'ast, Statement<'ast>>,
    mut node: Statement<'ast>,
    scoping: &Scoping,
  ) {
    match node {
      ref mut module_decl @ ast::match_module_declaration!(Statement) => {
        let module_decl = module_decl.to_module_declaration_mut();
        match module_decl {
          ast::ModuleDeclaration::ImportDeclaration(import_decl) => {
            let rec_id = self.ctx.module.imports[&import_decl.node_id()];
            let rec = &self.ctx.module.import_records[rec_id];
            let Some(importee_idx) = rec.resolved_module else {
              return;
            };
            let binding_name = self.binding_name_for_import(importee_idx, rec_id).to_string();
            let mut needs_importee_binding =
              import_decl.specifiers.as_ref().is_none_or(|specifiers| specifiers.is_empty());
            if let Some(specifiers) = &import_decl.specifiers {
              for spec in specifiers {
                match spec {
                  ast::ImportDeclarationSpecifier::ImportSpecifier(import_specifier) => {
                    let imported = Specifier::Literal(import_specifier.imported.name().into());
                    if let Some((import_binding, stmt)) = self
                      .create_import_binding_for_import_specifier(
                        rec_id,
                        import_specifier.local.symbol_id(),
                        &imported,
                        import_decl.span,
                      )
                    {
                      self
                        .import_bindings
                        .insert(import_specifier.local.symbol_id(), import_binding);
                      if let Some(stmt) = stmt {
                        program_body.push(stmt);
                      }
                    }
                  }
                  ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(default_specifier) => {
                    let imported = Specifier::Literal("default".into());
                    if let Some((import_binding, stmt)) = self
                      .create_import_binding_for_import_specifier(
                        rec_id,
                        default_specifier.local.symbol_id(),
                        &imported,
                        import_decl.span,
                      )
                    {
                      self
                        .import_bindings
                        .insert(default_specifier.local.symbol_id(), import_binding);
                      if let Some(stmt) = stmt {
                        program_body.push(stmt);
                      }
                    }
                  }
                  ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                    namespace_specifier,
                  ) => {
                    if self.should_require_importee(importee_idx) {
                      needs_importee_binding = true;
                      self.import_bindings.insert(
                        namespace_specifier.local.symbol_id(),
                        ImportBinding {
                          binding_name: binding_name.clone(),
                          imported: Specifier::Star,
                        },
                      );
                    }
                  }
                }
              }
            }
            if needs_importee_binding
              && self.should_require_importee(importee_idx)
              && let Some(stmt) =
                self.create_import_binding_stmt(&self.modules()[importee_idx], &binding_name)
            {
              program_body.push(stmt);
            }
          }
          ast::ModuleDeclaration::ExportFromDeclaration(decl) => {
            let rec_id = self.ctx.module.imports[&decl.node_id()];
            if decl.specifiers.is_empty() {
              let rec = &self.ctx.module.import_records[rec_id];
              let Some(importee_idx) = rec.resolved_module else {
                return;
              };
              if self.should_require_importee(importee_idx) {
                let binding_name = self.binding_name_for_import(importee_idx, rec_id).to_string();
                if let Some(stmt) =
                  self.create_import_binding_stmt(&self.modules()[importee_idx], &binding_name)
                {
                  program_body.push(stmt);
                }
              }
              return;
            }
            let mut props =
              oxc::allocator::Vec::with_capacity_in(decl.specifiers.len(), &self.ast_factory);
            for specifier in &decl.specifiers {
              let local = match &specifier.local {
                ast::ModuleExportName::IdentifierName(ident) => {
                  Specifier::Literal(ident.name.into())
                }
                ast::ModuleExportName::StringLiteral(str) => Specifier::Literal(str.value.into()),
                ast::ModuleExportName::IdentifierReference(_) => {
                  unreachable!(
                    "IdentifierReference is invalid in re-exported ExportFromDeclaration"
                  )
                }
              };
              let exported = specifier.exported.name();
              let Some((import_binding, stmt)) =
                self.create_import_binding_for_re_export(rec_id, &local, decl.span)
              else {
                continue;
              };
              if let Some(stmt) = stmt {
                program_body.push(stmt);
              }
              props.push(self.ast_factory.make_lazy_export_property(
                &exported,
                import_binding.to_expression(&self.ast_factory),
                !is_validate_identifier_name(&exported),
              ));
            }
            self.exports.extend(props);
          }
          ast::ModuleDeclaration::ExportDeclaration(decl) => {
            match &mut decl.declaration {
              ast::Declaration::VariableDeclaration(var_decl) => {
                for decl in &var_decl.declarations {
                  for ident in decl.id.get_binding_identifiers() {
                    let name = self.local_binding_name(ident.symbol_id(), ident.name.as_str());
                    self.exports.push(self.ast_factory.make_lazy_export_property(
                      ident.name.as_str(),
                      self.ast_factory.make_id_ref_expr(SPAN, &name),
                      false,
                    ));
                  }
                }
              }
              ast::Declaration::FunctionDeclaration(fn_decl) => {
                let ident = fn_decl.id.as_ref().expect("exported function should have id");
                let id = self.local_binding_name(ident.symbol_id(), ident.name.as_str());
                self.exports.push(self.ast_factory.make_lazy_export_property(
                  ident.name.as_str(),
                  self.ast_factory.make_id_ref_expr(SPAN, &id),
                  false,
                ));
              }
              ast::Declaration::ClassDeclaration(cls_decl) => {
                let ident = cls_decl.id.as_ref().expect("exported class should have id");
                let id = self.local_binding_name(ident.symbol_id(), ident.name.as_str());
                self.exports.push(self.ast_factory.make_lazy_export_property(
                  ident.name.as_str(),
                  self.ast_factory.make_id_ref_expr(SPAN, &id),
                  false,
                ));
              }
              _ => {}
            }
            program_body.push(Statement::from(decl.declaration.take_in(&self.ast_factory)));
          }
          ast::ModuleDeclaration::ExportNamedDeclaration(decl) => {
            for specifier in &decl.specifiers {
              if let Some(symbol_id) = scoping.get_root_binding(specifier.local.name().into()) {
                self
                  .named_exports
                  .insert(specifier.exported.name(), NamedExport { local_binding: symbol_id });
              }
            }
          }
          ast::ModuleDeclaration::ExportDefaultDeclaration(decl) => match &mut decl.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
              let id = if let Some(id) = &function.id {
                self.local_binding_name(id.symbol_id(), id.name.as_str())
              } else {
                let generated = self.generated_binding_name(scoping, "__default");
                function.id = Some(self.ast_factory.make_id(SPAN, &generated));
                generated
              };
              self.exports.push(self.ast_factory.make_lazy_export_property(
                "default",
                self.ast_factory.make_id_ref_expr(SPAN, &id),
                false,
              ));
              program_body.push(Statement::FunctionDeclaration(ArenaBox::new_in(
                function.as_mut().take_in(&self.ast_factory),
                &self.ast_factory,
              )));
            }
            ExportDefaultDeclarationKind::ClassDeclaration(class) => {
              let id = if let Some(id) = &class.id {
                self.local_binding_name(id.symbol_id(), id.name.as_str())
              } else {
                let generated = self.generated_binding_name(scoping, "__default");
                class.id = Some(self.ast_factory.make_id(SPAN, &generated));
                generated
              };
              self.exports.push(self.ast_factory.make_lazy_export_property(
                "default",
                self.ast_factory.make_id_ref_expr(SPAN, &id),
                false,
              ));
              program_body.push(Statement::ClassDeclaration(ArenaBox::new_in(
                class.as_mut().take_in(&self.ast_factory),
                &self.ast_factory,
              )));
            }
            expr @ ast::match_expression!(ExportDefaultDeclarationKind) => {
              let name = self.generated_binding_name(scoping, "__default");
              program_body.push(
                self
                  .ast_factory
                  .make_var_decl(&name, expr.to_expression_mut().take_in(&self.ast_factory)),
              );
              self.exports.push(self.ast_factory.make_lazy_export_property(
                "default",
                self.ast_factory.make_id_ref_expr(SPAN, &name),
                false,
              ));
            }
            _ => {}
          },
          ast::ModuleDeclaration::ExportAllDeclaration(export_all_decl) => {
            self.handle_export_all_declaration(program_body, export_all_decl);
          }
          ast::ModuleDeclaration::TSExportAssignment(_)
          | ast::ModuleDeclaration::TSNamespaceExportDeclaration(_) => program_body.push(node),
        }
      }
      _ => program_body.push(node),
    }
  }

  fn handle_export_all_declaration(
    &mut self,
    program_body: &mut oxc::allocator::Vec<'ast, Statement<'ast>>,
    export_all_decl: &ast::ExportAllDeclaration<'ast>,
  ) {
    let rec_id = self.ctx.module.imports[&export_all_decl.node_id()];
    let Some((_importee_idx, binding_name, stmt)) =
      self.ensure_import_for_record(rec_id, export_all_decl.span)
    else {
      return;
    };
    if let Some(stmt) = stmt {
      program_body.push(stmt);
    }
    if let Some(exported) = &export_all_decl.exported {
      let exported = exported.name();
      self.exports.push(self.ast_factory.make_lazy_export_property(
        &exported,
        self.ast_factory.make_id_ref_expr(SPAN, &binding_name),
        !is_validate_identifier_name(&exported),
      ));
    } else {
      program_body.push(self.create_re_export_all_stmt(&binding_name, export_all_decl.span));
    }
  }

  fn should_include_static_import_for_runtime_execution(&self, stmt: &Statement<'_>) -> bool {
    let Statement::ImportDeclaration(import_decl) = stmt else {
      return false;
    };
    let rec_id = self.ctx.module.imports[&import_decl.node_id()];
    let rec = &self.ctx.module.import_records[rec_id];
    rec.resolved_module.is_some_and(|importee_idx| self.metas()[importee_idx].is_included)
  }

  fn should_include_re_export_for_runtime_execution(&self, stmt: &Statement<'_>) -> bool {
    let rec_id = match stmt {
      Statement::ExportFromDeclaration(decl) => self.ctx.module.imports[&decl.node_id()],
      Statement::ExportAllDeclaration(decl) => self.ctx.module.imports[&decl.node_id()],
      _ => return false,
    };
    let rec = &self.ctx.module.import_records[rec_id];
    rec.resolved_module.is_some_and(|importee_idx| self.metas()[importee_idx].is_included)
  }

  fn create_re_export_all_stmt(&self, binding_name: &str, span: Span) -> Statement<'ast> {
    let call = Expression::new_call_expression(
      span,
      self.ast_factory.make_member_access_expr(ROLLIPOP_REQUIRE_NAME, "re"),
      None,
      oxc::allocator::Vec::from_array_in(
        [
          ast::Argument::from(self.ast_factory.make_id_ref_expr(SPAN, ROLLIPOP_EXPORTS_NAME)),
          ast::Argument::from(self.ast_factory.make_id_ref_expr(SPAN, binding_name)),
        ],
        &self.ast_factory,
      ),
      false,
      &self.ast_factory,
    );
    Statement::new_expression_statement(span, call, &self.ast_factory)
  }

  fn create_mark_esm_stmt(&self) -> Statement<'ast> {
    let call = Expression::new_call_expression(
      SPAN,
      self.ast_factory.make_member_access_expr(ROLLIPOP_REQUIRE_NAME, "r"),
      None,
      oxc::allocator::Vec::from_value_in(
        ast::Argument::from(self.ast_factory.make_id_ref_expr(SPAN, ROLLIPOP_EXPORTS_NAME)),
        &self.ast_factory,
      ),
      false,
      &self.ast_factory,
    );
    Statement::new_expression_statement(SPAN, call, &self.ast_factory)
  }

  fn create_define_exports_stmt(&mut self, scoping: &Scoping) -> Option<Statement<'ast>> {
    for (exported, named_export) in &self.named_exports {
      let expr = if let Some(import_binding) = self.import_bindings.get(&named_export.local_binding)
      {
        import_binding.to_expression(&self.ast_factory)
      } else {
        let name = scoping.symbol_name(named_export.local_binding);
        let name = self.local_binding_name(named_export.local_binding, name);
        self.ast_factory.make_id_ref_expr(SPAN, &name)
      };
      let prop = self.ast_factory.make_lazy_export_property(
        exported,
        expr,
        !is_validate_identifier_name(exported.as_str()),
      );
      self.exports.push(prop);
    }
    self.add_json_metadata_exports();
    self.add_missing_re_exports();

    if self.exports.is_empty() {
      return None;
    }

    let mut obj = ast::ObjectExpression::boxed(
      SPAN,
      oxc::allocator::Vec::with_capacity_in(self.exports.len(), &self.ast_factory),
      &self.ast_factory,
    );
    obj.properties.extend(self.exports.drain(..));
    let call = Expression::new_call_expression(
      SPAN,
      self.ast_factory.make_member_access_expr(ROLLIPOP_REQUIRE_NAME, "d"),
      None,
      oxc::allocator::Vec::from_array_in(
        [
          ast::Argument::from(self.ast_factory.make_id_ref_expr(SPAN, ROLLIPOP_EXPORTS_NAME)),
          ast::Argument::ObjectExpression(obj),
        ],
        &self.ast_factory,
      ),
      false,
      &self.ast_factory,
    );
    Some(Statement::new_expression_statement(SPAN, call, &self.ast_factory))
  }

  fn add_json_metadata_exports(&mut self) {
    if !matches!(self.ctx.module.module_type, ModuleType::Json) {
      return;
    }

    for (exported, resolved_export) in self.linking_info().canonical_exports(true) {
      if exported == "default" {
        continue;
      }
      let canonical_ref = self.symbol_db().canonical_ref_for(resolved_export.symbol_ref);
      if canonical_ref.owner != self.ctx.module.idx {
        continue;
      }
      if !self
        .stmt_infos()
        .declared_stmts_by_symbol(&canonical_ref)
        .iter()
        .any(|stmt_info_idx| self.should_include_top_level_stmt(*stmt_info_idx))
      {
        continue;
      }
      let name = canonical_ref.name(self.symbol_db());
      self.exports.push(self.ast_factory.make_lazy_export_property(
        exported,
        self.ast_factory.make_id_ref_expr(SPAN, name),
        !is_validate_identifier_name(exported.as_str()),
      ));
    }
  }

  fn add_missing_re_exports(&mut self) {
    let existing_exports = self
      .exports
      .iter()
      .filter_map(|property| match property {
        ObjectPropertyKind::ObjectProperty(property) => {
          property.key.static_name().map(std::borrow::Cow::into_owned)
        }
        ObjectPropertyKind::SpreadProperty(_) => None,
      })
      .collect::<FxHashSet<_>>();
    // A retained barrel can forward through a removed barrel. Its declarations
    // no longer provide getters, but the live canonical exports still must.
    for (exported, resolved_export) in self.linking_info().canonical_exports(true) {
      if existing_exports.contains(exported.as_str()) {
        continue;
      }
      let canonical_ref = self.symbol_db().canonical_ref_for(resolved_export.symbol_ref);
      if canonical_ref.owner == self.ctx.module.idx
        || !self.ctx.link_output.retained_export_symbols.contains(&canonical_ref)
        || !self.should_require_importee(canonical_ref.owner)
      {
        continue;
      }
      let expr = self.inline_import_access_for_symbol_ref(canonical_ref, SPAN);
      self.exports.push(self.ast_factory.make_lazy_export_property(
        exported,
        expr,
        !is_validate_identifier_name(exported.as_str()),
      ));
    }
  }

  fn hot_context_name(&self) -> String {
    format!("hot_{}", self.ctx.module.repr_name)
  }

  fn rewrite_hot_accept_call_deps(&self, call_expr: &mut ast::CallExpression<'ast>) {
    if !call_expr.callee.is_import_meta_hot_accept() || call_expr.arguments.is_empty() {
      return;
    }
    match &mut call_expr.arguments[0] {
      ast::Argument::StringLiteral(lit) => {
        let Some(rec_idx) =
          self.ctx.module.hmr_info.module_request_to_import_record_idx.get(lit.value.as_str())
        else {
          return;
        };
        let Some(module_idx) = self.ctx.module.import_records[*rec_idx].resolved_module else {
          return;
        };
        lit.value =
          ast::Str::from_str_in(self.modules()[module_idx].stable_id(), &self.ast_factory);
      }
      ast::Argument::ArrayExpression(array) => {
        for element in &mut array.elements {
          if let ast::ArrayExpressionElement::StringLiteral(lit) = element {
            let Some(rec_idx) =
              self.ctx.module.hmr_info.module_request_to_import_record_idx.get(lit.value.as_str())
            else {
              continue;
            };
            let Some(module_idx) = self.ctx.module.import_records[*rec_idx].resolved_module else {
              continue;
            };
            lit.value =
              ast::Str::from_str_in(self.modules()[module_idx].stable_id(), &self.ast_factory);
          }
        }
      }
      _ => {}
    }
  }

  fn rewrite_dynamic_import(&self, node: &mut Expression<'ast>) {
    let Expression::ImportExpression(import_expr) = node else { return };
    let Some(rec_idx) = self.ctx.module.imports.get(&import_expr.node_id()) else { return };
    let rec = &self.ctx.module.import_records[*rec_idx];
    let Some(importee_idx) = rec.resolved_module else { return };
    let importee = &self.modules()[importee_idx];
    let (require_expr, needs_interop) = match importee {
      Module::Normal(module) => {
        (self.require_call_for_module(importee), module.exports_kind.is_commonjs())
      }
      Module::External(importee) => (self.require_call_for_external(importee), true),
    };
    let interop = needs_interop.then(|| {
      if self.ctx.module.should_consider_node_esm_spec_for_dynamic_import() {
        Interop::Node
      } else {
        Interop::Babel
      }
    });
    *node = self.ast_factory.make_promise_resolve_then(self.to_esm_expr(require_expr, interop));
  }

  fn rewrite_require(
    &self,
    node: &mut Expression<'ast>,
    ctx: &oxc_traverse::TraverseCtx<'ast, ()>,
  ) {
    let scoping = ctx.scoping();
    if let Some(id_ref) = node.as_identifier()
      && id_ref.name == CJS_REQUIRE_REF_STR
      && id_ref.is_global_reference(scoping)
      && !ctx.parent().is_call_expression()
    {
      *node = self.ast_factory.make_id_ref_expr(SPAN, ROLLIPOP_REQUIRE_NAME);
      return;
    }

    let Expression::CallExpression(call_expr) = node else { return };
    if !call_expr
      .callee
      .as_identifier()
      .is_some_and(|id| id.name == CJS_REQUIRE_REF_STR && id.is_global_reference(scoping))
    {
      return;
    }
    let Some(rec_idx) = self.ctx.module.imports.get(&call_expr.node_id()) else { return };
    let rec = &self.ctx.module.import_records[*rec_idx];
    let Some(importee_idx) = rec.resolved_module else { return };
    let importee = &self.modules()[importee_idx];
    *node = match importee {
      Module::Normal(_) => self.require_call_for_module(importee),
      Module::External(importee) => self.require_call_for_external(importee),
    };
  }

  fn rewrite_import_meta_hot(&mut self, node: &mut Expression<'ast>) {
    if node.is_import_meta_hot() {
      if self.is_dev_mode {
        self.uses_import_meta_hot = true;
        *node = self.ast_factory.make_id_ref_expr(SPAN, &self.hot_context_name());
      } else {
        *node = Expression::new_void_0(SPAN, &self.ast_factory);
      }
    }
  }
}

impl<'me, 'ast> HmrAstBuilder<'me, 'ast> for RollipopAstFinalizer<'me, 'ast> {
  fn builder(&self) -> AstBuilder<'ast> {
    AstBuilder::new(self.ast_factory.allocator())
  }

  fn module(&self) -> &NormalModule {
    self.ctx.module
  }

  fn binding_name_for_namespace_object_ref_atom(&self) -> ast::Str<'ast> {
    ast::Str::from_str_in(ROLLIPOP_EXPORTS_NAME, &self.ast_factory)
  }

  fn alias_name_for_import_meta_hot(&self) -> ast::Str<'ast> {
    ast::Str::from_str_in(&self.hot_context_name(), &self.ast_factory)
  }

  fn cjs_module_name() -> &'static str {
    ROLLIPOP_MODULE_NAME
  }
}

impl<'ast> Traverse<'ast, ()> for RollipopAstFinalizer<'_, 'ast> {
  fn enter_program(
    &mut self,
    node: &mut ast::Program<'ast>,
    ctx: &mut oxc_traverse::TraverseCtx<'ast, ()>,
  ) {
    self.collect_factory_param_binding_renames(ctx.scoping());
    // The chunk renderer emits the entry hashbang outside all module factories.
    node.hashbang.take();
    match self.ctx.options.strict {
      StrictMode::Always if !node.has_use_strict_directive() => {
        node.directives.insert(
          0,
          ast::Directive::new(
            SPAN,
            ast::StringLiteral::new(SPAN, "use strict", None, &self.ast_factory),
            "use strict",
            &self.ast_factory,
          ),
        );
      }
      StrictMode::Never => node.directives.retain(|directive| !directive.is_use_strict()),
      _ => {}
    }

    let body = node.body.take_in(&self.ast_factory);
    node.body.reserve_exact(body.len() + 3);
    if self.is_runtime_module {
      for stmt in body {
        self.handle_top_level_stmt(&mut node.body, stmt, ctx.scoping());
      }
    } else {
      for (stmt, (stmt_info_idx, _stmt_info)) in
        body.into_iter().zip(self.stmt_infos().iter_enumerated().skip(1))
      {
        if self.should_include_top_level_stmt(stmt_info_idx)
          || is_export_specifier_declaration(&stmt)
          || self.should_include_static_import_for_runtime_execution(&stmt)
          || self.should_include_re_export_for_runtime_execution(&stmt)
        {
          self.handle_top_level_stmt(&mut node.body, stmt, ctx.scoping());
        }
      }
    }
  }

  fn exit_program(
    &mut self,
    node: &mut ast::Program<'ast>,
    ctx: &mut oxc_traverse::TraverseCtx<'ast, ()>,
  ) {
    let body = node.body.take_in(&self.ast_factory);
    let mut next_body = oxc::allocator::Vec::with_capacity_in(body.len() + 3, &self.ast_factory);
    if self.ctx.module.exports_kind.is_esm() && !self.is_runtime_module {
      next_body.push(self.create_mark_esm_stmt());
      if let Some(stmt) = self.create_define_exports_stmt(ctx.scoping()) {
        next_body.push(stmt);
      }
    }
    if self.uses_import_meta_hot && self.is_dev_mode {
      next_body.push(self.create_module_hot_context_initializer_stmt());
    }
    if self.is_dev_mode && !self.is_runtime_module {
      next_body.push(self.create_register_module_stmt());
    }
    next_body.extend(body);
    node.body = next_body;
  }

  fn enter_call_expression(
    &mut self,
    node: &mut ast::CallExpression<'ast>,
    _ctx: &mut oxc_traverse::TraverseCtx<'ast, ()>,
  ) {
    self.rewrite_hot_accept_call_deps(node);
  }

  fn exit_expression(
    &mut self,
    node: &mut Expression<'ast>,
    ctx: &mut oxc_traverse::TraverseCtx<'ast, ()>,
  ) {
    if let Some(expr) = self.rewrite_resolved_member_expr(node, ctx.scoping()) {
      *node = expr;
      return;
    }

    if let Expression::Identifier(ident) = node
      && let Some(reference_id) = ident.reference_id.get()
      && let Some(symbol_id) = ctx.scoping().get_reference(reference_id).symbol_id()
      && let Some(import_binding) = self.import_bindings.get(&symbol_id)
    {
      let mut expr = import_binding.to_expression(&self.ast_factory);
      // Imported bindings are bare values, not methods of the generated namespace.
      let parent = ctx
        .ancestors()
        .find(|ancestor| !matches!(ancestor, Ancestor::ParenthesizedExpressionExpression(_)));
      if matches!(
        parent,
        Some(Ancestor::CallExpressionCallee(_) | Ancestor::TaggedTemplateExpressionTag(_))
      ) && !matches!(import_binding.imported, Specifier::Star)
      {
        expr = Expression::new_seq_in_parens(
          Expression::new_numeric_literal(
            SPAN,
            0.0,
            None,
            ast::NumberBase::Decimal,
            &self.ast_factory,
          ),
          expr,
          &self.ast_factory,
        );
      }
      *node = expr;
      return;
    }

    self.rewrite_dynamic_import(node);
    self.rewrite_require(node, ctx);
  }

  fn enter_expression(
    &mut self,
    node: &mut Expression<'ast>,
    _ctx: &mut oxc_traverse::TraverseCtx<'ast, ()>,
  ) {
    // Handle hot before visiting its import.meta object.
    self.rewrite_import_meta_hot(node);
    match node {
      Expression::ImportMeta(_) => {
        *node = Expression::new_object_expression(SPAN, [], &self.ast_factory);
      }
      Expression::ThisExpression(expr) => {
        if let Some(kind) = self.ctx.module.this_expr_replace_map.get(&expr.node_id()) {
          *node = match kind {
            ThisExprReplaceKind::Exports => {
              self.ast_factory.make_id_ref_expr(SPAN, ROLLIPOP_EXPORTS_NAME)
            }
            ThisExprReplaceKind::Context if self.ctx.options.context.is_empty() => {
              Expression::new_void_0(SPAN, &self.ast_factory)
            }
            ThisExprReplaceKind::Context => {
              self.ast_factory.make_id_ref_expr(SPAN, self.ctx.options.context.as_str())
            }
          };
        }
      }
      _ => {}
    }
  }

  fn exit_identifier_reference(
    &mut self,
    ident: &mut ast::IdentifierReference<'ast>,
    ctx: &mut oxc_traverse::TraverseCtx<'ast, ()>,
  ) {
    if ident.name == "exports" && ident.is_global_reference(ctx.scoping()) {
      ident.name = ast::Str::from_str_in(ROLLIPOP_EXPORTS_NAME, &self.ast_factory).into();
      return;
    }

    let Some(reference_id) = ident.reference_id.get() else { return };
    let reference = ctx.scoping().get_reference(reference_id);
    let Some(symbol_id) = reference.symbol_id() else { return };
    if let Some(binding_name) = self.renamed_factory_param_bindings.get(&symbol_id) {
      ident.name = ast::Str::from_str_in(binding_name, &self.ast_factory).into();
    }
  }

  fn exit_binding_identifier(
    &mut self,
    ident: &mut ast::BindingIdentifier<'ast>,
    _ctx: &mut oxc_traverse::TraverseCtx<'ast, ()>,
  ) {
    if let Some(symbol_id) = ident.symbol_id.get()
      && let Some(binding_name) = self.renamed_factory_param_bindings.get(&symbol_id)
    {
      ident.name = ast::Str::from_str_in(binding_name, &self.ast_factory).into();
    }
  }
}

struct NamedExport {
  local_binding: SymbolId,
}

struct ImportBinding {
  binding_name: String,
  imported: Specifier,
}

impl ImportBinding {
  fn to_expression<'ast>(&self, ast_factory: &AstFactory<'ast>) -> Expression<'ast> {
    make_import_access_expr(ast_factory, &self.binding_name, &self.imported)
  }
}

fn make_import_access_expr<'ast>(
  ast_factory: &AstFactory<'ast>,
  binding_name: &str,
  imported: &Specifier,
) -> Expression<'ast> {
  make_import_access_expr_for_object(
    ast_factory,
    ast_factory.make_id_ref_expr(SPAN, binding_name),
    imported,
  )
}

fn make_import_access_expr_for_object<'ast>(
  ast_factory: &AstFactory<'ast>,
  object: Expression<'ast>,
  imported: &Specifier,
) -> Expression<'ast> {
  match imported {
    Specifier::Star => object,
    Specifier::Literal(name) if is_validate_identifier_name(name.as_str()) => {
      Expression::StaticMemberExpression(ast::StaticMemberExpression::boxed(
        SPAN,
        object,
        ast_factory.make_id_name(SPAN, name.as_str()),
        false,
        ast_factory,
      ))
    }
    Specifier::Literal(name) => {
      Expression::ComputedMemberExpression(ast::ComputedMemberExpression::boxed(
        SPAN,
        object,
        Expression::new_string_literal(
          SPAN,
          ast::Str::from_str_in(name.as_str(), ast_factory),
          None,
          ast_factory,
        ),
        false,
        ast_factory,
      ))
    }
  }
}

fn is_export_specifier_declaration(stmt: &Statement<'_>) -> bool {
  matches!(stmt, Statement::ExportNamedDeclaration(_))
}
