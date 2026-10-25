use rustc_hash::FxHashSet;

use swc_common::{DUMMY_SP, SyntaxContext};
use swc_ecma_ast::{ArrowExpr, ArrowFunctionBody, BinExpr, BinaryOp, CondExpr, Expr, Ident, Pat};
use swc_ecma_hooks::VisitMutHook;
use swc_ecma_transformer::TraverseCtx;
use swc_ecma_utils::private_ident;
use swc_ecma_visit::{Visit, VisitWith};

use super::ast::{
  block_from_arrow_body, call_expr, function_body_from_block, take_arrow_body, var_decl_stmt,
};

/// Source: https://github.com/expo/expo/blob/d427a802b11d79c8e45be8ceb582ca10fae103af/packages/babel-preset-expo/src/plugins/fix-hermes-v1-async-arrow-non-simple-params.ts
pub struct AsyncArrowNonSimpleParamsVisitor;

impl VisitMutHook<TraverseCtx> for AsyncArrowNonSimpleParamsVisitor {
  fn enter_arrow_expr(&mut self, arrow: &mut ArrowExpr, _ctx: &mut TraverseCtx) {
    if arrow.is_async && arrow.params.iter().any(|p| !matches!(p, Pat::Ident(_))) {
      rewrite_async_arrow_non_simple_params(arrow);
    }
  }
}

fn rewrite_async_arrow_non_simple_params(arrow: &mut ArrowExpr) {
  if arrow.params.iter().any(|p| matches!(p, Pat::Rest(_))) {
    let body = block_from_arrow_body(take_arrow_body(arrow));
    let inner_async = Expr::Arrow(ArrowExpr {
      span: DUMMY_SP,
      ctxt: arrow.ctxt,
      params: vec![],
      body: Box::new(ArrowFunctionBody::FunctionBody(function_body_from_block(body))),
      is_async: true,
      is_generator: false,
      type_params: None,
      return_type: None,
    });

    arrow.is_async = false;
    *arrow.body = ArrowFunctionBody::Expr(Box::new(call_expr(inner_async)));
    return;
  }

  let mut used_names = UsedNameCollector::default();
  arrow.params.visit_with(&mut used_names);
  arrow.body.visit_with(&mut used_names);

  let params = std::mem::take(&mut arrow.params);
  let mut new_params = Vec::with_capacity(params.len());
  let mut init_stmts = vec![];

  for param in params {
    match param {
      Pat::Ident(_) => new_params.push(param),
      Pat::Assign(assign) => {
        let sym = next_param_ident(&mut used_names.names);
        new_params.push(Pat::Ident(sym.clone().into()));
        init_stmts.push(var_decl_stmt(
          *assign.left,
          Expr::Cond(CondExpr {
            span: DUMMY_SP,
            test: Box::new(Expr::Bin(BinExpr {
              span: DUMMY_SP,
              op: BinaryOp::EqEqEq,
              left: Box::new(Expr::Ident(sym.clone())),
              right: Box::new(undefined_ident()),
            })),
            cons: assign.right,
            alt: Box::new(Expr::Ident(sym)),
          }),
        ));
      }
      other => {
        let sym = next_param_ident(&mut used_names.names);
        new_params.push(Pat::Ident(sym.clone().into()));
        init_stmts.push(var_decl_stmt(other, Expr::Ident(sym)));
      }
    }
  }

  let mut body = block_from_arrow_body(take_arrow_body(arrow));
  body.stmts.splice(0..0, init_stmts);
  arrow.params = new_params;
  *arrow.body = ArrowFunctionBody::FunctionBody(function_body_from_block(body));
}

fn undefined_ident() -> Expr {
  Expr::Ident(Ident::new("undefined".into(), DUMMY_SP, SyntaxContext::empty()))
}

fn next_param_ident(used_names: &mut FxHashSet<String>) -> Ident {
  let mut idx = 1;
  loop {
    let name = if idx == 1 { "_p".to_string() } else { format!("_p{idx}") };

    if used_names.insert(name.clone()) {
      return private_ident!(name);
    }
    idx += 1;
  }
}

#[derive(Default)]
struct UsedNameCollector {
  names: FxHashSet<String>,
}

impl Visit for UsedNameCollector {
  fn visit_ident(&mut self, ident: &Ident) {
    self.names.insert(ident.sym.to_string());
  }
}
