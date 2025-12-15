use clap::Parser;

use crate::output::OutputMode;

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
pub struct Cli {
    /// Configuration file to use
    #[arg(short = 'f', long = "file", default_value = "compi.toml")]
    pub file: String,

    /// Enable verbose output
    #[arg(short = 'v', long = "verbose")]
    pub verbose: bool,

    /// Remove outputs after successful task execution
    #[arg(long = "rm")]
    pub rm: bool,

    /// Override number of worker threads for parallel execution
    #[arg(short = 'j', long = "workers")]
    pub workers: Option<usize>,

    /// Override default timeout (e.g., "5m", "30s", "1h30m")
    #[arg(short = 't', long = "timeout")]
    pub timeout: Option<String>,

    /// Show what would be executed without running tasks
    #[arg(long = "dry-run")]
    pub dry_run: bool,

    /// Continue executing independent tasks even if some fail
    #[arg(long = "continue-on-failure")]
    pub continue_on_failure: bool,

    /// How to display task output in the terminal
    #[arg(long = "output", value_enum)]
    pub output: Option<OutputMode>,

    /// Task to run, runs default task or all tasks if not specified
    pub task: Option<String>,
}
