use std::collections::BTreeMap;
use std::path::{Path, PathBuf};

use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Serialize, Deserialize)]
#[serde(rename_all = "snake_case")]
pub enum Check {
    Filament,
    Models,
    Phpdoc,
}

impl Check {
    pub const ALL: [Check; 3] = [Check::Filament, Check::Models, Check::Phpdoc];
}

#[derive(Debug, Clone, Serialize, PartialEq)]
pub struct Config {
    pub model_paths: Vec<PathBuf>,
    pub filament_paths: Vec<PathBuf>,
    pub migration_paths: Vec<PathBuf>,
    pub output_path: PathBuf,
    pub checks: ChecksConfig,
    pub ignore: IgnoreConfig,
    pub unignore: IgnoreConfig,
    pub models: ModelsConfig,
    pub filament: FilamentConfig,
    pub migrations: MigrationsConfig,
    pub column_type_mappings: BTreeMap<String, String>,
    pub type_normalizations: BTreeMap<String, String>,
    pub cast_type_mappings: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ChecksConfig {
    #[serde(default = "default_true")]
    pub filament: bool,
    #[serde(default = "default_true")]
    pub models: bool,
    #[serde(default = "default_true")]
    pub phpdoc: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct IgnoreScope {
    #[serde(default)]
    pub tables: Vec<String>,
    #[serde(default)]
    pub fields: Vec<String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct IgnoreRule {
    #[serde(default)]
    pub tables: Vec<String>,
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default)]
    pub checks: Vec<Check>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct IgnoreConfig {
    /// When false, built-in Laravel table/field ignores are dropped.
    #[serde(default = "default_true")]
    pub defaults: bool,
    #[serde(default)]
    pub tables: Vec<String>,
    #[serde(default)]
    pub fields: Vec<String>,
    #[serde(default)]
    pub filament: IgnoreScope,
    #[serde(default)]
    pub models: IgnoreScope,
    #[serde(default)]
    pub phpdoc: IgnoreScope,
    #[serde(default)]
    pub rules: Vec<IgnoreRule>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct FilamentConfig {
    pub component_classes: Vec<String>,
    pub component_namespaces: Vec<String>,
    pub component_types: BTreeMap<String, String>,
    pub required_methods: Vec<String>,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct ModelsConfig {
    pub parents: Vec<String>,
    pub table_defaults: BTreeMap<String, String>,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct ColumnMacro {
    pub columns: Vec<String>,
    #[serde(rename = "type")]
    pub r#type: String,
    #[serde(default)]
    pub nullable: bool,
}

#[derive(Debug, Clone, Default, Serialize, Deserialize, PartialEq)]
pub struct MorphMacro {
    pub id_type: String,
    #[serde(default)]
    pub nullable: bool,
}

#[derive(Debug, Clone, Serialize, Deserialize, PartialEq)]
pub struct MigrationsConfig {
    pub default_column_names: BTreeMap<String, String>,
    pub column_macros: BTreeMap<String, ColumnMacro>,
    pub morph_macros: BTreeMap<String, MorphMacro>,
}

fn default_true() -> bool {
    true
}

fn default_model_paths() -> Vec<PathBuf> {
    vec![PathBuf::from("app/Models")]
}

fn default_filament_paths() -> Vec<PathBuf> {
    vec![PathBuf::from("app/Filament/Resources")]
}

fn default_migration_paths() -> Vec<PathBuf> {
    vec![PathBuf::from("database/migrations")]
}

fn default_output() -> PathBuf {
    PathBuf::from("reports/model_schema_report.json")
}

fn laravel_ignore_tables() -> Vec<String> {
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
    .map(str::to_string)
    .collect()
}

fn laravel_model_ignore_fields() -> Vec<String> {
    ["id", "created_at", "updated_at", "deleted_at"]
        .into_iter()
        .map(str::to_string)
        .collect()
}

impl IgnoreConfig {
    pub fn empty() -> Self {
        Self {
            defaults: false,
            tables: Vec::new(),
            fields: Vec::new(),
            filament: IgnoreScope::default(),
            models: IgnoreScope::default(),
            phpdoc: IgnoreScope::default(),
            rules: Vec::new(),
        }
    }

    pub fn laravel() -> Self {
        Self {
            defaults: true,
            tables: laravel_ignore_tables(),
            fields: Vec::new(),
            filament: IgnoreScope::default(),
            models: IgnoreScope {
                tables: Vec::new(),
                fields: laravel_model_ignore_fields(),
            },
            phpdoc: IgnoreScope::default(),
            rules: Vec::new(),
        }
    }

    fn scope(&self, check: Check) -> &IgnoreScope {
        match check {
            Check::Filament => &self.filament,
            Check::Models => &self.models,
            Check::Phpdoc => &self.phpdoc,
        }
    }

    fn matches_table(&self, check: Check, table: &str) -> bool {
        contains_name(&self.tables, table)
            || contains_name(&self.scope(check).tables, table)
            || self.rules.iter().any(|rule| {
                rule.applies_to_check(check)
                    && rule.applies_to_table(table)
                    && rule.fields.is_empty()
            })
    }

    fn matches_field(&self, check: Check, table: &str, field: &str) -> bool {
        contains_name(&self.fields, field)
            || contains_name(&self.scope(check).fields, field)
            || self.rules.iter().any(|rule| {
                rule.applies_to_check(check)
                    && rule.applies_to_table(table)
                    && !rule.fields.is_empty()
                    && rule.applies_to_field(field)
            })
    }
}

impl IgnoreRule {
    fn applies_to_check(&self, check: Check) -> bool {
        self.checks.is_empty() || self.checks.contains(&check)
    }

    fn applies_to_table(&self, table: &str) -> bool {
        self.tables.is_empty() || contains_name(&self.tables, table)
    }

    fn applies_to_field(&self, field: &str) -> bool {
        contains_name(&self.fields, field)
    }
}

fn default_model_parents() -> Vec<String> {
    [
        "Illuminate\\Database\\Eloquent\\Model",
        "Illuminate\\Foundation\\Auth\\User",
        "Illuminate\\Foundation\\Auth\\Authenticatable",
        "Illuminate\\Database\\Eloquent\\Relations\\Pivot",
        "Illuminate\\Database\\Eloquent\\Relations\\MorphPivot",
        "Illuminate\\Notifications\\DatabaseNotification",
    ]
    .into_iter()
    .map(str::to_string)
    .collect()
}

fn default_table_defaults() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("DatabaseNotification".into(), "notifications".into()),
        (
            "Illuminate\\Notifications\\DatabaseNotification".into(),
            "notifications".into(),
        ),
    ])
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
        ("ulid", "string"),
        ("foreignUuid", "string"),
        ("foreignUlid", "string"),
        ("ipAddress", "string"),
        ("macAddress", "string"),
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
    BTreeMap::from([(
        "Carbon".to_string(),
        "Illuminate\\Support\\Carbon".to_string(),
    )])
}

fn default_cast_type_mappings() -> BTreeMap<String, String> {
    BTreeMap::from([
        (
            "datetime".to_string(),
            "Illuminate\\Support\\Carbon".to_string(),
        ),
        (
            "immutable_datetime".to_string(),
            "Illuminate\\Support\\Carbon".to_string(),
        ),
        (
            "timestamp".to_string(),
            "Illuminate\\Support\\Carbon".to_string(),
        ),
        (
            "immutable_timestamp".to_string(),
            "Illuminate\\Support\\Carbon".to_string(),
        ),
        ("json".to_string(), "array".to_string()),
        ("array".to_string(), "array".to_string()),
        ("collection".to_string(), "array".to_string()),
        ("boolean".to_string(), "bool".to_string()),
        ("bool".to_string(), "bool".to_string()),
        ("integer".to_string(), "int".to_string()),
        ("int".to_string(), "int".to_string()),
        ("real".to_string(), "float".to_string()),
        ("float".to_string(), "float".to_string()),
        ("double".to_string(), "float".to_string()),
        ("decimal".to_string(), "float".to_string()),
        ("hashed".to_string(), "string".to_string()),
        ("encrypted".to_string(), "string".to_string()),
        ("encrypted:array".to_string(), "array".to_string()),
        ("encrypted:json".to_string(), "array".to_string()),
        ("encrypted:collection".to_string(), "array".to_string()),
        ("encrypted:object".to_string(), "object".to_string()),
    ])
}

fn default_filament_component_classes() -> Vec<String> {
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

fn default_filament_component_namespaces() -> Vec<String> {
    vec!["Filament\\Forms\\Components".into()]
}

fn default_filament_component_types() -> BTreeMap<String, String> {
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

fn default_filament_required_methods() -> Vec<String> {
    vec!["required".into()]
}

fn default_column_macros() -> BTreeMap<String, ColumnMacro> {
    let timestamps = ColumnMacro {
        columns: vec!["created_at".into(), "updated_at".into()],
        r#type: "Carbon".into(),
        nullable: true,
    };
    let soft_deletes = ColumnMacro {
        columns: vec!["deleted_at".into()],
        r#type: "Carbon".into(),
        nullable: true,
    };
    BTreeMap::from([
        ("timestamps".into(), timestamps.clone()),
        ("timestampsTz".into(), timestamps),
        ("softDeletes".into(), soft_deletes.clone()),
        ("softDeletesTz".into(), soft_deletes),
        (
            "rememberToken".into(),
            ColumnMacro {
                columns: vec!["remember_token".into()],
                r#type: "string".into(),
                nullable: true,
            },
        ),
    ])
}

fn default_morph_macros() -> BTreeMap<String, MorphMacro> {
    let int_required = MorphMacro {
        id_type: "int".into(),
        nullable: false,
    };
    let int_nullable = MorphMacro {
        id_type: "int".into(),
        nullable: true,
    };
    let string_required = MorphMacro {
        id_type: "string".into(),
        nullable: false,
    };
    let string_nullable = MorphMacro {
        id_type: "string".into(),
        nullable: true,
    };
    BTreeMap::from([
        ("morphs".into(), int_required.clone()),
        ("numericMorphs".into(), int_required),
        ("nullableMorphs".into(), int_nullable.clone()),
        ("nullableNumericMorphs".into(), int_nullable),
        ("uuidMorphs".into(), string_required.clone()),
        ("ulidMorphs".into(), string_required),
        ("nullableUuidMorphs".into(), string_nullable.clone()),
        ("nullableUlidMorphs".into(), string_nullable),
    ])
}

fn default_column_names() -> BTreeMap<String, String> {
    BTreeMap::from([
        ("id".into(), "id".into()),
        ("uuid".into(), "uuid".into()),
        ("ulid".into(), "ulid".into()),
        ("ipAddress".into(), "ip_address".into()),
        ("macAddress".into(), "mac_address".into()),
    ])
}

impl Default for ChecksConfig {
    fn default() -> Self {
        Self {
            filament: true,
            models: true,
            phpdoc: true,
        }
    }
}

impl Default for Config {
    fn default() -> Self {
        Self {
            model_paths: default_model_paths(),
            filament_paths: default_filament_paths(),
            migration_paths: default_migration_paths(),
            output_path: default_output(),
            checks: ChecksConfig::default(),
            ignore: IgnoreConfig::laravel(),
            unignore: IgnoreConfig::empty(),
            models: ModelsConfig {
                parents: default_model_parents(),
                table_defaults: default_table_defaults(),
            },
            filament: FilamentConfig {
                component_classes: default_filament_component_classes(),
                component_namespaces: default_filament_component_namespaces(),
                component_types: default_filament_component_types(),
                required_methods: default_filament_required_methods(),
            },
            migrations: MigrationsConfig {
                default_column_names: default_column_names(),
                column_macros: default_column_macros(),
                morph_macros: default_morph_macros(),
            },
            column_type_mappings: default_column_type_mappings(),
            type_normalizations: default_type_normalizations(),
            cast_type_mappings: default_cast_type_mappings(),
        }
    }
}

impl Config {
    pub fn from_toml_str(s: &str) -> crate::Result<Self> {
        let file: ConfigFile =
            toml::from_str(s).map_err(|e| crate::Error::Config(e.to_string()))?;
        Ok(file.into_config())
    }

    pub fn from_json_str(s: &str) -> crate::Result<Self> {
        let file: ConfigFile =
            serde_json::from_str(s).map_err(|e| crate::Error::Config(e.to_string()))?;
        Ok(file.into_config())
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
        self.filament_paths = self
            .filament_paths
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

    pub fn check_enabled(&self, check: Check) -> bool {
        match check {
            Check::Filament => self.checks.filament,
            Check::Models => self.checks.models,
            Check::Phpdoc => self.checks.phpdoc,
        }
    }

    pub fn ignores_table(&self, check: Check, table: &str) -> bool {
        if self.unignore.matches_table(check, table) {
            return false;
        }
        self.ignore.matches_table(check, table)
    }

    pub fn ignores_field(&self, check: Check, table: &str, field: &str) -> bool {
        if self.unignore.matches_field(check, table, field) {
            return false;
        }
        self.ignore.matches_field(check, table, field)
    }

    pub fn is_model_parent(&self, parent: &str) -> bool {
        is_configured_parent(parent, &self.models.parents)
    }

    pub fn table_default_for(&self, class_or_parent: &str) -> Option<String> {
        let trimmed = class_or_parent.trim_start_matches('\\');
        self.models
            .table_defaults
            .get(trimmed)
            .or_else(|| self.models.table_defaults.get(class_or_parent))
            .or_else(|| self.models.table_defaults.get(class_short_name(trimmed)))
            .cloned()
    }

    pub fn in_filament_component_namespace(&self, fqcn: &str) -> bool {
        let fqcn = fqcn.trim_start_matches('\\');
        self.filament.component_namespaces.iter().any(|ns| {
            let ns = ns.trim_start_matches('\\');
            fqcn == ns || fqcn.starts_with(&format!("{ns}\\"))
        })
    }
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ConfigFile {
    model_paths: Option<Vec<PathBuf>>,
    filament_paths: Option<Vec<PathBuf>>,
    migration_paths: Option<Vec<PathBuf>>,
    output_path: Option<PathBuf>,
    checks: Option<ChecksFile>,
    ignore: Option<IgnoreFile>,
    unignore: Option<IgnoreFile>,
    models: Option<ModelsFile>,
    filament: Option<FilamentFile>,
    migrations: Option<MigrationsFile>,
    column_type_mappings: Option<BTreeMap<String, String>>,
    type_normalizations: Option<BTreeMap<String, String>>,
    cast_type_mappings: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ChecksFile {
    filament: Option<bool>,
    models: Option<bool>,
    phpdoc: Option<bool>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct IgnoreFile {
    defaults: Option<bool>,
    tables: Option<Vec<String>>,
    fields: Option<Vec<String>>,
    filament: Option<IgnoreScope>,
    models: Option<IgnoreScope>,
    phpdoc: Option<IgnoreScope>,
    rules: Option<Vec<IgnoreRule>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct FilamentFile {
    component_classes: Option<Vec<String>>,
    component_namespaces: Option<Vec<String>>,
    component_types: Option<BTreeMap<String, String>>,
    required_methods: Option<Vec<String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct ModelsFile {
    parents: Option<Vec<String>>,
    #[serde(default)]
    replace_parents: bool,
    table_defaults: Option<BTreeMap<String, String>>,
}

#[derive(Debug, Default, Deserialize)]
#[serde(deny_unknown_fields)]
struct MigrationsFile {
    default_column_names: Option<BTreeMap<String, String>>,
    column_macros: Option<BTreeMap<String, ColumnMacro>>,
    morph_macros: Option<BTreeMap<String, MorphMacro>>,
}

impl ConfigFile {
    fn into_config(self) -> Config {
        let mut cfg = Config::default();
        if let Some(paths) = self.model_paths {
            cfg.model_paths = paths;
        }
        if let Some(paths) = self.filament_paths {
            cfg.filament_paths = paths;
        }
        if let Some(paths) = self.migration_paths {
            cfg.migration_paths = paths;
        }
        if let Some(path) = self.output_path {
            cfg.output_path = path;
        }
        if let Some(checks) = self.checks {
            if let Some(value) = checks.filament {
                cfg.checks.filament = value;
            }
            if let Some(value) = checks.models {
                cfg.checks.models = value;
            }
            if let Some(value) = checks.phpdoc {
                cfg.checks.phpdoc = value;
            }
        }
        if let Some(ignore) = self.ignore {
            apply_ignore_file(&mut cfg.ignore, ignore, false);
        }
        if let Some(unignore) = self.unignore {
            apply_ignore_file(&mut cfg.unignore, unignore, true);
        }
        if let Some(models) = self.models {
            if let Some(parents) = models.parents {
                if models.replace_parents {
                    cfg.models.parents = parents;
                } else {
                    extend_unique(&mut cfg.models.parents, parents);
                }
            }
            if let Some(tables) = models.table_defaults {
                cfg.models.table_defaults.extend(tables);
            }
        }
        if let Some(migrations) = self.migrations {
            if let Some(names) = migrations.default_column_names {
                cfg.migrations.default_column_names.extend(names);
            }
            if let Some(macros) = migrations.column_macros {
                cfg.migrations.column_macros.extend(macros);
            }
            if let Some(macros) = migrations.morph_macros {
                cfg.migrations.morph_macros.extend(macros);
            }
        }
        if let Some(map) = self.column_type_mappings {
            cfg.column_type_mappings.extend(map);
        }
        if let Some(map) = self.type_normalizations {
            cfg.type_normalizations.extend(map);
        }
        if let Some(map) = self.cast_type_mappings {
            cfg.cast_type_mappings.extend(map);
        }
        if let Some(filament) = self.filament {
            if let Some(classes) = filament.component_classes {
                extend_unique(&mut cfg.filament.component_classes, classes);
            }
            if let Some(namespaces) = filament.component_namespaces {
                extend_unique(&mut cfg.filament.component_namespaces, namespaces);
            }
            if let Some(map) = filament.component_types {
                cfg.filament.component_types.extend(map);
            }
            if let Some(methods) = filament.required_methods {
                extend_unique(&mut cfg.filament.required_methods, methods);
            }
        }
        cfg
    }
}

fn apply_ignore_file(target: &mut IgnoreConfig, file: IgnoreFile, is_unignore: bool) {
    if !is_unignore && file.defaults == Some(false) {
        *target = IgnoreConfig::empty();
    }
    if let Some(tables) = file.tables {
        extend_unique(&mut target.tables, tables);
    }
    if let Some(fields) = file.fields {
        extend_unique(&mut target.fields, fields);
    }
    if let Some(scope) = file.filament {
        extend_unique(&mut target.filament.tables, scope.tables);
        extend_unique(&mut target.filament.fields, scope.fields);
    }
    if let Some(scope) = file.models {
        extend_unique(&mut target.models.tables, scope.tables);
        extend_unique(&mut target.models.fields, scope.fields);
    }
    if let Some(scope) = file.phpdoc {
        extend_unique(&mut target.phpdoc.tables, scope.tables);
        extend_unique(&mut target.phpdoc.fields, scope.fields);
    }
    if let Some(rules) = file.rules {
        target.rules.extend(rules);
    }
}

fn contains_name(names: &[String], name: &str) -> bool {
    names.iter().any(|n| n == name)
}

fn extend_unique(dst: &mut Vec<String>, src: Vec<String>) {
    for value in src {
        if !dst.iter().any(|existing| existing == &value) {
            dst.push(value);
        }
    }
}

fn is_configured_parent(parent: &str, parents: &[String]) -> bool {
    let parent = parent.trim_start_matches('\\');
    let parent_short = class_short_name(parent);
    parents.iter().any(|configured| {
        let configured = configured.trim_start_matches('\\');
        parent == configured
            || parent.ends_with(&format!("\\{}", class_short_name(configured)))
            || parent_short.eq_ignore_ascii_case(class_short_name(configured))
    })
}

fn class_short_name(fqcn: &str) -> &str {
    fqcn.rsplit('\\').next().unwrap_or(fqcn)
}

fn absolutize(base: &Path, path: &Path) -> PathBuf {
    if path.is_absolute() {
        path.to_path_buf()
    } else {
        base.join(path)
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn overlay_merges_ignore_tables() {
        let cfg = Config::from_toml_str(
            r#"
[ignore]
tables = ["audit_logs"]
"#,
        )
        .unwrap();
        assert!(cfg.ignores_table(Check::Models, "failed_jobs"));
        assert!(cfg.ignores_table(Check::Filament, "audit_logs"));
        assert!(!cfg.ignores_table(Check::Models, "users"));
    }

    #[test]
    fn defaults_false_replaces_laravel_ignores() {
        let cfg = Config::from_toml_str(
            r#"
[ignore]
defaults = false
tables = ["only_this"]
"#,
        )
        .unwrap();
        assert!(!cfg.ignores_table(Check::Models, "failed_jobs"));
        assert!(cfg.ignores_table(Check::Phpdoc, "only_this"));
        assert!(!cfg.ignores_field(Check::Models, "users", "id"));
    }

    #[test]
    fn unignore_re_enables_default_table() {
        let cfg = Config::from_toml_str(
            r#"
[unignore]
tables = ["jobs"]
"#,
        )
        .unwrap();
        assert!(cfg.ignores_table(Check::Models, "failed_jobs"));
        assert!(!cfg.ignores_table(Check::Models, "jobs"));
    }

    #[test]
    fn per_check_field_ignore() {
        let cfg = Config::from_toml_str(
            r#"
[ignore.filament]
fields = ["remember_token"]
"#,
        )
        .unwrap();
        assert!(cfg.ignores_field(Check::Filament, "users", "remember_token"));
        assert!(!cfg.ignores_field(Check::Models, "users", "remember_token"));
        assert!(cfg.ignores_field(Check::Models, "users", "id"));
    }

    #[test]
    fn rule_scopes_field_to_table_and_check() {
        let cfg = Config::from_toml_str(
            r#"
[[ignore.rules]]
tables = ["users"]
fields = ["remember_token"]
checks = ["filament"]
"#,
        )
        .unwrap();
        assert!(cfg.ignores_field(Check::Filament, "users", "remember_token"));
        assert!(!cfg.ignores_field(Check::Filament, "posts", "remember_token"));
        assert!(!cfg.ignores_field(Check::Phpdoc, "users", "remember_token"));
    }

    #[test]
    fn unignore_field_wins() {
        let cfg = Config::from_toml_str(
            r#"
[unignore.models]
fields = ["id"]
"#,
        )
        .unwrap();
        assert!(!cfg.ignores_field(Check::Models, "users", "id"));
        assert!(cfg.ignores_field(Check::Models, "users", "created_at"));
    }

    #[test]
    fn disable_phpdoc_check() {
        let cfg = Config::from_toml_str(
            r#"
[checks]
phpdoc = false
"#,
        )
        .unwrap();
        assert!(cfg.check_enabled(Check::Filament));
        assert!(!cfg.check_enabled(Check::Phpdoc));
    }

    #[test]
    fn unknown_key_is_rejected() {
        let err = Config::from_toml_str("[ignored_tables]\njobs = [\"models\"]\n").unwrap_err();
        assert!(err.to_string().contains("unknown"));
    }

    #[test]
    fn model_parent_overlay_appends() {
        let cfg = Config::from_toml_str(
            r#"
[models]
parents = ["App\\Models\\BaseModel"]
"#,
        )
        .unwrap();
        assert!(cfg.is_model_parent("Illuminate\\Database\\Eloquent\\Relations\\Pivot"));
        assert!(cfg.is_model_parent("App\\Models\\BaseModel"));
    }

    #[test]
    fn example_toml_loads_as_overlay() {
        let text = include_str!("../model-schema.example.toml");
        let cfg = Config::from_toml_str(text).unwrap();
        assert!(cfg.ignores_table(Check::Models, "failed_jobs"));
        assert!(cfg.ignores_field(Check::Models, "users", "id"));
        assert!(cfg.check_enabled(Check::Phpdoc));
    }

    #[test]
    fn type_normalization_overlay_merges() {
        let cfg = Config::from_toml_str(
            r#"
[type_normalizations]
Point = "Clickbar\\Magellan\\Data\\Geometries\\Point"
"#,
        )
        .unwrap();
        assert_eq!(
            cfg.type_normalizations.get("Carbon").map(String::as_str),
            Some("Illuminate\\Support\\Carbon")
        );
        assert_eq!(
            cfg.type_normalizations.get("Point").map(String::as_str),
            Some("Clickbar\\Magellan\\Data\\Geometries\\Point")
        );
    }
}
