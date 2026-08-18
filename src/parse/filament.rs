use std::borrow::Cow;
use std::path::{Path, PathBuf};

use mago_allocator::LocalArena;
use mago_database::file::File;
use mago_names::resolver::NameResolver;
use mago_names::ResolvedNames;
use mago_syntax::cst::{ClassLikeMember, Expression, Property, Statement};
use mago_syntax::parser::parse_file;
use walkdir::WalkDir;

use crate::ir::Field;
use crate::Config;

use super::expr::{
    class_const_name, first_arg_string, identifier_short, innermost_call, method_chain_names,
    property_value, static_call_class_method, variable_name,
};
use super::names::{guess_table_name, owned_name, resolve_identifier, resolve_local, short_name};

pub fn parse_filament(config: &Config, analysis: &mut crate::ir::Analysis) {
    let files = collect_resource_files(&config.resource_paths);
    let component_refs = normalize_components(&config.form_component_classes);

    for resource_file in &files {
        let Some(model_fqcn) = resource_model_fqcn(resource_file) else {
            continue;
        };
        let table = table_for_model(analysis, &model_fqcn);
        let schema_dir = resource_file
            .parent()
            .map(|p| p.join("Schemas"))
            .unwrap_or_else(|| resource_file.clone());
        for form_file in gather_suffix_files(&schema_dir, "Form.php") {
            collect_form_fields(config, &form_file, &component_refs, &table, analysis);
        }
    }
}

fn table_for_model(analysis: &crate::ir::Analysis, model_fqcn: &str) -> String {
    if let Some((table, _)) = analysis.tables.iter().find(|(_, report)| {
        report.model_fqcn.as_deref() == Some(model_fqcn)
            || report.model_fqcn.as_deref().map(short_name) == Some(short_name(model_fqcn))
    }) {
        return table.clone();
    }
    guess_table_name(model_fqcn)
}

fn collect_resource_files(paths: &[PathBuf]) -> Vec<PathBuf> {
    let mut files = Vec::new();
    for root in paths {
        if root.is_file() {
            if is_resource_file(root) {
                files.push(root.clone());
            }
            continue;
        }
        if !root.is_dir() {
            continue;
        }
        for entry in WalkDir::new(root).into_iter().filter_map(|e| e.ok()) {
            let path = entry.path();
            if path.is_file() && is_resource_file(path) {
                files.push(path.to_path_buf());
            }
        }
    }
    files.sort();
    files
}

fn is_resource_file(path: &Path) -> bool {
    let name = path.file_name().and_then(|n| n.to_str()).unwrap_or("");
    path.extension().and_then(|e| e.to_str()) == Some("php")
        && name.ends_with("Resource.php")
        && !path.to_string_lossy().contains("RelationManagers")
}

fn gather_suffix_files(directory: &Path, suffix: &str) -> Vec<PathBuf> {
    if !directory.is_dir() {
        return Vec::new();
    }
    let mut files = Vec::new();
    for entry in WalkDir::new(directory).into_iter().filter_map(|e| e.ok()) {
        let path = entry.path();
        if path.is_file() {
            if let Some(name) = path.file_name().and_then(|n| n.to_str()) {
                if name.ends_with(suffix) {
                    files.push(path.to_path_buf());
                }
            }
        }
    }
    files
}

fn normalize_components(classes: &[String]) -> Vec<String> {
    let mut out = Vec::new();
    for class in classes {
        let trimmed = class.trim_start_matches('\\');
        out.push(trimmed.to_string());
        out.push(short_name(trimmed).to_string());
    }
    out.sort();
    out.dedup();
    out
}

fn resource_model_fqcn(path: &Path) -> Option<String> {
    let contents = std::fs::read(path).ok()?;
    if contents.is_empty() {
        return None;
    }
    let arena = LocalArena::new();
    let file = File::ephemeral(
        Cow::Owned(path.to_string_lossy().into_owned().into_bytes()),
        Cow::Owned(contents),
    );
    let program = parse_file(&arena, &file);
    if program.has_errors() {
        return None;
    }
    let resolver = NameResolver::new(&arena);
    let resolved = resolver.resolve(program);
    find_model_property(&program.statements, None, &resolved)
}

fn find_model_property(
    statements: &mago_syntax::cst::Sequence<'_, Statement<'_>>,
    namespace: Option<&str>,
    resolved: &ResolvedNames<'_>,
) -> Option<String> {
    for statement in statements {
        match statement {
            Statement::Namespace(ns) => {
                let name = ns.name.as_ref().and_then(|ident| owned_name(ident.value()));
                if let Some(found) =
                    find_model_property(ns.statements(), name.as_deref().or(namespace), resolved)
                {
                    return Some(found);
                }
            }
            Statement::Class(class) => {
                let _ = resolve_local(resolved, &class.name, namespace);
                for member in class.members.iter() {
                    let ClassLikeMember::Property(property) = member else {
                        continue;
                    };
                    if let Some(found) = model_from_property(property, resolved, namespace) {
                        return Some(found);
                    }
                }
            }
            _ => {}
        }
    }
    None
}

fn model_from_property(
    property: &Property<'_>,
    resolved: &ResolvedNames<'_>,
    namespace: Option<&str>,
) -> Option<String> {
    let items = match property {
        Property::Plain(plain) => plain.items.iter().collect::<Vec<_>>(),
        Property::Hooked(hooked) => vec![&hooked.item],
    };
    for item in items {
        if variable_name(item.variable().name) != "model" {
            continue;
        }
        let value = property_value(item)?;
        return class_const_name(value, resolved, namespace);
    }
    None
}

fn collect_form_fields(
    config: &Config,
    path: &Path,
    component_refs: &[String],
    table: &str,
    analysis: &mut crate::ir::Analysis,
) {
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
    let resolver = NameResolver::new(&arena);
    let resolved = resolver.resolve(program);

    super::expr::walk_statements(program.statements.iter(), &mut |expr| {
        collect_make(config, expr, component_refs, &resolved, table, analysis);
    });
}

fn collect_make(
    config: &Config,
    expr: &Expression<'_>,
    component_refs: &[String],
    resolved: &ResolvedNames<'_>,
    table: &str,
    analysis: &mut crate::ir::Analysis,
) {
    let names = method_chain_names(expr);
    let required = names.iter().any(|n| {
        config
            .resource_component_required_methods
            .iter()
            .any(|r| r == n)
    });
    let Some(call) = innermost_call(expr).or(match expr {
        Expression::Call(c) => Some(c),
        _ => None,
    }) else {
        return;
    };
    let Some((class_expr, method)) = static_call_class_method(call) else {
        return;
    };
    if method != "make" {
        return;
    }
    let Some(short) = identifier_short(class_expr) else {
        return;
    };
    let fqcn = match class_expr {
        Expression::Identifier(ident) => resolve_identifier(resolved, ident, None),
        _ => short.clone(),
    };
    let known = component_refs.iter().any(|c| {
        c == &short
            || c == &fqcn
            || fqcn.ends_with(&format!("\\{c}"))
            || c.ends_with(&format!("\\{short}"))
    });
    if !known && !fqcn.contains("Filament\\Forms\\Components") {
        return;
    }
    let Some(field_name) = first_arg_string(call) else {
        return;
    };
    let fields = &mut analysis.table_mut(table).filament_form_fields;
    if let Some(existing) = fields.get_mut(&field_name) {
        if required {
            existing.nullable = false;
        }
        return;
    }
    let field_type = infer_type(config, &fqcn, &short);
    fields.insert(
        field_name.clone(),
        Field {
            name: field_name,
            r#type: field_type,
            nullable: !required,
        },
    );
}

fn infer_type(config: &Config, fqcn: &str, short: &str) -> String {
    config
        .resource_component_type_map
        .get(fqcn)
        .or_else(|| config.resource_component_type_map.get(&format!("\\{fqcn}")))
        .or_else(|| config.resource_component_type_map.get(short))
        .cloned()
        .unwrap_or_else(|| "string".into())
}
