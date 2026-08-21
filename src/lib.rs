pub mod config;
pub mod error;
pub mod ir;
pub mod parse;
pub mod pipeline;
pub mod report;

pub use config::{Check, Config};
pub use error::{Error, Result};
pub use ir::Analysis;
pub use pipeline::{actionable, analyze, check, write_report, CheckStats};
pub use report::Report;
