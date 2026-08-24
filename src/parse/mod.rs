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

/// Live DB schema dump produced by the Artisan wrapper:
/// `{ "posts": { "title": { "name": "title", "type": "string", "nullable": false } } }`
pub fn load_schema_json(path: &Path, analysis: &mut Analysis) -> crate::Result<()> {
    let text = std::fs::read_to_string(path)?;
    let tables: BTreeMap<String, BTreeMap<String, Field>> = serde_json::from_str(&text)
        .map_err(|e| crate::Error::Config(format!("schema json: {e}")))?;
    for (table, fields) in tables {
        analysis.table_mut(&table).migration_fields = fields;
    }
    Ok(())
}
