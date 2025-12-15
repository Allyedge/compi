use clap::ValueEnum;
use serde::Deserialize;

#[derive(ValueEnum, Deserialize, Clone, Debug)]
#[serde(rename_all = "lowercase")]
pub enum OutputMode {
    /// Stream task output live.
    Stream,
    /// Print each task's output as a single block after it completes.
    Group,
}
