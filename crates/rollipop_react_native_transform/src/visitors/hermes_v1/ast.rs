use swc_common::{DUMMY_SP, SyntaxContext, util::take::Take};
use swc_ecma_ast::{
  ArrowExpr, ArrowFunctionBody, BlockStmt, CallExpr, Callee, Decl, Expr, FunctionBody, ParenExpr,
  Pat, ReturnStmt, Stmt, VarDecl, VarDeclKind, VarDeclarator,
};

pub(super) fn block_from_arrow_body(body: ArrowFunctionBody) -> BlockStmt {
  match body {
    ArrowFunctionBody::FunctionBody(body) => {
      BlockStmt { span: body.span, ctxt: SyntaxContext::empty(), stmts: body.stmts }
    }
    ArrowFunctionBody::Expr(expr) => BlockStmt {
      span: DUMMY_SP,
      ctxt: SyntaxContext::empty(),
      stmts: vec![Stmt::Return(ReturnStmt { span: DUMMY_SP, arg: Some(expr) })],
    },
  }
}

pub(super) fn function_body_from_block(body: BlockStmt) -> FunctionBody {
  FunctionBody { span: body.span, stmts: body.stmts }
}

pub(super) fn var_decl_stmt(name: Pat, init: Expr) -> Stmt {
  Stmt::Decl(Decl::Var(Box::new(VarDecl {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    kind: VarDeclKind::Var,
    declare: false,
    decls: vec![VarDeclarator {
      span: DUMMY_SP,
      name,
      init: Some(Box::new(init)),
      definite: false,
    }],
  })))
}

pub(super) fn call_expr(callee: Expr) -> Expr {
  Expr::Call(CallExpr {
    span: DUMMY_SP,
    ctxt: SyntaxContext::empty(),
    callee: Callee::Expr(Box::new(Expr::Paren(ParenExpr {
      span: DUMMY_SP,
      expr: Box::new(callee),
    }))),
    args: vec![],
    type_args: None,
  })
}

pub(super) fn take_arrow_body(arrow: &mut ArrowExpr) -> ArrowFunctionBody {
  *arrow.body.take()
}
