//! Syntax-derived SQL call sites. Inventory is not execution coverage.

mod resolve;

use quote::ToTokens;
use serde::Serialize;
use std::collections::HashMap;
use std::path::Path;
use syn::spanned::Spanned;
use syn::visit::{self, Visit};
use syn::{Expr, ExprCall, ImplItemFn, ItemFn};

#[derive(Debug, Serialize)]
pub struct QuerySite {
    pub file: String,
    pub line: usize,
    pub column: usize,
    pub function: String,
    pub api: String,
    pub expression: String,
    pub sql: Option<String>,
    pub template: Option<String>,
    pub direct_bind_count: Option<usize>,
}

fn test_only(attrs: &[syn::Attribute]) -> bool {
    attrs.iter().any(|a| {
        let text = a.meta.to_token_stream().to_string();
        text == "cfg (test)" || text == "test" || text == "tokio :: test"
    })
}

struct Catalog {
    file: String,
    function: String,
    constants: HashMap<String, Expr>,
    locals: HashMap<String, Expr>,
    sites: Vec<QuerySite>,
}

impl Catalog {
    fn record(&mut self, api: &str, expr: &Expr, location: proc_macro2::LineColumn) {
        let resolved = resolve::resolve(expr, &self.locals, &self.constants);
        self.sites.push(QuerySite {
            file: self.file.clone(),
            line: location.line,
            column: location.column,
            function: self.function.clone(),
            api: api.to_string(),
            expression: expr.to_token_stream().to_string(),
            sql: resolved
                .as_ref()
                .filter(|s| !s.dynamic)
                .map(|s| s.text.clone()),
            template: resolved.filter(|s| s.dynamic).map(|s| s.text),
            direct_bind_count: None,
        });
    }
}

impl<'ast> Visit<'ast> for Catalog {
    fn visit_item_mod(&mut self, node: &'ast syn::ItemMod) {
        if !test_only(&node.attrs) {
            visit::visit_item_mod(self, node);
        }
    }

    fn visit_item_fn(&mut self, node: &'ast ItemFn) {
        if test_only(&node.attrs) {
            return;
        }
        let previous = std::mem::replace(&mut self.function, node.sig.ident.to_string());
        let locals = std::mem::replace(&mut self.locals, resolve::locals(&node.block, &node.sig));
        visit::visit_item_fn(self, node);
        self.locals = locals;
        self.function = previous;
    }

    fn visit_impl_item_fn(&mut self, node: &'ast ImplItemFn) {
        if test_only(&node.attrs) {
            return;
        }
        let previous = std::mem::replace(&mut self.function, node.sig.ident.to_string());
        let locals = std::mem::replace(&mut self.locals, resolve::locals(&node.block, &node.sig));
        visit::visit_impl_item_fn(self, node);
        self.locals = locals;
        self.function = previous;
    }

    fn visit_expr_call(&mut self, node: &'ast ExprCall) {
        if let Expr::Path(p) = &*node.func {
            let last = p.path.segments.last().map(|s| s.ident.to_string());
            let builder = p.path.segments.iter().any(|s| s.ident == "QueryBuilder");
            if matches!(
                last.as_deref(),
                Some(
                    "query"
                        | "query_as"
                        | "query_scalar"
                        | "query_with"
                        | "query_as_with"
                        | "query_scalar_with"
                        | "raw_sql"
                )
            ) || (builder && last.as_deref() == Some("new"))
            {
                if let Some(arg) = node.args.first() {
                    self.record(
                        &p.path.to_token_stream().to_string(),
                        arg,
                        node.span().start(),
                    );
                }
            }
        }
        visit::visit_expr_call(self, node);
    }

    fn visit_expr_method_call(&mut self, node: &'ast syn::ExprMethodCall) {
        // Executor::execute("SQL") bypasses sqlx::query. Exclude .execute(&pool).
        if node.method == "execute" {
            if let Some(arg) = node.args.first() {
                if let Some(value) = resolve::resolve(arg, &self.locals, &self.constants) {
                    if resolve::looks_like_sql(&value.text) {
                        self.record("Executor::execute", arg, node.span().start());
                    }
                }
            }
        }
        visit::visit_expr_method_call(self, node);
        if matches!(
            node.method.to_string().as_str(),
            "execute"
                | "execute_many"
                | "fetch"
                | "fetch_many"
                | "fetch_one"
                | "fetch_all"
                | "fetch_optional"
        ) {
            if let Some((location, count)) = direct_bind_chain(&node.receiver) {
                if let Some(site) = self
                    .sites
                    .iter_mut()
                    .find(|s| s.line == location.line && s.column == location.column)
                {
                    site.direct_bind_count = Some(count);
                }
            }
        }
    }

    fn visit_expr_macro(&mut self, node: &'ast syn::ExprMacro) {
        let name = node.mac.path.segments.last().map(|s| s.ident.to_string());
        if matches!(
            name.as_deref(),
            Some("query" | "query_as" | "query_scalar" | "query_file" | "query_as_file")
        ) {
            if let Ok(args) = syn::parse::Parser::parse2(
                syn::punctuated::Punctuated::<Expr, syn::Token![,]>::parse_terminated,
                node.mac.tokens.clone(),
            ) {
                // query_as! takes its result type as the first argument.
                let index = usize::from(matches!(
                    name.as_deref(),
                    Some("query_as" | "query_as_file")
                ));
                if let Some(arg) = args.iter().nth(index) {
                    self.record(&format!("{}!", name.unwrap()), arg, node.span().start());
                }
            }
        }
        visit::visit_expr_macro(self, node);
    }
}

fn direct_bind_chain(expr: &Expr) -> Option<(proc_macro2::LineColumn, usize)> {
    let mut expr = expr;
    let mut count = 0;
    while let Expr::MethodCall(call) = expr {
        if call.method == "bind" || call.method == "try_bind" {
            count += 1;
        }
        expr = &call.receiver;
    }
    if let Expr::Call(call) = expr {
        if let Expr::Path(path) = &*call.func {
            if path.path.segments.last().is_some_and(|s| {
                matches!(
                    s.ident.to_string().as_str(),
                    "query" | "query_as" | "query_scalar" | "raw_sql"
                )
            }) {
                return Some((call.span().start(), count));
            }
        }
    }
    None
}

pub fn scan_source(file: &str, source: &str) -> Vec<QuerySite> {
    let ast = syn::parse_file(source).unwrap_or_else(|e| panic!("parse {file}: {e}"));
    let mut catalog = Catalog {
        file: file.to_string(),
        function: "<module>".to_string(),
        constants: resolve::constants(&ast),
        locals: HashMap::new(),
        sites: Vec::new(),
    };
    catalog.visit_file(&ast);
    catalog.sites
}

pub fn inventory(root: &Path) -> Vec<QuerySite> {
    fn walk(root: &Path, dir: &Path, out: &mut Vec<QuerySite>) {
        let mut entries: Vec<_> = std::fs::read_dir(dir)
            .unwrap_or_else(|e| panic!("read {}: {e}", dir.display()))
            .map(|e| e.expect("directory entry").path())
            .collect();
        entries.sort();
        for path in entries {
            if path.is_dir() {
                // Separate SQLite backend has a different SQL dialect.
                if path.file_name().is_some_and(|n| n == "sqlite") {
                    continue;
                }
                walk(root, &path, out);
            } else if path.extension().is_some_and(|e| e == "rs") {
                let file = path
                    .strip_prefix(root)
                    .expect("source under root")
                    .to_string_lossy();
                let source = std::fs::read_to_string(&path).expect("read source");
                out.extend(scan_source(&file, &source));
            }
        }
    }
    let mut sites = Vec::new();
    let crates = root.join("edgequake/crates");
    let mut dirs: Vec<_> = std::fs::read_dir(&crates)
        .expect("crates")
        .map(|e| e.expect("crate entry").path().join("src"))
        .filter(|p| p.is_dir())
        .collect();
    dirs.sort();
    for dir in dirs {
        walk(root, &dir, &mut sites);
    }
    walk(root, &root.join("edgequake/src"), &mut sites);
    sites
}
