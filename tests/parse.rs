use laravel_model_schema::ir::Analysis;
use laravel_model_schema::parse::{parse_filament, parse_migrations, parse_models};
use laravel_model_schema::Config;
use std::path::PathBuf;

fn fixture_config() -> Config {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let mut cfg = Config::default();
    cfg.model_paths = vec![root.join("models")];
    cfg.filament_paths = vec![root.join("filament")];
    cfg.migration_paths = vec![root.join("migrations")];
    cfg
}

#[test]
fn parses_post_model() {
    let cfg = fixture_config();
    let mut analysis = Analysis::default();
    parse_models(&cfg, &mut analysis);

    let posts = analysis.tables.get("posts").expect("posts table");
    assert_eq!(
        posts.model_fields.get("title").map(|f| f.fillable),
        Some(true)
    );
    assert_eq!(
        posts
            .model_fields
            .get("published_at")
            .and_then(|f| f.cast.as_deref()),
        Some("datetime")
    );
    assert_eq!(
        posts
            .model_fields
            .get("visibility")
            .and_then(|f| f.cast.as_deref()),
        Some("Tests\\Fixtures\\Models\\Visibility")
    );
    assert_eq!(
        posts.model_fields.get("status").map(|f| f.accessor),
        Some(true)
    );
    let user = posts
        .model_relationships
        .get("user")
        .expect("user relation");
    assert_eq!(user.r#type, "BelongsTo");
    assert_eq!(user.model, "Tests\\Fixtures\\Models\\User");
}

#[test]
fn parses_migration_columns() {
    let cfg = fixture_config();
    let mut analysis = Analysis::default();
    parse_migrations(&cfg, &mut analysis);
    let posts = analysis.tables.get("posts").expect("posts");
    assert!(posts.migration_fields.contains_key("title"));
    assert_eq!(posts.migration_fields["title"].r#type, "string");
    assert!(posts.migration_fields["published_at"].nullable);
    assert!(posts.migration_fields.contains_key("created_at"));
    assert_eq!(posts.migration_fields["id"].r#type, "int");
    assert!(!posts.migration_fields["id"].nullable);
    assert_eq!(posts.migration_fields["author_id"].r#type, "int");
    assert_eq!(posts.migration_fields["author_type"].r#type, "string");
    assert!(!posts.migration_fields["author_id"].nullable);

    let notes = analysis.tables.get("notes").expect("notes");
    assert_eq!(notes.migration_fields["id"].r#type, "string");
    assert!(notes.migration_fields["notable_id"].nullable);
    assert_eq!(notes.migration_fields["notable_type"].r#type, "string");

    let notifications = analysis.tables.get("notifications").expect("notifications");
    assert_eq!(
        notifications.migration_fields["notifiable_id"].r#type,
        "int"
    );
    assert_eq!(
        notifications.migration_fields["notifiable_type"].r#type,
        "string"
    );
}

#[test]
fn parses_property_casts_and_pivot_models() {
    let cfg = fixture_config();
    let mut analysis = Analysis::default();
    parse_models(&cfg, &mut analysis);

    let comments = analysis.tables.get("comments").expect("comments");
    assert_eq!(
        comments
            .model_fields
            .get("edited_at")
            .and_then(|f| f.cast.as_deref()),
        Some("datetime")
    );

    let pivot = analysis.tables.get("post_tag").expect("post_tag");
    assert_eq!(
        pivot.model_fqcn.as_deref(),
        Some("Tests\\Fixtures\\Models\\PostTag")
    );
    assert_eq!(
        pivot.model_fields.get("post_id").map(|f| f.fillable),
        Some(true)
    );

    let notifications = analysis.tables.get("notifications").expect("notifications");
    assert_eq!(
        notifications.model_fqcn.as_deref(),
        Some("Tests\\Fixtures\\Models\\DatabaseNotification")
    );
    assert!(!analysis.tables.contains_key("database_notifications"));
}

#[test]
fn parses_filament_form_fields() {
    let cfg = fixture_config();
    let mut analysis = Analysis::default();
    parse_models(&cfg, &mut analysis);
    parse_filament(&cfg, &mut analysis);
    let posts = analysis.tables.get("posts").expect("posts");
    assert!(posts.filament_form_fields.contains_key("title"));
    assert!(!posts.filament_form_fields["title"].nullable);
    assert!(posts.filament_form_fields.contains_key("legacy_field"));
}

#[test]
fn parses_fqcn_collection_property_read() {
    let cfg = fixture_config();
    let mut analysis = Analysis::default();
    parse_models(&cfg, &mut analysis);

    let accounts = analysis.tables.get("accounts").expect("accounts");
    let balances = accounts
        .phpdoc_read_fields
        .get("balances")
        .expect("balances @property-read");
    assert_eq!(balances.r#type, "Tests\\Fixtures\\Models\\AccountBalance");
    assert_eq!(balances.array_type.as_deref(), Some("Collection"));
    assert_eq!(balances.key_type.as_deref(), Some("int"));
}

#[test]
fn schema_json_maps_raw_database_types() {
    use laravel_model_schema::parse::load_schema_json;
    use std::io::Write;

    let mut file = tempfile::NamedTempFile::new().expect("temp file");
    write!(
        file,
        r#"{{
            "posts": {{
                "id": {{ "name": "id", "type": "bigint", "nullable": false }},
                "title": {{ "name": "title", "type": "varchar", "nullable": false }},
                "is_public": {{ "name": "is_public", "type": "tinyint(1)", "nullable": false }},
                "rank": {{ "name": "rank", "type": "tinyint", "nullable": true }},
                "published_at": {{ "name": "published_at", "type": "datetime", "nullable": true }},
                "meta": {{ "name": "meta", "type": "JSON", "nullable": true }},
                "location": {{ "name": "location", "type": "geography", "nullable": true }},
                "created_at": {{ "name": "created_at", "type": "Carbon", "nullable": true }}
            }}
        }}"#
    )
    .expect("write schema");

    let mut cfg = Config::default();
    // Project overrides win over the built-in database names.
    cfg.column_type_mappings
        .insert("geography".into(), "Point".into());

    let mut analysis = Analysis::default();
    load_schema_json(file.path(), &cfg, &mut analysis).expect("load schema");
    let fields = &analysis
        .tables
        .get("posts")
        .expect("posts")
        .migration_fields;
    let type_of = |name: &str| fields.get(name).map(|f| f.r#type.as_str());

    assert_eq!(type_of("id"), Some("int"));
    assert_eq!(type_of("title"), Some("string"));
    assert_eq!(type_of("is_public"), Some("bool"));
    assert_eq!(type_of("rank"), Some("int"));
    assert_eq!(type_of("published_at"), Some("Carbon"));
    assert_eq!(type_of("meta"), Some("array"));
    assert_eq!(type_of("location"), Some("Point"));
    // Already-normalized dumps pass through untouched.
    assert_eq!(type_of("created_at"), Some("Carbon"));
}
