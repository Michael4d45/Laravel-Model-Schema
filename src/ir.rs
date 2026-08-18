use std::collections::BTreeMap;
use std::path::PathBuf;

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Field {
    pub name: String,
    pub r#type: String,
    pub nullable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ModelField {
    pub name: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub cast: Option<String>,
    #[serde(default)]
    pub fillable: bool,
    #[serde(default)]
    pub hidden: bool,
    #[serde(default)]
    pub accessor: bool,
    #[serde(default)]
    pub nullable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct PhpDocField {
    pub name: String,
    pub r#type: String,
    pub nullable: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub array_type: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub key_type: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Relationship {
    pub name: String,
    pub r#type: String,
    pub model: String,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct TableAnalysis {
    #[serde(default)]
    pub migration_fields: BTreeMap<String, Field>,
    #[serde(default)]
    pub model_fields: BTreeMap<String, ModelField>,
    #[serde(default)]
    pub filament_form_fields: BTreeMap<String, Field>,
    #[serde(default)]
    pub phpdoc_fields: BTreeMap<String, PhpDocField>,
    #[serde(default)]
    pub phpdoc_read_fields: BTreeMap<String, PhpDocField>,
    #[serde(default)]
    pub model_relationships: BTreeMap<String, Relationship>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_file: Option<PathBuf>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub model_fqcn: Option<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct Analysis {
    pub tables: BTreeMap<String, TableAnalysis>,
}

impl Analysis {
    pub fn table_mut(&mut self, name: &str) -> &mut TableAnalysis {
        self.tables.entry(name.to_string()).or_default()
    }
}
