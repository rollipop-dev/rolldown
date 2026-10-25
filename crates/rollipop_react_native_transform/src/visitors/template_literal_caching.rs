use swc_common::DUMMY_SP;
use swc_common::util::take::Take;
use swc_ecma_ast::{
  ArrowExpr, ArrowFunctionBody, AssignExpr, BinExpr, CallExpr, Expr, Stmt, TaggedTpl, Tpl, VarDecl,
  VarDeclKind, VarDeclarator, op,
};
use swc_ecma_hooks::VisitMutHook;
use swc_ecma_transformer::TraverseCtx;
use swc_ecma_utils::{ExprFactory, prepend_stmt, private_ident};

pub fn template_literal_caching() -> impl VisitMutHook<TraverseCtx> {
  TemplateLiteralCaching::default()
}

// Upstream SWC's pass is Fold-based and marks JSX as unreachable. React Native
// can preserve JSX here, so use a hook in the shared compatibility traversal.
#[derive(Default)]
struct TemplateLiteralCaching {
  decls: Vec<VarDeclarator>,
  helper_ident: Option<swc_ecma_ast::Ident>,
}

impl TemplateLiteralCaching {
  fn create_binding(&mut self, name: swc_ecma_ast::Ident, init: Option<Expr>) {
    self.decls.push(VarDeclarator {
      span: DUMMY_SP,
      name: name.into(),
      init: init.map(Box::new),
      definite: false,
    });
  }

  fn create_var_decl(&mut self) -> Option<Stmt> {
    if self.decls.is_empty() {
      return None;
    }

    Some(
      VarDecl {
        span: DUMMY_SP,
        kind: VarDeclKind::Let,
        declare: false,
        decls: std::mem::take(&mut self.decls),
        ..Default::default()
      }
      .into(),
    )
  }

  fn transform_tagged_template(&mut self, tagged: TaggedTpl) -> Expr {
    let helper_ident = match &self.helper_ident {
      Some(helper_ident) => helper_ident.clone(),
      None => {
        let helper_ident = private_ident!("_");
        let t = private_ident!("t");
        self.helper_ident = Some(helper_ident.clone());
        self.create_binding(
          helper_ident.clone(),
          Some(
            ArrowExpr {
              span: DUMMY_SP,
              params: vec![t.clone().into()],
              body: Box::new(ArrowFunctionBody::Expr(t.into())),
              is_async: false,
              is_generator: false,
              ..Default::default()
            }
            .into(),
          ),
        );
        helper_ident
      }
    };

    let template = TaggedTpl {
      span: DUMMY_SP,
      tag: helper_ident.into(),
      tpl: Box::new(Tpl {
        span: DUMMY_SP,
        quasis: tagged.tpl.quasis,
        exprs: tagged.tpl.exprs.iter().map(|_| 0.0.into()).collect(),
      }),
      ..Default::default()
    };

    let cache_ident = private_ident!("t");
    self.create_binding(cache_ident.clone(), None);
    let inline_cache: Expr = BinExpr {
      span: DUMMY_SP,
      op: op!("||"),
      left: cache_ident.clone().into(),
      right: AssignExpr {
        span: DUMMY_SP,
        op: op!("="),
        left: cache_ident.into(),
        right: Box::new(Expr::TaggedTpl(template)),
      }
      .into(),
    }
    .into();

    CallExpr {
      span: DUMMY_SP,
      callee: tagged.tag.as_callee(),
      args: vec![inline_cache.as_arg()]
        .into_iter()
        .chain(tagged.tpl.exprs.into_iter().map(ExprFactory::as_arg))
        .collect(),
      ..Default::default()
    }
    .into()
  }
}

impl VisitMutHook<TraverseCtx> for TemplateLiteralCaching {
  fn exit_expr(&mut self, expr: &mut Expr, _ctx: &mut TraverseCtx) {
    *expr = match expr.take() {
      Expr::TaggedTpl(tagged) => self.transform_tagged_template(tagged),
      expr => expr,
    };
  }

  fn exit_module(&mut self, module: &mut swc_ecma_ast::Module, _ctx: &mut TraverseCtx) {
    if let Some(var) = self.create_var_decl() {
      prepend_stmt(&mut module.body, var.into());
    }
  }

  fn exit_script(&mut self, script: &mut swc_ecma_ast::Script, _ctx: &mut TraverseCtx) {
    if let Some(var) = self.create_var_decl() {
      prepend_stmt(&mut script.body, var);
    }
  }
}
