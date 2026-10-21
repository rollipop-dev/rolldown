// Copyright (c) Meta Platforms, Inc. and affiliates.
//
// This source code is licensed under the MIT license found in the
// LICENSE file in the root directory of this source tree.

//! Apply the React Compiler's binding renames to identifiers in uncompiled
//! sibling code. Compiled functions already carry the renamed identifiers.

use react_compiler::entrypoint::compile_result::BindingRenameInfo;
use react_compiler_ast::scope::{BindingId, ScopeInfo};
use rustc_hash::FxHashMap;
use swc_atoms::Atom;
use swc_ecma_ast as swc;
use swc_ecma_ast::Program;
use swc_ecma_visit::{VisitMut, VisitMutWith};

// Compiled identifiers can preserve the same source offset as an original
// binding reference. Their names already reflect compilation, so only rename
// an identifier that still has both the original offset and original name.
pub type RenamePlan = FxHashMap<(u32, Atom), String>;

/// Map each original reference's `(span.lo, name)` to its new name.
pub fn build_rename_plan(
    scope_info: &ScopeInfo,
    renames: &[BindingRenameInfo],
) -> RenamePlan {
    if renames.is_empty() {
        return FxHashMap::default();
    }

    let renames_by_declaration: FxHashMap<(u32, &str), &BindingRenameInfo> = renames
        .iter()
        .map(|rename| ((rename.declaration_start, rename.original.as_str()), rename))
        .collect();

    let mut renamed_bindings: FxHashMap<BindingId, (Atom, String)> = FxHashMap::default();
    for binding in &scope_info.bindings {
        let Some(rename) = binding
            .declaration_start
            .and_then(|start| renames_by_declaration.get(&(start, binding.name.as_str())))
        else {
            continue;
        };
        renamed_bindings.insert(
            binding.id,
            (Atom::from(binding.name.as_str()), rename.renamed.clone()),
        );
    }

    if renamed_bindings.is_empty() {
        return FxHashMap::default();
    }

    scope_info
        .ref_node_id_to_binding
        .iter()
        .filter_map(|(&position, binding_id)| {
            renamed_bindings
                .get(binding_id)
                .map(|(original, renamed)| ((position, original.clone()), renamed.clone()))
        })
        .collect()
}

/// Walk the program and apply `rename_plan`, expanding shorthand properties
/// when the value is renamed.
pub fn apply_renames(program: &mut Program, rename_plan: &RenamePlan) {
    if rename_plan.is_empty() {
        return;
    }
    program.visit_mut_with(&mut RenameApplyVisitor { rename_plan });
}

struct RenameApplyVisitor<'p> {
    rename_plan: &'p RenamePlan,
}

impl RenameApplyVisitor<'_> {
    fn renamed_at(&self, ident: &swc::Ident) -> Option<&str> {
        self.rename_plan
            .get(&(ident.span.lo.0, ident.sym.clone()))
            .map(std::string::String::as_str)
    }

    fn rename_ident(&self, ident: &mut swc::Ident) -> bool {
        let Some(renamed) = self.renamed_at(ident) else {
            return false;
        };

        ident.sym = Atom::from(renamed);
        true
    }
}

impl VisitMut for RenameApplyVisitor<'_> {
    fn visit_mut_ident(&mut self, ident: &mut swc::Ident) {
        self.rename_ident(ident);
    }

    /// Rename the object of a member expression, never a static property name.
    fn visit_mut_member_expr(&mut self, member: &mut swc::MemberExpr) {
        member.obj.visit_mut_with(self);
        if let swc::MemberProp::Computed(computed) = &mut member.prop {
            computed.visit_mut_with(self);
        }
    }

    /// Rename the JSX member object, never a static property name.
    fn visit_mut_jsx_member_expr(&mut self, member: &mut swc::JSXMemberExpr) {
        member.obj.visit_mut_with(self);
    }

    /// Preserve the imported name when a named import's local binding changes.
    fn visit_mut_import_named_specifier(&mut self, specifier: &mut swc::ImportNamedSpecifier) {
        if specifier.imported.is_none() {
            let imported = swc::ModuleExportName::Ident(specifier.local.clone());
            if self.rename_ident(&mut specifier.local) {
                specifier.imported = Some(imported);
            }
            return;
        }

        self.rename_ident(&mut specifier.local);
    }

    /// Preserve the exported name when a local named export binding changes.
    fn visit_mut_export_named_specifier(&mut self, specifier: &mut swc::ExportNamedSpecifier) {
        if specifier.exported.is_some() {
            if let swc::ModuleExportName::Ident(orig) = &mut specifier.orig {
                self.rename_ident(orig);
            }
            return;
        }

        if let swc::ModuleExportName::Ident(orig) = &mut specifier.orig {
            let exported = swc::ModuleExportName::Ident(orig.clone());
            if self.rename_ident(orig) {
                specifier.exported = Some(exported);
            }
        }
    }

    /// Expand a renamed shorthand object property `{x}` into `{x: x_0}`.
    fn visit_mut_prop(&mut self, prop: &mut swc::Prop) {
        match prop {
            swc::Prop::Shorthand(ident) => {
                if let Some(renamed) = self.renamed_at(ident) {
                    let key = swc::PropName::Ident(swc::IdentName::from(ident.clone()));
                    let mut value = ident.clone();
                    value.sym = Atom::from(renamed);
                    *prop = swc::Prop::KeyValue(swc::KeyValueProp {
                        key,
                        value: Box::new(swc::Expr::Ident(value)),
                    });
                }
            }
            swc::Prop::Assign(assign) => {
                assign.value.visit_mut_with(self);
            }
            _ => prop.visit_mut_children_with(self),
        }
    }

    /// Expand a renamed shorthand binding property `{x}` into `{x: x_0}`.
    fn visit_mut_object_pat_prop(&mut self, prop: &mut swc::ObjectPatProp) {
        match prop {
            swc::ObjectPatProp::Assign(assign) => {
                assign.value.visit_mut_with(self);

                if let Some(renamed) = self.renamed_at(&assign.key.id) {
                    let mut binding = assign.key.clone();
                    binding.id.sym = Atom::from(renamed);
                    let value = match assign.value.take() {
                        Some(default_value) => swc::Pat::Assign(swc::AssignPat {
                            span: assign.span,
                            left: Box::new(swc::Pat::Ident(binding)),
                            right: default_value,
                        }),
                        None => swc::Pat::Ident(binding),
                    };

                    let key = swc::PropName::Ident(swc::IdentName::from(assign.key.id.clone()));
                    *prop = swc::ObjectPatProp::KeyValue(swc::KeyValuePatProp {
                        key,
                        value: Box::new(value),
                    });
                }
            }
            _ => prop.visit_mut_children_with(self),
        }
    }
}

#[cfg(test)]
mod tests {
    use swc_common::{BytePos, Span, SyntaxContext};

    use super::*;

    fn identifier(name: &str) -> swc::Ident {
        swc::Ident::new(
            name.into(),
            Span::new(BytePos(10), BytePos(20)),
            SyntaxContext::empty(),
        )
    }

    #[test]
    fn shared_source_offsets_do_not_rename_generated_identifiers() {
        let plan: RenamePlan = [((10, Atom::from("item")), "item_0".into())]
            .into_iter()
            .collect();
        let mut visitor = RenameApplyVisitor { rename_plan: &plan };
        for name in ["item", "_temp2", "item_0", "data_0"] {
            let mut ident = identifier(name);
            assert_eq!(visitor.rename_ident(&mut ident), name == "item");
            assert_eq!(ident.sym.as_ref(), if name == "item" { "item_0" } else { name });
            assert_eq!(ident.span, identifier(name).span);

            let mut prop = swc::Prop::Shorthand(identifier(name));
            prop.visit_mut_with(&mut visitor);
            if name == "item" {
                let swc::Prop::KeyValue(prop) = prop else {
                    panic!("expected expanded shorthand")
                };
                assert!(matches!(prop.key, swc::PropName::Ident(key) if key.sym == "item"));
                assert!(matches!(*prop.value, swc::Expr::Ident(value) if value.sym == "item_0"));
            } else {
                assert!(matches!(prop, swc::Prop::Shorthand(ident) if ident.sym == name));
            }

            let mut pattern = swc::ObjectPatProp::Assign(swc::AssignPatProp {
                span: identifier(name).span,
                key: identifier(name).into(),
                value: None,
            });
            pattern.visit_mut_with(&mut visitor);
            if name == "item" {
                let swc::ObjectPatProp::KeyValue(prop) = pattern else {
                    panic!("expected expanded binding")
                };
                assert!(matches!(prop.key, swc::PropName::Ident(key) if key.sym == "item"));
                assert!(matches!(*prop.value, swc::Pat::Ident(value) if value.id.sym == "item_0"));
            } else {
                assert!(matches!(pattern, swc::ObjectPatProp::Assign(prop) if prop.key.id.sym == name));
            }
        }
    }

    #[test]
    fn generated_declaration_at_the_same_offset_does_not_hide_the_original_rename() {
        let (program, _, _) = crate::parse_source("const data = 1; data;", Default::default())
            .expect("valid source");
        let info = crate::convert_scope::SemanticBuilder::new().build(&program);
        let declaration_start = info
            .bindings
            .iter()
            .find(|binding| binding.name == "data")
            .unwrap()
            .declaration_start
            .unwrap();
        let plan = build_rename_plan(
            &info,
            &[
                BindingRenameInfo {
                    original: "data".into(),
                    renamed: "data_0".into(),
                    declaration_start,
                },
                BindingRenameInfo {
                    original: "_temp2".into(),
                    renamed: "data_1".into(),
                    declaration_start,
                },
            ],
        );
        assert!(!plan.is_empty());
        assert!(plan.iter().all(|((_, original), renamed)| original == "data" && renamed == "data_0"));
    }
}
