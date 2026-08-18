use laravel_model_schema::ir::Analysis;
use laravel_model_schema::parse::{parse_filament, parse_migrations, parse_models};
use laravel_model_schema::Config;
use std::path::PathBuf;

fn fixture_config() -> Config {
    let root = PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("fixtures");
    let mut cfg = Config::default();
    cfg.model_paths = vec![root.join("models")];
    cfg.resource_paths = vec![root.join("filament")];
    cfg.migration_paths = vec![root.join("migrations")];
    cfg
}

#[test]
fn parses_surveyor_post_model() {
    let cfg = fixture_config();
    let mut analysis = Analysis::default();
    parse_models(&cfg, &mut analysis);

    let posts = analysis
        .tables
        .get("surveyor_posts")
        .expect("surveyor_posts table");
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
        posts.model_fields.get("status").map(|f| f.accessor),
        Some(true)
    );
    let user = posts
        .model_relationships
        .get("user")
        .expect("user relation");
    assert_eq!(user.r#type, "BelongsTo");
    assert!(
        user.model.contains("SurveyorUser"),
        "related model: {}",
        user.model
    );
}

#[test]
fn parses_migration_columns() {
    let cfg = fixture_config();
    let mut analysis = Analysis::default();
    parse_migrations(&cfg, &mut analysis);
    let posts = analysis
        .tables
        .get("surveyor_posts")
        .expect("surveyor_posts");
    assert!(posts.migration_fields.contains_key("title"));
    assert_eq!(posts.migration_fields["title"].r#type, "string");
    assert!(posts.migration_fields["published_at"].nullable);
    assert!(posts.migration_fields.contains_key("created_at"));
}

#[test]
fn parses_filament_form_fields() {
    let cfg = fixture_config();
    let mut analysis = Analysis::default();
    parse_models(&cfg, &mut analysis);
    parse_filament(&cfg, &mut analysis);
    let posts = analysis
        .tables
        .get("surveyor_posts")
        .expect("surveyor_posts");
    assert!(posts.filament_form_fields.contains_key("title"));
    assert!(!posts.filament_form_fields["title"].nullable);
    assert!(posts.filament_form_fields.contains_key("legacy_field"));
}
