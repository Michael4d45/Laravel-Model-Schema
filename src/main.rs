use clap::{Parser, Subcommand};
use laravel_model_schema::{actionable, analyze, write_report, Config};
use std::path::PathBuf;

#[derive(Parser)]
#[command(
    name = "model-schema",
    about = "Report drift between Laravel migrations, Eloquent models, and Filament forms"
)]
struct Cli {
    #[command(subcommand)]
    command: Commands,
}

#[derive(Subcommand)]
enum Commands {
    /// Compare models, Filament forms, and migrations or a live schema dump
    Check {
        /// TOML or JSON config file
        #[arg(long, short)]
        config: Option<PathBuf>,
        /// JSON config string
        #[arg(long)]
        config_json: Option<String>,
        /// Live DB schema dump (skips migration parse)
        #[arg(long)]
        schema_json: Option<PathBuf>,
        /// Write the full JSON report to this path
        #[arg(long)]
        output: Option<PathBuf>,
        /// Only print the actionable report JSON
        #[arg(long)]
        json_only: bool,
    },
}

fn main() {
    let cli = Cli::parse();
    if let Err(err) = run(cli) {
        eprintln!("Error: {err}");
        std::process::exit(1);
    }
}

fn run(cli: Cli) -> laravel_model_schema::Result<()> {
    match cli.command {
        Commands::Check {
            config,
            config_json,
            schema_json,
            output,
            json_only,
        } => {
            let cfg = if let Some(json) = config_json {
                Config::from_json_str(&json)?
            } else if let Some(path) = config {
                Config::load_file(&path)?
            } else if PathBuf::from("model-schema.toml").is_file() {
                Config::load_file(&PathBuf::from("model-schema.toml"))?
            } else {
                Config::default()
            };
            let (analysis, report) = analyze(&cfg, schema_json.as_deref())?;
            let full = serde_json::json!({
                "report": report,
                "tables": analysis.tables,
            });
            let output_path = output.unwrap_or_else(|| cfg.output_path.clone());
            if !json_only {
                write_report(&output_path, &full)?;
                eprintln!("Report written to: {}", output_path.display());
            }
            let actionable_json = serde_json::to_string_pretty(&actionable(&report))
                .map_err(|e| laravel_model_schema::Error::Other(e.to_string()))?;
            println!("{actionable_json}");
            Ok(())
        }
    }
}
