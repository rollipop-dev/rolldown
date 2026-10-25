use swc_ecma_ast::{
  Class, ComputedPropName, Expr, Function, FunctionBody, GetterProp, Lit, MethodProp, PropName,
  SetterProp, Str, SuperPropExpr,
};
use swc_ecma_hooks::VisitMutHook;
use swc_ecma_transformer::TraverseCtx;

#[derive(Default)]
pub struct SuperInObjectAccessor {
  direct_super: Option<bool>,
  scopes: Vec<Option<bool>>,
  accessors: Vec<Accessor>,
}

struct Accessor {
  body_address: Option<usize>,
  found: bool,
}

impl SuperInObjectAccessor {
  fn enter_scope(&mut self) {
    self.scopes.push(self.direct_super.take());
  }

  fn exit_scope(&mut self) {
    self.direct_super = self.scopes.pop().expect("balanced accessor scope hooks");
  }

  fn enter_accessor(&mut self, body: Option<&FunctionBody>) {
    self.enter_scope();
    self.accessors.push(Accessor {
      body_address: body.map(|body| std::ptr::from_ref(body).addr()),
      found: false,
    });
  }

  fn exit_accessor(&mut self, key: &mut PropName) {
    if self.accessors.pop().expect("balanced accessor hooks").found {
      make_accessor_key_computed_string(key);
    }
    self.exit_scope();
  }

  fn is_accessor_body(&self, body: &FunctionBody) -> bool {
    self
      .accessors
      .last()
      .is_some_and(|accessor| accessor.body_address == Some(std::ptr::from_ref(body).addr()))
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

impl VisitMutHook<TraverseCtx> for SuperInObjectAccessor {
  fn enter_getter_prop(&mut self, prop: &mut GetterProp, _ctx: &mut TraverseCtx) {
    self.enter_accessor(prop.function.body.as_ref());
  }

  fn exit_getter_prop(&mut self, prop: &mut GetterProp, _ctx: &mut TraverseCtx) {
    self.exit_accessor(&mut prop.key);
  }

  fn enter_setter_prop(&mut self, prop: &mut SetterProp, _ctx: &mut TraverseCtx) {
    self.enter_accessor(prop.function.body.as_ref());
  }

  fn exit_setter_prop(&mut self, prop: &mut SetterProp, _ctx: &mut TraverseCtx) {
    self.exit_accessor(&mut prop.key);
  }

  fn enter_function_body(&mut self, body: &mut FunctionBody, _ctx: &mut TraverseCtx) {
    if self.is_accessor_body(body) {
      self.direct_super = Some(false);
    }
  }

  fn exit_function_body(&mut self, body: &mut FunctionBody, _ctx: &mut TraverseCtx) {
    if self.is_accessor_body(body) {
      self.accessors.last_mut().expect("accessor body").found =
        self.direct_super.take().unwrap_or(false);
    }
  }

  fn enter_super_prop_expr(&mut self, _expr: &mut SuperPropExpr, _ctx: &mut TraverseCtx) {
    if let Some(found) = &mut self.direct_super {
      *found = true;
    }
  }

  // Arrows inherit the accessor's super binding. Ordinary functions, object
  // methods and classes establish boundaries, including their computed keys.
  scope_hooks!(enter_function, exit_function, Function);
  scope_hooks!(enter_method_prop, exit_method_prop, MethodProp);
  scope_hooks!(enter_class, exit_class, Class);
}

fn make_accessor_key_computed_string(key: &mut PropName) {
  match key {
    PropName::Ident(ident) => {
      let span = ident.span;
      let value = ident.sym.clone();
      *key = PropName::Computed(ComputedPropName {
        span,
        expr: Box::new(Expr::Lit(Lit::Str(Str { span, value: value.into(), raw: None }))),
      });
    }
    PropName::Str(str_lit) => {
      let span = str_lit.span;
      *key = PropName::Computed(ComputedPropName {
        span,
        expr: Box::new(Expr::Lit(Lit::Str(str_lit.clone()))),
      });
    }
    _ => {}
  }
}
