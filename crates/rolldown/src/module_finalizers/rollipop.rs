use oxc::{
  allocator::{Box as ArenaBox, IntoIn, TakeIn},
  ast::{
    NONE,
    ast::{self, ExportDefaultDeclarationKind, Expression, ObjectPropertyKind, Statement},
  },
  semantic::{IsGlobalReference, Scoping, SymbolId},
  span::{GetSpanMut, SPAN, Span},
};
use oxc_traverse::Traverse;
use rolldown_common::{
  ExternalModule, ImportRecordIdx, IndexModules, Interop, Module, ModuleIdx, ModuleType,
  NormalModule, Specifier, StmtInfoIdx, StmtInfos, SymbolRefDb,
};
use rolldown_ecmascript::CJS_REQUIRE_REF_STR;
use rolldown_ecmascript_utils::{AstFactory, ExpressionExt};
use rolldown_utils::ecmascript::is_validate_identifier_name;
use rustc_hash::{FxHashMap, FxHashSet};

use crate::{
  hmr::utils::HmrAstBuilder,
  rollipop::{
    ROLLIPOP_EXPORTS_NAME, ROLLIPOP_GLOBAL_NAME, ROLLIPOP_MODULE_NAME, ROLLIPOP_REQUIRE_NAME,
  },
  types::linking_metadata::{LinkingMetadata, LinkingMetadataVec},
};

const FACTORY_PARAM_NAMES: [&str; 4] =
  [ROLLIPOP_GLOBAL_NAME, ROLLIPOP_MODULE_NAME, ROLLIPOP_EXPORTS_NAME, ROLLIPOP_REQUIRE_NAME];

#[derive(Clone, Copy)]
pub struct RollipopAstFinalizerParams<'me, 'ast> {
  pub ast_factory: AstFactory<'ast>,
  pub modules: &'me IndexModules,
  pub module: &'me NormalModule,
  pub metas: &'me LinkingMetadataVec,
  pub linking_info: &'me LinkingMetadata,
  pub stmt_infos: &'me StmtInfos,
  pub symbol_db: &'me SymbolRefDb,
  pub unique_index: usize,
  pub is_dev_mode: bool,
  pub is_runtime_module: bool,
}

pub struct RollipopAstFinalizer<'me, 'ast> {
  pub ast_factory: AstFactory<'ast>,
  pub modules: &'me IndexModules,
  pub module: &'me NormalModule,
  pub metas: &'me LinkingMetadataVec,
  pub linking_info: &'me LinkingMetadata,
  pub stmt_infos: &'me StmtInfos,
  pub symbol_db: &'me SymbolRefDb,
  pub unique_index: usize,

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
  pub fn new(params: RollipopAstFinalizerParams<'me, 'ast>) -> Self {
    let RollipopAstFinalizerParams {
      ast_factory,
      modules,
      module,
      metas,
      linking_info,
      stmt_infos,
      symbol_db,
      unique_index,
      is_dev_mode,
      is_runtime_module,
    } = params;

    Self {
      ast_factory,
      modules,
      module,
      metas,
      linking_info,
      stmt_infos,
      symbol_db,
      unique_index,
      import_bindings: FxHashMap::default(),
      generated_static_import_infos: FxHashMap::default(),
      generated_imports: FxHashSet::default(),
      exports: ast_factory.vec(),
      named_exports: FxHashMap::default(),
      uses_import_meta_hot: false,
      is_dev_mode,
      is_runtime_module,
      renamed_factory_param_bindings: FxHashMap::default(),
    }
  }

  fn runtime_id_for(&self, module: &Module) -> u32 {
    module.idx().raw()
  }

  fn binding_name_for_import(&mut self, importee_idx: ModuleIdx, rec_id: ImportRecordIdx) -> &str {
    self.generated_static_import_infos.entry(importee_idx).or_insert_with(|| {
      let importee = &self.modules[importee_idx];
      format!("import_{}_{}{}", importee.repr_name(), self.unique_index, rec_id.raw())
    })
  }

  fn require_call_for_module(&self, importee: &Module) -> Expression<'ast> {
    let callee = self.ast_factory.make_id_ref_expr(SPAN, ROLLIPOP_REQUIRE_NAME);
    self.ast_factory.make_call_with_arg(
      callee,
      self.ast_factory.expression_numeric_literal(
        SPAN,
        f64::from(self.runtime_id_for(importee)),
        None,
        oxc::ast::ast::NumberBase::Decimal,
      ),
      false,
    )
  }

  fn require_call_for_external(&self, importee: &ExternalModule) -> Expression<'ast> {
    let callee = self.ast_factory.make_member_access_expr(ROLLIPOP_REQUIRE_NAME, "e");
    self.ast_factory.make_call_with_arg(
      callee,
      self.ast_factory.expression_string_literal(
        SPAN,
        self.ast_factory.str(importee.id.as_str()),
        None,
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
      Module::Normal(importee) => {
        (self.require_call_for_module(&self.modules[importee.idx]), self.module.interop(importee))
      }
      Module::External(importee) => {
        (self.require_call_for_external(importee), Some(Interop::Babel))
      }
    };

    Some(self.ast_factory.make_var_decl(binding_name, self.to_esm_expr(require_expr, interop)))
  }

  fn import_binding_for_imported(&self, binding_name: &str, imported: &Specifier) -> ImportBinding {
    ImportBinding { binding_name: binding_name.to_string(), imported: imported.clone() }
  }

  fn should_include_top_level_stmt(&self, stmt_info_idx: StmtInfoIdx) -> bool {
    self.is_runtime_module || self.linking_info.stmt_info_included.has_bit(stmt_info_idx)
  }

  fn collect_factory_param_binding_renames(&mut self, scoping: &Scoping) {
    let mut used_names =
      FACTORY_PARAM_NAMES.iter().map(std::string::ToString::to_string).collect::<FxHashSet<_>>();
    for (name, _) in scoping.iter_bindings().flat_map(|(_, bindings)| bindings) {
      used_names.insert(name.to_string());
    }

    for (name, symbol_id) in scoping.iter_bindings().flat_map(|(_, bindings)| bindings) {
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

  fn ensure_import_for_record(
    &mut self,
    rec_id: ImportRecordIdx,
    span: Span,
  ) -> Option<(ModuleIdx, String, Option<Statement<'ast>>)> {
    let rec = &self.module.import_records[rec_id];
    let importee_idx = rec.resolved_module?;
    let binding_name = self.binding_name_for_import(importee_idx, rec_id).to_string();
    let importee = &self.modules[importee_idx];
    let stmt = self.create_import_binding_stmt(importee, &binding_name).map(|mut stmt| {
      *stmt.span_mut() = span;
      stmt
    });
    Some((importee_idx, binding_name, stmt))
  }

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
            let rec_id = self.module.imports[&import_decl.span];
            let rec = &self.module.import_records[rec_id];
            let Some(importee_idx) = rec.resolved_module else {
              return;
            };
            let binding_name = self.binding_name_for_import(importee_idx, rec_id).to_string();
            let mut needs_runtime_binding =
              import_decl.specifiers.as_ref().is_none_or(|specifiers| specifiers.is_empty());
            if let Some(specifiers) = &import_decl.specifiers {
              for spec in specifiers {
                match spec {
                  ast::ImportDeclarationSpecifier::ImportSpecifier(import_specifier) => {
                    let imported = Specifier::Literal(import_specifier.imported.name().into());
                    let import_binding = self.import_binding_for_imported(&binding_name, &imported);
                    needs_runtime_binding |= import_binding.needs_runtime_binding();
                    self.import_bindings.insert(import_specifier.local.symbol_id(), import_binding);
                  }
                  ast::ImportDeclarationSpecifier::ImportDefaultSpecifier(default_specifier) => {
                    let imported = Specifier::Literal("default".into());
                    let import_binding = self.import_binding_for_imported(&binding_name, &imported);
                    needs_runtime_binding |= import_binding.needs_runtime_binding();
                    self
                      .import_bindings
                      .insert(default_specifier.local.symbol_id(), import_binding);
                  }
                  ast::ImportDeclarationSpecifier::ImportNamespaceSpecifier(
                    namespace_specifier,
                  ) => {
                    needs_runtime_binding = true;
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
            if needs_runtime_binding
              && let Some(stmt) =
                self.create_import_binding_stmt(&self.modules[importee_idx], &binding_name)
            {
              program_body.push(stmt);
            }
          }
          ast::ModuleDeclaration::ExportNamedDeclaration(decl) => {
            if decl.source.is_some() {
              let rec_id = self.module.imports[&decl.span];
              let Some((_importee_idx, binding_name, stmt)) =
                self.ensure_import_for_record(rec_id, decl.span)
              else {
                return;
              };
              if let Some(stmt) = stmt {
                program_body.push(stmt);
              }
              let mut props = self.ast_factory.vec_with_capacity(decl.specifiers.len());
              for specifier in &decl.specifiers {
                let local = match &specifier.local {
                  ast::ModuleExportName::IdentifierName(ident) => {
                    Specifier::Literal(ident.name.into())
                  }
                  ast::ModuleExportName::StringLiteral(str) => Specifier::Literal(str.value.into()),
                  ast::ModuleExportName::IdentifierReference(_) => unreachable!(
                    "IdentifierReference is invalid in re-exported ExportNamedDeclaration"
                  ),
                };
                let exported = specifier.exported.name();
                props.push(self.ast_factory.make_lazy_export_property(
                  &exported,
                  make_import_access_expr(self.ast_factory, &binding_name, &local),
                  !is_validate_identifier_name(&exported),
                ));
              }
              self.exports.extend(props);
            } else if let Some(decl) = &mut decl.declaration {
              match decl {
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
              program_body.push(Statement::from(decl.take_in(self.ast_factory.allocator)));
            } else {
              for specifier in &decl.specifiers {
                if let Some(symbol_id) = scoping.get_root_binding(specifier.local.name().into()) {
                  self
                    .named_exports
                    .insert(specifier.exported.name(), NamedExport { local_binding: symbol_id });
                }
              }
            }
          }
          ast::ModuleDeclaration::ExportDefaultDeclaration(decl) => match &mut decl.declaration {
            ExportDefaultDeclarationKind::FunctionDeclaration(function) => {
              let id = if let Some(id) = &function.id {
                self.local_binding_name(id.symbol_id(), id.name.as_str())
              } else {
                let generated = "__default".to_string();
                function.id = Some(self.ast_factory.make_id(SPAN, &generated));
                generated
              };
              self.exports.push(self.ast_factory.make_lazy_export_property(
                "default",
                self.ast_factory.make_id_ref_expr(SPAN, &id),
                false,
              ));
              program_body.push(Statement::FunctionDeclaration(ArenaBox::new_in(
                function.as_mut().take_in(self.ast_factory.allocator),
                self.ast_factory.allocator,
              )));
            }
            ExportDefaultDeclarationKind::ClassDeclaration(class) => {
              let id = if let Some(id) = &class.id {
                self.local_binding_name(id.symbol_id(), id.name.as_str())
              } else {
                let generated = "__default".to_string();
                class.id = Some(self.ast_factory.make_id(SPAN, &generated));
                generated
              };
              self.exports.push(self.ast_factory.make_lazy_export_property(
                "default",
                self.ast_factory.make_id_ref_expr(SPAN, &id),
                false,
              ));
              program_body.push(Statement::ClassDeclaration(ArenaBox::new_in(
                class.as_mut().take_in(self.ast_factory.allocator),
                self.ast_factory.allocator,
              )));
            }
            expr @ ast::match_expression!(ExportDefaultDeclarationKind) => {
              let name = "__default".to_string();
              program_body.push(self.ast_factory.make_var_decl(
                &name,
                expr.to_expression_mut().take_in(self.ast_factory.allocator),
              ));
              self.exports.push(self.ast_factory.make_lazy_export_property(
                "default",
                self.ast_factory.make_id_ref_expr(SPAN, &name),
                false,
              ));
            }
            _ => {}
          },
          ast::ModuleDeclaration::ExportAllDeclaration(export_all_decl) => {
            let rec_id = self.module.imports[&export_all_decl.span];
            let Some((_importee_idx, binding_name, stmt)) =
              self.ensure_import_for_record(rec_id, export_all_decl.span)
            else {
              return;
            };
            if let Some(stmt) = stmt {
              program_body.push(stmt);
            }
            program_body.push(self.create_re_export_all_stmt(&binding_name, export_all_decl.span));
          }
          ast::ModuleDeclaration::TSExportAssignment(_)
          | ast::ModuleDeclaration::TSNamespaceExportDeclaration(_) => program_body.push(node),
        }
      }
      _ => program_body.push(node),
    }
  }

  fn should_include_static_import_for_runtime_execution(&self, stmt: &Statement<'_>) -> bool {
    let Statement::ImportDeclaration(import_decl) = stmt else {
      return false;
    };
    let rec_id = self.module.imports[&import_decl.span];
    let rec = &self.module.import_records[rec_id];
    rec.resolved_module.is_some_and(|importee_idx| self.metas[importee_idx].is_included)
  }

  fn create_re_export_all_stmt(&self, binding_name: &str, span: Span) -> Statement<'ast> {
    let call = self.ast_factory.expression_call(
      span,
      self.ast_factory.make_member_access_expr(ROLLIPOP_REQUIRE_NAME, "re"),
      NONE,
      self.ast_factory.vec_from_array([
        ast::Argument::from(self.ast_factory.make_id_ref_expr(SPAN, ROLLIPOP_EXPORTS_NAME)),
        ast::Argument::from(self.ast_factory.make_id_ref_expr(SPAN, binding_name)),
      ]),
      false,
    );
    self.ast_factory.statement_expression(span, call)
  }

  fn create_mark_esm_stmt(&self) -> Statement<'ast> {
    let call = self.ast_factory.expression_call(
      SPAN,
      self.ast_factory.make_member_access_expr(ROLLIPOP_REQUIRE_NAME, "r"),
      NONE,
      self
        .ast_factory
        .vec1(ast::Argument::from(self.ast_factory.make_id_ref_expr(SPAN, ROLLIPOP_EXPORTS_NAME))),
      false,
    );
    self.ast_factory.statement_expression(SPAN, call)
  }

  fn create_define_exports_stmt(&mut self, scoping: &Scoping) -> Option<Statement<'ast>> {
    for (exported, named_export) in &self.named_exports {
      let expr = if let Some(import_binding) = self.import_bindings.get(&named_export.local_binding)
      {
        import_binding.to_expression(self.ast_factory)
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

    if self.exports.is_empty() {
      return None;
    }

    let mut obj = self
      .ast_factory
      .alloc_object_expression(SPAN, self.ast_factory.vec_with_capacity(self.exports.len()));
    obj.properties.extend(self.exports.drain(..));
    let call = self.ast_factory.expression_call(
      SPAN,
      self.ast_factory.make_member_access_expr(ROLLIPOP_REQUIRE_NAME, "d"),
      NONE,
      self.ast_factory.vec_from_array([
        ast::Argument::from(self.ast_factory.make_id_ref_expr(SPAN, ROLLIPOP_EXPORTS_NAME)),
        ast::Argument::ObjectExpression(obj.into_in(self.ast_factory.allocator)),
      ]),
      false,
    );
    Some(self.ast_factory.statement_expression(SPAN, call))
  }

  fn add_json_metadata_exports(&mut self) {
    if !matches!(self.module.module_type, ModuleType::Json) {
      return;
    }

    for (exported, resolved_export) in self.linking_info.canonical_exports(true) {
      if exported == "default" {
        continue;
      }
      let canonical_ref = self.symbol_db.canonical_ref_for(resolved_export.symbol_ref);
      if canonical_ref.owner != self.module.idx {
        continue;
      }
      if !self
        .stmt_infos
        .declared_stmts_by_symbol(&canonical_ref)
        .iter()
        .any(|stmt_info_idx| self.should_include_top_level_stmt(*stmt_info_idx))
      {
        continue;
      }
      let name = canonical_ref.name(self.symbol_db);
      self.exports.push(self.ast_factory.make_lazy_export_property(
        exported,
        self.ast_factory.make_id_ref_expr(SPAN, name),
        !is_validate_identifier_name(exported.as_str()),
      ));
    }
  }

  fn hot_context_name(&self) -> String {
    format!("hot_{}", self.module.repr_name)
  }

  fn rewrite_hot_accept_call_deps(&self, call_expr: &mut ast::CallExpression<'ast>) {
    if !call_expr.callee.is_import_meta_hot_accept() || call_expr.arguments.is_empty() {
      return;
    }
    match &mut call_expr.arguments[0] {
      ast::Argument::StringLiteral(lit) => {
        let Some(rec_idx) =
          self.module.hmr_info.module_request_to_import_record_idx.get(lit.value.as_str())
        else {
          return;
        };
        let Some(module_idx) = self.module.import_records[*rec_idx].resolved_module else { return };
        lit.value = self.ast_factory.str(self.modules[module_idx].stable_id());
      }
      ast::Argument::ArrayExpression(array) => {
        for element in &mut array.elements {
          if let ast::ArrayExpressionElement::StringLiteral(lit) = element {
            let Some(rec_idx) =
              self.module.hmr_info.module_request_to_import_record_idx.get(lit.value.as_str())
            else {
              continue;
            };
            let Some(module_idx) = self.module.import_records[*rec_idx].resolved_module else {
              continue;
            };
            lit.value = self.ast_factory.str(self.modules[module_idx].stable_id());
          }
        }
      }
      _ => {}
    }
  }

  fn rewrite_dynamic_import(&self, node: &mut Expression<'ast>) {
    let Expression::ImportExpression(import_expr) = node else { return };
    let Some(rec_idx) = self.module.imports.get(&import_expr.span) else { return };
    let rec = &self.module.import_records[*rec_idx];
    let Some(importee_idx) = rec.resolved_module else { return };
    let importee = &self.modules[importee_idx];
    let require_expr = match importee {
      Module::Normal(_) => self.require_call_for_module(importee),
      Module::External(importee) => self.require_call_for_external(importee),
    };
    *node = self.ast_factory.make_promise_resolve_then(self.to_esm_expr(require_expr, None));
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
    let Some(rec_idx) = self.module.imports.get(&call_expr.span) else { return };
    let rec = &self.module.import_records[*rec_idx];
    let Some(importee_idx) = rec.resolved_module else { return };
    let importee = &self.modules[importee_idx];
    *node = match importee {
      Module::Normal(_) => self.require_call_for_module(importee),
      Module::External(importee) => self.require_call_for_external(importee),
    };
  }

  fn rewrite_import_meta_hot(&mut self, node: &mut Expression<'ast>) {
    if node.is_import_meta_hot() {
      self.uses_import_meta_hot = true;
      *node = self.ast_factory.make_id_ref_expr(SPAN, &self.hot_context_name());
    }
  }
}

impl<'me, 'ast> HmrAstBuilder<'me, 'ast> for RollipopAstFinalizer<'me, 'ast> {
  fn builder(&self) -> oxc::ast::AstBuilder<'ast> {
    *self.ast_factory
  }

  fn module(&self) -> &NormalModule {
    self.module
  }

  fn binding_name_for_namespace_object_ref_atom(&self) -> ast::Str<'ast> {
    self.builder().str(ROLLIPOP_EXPORTS_NAME)
  }

  fn alias_name_for_import_meta_hot(&self) -> ast::Str<'ast> {
    self.builder().str(&self.hot_context_name())
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

    let body = node.body.take_in(self.ast_factory.allocator);
    node.body.reserve_exact(body.len() + 3);
    if self.is_runtime_module {
      for stmt in body {
        self.handle_top_level_stmt(&mut node.body, stmt, ctx.scoping());
      }
    } else {
      for (stmt, (stmt_info_idx, _stmt_info)) in
        body.into_iter().zip(self.stmt_infos.iter_enumerated().skip(1))
      {
        if self.should_include_top_level_stmt(stmt_info_idx)
          || is_export_specifier_declaration(&stmt)
          || self.should_include_static_import_for_runtime_execution(&stmt)
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
    let body = node.body.take_in(self.ast_factory.allocator);
    let mut next_body = self.ast_factory.vec_with_capacity(body.len() + 3);
    if self.module.exports_kind.is_esm() && !self.is_runtime_module {
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
    if let Expression::Identifier(ident) = node
      && let Some(reference_id) = ident.reference_id.get()
      && let Some(symbol_id) = ctx.scoping().get_reference(reference_id).symbol_id()
      && let Some(import_binding) = self.import_bindings.get(&symbol_id)
    {
      *node = import_binding.to_expression(self.ast_factory);
      return;
    }

    self.rewrite_dynamic_import(node);
    self.rewrite_require(node, ctx);
    self.rewrite_import_meta_hot(node);
  }

  fn exit_identifier_reference(
    &mut self,
    ident: &mut ast::IdentifierReference<'ast>,
    ctx: &mut oxc_traverse::TraverseCtx<'ast, ()>,
  ) {
    if ident.name == "exports" && ident.is_global_reference(ctx.scoping()) {
      ident.name = self.ast_factory.str(ROLLIPOP_EXPORTS_NAME).into();
      return;
    }

    let Some(reference_id) = ident.reference_id.get() else { return };
    let reference = ctx.scoping().get_reference(reference_id);
    let Some(symbol_id) = reference.symbol_id() else { return };
    if let Some(binding_name) = self.renamed_factory_param_bindings.get(&symbol_id) {
      ident.name = self.ast_factory.str(binding_name).into();
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
      ident.name = self.ast_factory.str(binding_name).into();
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
  fn needs_runtime_binding(&self) -> bool {
    true
  }

  fn to_expression<'ast>(&self, ast_factory: AstFactory<'ast>) -> Expression<'ast> {
    make_import_access_expr(ast_factory, &self.binding_name, &self.imported)
  }
}

fn make_import_access_expr<'ast>(
  ast_factory: AstFactory<'ast>,
  binding_name: &str,
  imported: &Specifier,
) -> Expression<'ast> {
  match imported {
    Specifier::Star => ast_factory.make_id_ref_expr(SPAN, binding_name),
    Specifier::Literal(name) if is_validate_identifier_name(name.as_str()) => {
      Expression::StaticMemberExpression(ast_factory.alloc_static_member_expression(
        SPAN,
        ast_factory.make_id_ref_expr(SPAN, binding_name),
        ast_factory.identifier_name(SPAN, ast_factory.str(name.as_str())),
        false,
      ))
    }
    Specifier::Literal(name) => {
      Expression::ComputedMemberExpression(ast_factory.alloc_computed_member_expression(
        SPAN,
        ast_factory.make_id_ref_expr(SPAN, binding_name),
        ast_factory.expression_string_literal(SPAN, ast_factory.str(name.as_str()), None),
        false,
      ))
    }
  }
}

fn is_export_specifier_declaration(stmt: &Statement<'_>) -> bool {
  matches!(
    stmt,
    Statement::ExportNamedDeclaration(decl)
      if decl.source.is_none() && decl.declaration.is_none()
  )
}
