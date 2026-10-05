//! Conservative resolution: ambiguous or mutable bindings stay dynamic.

use std::collections::{HashMap, HashSet};
use syn::visit::{self, Visit};
use syn::{Expr, Lit};

pub struct SqlText {
    pub text: String,
    pub dynamic: bool,
}

pub fn resolve(
    expr: &Expr,
    locals: &HashMap<String, Expr>,
    constants: &HashMap<String, Expr>,
) -> Option<SqlText> {
    fn inner(
        expr: &Expr,
        locals: &HashMap<String, Expr>,
        constants: &HashMap<String, Expr>,
        depth: usize,
    ) -> Option<SqlText> {
        if depth > 16 {
            return None;
        }
        match expr {
            Expr::Lit(l) => match &l.lit {
                Lit::Str(s) => Some(SqlText {
                    text: s.value(),
                    dynamic: false,
                }),
                _ => None,
            },
            Expr::Reference(r) => inner(&r.expr, locals, constants, depth + 1),
            Expr::Paren(p) => inner(&p.expr, locals, constants, depth + 1),
            Expr::Path(p) if p.path.segments.len() == 1 => {
                let name = p.path.segments[0].ident.to_string();
                inner(
                    locals.get(&name).or_else(|| constants.get(&name))?,
                    locals,
                    constants,
                    depth + 1,
                )
            }
            Expr::MethodCall(m)
                if m.method == "as_str" || m.method == "to_string" || m.method == "to_owned" =>
            {
                inner(&m.receiver, locals, constants, depth + 1)
            }
            Expr::Macro(m) if m.mac.path.is_ident("format") => {
                use syn::parse::Parser;
                let args = syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated
                    .parse2(m.mac.tokens.clone())
                    .ok()?;
                let mut value = inner(args.first()?, locals, constants, depth + 1)?;
                value.dynamic = true;
                Some(value)
            }
            _ => None,
        }
    }
    inner(expr, locals, constants, 0)
}

#[derive(Default)]
struct Bindings {
    values: HashMap<String, Expr>,
    ambiguous: HashSet<String>,
}

impl Bindings {
    fn add(&mut self, name: String, expr: &Expr, mutable: bool) {
        if mutable || self.values.insert(name.clone(), expr.clone()).is_some() {
            self.ambiguous.insert(name);
        }
    }

    fn finish(mut self) -> HashMap<String, Expr> {
        // Keep an opaque binding so an ambiguous local cannot fall back to a
        // same-named constant from another scope.
        for name in self.ambiguous {
            self.values.insert(name, Expr::Verbatim(Default::default()));
        }
        self.values
    }
}

impl<'ast> Visit<'ast> for Bindings {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if !super::test_only(&node.attrs) {
            visit::visit_item_mod(self, node);
        }
    }
    fn visit_item_fn(&mut self, _: &'ast syn::ItemFn) {}
    fn visit_impl_item_fn(&mut self, _: &'ast syn::ImplItemFn) {}
    fn visit_item_const(&mut self, node: &'ast syn::ItemConst) {
        self.add(node.ident.to_string(), &node.expr, false);
    }
    fn visit_local(&mut self, node: &'ast syn::Local) {
        let pat = match &node.pat {
            syn::Pat::Type(t) => &*t.pat,
            p => p,
        };
        if let syn::Pat::Ident(p) = pat {
            let expr = node
                .init
                .as_ref()
                .map(|i| (*i.expr).clone())
                .unwrap_or_else(|| Expr::Verbatim(Default::default()));
            self.add(p.ident.to_string(), &expr, p.mutability.is_some());
        }
        visit::visit_local(self, node);
    }
}

pub fn locals(block: &syn::Block, signature: &syn::Signature) -> HashMap<String, Expr> {
    let mut bindings = Bindings::default();
    for arg in &signature.inputs {
        if let syn::FnArg::Typed(arg) = arg {
            if let syn::Pat::Ident(p) = &*arg.pat {
                bindings.add(
                    p.ident.to_string(),
                    &Expr::Verbatim(Default::default()),
                    false,
                );
            }
        }
    }
    bindings.visit_block(block);
    bindings.finish()
}

pub fn constants(file: &syn::File) -> HashMap<String, Expr> {
    let mut bindings = Bindings::default();
    bindings.visit_file(file);
    bindings.finish()
}

pub fn looks_like_sql(text: &str) -> bool {
    let first = text
        .split_whitespace()
        .next()
        .unwrap_or("")
        .to_ascii_uppercase();
    matches!(
        first.as_str(),
        "SELECT"
            | "WITH"
            | "INSERT"
            | "UPDATE"
            | "DELETE"
            | "SET"
            | "RESET"
            | "LOAD"
            | "CREATE"
            | "ALTER"
            | "DROP"
            | "TRUNCATE"
            | "DO"
            | "BEGIN"
            | "COMMIT"
            | "ROLLBACK"
    )
}
