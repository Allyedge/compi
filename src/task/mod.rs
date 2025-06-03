pub mod analysis;
pub mod config;

use serde::Deserialize;
use std::path::PathBuf;

#[derive(Debug, Deserialize, Clone)]
pub struct Task {
    #[serde(default)]
    pub id: String,
    pub command: String,
    #[serde(default)]
    pub dependencies: Vec<String>,
    #[serde(default)]
    pub inputs: Vec<PathBuf>,
    #[serde(default)]
    pub outputs: Vec<PathBuf>,
    #[serde(default)]
    pub auto_remove: bool,
}

pub use analysis::{get_required_tasks, show_task_relationships, sort_topologically};
pub use config::load_tasks;
