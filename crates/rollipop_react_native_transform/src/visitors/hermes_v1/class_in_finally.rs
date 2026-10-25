use swc_common::{DUMMY_SP, SyntaxContext};
use swc_ecma_ast::{
  ArrowExpr, ArrowFunctionBody, BlockStmt, ClassDecl, ClassMember, Decl, Expr, Function,
  GetterProp, MethodProp, Pat, ReturnStmt, SetterProp, Stmt, TryStmt,
};
use swc_ecma_hooks::VisitMutHook;
use swc_ecma_transformer::TraverseCtx;

use super::ast::{call_expr, function_body_from_block, var_decl_stmt};

#[derive(Default)]
pub struct ClassInFinally {
  finalizer_depth: usize,
  scope_depths: Vec<usize>,
  // Addresses identify finalizer blocks even when generated nodes share DUMMY_SP.
  // They are compared only during their containing try statement's traversal.
  finalizers: Vec<Option<usize>>,
}

impl ClassInFinally {
  fn enter_scope(&mut self) {
    self.scope_depths.push(std::mem::take(&mut self.finalizer_depth));
  }

  fn exit_scope(&mut self) {
    self.finalizer_depth = self.scope_depths.pop().expect("balanced scope hooks");
  }

  fn is_finalizer(&self, block: &BlockStmt) -> bool {
    self
      .finalizers
      .last()
      .is_some_and(|finalizer| *finalizer == Some(std::ptr::from_ref(block).addr()))
  }
}

macro_rules! scope_hooks {
  ($enter:ident, $exit:ident, $node:ty) => {
    fn $enter(&mut self, _node: &mut $node, _ctx: &mut TraverseCtx) {
      self.enter_scope();
    }
    fn $exit(&mut self, _node: &mut $node, _ctx: &mut TraverseCtx) {
      self.exit_scope();
    }
  };
}

impl VisitMutHook<TraverseCtx> for ClassInFinally {
  fn enter_try_stmt(&mut self, stmt: &mut TryStmt, _ctx: &mut TraverseCtx) {
    self.finalizers.push(stmt.finalizer.as_ref().map(|block| std::ptr::from_ref(block).addr()));
  }

  fn exit_try_stmt(&mut self, _stmt: &mut TryStmt, _ctx: &mut TraverseCtx) {
    self.finalizers.pop().expect("balanced try hooks");
  }

  fn enter_block_stmt(&mut self, block: &mut BlockStmt, _ctx: &mut TraverseCtx) {
    if self.is_finalizer(block) {
      self.finalizer_depth += 1;
    }
  }

  fn exit_block_stmt(&mut self, block: &mut BlockStmt, _ctx: &mut TraverseCtx) {
    if self.is_finalizer(block) {
      self.finalizer_depth -= 1;
    }
  }

  fn enter_stmt(&mut self, stmt: &mut Stmt, _ctx: &mut TraverseCtx) {
    if self.finalizer_depth > 0
      && let Stmt::Decl(Decl::Class(class)) = stmt
      && class.class.decorators.is_empty()
    {
      *stmt = class_decl_in_finalizer_stmt(class);
    }
  }

  fn enter_expr(&mut self, expr: &mut Expr, _ctx: &mut TraverseCtx) {
    if self.finalizer_depth > 0
      && let Expr::Class(class) = expr
      && class.class.decorators.is_empty()
    {
      *expr = call_expr(Expr::Arrow(ArrowExpr {
        span: DUMMY_SP,
        body: Box::new(ArrowFunctionBody::Expr(Box::new(Expr::Class(class.clone())))),
        ..Default::default()
      }));
    }
  }

  scope_hooks!(enter_function, exit_function, Function);
  scope_hooks!(enter_arrow_expr, exit_arrow_expr, ArrowExpr);
  scope_hooks!(enter_method_prop, exit_method_prop, MethodProp);
  scope_hooks!(enter_getter_prop, exit_getter_prop, GetterProp);
  scope_hooks!(enter_setter_prop, exit_setter_prop, SetterProp);
  scope_hooks!(enter_class_members, exit_class_members, Vec<ClassMember>);
}

fn class_decl_in_finalizer_stmt(class_decl: &ClassDecl) -> Stmt {
  let ident = class_decl.ident.clone();
  let mut inner_class = (*class_decl.class).clone();
  inner_class.decorators.clear();

  let inner = Stmt::Decl(Decl::Class(ClassDecl {
    ident: ident.clone(),
    declare: false,
    class: Box::new(inner_class),
  }));

  let arrow = Expr::Arrow(ArrowExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    params: vec![],
    body: Box::new(ArrowFunctionBody::FunctionBody(function_body_from_block(BlockStmt {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      stmts: vec![
        inner,
        Stmt::Return(ReturnStmt {
          span: DUMMY_SP,
          arg: Some(Box::new(Expr::Ident(ident.clone()))),
        }),
      ],
    }))),
    is_async: false,
    is_generator: false,
    type_params: None,
    return_type: None,
  });

  var_decl_stmt(Pat::Ident(ident.into()), call_expr(arrow))
}
