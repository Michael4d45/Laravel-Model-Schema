use std::collections::BTreeMap;

use serde::Serialize;
use serde_json::{Map, Value};

use crate::config::Check;
use crate::ir::{Analysis, Field, TableAnalysis};
use crate::parse::{camel_case, short_name};
use crate::Config;

#[derive(Debug, Clone, Default, Serialize)]
pub struct Report {
    pub add_fields_to_filament_form: BTreeMap<String, BTreeMap<String, Field>>,
    pub remove_fields_from_filament_form: BTreeMap<String, BTreeMap<String, Field>>,
    pub add_filament_resources: Vec<String>,
    pub remove_filament_resources: Vec<String>,
    pub add_fields_to_models: BTreeMap<String, BTreeMap<String, Field>>,
    pub remove_fields_from_models: BTreeMap<String, BTreeMap<String, Field>>,
    pub add_models: Vec<String>,
    pub remove_models: Vec<String>,
    pub add_fields_to_model_docs: BTreeMap<String, BTreeMap<String, Field>>,
    pub remove_fields_from_model_docs: BTreeMap<String, BTreeMap<String, Field>>,
    pub wrong_model_doc_types: BTreeMap<String, BTreeMap<String, WrongType>>,
    pub wrong_relationship_phpdoc_read_types:
        BTreeMap<String, BTreeMap<String, WrongRelationshipRead>>,
    pub should_be_camel_case_relationship:
        BTreeMap<String, BTreeMap<String, WrongRelationshipName>>,
    pub add_property_read: BTreeMap<String, BTreeMap<String, Field>>,
    pub morph_to_relationships: BTreeMap<String, BTreeMap<String, MorphToInfo>>,
    pub model_evidence_conflicts: BTreeMap<String, BTreeMap<String, WrongType>>,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
#[serde(rename_all = "camelCase")]
pub struct WrongType {
    pub field_name: String,
    pub expected_type: String,
    pub actual_type: String,
    pub expected_nullable: bool,
    pub actual_nullable: bool,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WrongRelationshipRead {
    pub relationship_name: String,
    pub relationship_type: String,
    pub related_model: String,
    pub phpdoc_type: String,
    pub nullable: bool,
    pub issue_codes: Vec<String>,
    pub summary: String,
    pub suggested_phpdoc_type: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct WrongRelationshipName {
    #[serde(rename = "relationshipName")]
    pub relationship_name: String,
    #[serde(rename = "expectedName")]
    pub expected_name: String,
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct MorphToInfo {
    pub relationship_type: String,
    pub has_phpdoc_read: bool,
    pub phpdoc_type: Option<String>,
    pub nullable: Option<bool>,
}

pub fn generate_report(config: &Config, analysis: &Analysis) -> Report {
    let mut report = Report::default();

    for (table, resource) in &analysis.tables {
        if config.check_enabled(Check::Filament) && !config.ignores_table(Check::Filament, table) {
            add_remove_filament(config, table, resource, &mut report);
        }
        if config.check_enabled(Check::Models) && !config.ignores_table(Check::Models, table) {
            add_remove_models(config, table, resource, &mut report);
        }
        if config.check_enabled(Check::Phpdoc) && !config.ignores_table(Check::Phpdoc, table) {
            phpdoc_sections(config, table, resource, &mut report);
        }
    }

    report
}

fn add_remove_filament(
    config: &Config,
    table: &str,
    resource: &TableAnalysis,
    report: &mut Report,
) {
    let mut to_add = BTreeMap::new();
    for (name, field) in &resource.migration_fields {
        if config.ignores_field(Check::Filament, table, name) {
            continue;
        }
        if resource.filament_form_fields.contains_key(name) {
            continue;
        }
        to_add.insert(name.clone(), field.clone());
    }
    if !to_add.is_empty() {
        report
            .add_fields_to_filament_form
            .insert(table.into(), to_add);
    }

    let mut to_remove = BTreeMap::new();
    for (name, field) in &resource.filament_form_fields {
        if config.ignores_field(Check::Filament, table, name) {
            continue;
        }
        if resource.migration_fields.contains_key(name) {
            continue;
        }
        to_remove.insert(name.clone(), field.clone());
    }
    if !to_remove.is_empty() {
        report
            .remove_fields_from_filament_form
            .insert(table.into(), to_remove);
    }

    if resource.filament_form_fields.is_empty() {
        report.add_filament_resources.push(table.into());
    }
}

fn add_remove_models(config: &Config, table: &str, resource: &TableAnalysis, report: &mut Report) {
    let mut to_add = BTreeMap::new();
    for (name, field) in &resource.migration_fields {
        if config.ignores_field(Check::Models, table, name) {
            continue;
        }
        let has_model =
            resource.model_fields.contains_key(name) || resource.phpdoc_fields.contains_key(name);
        if has_model {
            continue;
        }
        to_add.insert(name.clone(), field.clone());
    }
    if !to_add.is_empty() {
        report.add_fields_to_models.insert(table.into(), to_add);
    }

    let mut to_remove = BTreeMap::new();
    for (name, field) in &resource.model_fields {
        if field.accessor {
            continue;
        }
        if config.ignores_field(Check::Models, table, name) {
            continue;
        }
        if resource.migration_fields.contains_key(name) {
            continue;
        }
        to_remove.insert(
            name.clone(),
            Field {
                name: name.clone(),
                r#type: field.cast.clone().unwrap_or_else(|| "mixed".into()),
                nullable: false,
            },
        );
    }
    if !to_remove.is_empty() {
        report
            .remove_fields_from_models
            .insert(table.into(), to_remove);
    }

    if resource.model_fqcn.is_none() {
        report.add_models.push(table.into());
    }
}

fn phpdoc_sections(config: &Config, table: &str, resource: &TableAnalysis, report: &mut Report) {
    let mut add_docs = BTreeMap::new();
    for (name, field) in &resource.migration_fields {
        if config.ignores_field(Check::Phpdoc, table, name) {
            continue;
        }
        if resource.phpdoc_fields.contains_key(name) {
            continue;
        }
        add_docs.insert(name.clone(), field.clone());
    }
    if !add_docs.is_empty() {
        report
            .add_fields_to_model_docs
            .insert(table.into(), add_docs);
    }

    let mut remove_docs = BTreeMap::new();
    for (name, phpdoc) in &resource.phpdoc_fields {
        if config.ignores_field(Check::Phpdoc, table, name) {
            continue;
        }
        if resource.migration_fields.contains_key(name) {
            continue;
        }
        remove_docs.insert(
            name.clone(),
            Field {
                name: name.clone(),
                r#type: phpdoc.r#type.clone(),
                nullable: phpdoc.nullable,
            },
        );
    }
    if !remove_docs.is_empty() {
        report
            .remove_fields_from_model_docs
            .insert(table.into(), remove_docs);
    }

    let mut wrong = BTreeMap::new();
    for (name, phpdoc) in &resource.phpdoc_fields {
        if config.ignores_field(Check::Phpdoc, table, name) {
            continue;
        }
        let Some(migration) = resource.migration_fields.get(name) else {
            continue;
        };
        let cast = resource.model_fields.get(name).and_then(|f| f.cast.clone());
        let expected_type = if let Some(cast) = &cast {
            expected_phpdoc_from_cast(config, cast)
        } else {
            expected_phpdoc_from_migration(config, &migration.r#type)
        };
        let expected_nullable = migration.nullable;
        let mut actual = phpdoc.r#type.clone();
        if phpdoc.r#type == "mixed" && cast.as_deref() == Some("array") {
            actual = "array".into();
        }
        if expected_type == "array"
            && matches!(phpdoc.array_type.as_deref(), Some("array" | "Collection"))
        {
            actual = "array".into();
        }
        if actual != expected_type || phpdoc.nullable != expected_nullable {
            wrong.insert(
                name.clone(),
                WrongType {
                    field_name: name.clone(),
                    expected_type,
                    actual_type: actual,
                    expected_nullable,
                    actual_nullable: phpdoc.nullable,
                },
            );
        }
    }
    if !wrong.is_empty() {
        report.wrong_model_doc_types.insert(table.into(), wrong);
    }

    let mut wrong_rel = BTreeMap::new();
    for (name, phpdoc) in &resource.phpdoc_read_fields {
        let Some(rel) = resource.model_relationships.get(name) else {
            continue;
        };
        if is_morph_to(&rel.r#type) {
            continue;
        }
        if relationship_phpdoc_matches(&phpdoc.r#type, &rel.model) {
            continue;
        }
        let related_fqcn = format!("\\{}", normalize_class(&rel.model));
        let suggested =
            suggested_property_read(relationship_expects_collection(&rel.r#type), &rel.model);
        wrong_rel.insert(
            name.clone(),
            WrongRelationshipRead {
                relationship_name: name.clone(),
                relationship_type: rel.r#type.clone(),
                related_model: rel.model.clone(),
                phpdoc_type: phpdoc.r#type.clone(),
                nullable: phpdoc.nullable,
                issue_codes: vec!["type_does_not_match_related_model".into()],
                summary: format!(
                    "The type named in @property-read does not match the related model {related_fqcn} inferred from the {} relationship.",
                    rel.r#type
                ),
                suggested_phpdoc_type: suggested,
            },
        );
    }
    if !wrong_rel.is_empty() {
        report
            .wrong_relationship_phpdoc_read_types
            .insert(table.into(), wrong_rel);
    }

    let mut camel = BTreeMap::new();
    for (name, _) in &resource.model_relationships {
        if !name.contains('_') {
            continue;
        }
        let expected = camel_case(name);
        if name != &expected {
            camel.insert(
                name.clone(),
                WrongRelationshipName {
                    relationship_name: name.clone(),
                    expected_name: expected,
                },
            );
        }
    }
    if !camel.is_empty() {
        report
            .should_be_camel_case_relationship
            .insert(table.into(), camel);
    }

    let mut add_read = BTreeMap::new();
    for (name, rel) in &resource.model_relationships {
        if resource.phpdoc_read_fields.contains_key(name) {
            continue;
        }
        add_read.insert(
            name.clone(),
            Field {
                name: name.clone(),
                r#type: rel.model.clone(),
                nullable: false,
            },
        );
    }
    for (name, field) in &resource.model_fields {
        if !field.accessor {
            continue;
        }
        if resource.phpdoc_read_fields.contains_key(name) {
            continue;
        }
        add_read.insert(
            name.clone(),
            Field {
                name: name.clone(),
                r#type: field.cast.clone().unwrap_or_else(|| "mixed".into()),
                nullable: field.nullable,
            },
        );
    }
    if !add_read.is_empty() {
        report.add_property_read.insert(table.into(), add_read);
    }

    let mut morph = BTreeMap::new();
    for (name, rel) in &resource.model_relationships {
        if !is_morph_to(&rel.r#type) {
            continue;
        }
        let phpdoc = resource.phpdoc_read_fields.get(name);
        morph.insert(
            name.clone(),
            MorphToInfo {
                relationship_type: rel.r#type.clone(),
                has_phpdoc_read: phpdoc.is_some(),
                phpdoc_type: phpdoc.map(|p| p.r#type.clone()),
                nullable: phpdoc.map(|p| p.nullable),
            },
        );
    }
    if !morph.is_empty() {
        report.morph_to_relationships.insert(table.into(), morph);
    }

    let mut conflicts = BTreeMap::new();
    for (name, phpdoc) in &resource.phpdoc_fields {
        if config.ignores_field(Check::Phpdoc, table, name) {
            continue;
        }
        let Some(cast) = resource
            .model_fields
            .get(name)
            .and_then(|f| f.cast.as_ref())
            .filter(|c| !c.is_empty())
        else {
            continue;
        };
        let expected = expected_phpdoc_from_cast(config, cast);
        let mut actual = phpdoc.r#type.clone();
        if expected == "array"
            && matches!(phpdoc.array_type.as_deref(), Some("array" | "Collection"))
        {
            actual = "array".into();
        }
        if expected == actual {
            continue;
        }
        conflicts.insert(
            name.clone(),
            WrongType {
                field_name: name.clone(),
                expected_type: expected,
                actual_type: actual,
                expected_nullable: phpdoc.nullable,
                actual_nullable: phpdoc.nullable,
            },
        );
    }
    if !conflicts.is_empty() {
        report
            .model_evidence_conflicts
            .insert(table.into(), conflicts);
    }
}

fn expected_phpdoc_from_migration(config: &Config, migration_type: &str) -> String {
    config
        .type_normalizations
        .get(migration_type)
        .cloned()
        .unwrap_or_else(|| migration_type.to_string())
}

fn expected_phpdoc_from_cast(config: &Config, cast: &str) -> String {
    let trimmed = cast.trim();
    if trimmed.is_empty() {
        return "mixed".into();
    }
    if let Some(mapped) = lookup_cast(config, trimmed) {
        return mapped;
    }
    let lower = trimmed.to_ascii_lowercase();
    if let Some((base, arg)) = lower.split_once(':') {
        if let Some(mapped) = lookup_cast(config, &lower) {
            return mapped;
        }
        if let Some(mapped) = lookup_cast(config, base) {
            if base != "encrypted" {
                return mapped;
            }
        }
        if base == "encrypted" {
            return match arg {
                "array" | "json" | "collection" => "array".into(),
                "object" => "object".into(),
                _ => lookup_cast(config, "encrypted").unwrap_or_else(|| "string".into()),
            };
        }
    }
    if let Some(mapped) = lookup_cast(config, &lower) {
        return mapped;
    }
    trimmed.to_string()
}

fn lookup_cast(config: &Config, key: &str) -> Option<String> {
    config.cast_type_mappings.get(key).cloned().or_else(|| {
        config
            .cast_type_mappings
            .get(&key.to_ascii_lowercase())
            .cloned()
    })
}

fn is_morph_to(relationship_type: &str) -> bool {
    relationship_type.trim().eq_ignore_ascii_case("morphto")
}

fn relationship_expects_collection(relationship_type: &str) -> bool {
    matches!(
        relationship_type.to_ascii_lowercase().as_str(),
        "hasmany" | "belongstomany" | "morphmany" | "morphtomany" | "hasmanythrough" | "collection"
    )
}

fn normalize_class(fqcn: &str) -> String {
    fqcn.trim().trim_start_matches('\\').to_string()
}

fn suggested_property_read(expects_collection: bool, related: &str) -> String {
    let inner = format!("\\{}", normalize_class(related));
    if expects_collection {
        format!("\\Illuminate\\Database\\Eloquent\\Collection<int, {inner}>")
    } else {
        inner
    }
}

fn relationship_phpdoc_matches(phpdoc_type: &str, relationship_model: &str) -> bool {
    let Some(from_doc) = extract_relationship_model(phpdoc_type) else {
        return false;
    };
    let from_doc = normalize_class(&from_doc);
    let model = normalize_class(relationship_model);
    from_doc == model || short_name(&from_doc) == short_name(&model)
}

fn extract_relationship_model(phpdoc_type: &str) -> Option<String> {
    let normalized = phpdoc_type.trim().trim_start_matches('\\');
    if normalized.is_empty() {
        return None;
    }
    if let Some(inner) = two_generic(normalized) {
        return Some(inner);
    }
    if let Some(inner) = one_generic(normalized) {
        return Some(inner);
    }
    if let Some(stripped) = normalized.strip_suffix("[]") {
        return Some(stripped.to_string());
    }
    Some(normalized.to_string())
}

fn two_generic(ty: &str) -> Option<String> {
    let lower = ty.to_ascii_lowercase();
    for prefix in ["collection<", "array<"] {
        if lower.starts_with(prefix) && ty.ends_with('>') {
            let inner = &ty[prefix.len()..ty.len() - 1];
            if let Some((_, rest)) = inner.split_once(',') {
                return Some(rest.trim().to_string());
            }
        }
    }
    None
}

fn one_generic(ty: &str) -> Option<String> {
    let lower = ty.to_ascii_lowercase();
    for prefix in ["collection<", "array<"] {
        if lower.starts_with(prefix) && ty.ends_with('>') {
            let inner = &ty[prefix.len()..ty.len() - 1];
            if !inner.contains(',') {
                return Some(inner.trim().to_string());
            }
        }
    }
    None
}

pub fn actionable_report(report: &Report) -> Value {
    let full = serde_json::to_value(report).unwrap_or(Value::Null);
    let Value::Object(map) = full else {
        return serde_json::json!({});
    };
    let mut out = Map::new();
    for (key, value) in map {
        let value = if key == "morph_to_relationships" {
            filter_morph_to(value)
        } else {
            value
        };
        if is_actionable(&value) {
            out.insert(key, value);
        }
    }
    Value::Object(out)
}

fn filter_morph_to(value: Value) -> Value {
    let Value::Object(tables) = value else {
        return value;
    };
    let mut out = Map::new();
    for (table, rels) in tables {
        let Value::Object(rels) = rels else {
            continue;
        };
        let mut kept = Map::new();
        for (name, details) in rels {
            let Value::Object(ref obj) = details else {
                continue;
            };
            let has_read = obj
                .get("has_phpdoc_read")
                .and_then(|v| v.as_bool())
                .unwrap_or(false);
            let phpdoc = obj
                .get("phpdoc_type")
                .and_then(|v| v.as_str())
                .unwrap_or("")
                .trim();
            if has_read && !phpdoc.is_empty() {
                continue;
            }
            kept.insert(name, details);
        }
        if !kept.is_empty() {
            out.insert(table, Value::Object(kept));
        }
    }
    Value::Object(out)
}

fn is_actionable(value: &Value) -> bool {
    match value {
        Value::Array(arr) => !arr.is_empty(),
        Value::Object(obj) => !obj.is_empty(),
        Value::Null => false,
        _ => true,
    }
}

#[cfg(test)]
mod tests {
    use super::*;
    use crate::ir::{ModelField, PhpDocField, Relationship};
    use std::collections::BTreeMap;

    fn table(resource: TableAnalysis) -> Analysis {
        Analysis {
            tables: BTreeMap::from([("t".into(), resource)]),
        }
    }

    #[test]
    fn model_evidence_conflicts() {
        let resource = TableAnalysis {
            model_fields: BTreeMap::from([(
                "payload".into(),
                ModelField {
                    name: "payload".into(),
                    cast: Some("json".into()),
                    ..Default::default()
                },
            )]),
            phpdoc_fields: BTreeMap::from([(
                "payload".into(),
                PhpDocField {
                    name: "payload".into(),
                    r#type: "string".into(),
                    nullable: false,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        let report = generate_report(&Config::default(), &table(resource));
        let conflict = &report.model_evidence_conflicts["t"]["payload"];
        assert_eq!(conflict.expected_type, "array");
        assert_eq!(conflict.actual_type, "string");
    }

    #[test]
    fn ignores_configured_resource_fields() {
        let mut cfg = Config::default();
        cfg.ignore.filament.fields.push("remember_token".into());
        let resource = TableAnalysis {
            migration_fields: BTreeMap::from([
                (
                    "name".into(),
                    Field {
                        name: "name".into(),
                        r#type: "string".into(),
                        nullable: false,
                    },
                ),
                (
                    "remember_token".into(),
                    Field {
                        name: "remember_token".into(),
                        r#type: "string".into(),
                        nullable: true,
                    },
                ),
            ]),
            filament_form_fields: BTreeMap::from([
                (
                    "legacy_field".into(),
                    Field {
                        name: "legacy_field".into(),
                        r#type: "string".into(),
                        nullable: true,
                    },
                ),
                (
                    "remember_token".into(),
                    Field {
                        name: "remember_token".into(),
                        r#type: "string".into(),
                        nullable: true,
                    },
                ),
            ]),
            ..Default::default()
        };
        let report = generate_report(
            &cfg,
            &Analysis {
                tables: BTreeMap::from([("users".into(), resource)]),
            },
        );
        assert!(report.add_fields_to_filament_form["users"].contains_key("name"));
        assert!(!report.add_fields_to_filament_form["users"].contains_key("remember_token"));
        assert!(report.remove_fields_from_filament_form["users"].contains_key("legacy_field"));
        assert!(!report.remove_fields_from_filament_form["users"].contains_key("remember_token"));
    }

    #[test]
    fn ignores_phpdoc_fields() {
        let mut cfg = Config::default();
        cfg.ignore.phpdoc.fields.push("id".into());
        let resource = TableAnalysis {
            migration_fields: BTreeMap::from([
                (
                    "id".into(),
                    Field {
                        name: "id".into(),
                        r#type: "int".into(),
                        nullable: false,
                    },
                ),
                (
                    "name".into(),
                    Field {
                        name: "name".into(),
                        r#type: "string".into(),
                        nullable: false,
                    },
                ),
            ]),
            phpdoc_fields: BTreeMap::from([(
                "name".into(),
                PhpDocField {
                    name: "name".into(),
                    r#type: "string".into(),
                    nullable: false,
                    ..Default::default()
                },
            )]),
            model_fqcn: Some("App\\Models\\User".into()),
            ..Default::default()
        };
        let report = generate_report(&cfg, &table(resource));
        assert!(!report
            .add_fields_to_model_docs
            .get("t")
            .is_some_and(|fields| fields.contains_key("id")));
    }

    #[test]
    fn attached_model_is_not_reported_as_missing() {
        let resource = TableAnalysis {
            model_fqcn: Some("App\\Models\\Note".into()),
            ..Default::default()
        };
        let report = generate_report(&Config::default(), &table(resource));
        assert!(report.add_models.is_empty());
    }

    #[test]
    fn enum_cast_matches_phpdoc() {
        let resource = TableAnalysis {
            migration_fields: BTreeMap::from([(
                "role".into(),
                Field {
                    name: "role".into(),
                    r#type: "string".into(),
                    nullable: false,
                },
            )]),
            model_fields: BTreeMap::from([(
                "role".into(),
                ModelField {
                    name: "role".into(),
                    cast: Some("App\\Enums\\Role".into()),
                    ..Default::default()
                },
            )]),
            phpdoc_fields: BTreeMap::from([(
                "role".into(),
                PhpDocField {
                    name: "role".into(),
                    r#type: "App\\Enums\\Role".into(),
                    nullable: false,
                    ..Default::default()
                },
            )]),
            model_fqcn: Some("App\\Models\\User".into()),
            ..Default::default()
        };
        let report = generate_report(&Config::default(), &table(resource));
        assert!(report.wrong_model_doc_types.is_empty());
        assert!(report.model_evidence_conflicts.is_empty());
    }

    #[test]
    fn phpdoc_covers_model_fields() {
        let resource = TableAnalysis {
            migration_fields: BTreeMap::from([(
                "id".into(),
                Field {
                    name: "id".into(),
                    r#type: "string".into(),
                    nullable: false,
                },
            )]),
            phpdoc_fields: BTreeMap::from([(
                "id".into(),
                PhpDocField {
                    name: "id".into(),
                    r#type: "string".into(),
                    nullable: false,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        let report = generate_report(&Config::default(), &table(resource));
        assert!(report.add_fields_to_models.is_empty());
    }

    #[test]
    fn relationship_short_class_name_matches() {
        let resource = TableAnalysis {
            phpdoc_read_fields: BTreeMap::from([(
                "devices".into(),
                PhpDocField {
                    name: "devices".into(),
                    r#type: "Device".into(),
                    nullable: false,
                    array_type: Some("Collection".into()),
                    key_type: Some("array-key".into()),
                },
            )]),
            model_relationships: BTreeMap::from([(
                "devices".into(),
                Relationship {
                    name: "devices".into(),
                    r#type: "HasMany".into(),
                    model: "App\\Models\\Device".into(),
                },
            )]),
            ..Default::default()
        };
        let report = generate_report(&Config::default(), &table(resource));
        assert!(report.wrong_relationship_phpdoc_read_types.is_empty());
    }

    #[test]
    fn relationship_collection_generic_matches() {
        let resource = TableAnalysis {
            phpdoc_read_fields: BTreeMap::from([(
                "devices".into(),
                PhpDocField {
                    name: "devices".into(),
                    r#type: "Collection<array-key,\\Device>".into(),
                    nullable: false,
                    ..Default::default()
                },
            )]),
            model_relationships: BTreeMap::from([(
                "devices".into(),
                Relationship {
                    name: "devices".into(),
                    r#type: "HasMany".into(),
                    model: "App\\Models\\Device".into(),
                },
            )]),
            ..Default::default()
        };
        let report = generate_report(&Config::default(), &table(resource));
        assert!(report.wrong_relationship_phpdoc_read_types.is_empty());
    }

    #[test]
    fn decimal_and_encrypted_casts_match() {
        let resource = TableAnalysis {
            model_fields: BTreeMap::from([
                (
                    "price".into(),
                    ModelField {
                        name: "price".into(),
                        cast: Some("decimal:2".into()),
                        ..Default::default()
                    },
                ),
                (
                    "access_token".into(),
                    ModelField {
                        name: "access_token".into(),
                        cast: Some("encrypted".into()),
                        ..Default::default()
                    },
                ),
            ]),
            phpdoc_fields: BTreeMap::from([
                (
                    "price".into(),
                    PhpDocField {
                        name: "price".into(),
                        r#type: "float".into(),
                        nullable: true,
                        ..Default::default()
                    },
                ),
                (
                    "access_token".into(),
                    PhpDocField {
                        name: "access_token".into(),
                        r#type: "string".into(),
                        nullable: false,
                        ..Default::default()
                    },
                ),
            ]),
            ..Default::default()
        };
        let report = generate_report(&Config::default(), &table(resource));
        assert!(report.model_evidence_conflicts.is_empty());
    }

    #[test]
    fn morph_to_is_listed_not_flagged() {
        let resource = TableAnalysis {
            phpdoc_read_fields: BTreeMap::from([(
                "trackable".into(),
                PhpDocField {
                    name: "trackable".into(),
                    r#type: "Illuminate\\Database\\Eloquent\\Model".into(),
                    nullable: true,
                    ..Default::default()
                },
            )]),
            model_relationships: BTreeMap::from([
                (
                    "trackable".into(),
                    Relationship {
                        name: "trackable".into(),
                        r#type: "MorphTo".into(),
                        model: "App\\Models\\GpsArchive".into(),
                    },
                ),
                (
                    "device".into(),
                    Relationship {
                        name: "device".into(),
                        r#type: "BelongsTo".into(),
                        model: "App\\Models\\Device".into(),
                    },
                ),
            ]),
            ..Default::default()
        };
        let report = generate_report(
            &Config::default(),
            &Analysis {
                tables: BTreeMap::from([("gps_archives".into(), resource)]),
            },
        );
        assert!(report.wrong_relationship_phpdoc_read_types.is_empty());
        assert!(report.morph_to_relationships["gps_archives"].contains_key("trackable"));
        assert!(!report.morph_to_relationships["gps_archives"].contains_key("device"));
    }

    #[test]
    fn accessors_are_not_removed() {
        let resource = TableAnalysis {
            model_fields: BTreeMap::from([(
                "status".into(),
                ModelField {
                    name: "status".into(),
                    cast: Some("App\\Enums\\ActivityStatus".into()),
                    accessor: true,
                    ..Default::default()
                },
            )]),
            phpdoc_read_fields: BTreeMap::from([(
                "status".into(),
                PhpDocField {
                    name: "status".into(),
                    r#type: "App\\Enums\\ActivityStatus".into(),
                    nullable: false,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        let report = generate_report(&Config::default(), &table(resource));
        assert!(report.remove_fields_from_models.is_empty());
        assert!(report.add_property_read.is_empty());
    }

    #[test]
    fn undocumented_accessors_need_property_read() {
        let resource = TableAnalysis {
            model_fields: BTreeMap::from([(
                "batch".into(),
                ModelField {
                    name: "batch".into(),
                    cast: Some("Protos\\Gps\\GpsPointBatch".into()),
                    accessor: true,
                    nullable: true,
                    ..Default::default()
                },
            )]),
            ..Default::default()
        };
        let report = generate_report(&Config::default(), &table(resource));
        assert_eq!(
            report.add_property_read["t"]["batch"].r#type,
            "Protos\\Gps\\GpsPointBatch"
        );
        assert!(report.add_property_read["t"]["batch"].nullable);
    }

    #[test]
    fn flags_relationship_phpdoc_mismatch() {
        let resource = TableAnalysis {
            phpdoc_read_fields: BTreeMap::from([(
                "devices".into(),
                PhpDocField {
                    name: "devices".into(),
                    r#type: "string".into(),
                    nullable: false,
                    ..Default::default()
                },
            )]),
            model_relationships: BTreeMap::from([(
                "devices".into(),
                Relationship {
                    name: "devices".into(),
                    r#type: "HasMany".into(),
                    model: "App\\Models\\Device".into(),
                },
            )]),
            ..Default::default()
        };
        let report = generate_report(&Config::default(), &table(resource));
        let entry = &report.wrong_relationship_phpdoc_read_types["t"]["devices"];
        assert_eq!(entry.issue_codes, ["type_does_not_match_related_model"]);
        assert_eq!(
            entry.suggested_phpdoc_type,
            "\\Illuminate\\Database\\Eloquent\\Collection<int, \\App\\Models\\Device>"
        );
    }
}
