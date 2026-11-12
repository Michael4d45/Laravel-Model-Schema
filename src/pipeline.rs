use std::path::Path;

use crate::ir::Analysis;
use crate::parse::{load_schema_json, parse_filament, parse_migrations, parse_models};
use crate::report::{actionable_report, generate_report, Report};
use crate::{Config, Result};

#[derive(Debug, Clone)]
pub struct CheckStats {
    pub tables: usize,
}

pub fn check(
    config: &Config,
    schema_json: Option<&Path>,
) -> Result<(serde_json::Value, CheckStats)> {
    let (analysis, report) = analyze(config, schema_json)?;
    let stats = CheckStats {
        tables: analysis.tables.len(),
    };
    let full = serde_json::json!({
        "report": report,
        "tables": analysis.tables,
    });
    Ok((full, stats))
}

pub fn analyze(config: &Config, schema_json: Option<&Path>) -> Result<(Analysis, Report)> {
    let mut analysis = Analysis::default();
    parse_models(config, &mut analysis);
    parse_filament(config, &mut analysis);
    if let Some(path) = schema_json {
        load_schema_json(path, config, &mut analysis)?;
    } else {
        parse_migrations(config, &mut analysis);
    }
    let report = generate_report(config, &analysis);
    Ok((analysis, report))
}

pub fn actionable(report: &Report) -> serde_json::Value {
    actionable_report(report)
}

pub fn write_report(path: &Path, full: &serde_json::Value) -> Result<()> {
    if let Some(parent) = path.parent() {
        if !parent.as_os_str().is_empty() {
            std::fs::create_dir_all(parent)?;
        }
    }
    let json =
        serde_json::to_string_pretty(full).map_err(|e| crate::Error::Other(e.to_string()))?;
    std::fs::write(path, json + "\n")?;
    Ok(())
}
