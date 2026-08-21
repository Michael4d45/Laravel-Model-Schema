use std::borrow::Cow;
use std::collections::{BTreeMap, HashMap, HashSet};
use std::path::{Path, PathBuf};

use mago_allocator::LocalArena;
use mago_database::file::File;
use mago_names::resolver::NameResolver;
use mago_names::ResolvedNames;
use mago_span::HasSpan;
use mago_syntax::cst::{
    Call, Class, ClassLikeMember, Expression, Method, MethodBody, Program, Property, Statement,
};
use mago_syntax::parser::parse_file;
use walkdir::WalkDir;

use crate::ir::{ModelField, Relationship, TableAnalysis};
use crate::Config;

use super::expr::{
    array_string_map, array_string_values, class_const_name, first_arg_string, hint_to_string,
    property_value, selector_name, variable_name, walk_expression,
};
use super::names::{
    guess_table_name, owned_name, resolve_identifier, resolve_local, short_name, snake_case,
};
use super::phpdoc::{class_doc_between, resolve_phpdoc_class};
use super::uses::collect_use_map;

const RELATION_RETURNS: &[&str] = &[
    "BelongsTo",
    "HasMany",
    "HasOne",
    "BelongsToMany",
    "MorphTo",
    "MorphMany",
    "MorphToMany",
    "MorphOne",
    "HasManyThrough",
    "HasOneThrough",
    "Relation",
];

const RELATION_CALLS: &[(&str, &str)] = &[
    ("belongsTo", "BelongsTo"),
    ("hasMany", "HasMany"),
    ("hasOne", "HasOne"),
    ("belongsToMany", "BelongsToMany"),
    ("morphTo", "MorphTo"),
    ("morphMany", "MorphMany"),
    ("morphToMany", "MorphToMany"),
    ("morphOne", "MorphOne"),
    ("hasManyThrough", "HasManyThrough"),
    ("hasOneThrough", "HasOneThrough"),
];

#[derive(Clone)]
struct ParsedClass {
    namespace: String,
    fqcn: String,
    file: PathBuf,
    uses: BTreeMap<String, String>,
    parent: Option<String>,
    table: Option<String>,
    fillable: Vec<String>,
    hidden: Vec<String>,
    casts: Vec<(String, String)>,
    accessors: Vec<String>,
    relationships: Vec<Relationship>,
    phpdoc: super::phpdoc::ClassDoc,
}

pub fn parse_models(config: &Config, analysis: &mut crate::ir::Analysis) {
    let files = collect_php_files(&config.model_paths);
    let mut parsed = Vec::new();
    for file in &files {
        parsed.extend(parse_file_classes(file));
    }

    let by_fqcn: HashMap<String, ParsedClass> =
        parsed.iter().map(|c| (c.fqcn.clone(), c.clone())).collect();

    for class in parsed {
        if !is_eloquent_model(&class, &by_fqcn, config) {
            continue;
        }
        let table = table_name_for(config, &class);
        let report = analysis.table_mut(&table);
        merge_model(report, &class);
        report.model_file = Some(class.file.clone());
        report.model_fqcn = Some(class.fqcn.clone());
    }
}

fn merge_model(report: &mut TableAnalysis, class: &ParsedClass) {
    for name in &class.fillable {
        let field = report
            .model_fields
            .entry(name.clone())
            .or_insert_with(|| ModelField {
                name: name.clone(),
                ..Default::default()
            });
        field.fillable = true;
    }
    for name in &class.hidden {
        let field = report
            .model_fields
            .entry(name.clone())
            .or_insert_with(|| ModelField {
                name: name.clone(),
                ..Default::default()
            });
        field.hidden = true;
    }
    for (name, cast) in &class.casts {
        let field = report
            .model_fields
            .entry(name.clone())
            .or_insert_with(|| ModelField {
                name: name.clone(),
                ..Default::default()
            });
        field.cast = Some(cast.clone());
    }
    for name in &class.accessors {
        let field = report
            .model_fields
            .entry(name.clone())
            .or_insert_with(|| ModelField {
                name: name.clone(),
                ..Default::default()
            });
        field.accessor = true;
    }
    for rel in &class.relationships {
        report
            .model_relationships
            .insert(rel.name.clone(), rel.clone());
    }
    for mut field in class.phpdoc.properties.clone() {
        if is_probably_class(&field.r#type) {
            field.r#type = resolve_phpdoc_class(&field.r#type, &class.uses, &class.namespace);
        }
        report.phpdoc_fields.insert(field.name.clone(), field);
    }
    for mut field in class.phpdoc.reads.clone() {
        if is_probably_class(&field.r#type) {
            field.r#type = resolve_phpdoc_class(&field.r#type, &class.uses, &class.namespace);
        }
        report.phpdoc_read_fields.insert(field.name.clone(), field);
    }
}

fn is_probably_class(ty: &str) -> bool {
    ty.contains('\\') || ty.chars().next().is_some_and(|c| c.is_uppercase())
}

fn table_name_for(config: &Config, class: &ParsedClass) -> String {
    if let Some(table) = &class.table {
        return table.clone();
    }
    if let Some(table) = config.table_default_for(&class.fqcn) {
        return table;
    }
    if let Some(parent) = &class.parent {
        if let Some(table) = config.table_default_for(parent) {
            return table;
        }
    }
    guess_table_name(&class.fqcn)
}

fn is_eloquent_model(
    class: &ParsedClass,
    by_fqcn: &HashMap<String, ParsedClass>,
    config: &Config,
) -> bool {
    let mut current = class.parent.as_deref();
    let mut seen = HashSet::new();
    while let Some(parent) = current {
        if !seen.insert(parent.to_string()) {
            break;
        }
        if config.is_model_parent(parent) {
            return true;
        }
        current = by_fqcn.get(parent).and_then(|c| c.parent.as_deref());
    }
    false
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
    files.sort();
    files
}

fn parse_file_classes(path: &Path) -> Vec<ParsedClass> {
    let Ok(contents) = std::fs::read(path) else {
        return Vec::new();
    };
    if contents.is_empty() {
        return Vec::new();
    }
    let arena = LocalArena::new();
    let file = File::ephemeral(
        Cow::Owned(path.to_string_lossy().into_owned().into_bytes()),
        Cow::Owned(contents),
    );
    let program = parse_file(&arena, &file);
    if program.has_errors() {
        return Vec::new();
    }
    let resolver = NameResolver::new(&arena);
    let resolved = resolver.resolve(program);
    let uses = collect_use_map(&program.statements);
    let mut out = Vec::new();
    collect_classes(
        path,
        &arena,
        program,
        &resolved,
        uses,
        &program.statements,
        None,
        &mut out,
    );
    out
}

#[allow(clippy::too_many_arguments)]
fn collect_classes(
    path: &Path,
    arena: &LocalArena,
    program: &Program<'_>,
    resolved: &ResolvedNames<'_>,
    uses: BTreeMap<String, String>,
    statements: &mago_syntax::cst::Sequence<'_, Statement<'_>>,
    namespace: Option<&str>,
    out: &mut Vec<ParsedClass>,
) {
    for statement in statements {
        match statement {
            Statement::Namespace(ns) => {
                let name = ns.name.as_ref().and_then(|ident| owned_name(ident.value()));
                collect_classes(
                    path,
                    arena,
                    program,
                    resolved,
                    uses.clone(),
                    ns.statements(),
                    name.as_deref().or(namespace),
                    out,
                );
            }
            Statement::Block(block) => {
                collect_classes(
                    path,
                    arena,
                    program,
                    resolved,
                    uses.clone(),
                    &block.statements,
                    namespace,
                    out,
                );
            }
            Statement::Class(class) => {
                if let Some(parsed) = class_from_cst(
                    path,
                    arena,
                    program,
                    resolved,
                    uses.clone(),
                    class,
                    namespace,
                ) {
                    out.push(parsed);
                }
            }
            _ => {}
        }
    }
}

fn class_from_cst(
    path: &Path,
    arena: &LocalArena,
    program: &Program<'_>,
    resolved: &ResolvedNames<'_>,
    uses: BTreeMap<String, String>,
    class: &Class<'_>,
    namespace: Option<&str>,
) -> Option<ParsedClass> {
    let fqcn = resolve_local(resolved, &class.name, namespace)?;
    let ns = namespace.unwrap_or("").to_string();
    let parent = class
        .extends
        .as_ref()
        .and_then(|ext| ext.types.nodes.first())
        .map(|ident| resolve_identifier(resolved, ident, namespace));
    let phpdoc = class_doc_between(arena, program, 0, class.span().start.offset);

    let mut parsed = ParsedClass {
        namespace: ns,
        fqcn,
        file: path.to_path_buf(),
        uses,
        parent,
        table: None,
        fillable: Vec::new(),
        hidden: Vec::new(),
        casts: Vec::new(),
        accessors: Vec::new(),
        relationships: Vec::new(),
        phpdoc,
    };

    for member in class.members.iter() {
        match member {
            ClassLikeMember::Property(property) => {
                apply_property(&mut parsed, property, resolved, namespace);
            }
            ClassLikeMember::Method(method) => {
                apply_method(&mut parsed, method, resolved, namespace);
            }
            _ => {}
        }
    }

    Some(parsed)
}

fn apply_property(
    parsed: &mut ParsedClass,
    property: &Property<'_>,
    resolved: &ResolvedNames<'_>,
    namespace: Option<&str>,
) {
    let items = match property {
        Property::Plain(plain) => plain.items.iter().collect::<Vec<_>>(),
        Property::Hooked(hooked) => vec![&hooked.item],
    };
    for item in items {
        let name = variable_name(item.variable().name);
        let Some(value) = property_value(item) else {
            continue;
        };
        match name.as_str() {
            "table" => parsed.table = super::expr::literal_string(value),
            "fillable" => parsed.fillable = array_string_values(value),
            "hidden" => parsed.hidden = array_string_values(value),
            "casts" => parsed.casts = array_string_map(value, resolved, namespace),
            _ => {}
        }
    }
}

fn apply_method(
    parsed: &mut ParsedClass,
    method: &Method<'_>,
    resolved: &ResolvedNames<'_>,
    namespace: Option<&str>,
) {
    let name = owned_name(method.name.value).unwrap_or_default();
    if name == "casts" {
        if let Some(expr) = method_return_expr(method) {
            parsed.casts = array_string_map(expr, resolved, namespace);
        }
        return;
    }
    if let Some(rest) = name.strip_prefix("get") {
        if let Some(rest) = rest.strip_suffix("Attribute") {
            if !rest.is_empty() {
                parsed.accessors.push(snake_case(rest));
            }
        }
    }

    let return_name = method
        .return_type_hint
        .as_ref()
        .map(|hint| hint_to_string(&hint.hint, resolved, namespace));
    let return_short = return_name.as_deref().map(short_name);

    let mut rel_type = return_short
        .filter(|s| RELATION_RETURNS.iter().any(|r| r.eq_ignore_ascii_case(s)))
        .map(|s| s.to_string());
    let mut related = String::from("mixed");

    if let MethodBody::Concrete(body) = &method.body {
        let mut found_call: Option<(String, String)> = None;
        for statement in body.statements.iter() {
            let expr = match statement {
                Statement::Return(ret) => ret.value,
                Statement::Expression(stmt) => Some(stmt.expression),
                _ => None,
            };
            let Some(expr) = expr else {
                continue;
            };
            walk_expression(expr, &mut |inner| {
                if found_call.is_some() {
                    return;
                }
                let Expression::Call(call @ Call::Method(m)) = inner else {
                    return;
                };
                let Some(method_name) = selector_name(&m.method) else {
                    return;
                };
                if let Some((_, ty)) = RELATION_CALLS
                    .iter()
                    .find(|(c, _)| c.eq_ignore_ascii_case(&method_name))
                {
                    let model = super::expr::nth_arg(call, 0)
                        .and_then(|arg| class_const_name(arg, resolved, namespace))
                        .or_else(|| first_arg_string(call))
                        .unwrap_or_else(|| "mixed".into());
                    found_call = Some(((*ty).to_string(), model));
                }
            });
        }
        if let Some((ty, model)) = found_call {
            if rel_type.is_none() {
                rel_type = Some(ty);
            }
            related = model;
        }
    }

    if let Some(rel_type) = rel_type {
        if related == "mixed" && rel_type != "MorphTo" {
            related = "mixed".into();
        }
        parsed.relationships.push(Relationship {
            name,
            r#type: rel_type,
            model: related,
        });
    }
}

fn method_return_expr<'a>(method: &'a Method<'a>) -> Option<&'a Expression<'a>> {
    let MethodBody::Concrete(body) = &method.body else {
        return None;
    };
    for statement in body.statements.iter() {
        if let Statement::Return(ret) = statement {
            if let Some(value) = ret.value {
                return Some(value);
            }
        }
    }
    None
}
