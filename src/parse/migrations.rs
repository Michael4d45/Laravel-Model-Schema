use std::borrow::Cow;
use std::path::{Path, PathBuf};

use mago_allocator::LocalArena;
use mago_database::file::File;
use mago_syntax::cst::{Call, Expression, Statement};
use mago_syntax::parser::parse_file;
use walkdir::WalkDir;

use crate::ir::Field;
use crate::Config;

use super::expr::{
    first_arg_string, identifier_short, innermost_call, is_variable, method_chain_names, nth_arg,
    selector_name, static_call_class_method,
};

pub fn parse_migrations(config: &Config, analysis: &mut crate::ir::Analysis) {
    let mut files = collect_php_files(&config.migration_paths);
    files.sort();
    for file in files {
        parse_migration_file(config, &file, analysis);
    }
}

fn collect_php_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in paths {
        if root.is_file() {
            files.push(root.clone());
            continue;
        }
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && path.extension().and_then(|e| e.to_str()) == Some("php") {
                files.push(path.to_path_buf());
            }
        }
    }
    files
}

fn parse_migration_file(config: &Config, path: &Path, analysis: &mut crate::ir::Analysis) {
    let Ok(contents) = std::fs::read(path) else {
        return;
    };
    if contents.is_empty() {
        return;
    }
    let arena = LocalArena::new();
    let file = File::ephemeral(
        Cow::Owned(path.to_string_lossy().into_owned().into_bytes()),
        Cow::Owned(contents),
    );
    let program = parse_file(&arena, &file);
    if program.has_errors() {
        return;
    }

    walk_expression_tree(&program.statements, &mut |expr| {
        let Some(call) = innermost_call(expr).or(match expr {
            Expression::Call(c) => Some(c),
            _ => None,
        }) else {
            return;
        };
        let Some((class_expr, method)) = static_call_class_method(call) else {
            return;
        };
        if identifier_short(class_expr).as_deref() != Some("Schema") {
            return;
        }
        if method != "create" && method != "table" {
            return;
        }
        let Some(table) = first_arg_string(call) else {
            return;
        };
        let Some(closure_expr) = nth_arg(call, 1) else {
            return;
        };
        let Expression::Closure(closure) = closure_expr else {
            return;
        };
        collect_blueprint(config, &table, &closure.body.statements, analysis);
    });
}

fn walk_expression_tree(
    statements: &mago_syntax::cst::Sequence<'_, Statement<'_>>,
    visit: &mut impl FnMut(&Expression<'_>),
) {
    super::expr::walk_statements(statements.iter(), visit);
}

fn collect_blueprint(
    config: &Config,
    table: &str,
    statements: &mago_syntax::cst::Sequence<'_, Statement<'_>>,
    analysis: &mut crate::ir::Analysis,
) {
    for statement in statements {
        let Statement::Expression(expr_stmt) = statement else {
            continue;
        };
        record_column(config, table, expr_stmt.expression, analysis);
    }
}

fn record_column(
    config: &Config,
    table: &str,
    expr: &Expression<'_>,
    analysis: &mut crate::ir::Analysis,
) {
    if !chain_starts_with_table(expr) {
        return;
    }
    let names = method_chain_names(expr);
    let nullable = names.iter().any(|n| n == "nullable");
    let first_method = innermost_call(expr)
        .and_then(|call| match call {
            Call::Method(m) => selector_name(&m.method),
            _ => None,
        })
        .or_else(|| names.last().cloned());

    if let Some(column) = column_name_from_chain(config, expr) {
        let type_method = first_column_method(config, expr).unwrap_or_else(|| "mixed".into());
        let mapped = config
            .column_type_mappings
            .get(&type_method)
            .cloned()
            .unwrap_or_else(|| "mixed".into());
        analysis.table_mut(table).migration_fields.insert(
            column.clone(),
            Field {
                name: column,
                r#type: mapped,
                nullable,
            },
        );
        return;
    }

    let Some(method) = first_method else {
        return;
    };
    let report = analysis.table_mut(table);
    if let Some(macro_) = config.migrations.column_macros.get(&method) {
        for name in &macro_.columns {
            report.migration_fields.insert(
                name.clone(),
                Field {
                    name: name.clone(),
                    r#type: macro_.r#type.clone(),
                    nullable: macro_.nullable,
                },
            );
        }
        return;
    }
    if let Some(morph) = config.migrations.morph_macros.get(&method) {
        insert_morph_columns(report, expr, &method, &morph.id_type, morph.nullable);
    }
}

fn insert_morph_columns(
    report: &mut crate::ir::TableAnalysis,
    expr: &Expression<'_>,
    method: &str,
    id_type: &str,
    nullable: bool,
) {
    let Some(name) = first_string_arg_for_method(expr, method) else {
        return;
    };
    for (suffix, r#type) in [("_type", "string"), ("_id", id_type)] {
        let column = format!("{name}{suffix}");
        report.migration_fields.insert(
            column.clone(),
            Field {
                name: column,
                r#type: r#type.into(),
                nullable,
            },
        );
    }
}

fn chain_starts_with_table(expr: &Expression<'_>) -> bool {
    let mut current = expr;
    loop {
        match current {
            Expression::Call(Call::Method(m)) => current = m.object,
            Expression::Call(Call::NullSafeMethod(m)) => current = m.object,
            Expression::Parenthesized(inner) => current = inner.expression,
            other => return is_variable(other, "table"),
        }
    }
}

fn column_name_from_chain(config: &Config, expr: &Expression<'_>) -> Option<String> {
    let mut current = expr;
    loop {
        match current {
            Expression::Call(Call::Method(m)) => {
                if let Some(name) = selector_name(&m.method) {
                    if config.column_type_mappings.contains_key(&name) {
                        if let Some(col) = first_arg_string(&Call::Method(m.clone())) {
                            return Some(col);
                        }
                        if let Some(col) = config.migrations.default_column_names.get(&name) {
                            return Some(col.clone());
                        }
                    }
                }
                current = m.object;
            }
            Expression::Parenthesized(inner) => current = inner.expression,
            _ => return None,
        }
    }
}

fn first_string_arg_for_method(expr: &Expression<'_>, method: &str) -> Option<String> {
    let mut current = expr;
    loop {
        match current {
            Expression::Call(Call::Method(m)) => {
                if selector_name(&m.method).as_deref() == Some(method) {
                    return first_arg_string(&Call::Method(m.clone()));
                }
                current = m.object;
            }
            Expression::Call(Call::NullSafeMethod(m)) => current = m.object,
            Expression::Parenthesized(inner) => current = inner.expression,
            _ => return None,
        }
    }
}

fn first_column_method(config: &Config, expr: &Expression<'_>) -> Option<String> {
    let mut current = expr;
    let mut last = None;
    loop {
        match current {
            Expression::Call(Call::Method(m)) => {
                if let Some(name) = selector_name(&m.method) {
                    if config.column_type_mappings.contains_key(&name) {
                        last = Some(name);
                    }
                }
                current = m.object;
            }
            Expression::Parenthesized(inner) => current = inner.expression,
            _ => return last,
        }
    }
}
