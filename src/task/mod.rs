pub mod analysis;
pub mod config;
pub mod dependency;

pub use analysis::show_task_relationships;
pub use config::load_tasks;
pub use dependency::{get_required_tasks, sort_topologically};

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
    #[serde(default)]
    pub timeout: Option<String>,
}
