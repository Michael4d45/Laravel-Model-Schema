use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct Config {
    #[serde(default = "default_model_paths")]
    pub model_paths: Vec<PathBuf>,
    #[serde(default = "default_resource_paths")]
    pub resource_paths: Vec<PathBuf>,
    #[serde(default = "default_migration_paths")]
    pub migration_paths: Vec<PathBuf>,
    #[serde(default = "default_output")]
    pub output_path: PathBuf,
    #[serde(default = "default_ignored_tables")]
    pub ignored_tables: BTreeMap<String, Vec<String>>,
    #[serde(default = "default_ignored_fields")]
    pub ignored_fields: BTreeMap<String, Vec<String>>,
    #[serde(default = "default_column_type_mappings")]
    pub column_type_mappings: BTreeMap<String, String>,
    #[serde(default = "default_type_normalizations")]
    pub type_normalizations: BTreeMap<String, String>,
    #[serde(default = "default_cast_type_mappings")]
    pub cast_type_mappings: BTreeMap<String, String>,
    #[serde(default = "default_form_component_classes")]
    pub form_component_classes: Vec<String>,
    #[serde(default = "default_resource_component_type_map")]
    pub resource_component_type_map: BTreeMap<String, String>,
    #[serde(default = "default_required_methods")]
    pub resource_component_required_methods: Vec<String>,
}

fn default_model_paths() -> Vec<PathBuf> {
    vec![PathBuf::from("app/Models")]
}

fn default_resource_paths() -> Vec<PathBuf> {
    vec![PathBuf::from("app/Filament/Resources")]
}

fn default_migration_paths() -> Vec<PathBuf> {
    vec![PathBuf::from("database/migrations")]
}

fn default_output() -> PathBuf {
    PathBuf::from("reports/model_schema_report.json")
}

fn default_ignored_tables() -> BTreeMap<String, Vec<String>> {
    let all = vec![
        "resources".to_string(),
        "models".to_string(),
        "phpdoc".to_string(),
    ];
    [
        "failed_jobs",
        "jobs",
        "migrations",
        "personal_access_tokens",
        "password_resets",
        "password_reset_tokens",
        "sessions",
        "cache",
        "cache_locks",
        "job_batches",
        "activity_log",
        "webhooks",
    ]
    .into_iter()
    .map(|table| (table.to_string(), all.clone()))
    .collect()
}

fn default_ignored_fields() -> BTreeMap<String, Vec<String>> {
    ["id", "created_at", "updated_at", "deleted_at"]
        .into_iter()
        .map(|field| (field.to_string(), vec!["models".to_string()]))
        .collect()
}

fn default_column_type_mappings() -> BTreeMap<String, String> {
    let pairs = [
        ("id", "int"),
        ("int", "int"),
        ("int2", "int"),
        ("int4", "int"),
        ("int8", "int"),
        ("bigint", "int"),
        ("bigIncrements", "int"),
        ("increments", "int"),
        ("smallIncrements", "int"),
        ("mediumIncrements", "int"),
        ("tinyIncrements", "int"),
        ("integer", "int"),
        ("bigInteger", "int"),
        ("smallInteger", "int"),
        ("mediumInteger", "int"),
        ("tinyInteger", "int"),
        ("unsignedInteger", "int"),
        ("unsignedBigInteger", "int"),
        ("unsignedSmallInteger", "int"),
        ("unsignedMediumInteger", "int"),
        ("unsignedTinyInteger", "int"),
        ("foreignId", "int"),
        ("foreignIdFor", "int"),
        ("string", "string"),
        ("char", "string"),
        ("varchar", "string"),
        ("text", "string"),
        ("mediumText", "string"),
        ("mediumtext", "string"),
        ("longText", "string"),
        ("longtext", "string"),
        ("enum", "string"),
        ("uuid", "string"),
        ("foreignUuid", "string"),
        ("rememberToken", "string"),
        ("binary", "resource"),
        ("bytea", "resource"),
        ("tinyint", "bool"),
        ("bool", "bool"),
        ("boolean", "bool"),
        ("decimal", "float"),
        ("float", "float"),
        ("float8", "float"),
        ("double", "float"),
        ("timestamp", "Carbon"),
        ("timestampTz", "Carbon"),
        ("timestamptz", "Carbon"),
        ("date", "Carbon"),
        ("dateTime", "Carbon"),
        ("dateTimeTz", "Carbon"),
        ("json", "array"),
        ("jsonb", "array"),
    ];
    pairs
        .into_iter()
        .map(|(k, v)| (k.to_string(), v.to_string()))
        .collect()
}

fn default_type_normalizations() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "Carbon".to_string(),
            "Illuminate\\Support\\Carbon".to_string(),
        ),
        (
            "Point".to_string(),
            "Clickbar\\Magellan\\Data\\Geometries\\Point".to_string(),
        ),
        (
            "Box2D".to_string(),
            "Clickbar\\Magellan\\Data\\Boxes\\Box2D".to_string(),
        ),
    ])
}

fn default_cast_type_mappings() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "datetime".to_string(),
            "Illuminate\\Support\\Carbon".to_string(),
        ),
        (
            "timestamp".to_string(),
            "Illuminate\\Support\\Carbon".to_string(),
        ),
        ("json".to_string(), "array".to_string()),
        ("boolean".to_string(), "bool".to_string()),
        ("integer".to_string(), "int".to_string()),
        ("hashed".to_string(), "string".to_string()),
    ])
}

fn default_form_component_classes() -> Vec<String> {
    vec![
        "Filament\\Forms\\Components".into(),
        "Filament\\Forms\\Components\\Field".into(),
        "Filament\\Forms\\Components\\TextInput".into(),
        "Filament\\Forms\\Components\\Select".into(),
        "Filament\\Forms\\Components\\Textarea".into(),
        "Filament\\Forms\\Components\\Checkbox".into(),
        "Filament\\Forms\\Components\\DateTimePicker".into(),
        "Filament\\Forms\\Components\\FileUpload".into(),
        "Filament\\Forms\\Components\\KeyValue".into(),
        "Filament\\Forms\\Components\\Repeater".into(),
        "Filament\\Forms\\Components\\Builder".into(),
        "Filament\\Forms\\Components\\Toggle".into(),
        "Filament\\Forms\\Components\\Radio".into(),
        "Filament\\Forms\\Components\\TagsInput".into(),
        "Filament\\Forms\\Components\\ColorPicker".into(),
    ]
}

fn default_resource_component_type_map() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "Filament\\Forms\\Components\\TextInput".to_string(),
            "string".to_string(),
        ),
        (
            "Filament\\Forms\\Components\\Textarea".to_string(),
            "string".to_string(),
        ),
        (
            "Filament\\Forms\\Components\\Select".to_string(),
            "string".to_string(),
        ),
        (
            "Filament\\Forms\\Components\\Toggle".to_string(),
            "bool".to_string(),
        ),
        (
            "Filament\\Forms\\Components\\Checkbox".to_string(),
            "bool".to_string(),
        ),
        (
            "Filament\\Forms\\Components\\Radio".to_string(),
            "string".to_string(),
        ),
        (
            "Filament\\Forms\\Components\\DatePicker".to_string(),
            "Carbon".to_string(),
        ),
        (
            "Filament\\Forms\\Components\\DateTimePicker".to_string(),
            "Carbon".to_string(),
        ),
        (
            "Filament\\Forms\\Components\\TimePicker".to_string(),
            "Carbon".to_string(),
        ),
        (
            "Filament\\Forms\\Components\\TagsInput".to_string(),
            "array".to_string(),
        ),
        (
            "Filament\\Forms\\Components\\KeyValue".to_string(),
            "array".to_string(),
        ),
        (
            "Filament\\Forms\\Components\\FileUpload".to_string(),
            "string".to_string(),
        ),
        (
            "Filament\\Forms\\Components\\ColorPicker".to_string(),
            "string".to_string(),
        ),
    ])
}

fn default_required_methods() -> Vec<String> {
    vec!["required".into()]
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model_paths: default_model_paths(),
            resource_paths: default_resource_paths(),
            migration_paths: default_migration_paths(),
            output_path: default_output(),
            ignored_tables: default_ignored_tables(),
            ignored_fields: default_ignored_fields(),
            column_type_mappings: default_column_type_mappings(),
            type_normalizations: default_type_normalizations(),
            cast_type_mappings: default_cast_type_mappings(),
            form_component_classes: default_form_component_classes(),
            resource_component_type_map: default_resource_component_type_map(),
            resource_component_required_methods: default_required_methods(),
        }
    }
}

impl Config {
    pub fn from_toml_str(s: &str) -> crate::Result<Self> {
        let mut defaults = Self::default();
        let parsed: Self = toml::from_str(s).map_err(|e| crate::Error::Config(e.to_string()))?;
        merge_config(&mut defaults, parsed);
        Ok(defaults)
    }

    pub fn from_json_str(s: &str) -> crate::Result<Self> {
        serde_json::from_str(s).map_err(|e| crate::Error::Config(e.to_string()))
    }

    pub fn load_file(path: &Path) -> crate::Result<Self> {
        let text = std::fs::read_to_string(path)?;
        let mut cfg = if path.extension().and_then(|e| e.to_str()) == Some("json") {
            Self::from_json_str(&text)?
        } else {
            Self::from_toml_str(&text)?
        };
        let base = path
            .parent()
            .and_then(|parent| parent.canonicalize().ok())
            .unwrap_or_else(|| PathBuf::from("."));
        cfg.resolve_relative_paths(&base);
        Ok(cfg)
    }

    fn resolve_relative_paths(&mut self, base: &Path) {
        self.model_paths = self
            .model_paths
            .iter()
            .map(|p| absolutize(base, p))
            .collect();
        self.resource_paths = self
            .resource_paths
            .iter()
            .map(|p| absolutize(base, p))
            .collect();
        self.migration_paths = self
            .migration_paths
            .iter()
            .map(|p| absolutize(base, p))
            .collect();
        self.output_path = absolutize(base, &self.output_path);
    }

    pub fn ignored_for(&self, component: &str) -> Vec<String> {
        self.ignored_tables
            .iter()
            .filter(|(_, components)| components.iter().any(|c| c == component))
            .map(|(table, _)| table.clone())
            .collect()
    }

    pub fn ignored_fields_for(&self, component: &str) -> Vec<String> {
        self.ignored_fields
            .iter()
            .filter(|(_, components)| components.iter().any(|c| c == component))
            .map(|(field, _)| field.clone())
            .collect()
    }
}

fn merge_config(base: &mut Config, overlay: Config) {
    if overlay.model_paths != default_model_paths() {
        base.model_paths = overlay.model_paths;
    }
    if overlay.resource_paths != default_resource_paths() {
        base.resource_paths = overlay.resource_paths;
    }
    if overlay.migration_paths != default_migration_paths() {
        base.migration_paths = overlay.migration_paths;
    }
    if overlay.output_path != default_output() {
        base.output_path = overlay.output_path;
    }
    if !overlay.ignored_tables.is_empty() {
        base.ignored_tables = overlay.ignored_tables;
    }
    if !overlay.ignored_fields.is_empty() {
        base.ignored_fields = overlay.ignored_fields;
    }
    if !overlay.column_type_mappings.is_empty()
        && overlay.column_type_mappings != default_column_type_mappings()
    {
        for (k, v) in overlay.column_type_mappings {
            base.column_type_mappings.insert(k, v);
        }
    }
    if !overlay.type_normalizations.is_empty()
        && overlay.type_normalizations != default_type_normalizations()
    {
        for (k, v) in overlay.type_normalizations {
            base.type_normalizations.insert(k, v);
        }
    }
    if !overlay.cast_type_mappings.is_empty()
        && overlay.cast_type_mappings != default_cast_type_mappings()
    {
        for (k, v) in overlay.cast_type_mappings {
            base.cast_type_mappings.insert(k, v);
        }
    }
    if overlay.form_component_classes != default_form_component_classes() {
        base.form_component_classes = overlay.form_component_classes;
    }
    if overlay.resource_component_type_map != default_resource_component_type_map() {
        for (k, v) in overlay.resource_component_type_map {
            base.resource_component_type_map.insert(k, v);
        }
    }
    if overlay.resource_component_required_methods != default_required_methods() {
        base.resource_component_required_methods = overlay.resource_component_required_methods;
    }
}

fn absolutize(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}
