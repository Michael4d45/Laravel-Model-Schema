mod expr;
mod filament;
mod migrations;
mod models;
mod names;
mod phpdoc;
mod uses;

pub use filament::parse_filament;
pub use migrations::parse_migrations;
pub use models::parse_models;
pub(crate) use names::{camel_case, short_name};
pub(crate) use phpdoc::parse_phpdoc_type;

use std::collections::BTreeMap;
use std::path::Path;

use crate::ir::{Analysis, Field};
use crate::Config;

/// Database type names as reported by Laravel's `Schema::getColumns()`
/// (`type_name`, or `tinyint(1)` for MySQL booleans), lowercased.
///
/// Kept apart from `column_type_mappings` because the migration parser treats
/// those keys as Blueprint column methods.
const DATABASE_TYPE_MAPPINGS: &[(&str, &str)] = &[
    ("bigint", "int"),
    ("int", "int"),
    ("integer", "int"),
    ("mediumint", "int"),
    ("smallint", "int"),
    ("tinyint", "int"),
    ("int2", "int"),
    ("int4", "int"),
    ("int8", "int"),
    ("serial", "int"),
    ("bigserial", "int"),
    ("year", "int"),
    ("tinyint(1)", "bool"),
    ("bool", "bool"),
    ("boolean", "bool"),
    ("bit", "bool"),
    ("char", "string"),
    ("varchar", "string"),
    ("character varying", "string"),
    ("bpchar", "string"),
    ("tinytext", "string"),
    ("text", "string"),
    ("mediumtext", "string"),
    ("longtext", "string"),
    ("enum", "string"),
    ("set", "string"),
    ("uuid", "string"),
    ("inet", "string"),
    ("macaddr", "string"),
    ("time", "string"),
    ("timetz", "string"),
    ("decimal", "float"),
    ("numeric", "float"),
    ("float", "float"),
    ("float4", "float"),
    ("float8", "float"),
    ("double", "float"),
    ("double precision", "float"),
    ("real", "float"),
    ("timestamp", "Carbon"),
    ("timestamptz", "Carbon"),
    ("datetime", "Carbon"),
    ("datetimetz", "Carbon"),
    ("date", "Carbon"),
    ("json", "array"),
    ("jsonb", "array"),
    ("binary", "resource"),
    ("varbinary", "resource"),
    ("blob", "resource"),
    ("tinyblob", "resource"),
    ("mediumblob", "resource"),
    ("longblob", "resource"),
    ("bytea", "resource"),
];

/// Live DB schema dump, e.g. from the PHP wrapper:
/// `{ "posts": { "title": { "name": "title", "type": "varchar", "nullable": false } } }`
///
/// Types may be raw database names (`varchar`, `bigint`, `tinyint(1)`) or
/// already normalized (`string`, `int`, `Carbon`); see [`schema_field_type`].
pub fn load_schema_json(
    path: &Path,
    config: &Config,
    analysis: &mut Analysis,
) -> crate::Result<()> {
    let text = std::fs::read_to_string(path)?;
    let tables: BTreeMap<String, BTreeMap<String, Field>> = serde_json::from_str(&text)
        .map_err(|e| crate::Error::Config(format!("schema json: {e}")))?;
    for (table, mut fields) in tables {
        for field in fields.values_mut() {
            field.r#type = schema_field_type(config, &field.r#type);
        }
        analysis.table_mut(&table).migration_fields = fields;
    }
    Ok(())
}

/// Map a schema-dump column type onto the report's type vocabulary.
///
/// Project `column_type_mappings` win (exact, then case-insensitive), then the
/// built-in database names. Anything unrecognized passes through unchanged so
/// dumps that are already normalized keep working.
pub fn schema_field_type(config: &Config, raw: &str) -> String {
    if let Some(mapped) = config.column_type_mappings.get(raw) {
        return mapped.clone();
    }
    let lower = raw.to_ascii_lowercase();
    if let Some((_, mapped)) = config
        .column_type_mappings
        .iter()
        .find(|(key, _)| key.to_ascii_lowercase() == lower)
    {
        return mapped.clone();
    }
    if let Some((_, mapped)) = DATABASE_TYPE_MAPPINGS.iter().find(|(key, _)| *key == lower) {
        return (*mapped).to_string();
    }
    raw.to_string()
}
