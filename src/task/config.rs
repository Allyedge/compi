use std::{collections::HashMap, env, fs, path::PathBuf};

use regex::Regex;
use serde::Deserialize;

use super::{Task, dependency::validate_tasks};
use crate::error::Result;

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(rename = "task")]
    tasks: HashMap<String, Task>,
    config: Option<ConfigSection>,
    #[serde(default)]
    variables: HashMap<String, String>,
}

#[derive(Debug, Deserialize)]
struct ConfigSection {
    default: Option<String>,
    cache_dir: Option<String>,
    workers: Option<usize>,
    default_timeout: Option<String>,
}

#[derive(Debug)]
pub struct TaskConfiguration {
    pub tasks: Vec<Task>,
    pub default_task: Option<String>,
    pub cache_dir: Option<String>,
    pub workers: Option<usize>,
    pub default_timeout: Option<String>,
}

pub fn load_tasks(config_path: &str) -> Result<TaskConfiguration> {
    let config = load_and_parse_config(config_path)?;
    process_config(config)
}

fn load_and_parse_config(config_path: &str) -> Result<Config> {
    let contents = fs::read_to_string(config_path)?;
    let config = toml::from_str(&contents)?;
    Ok(config)
}

fn process_config(config: Config) -> Result<TaskConfiguration> {
    let default_task = config.config.as_ref().and_then(|c| c.default.clone());
    let cache_dir = config.config.as_ref().and_then(|c| c.cache_dir.clone());
    let workers = config.config.as_ref().and_then(|c| c.workers);
    let default_timeout = config
        .config
        .as_ref()
        .and_then(|c| c.default_timeout.clone());

    let mut variables = config.variables;
    add_builtin_variables(&mut variables);

    let tasks: Vec<Task> = config
        .tasks
        .into_iter()
        .map(|(name, mut task)| {
            if task.id.is_empty() {
                task.id = name;
            }
            substitute_variables_in_task(&mut task, &variables);
            task
        })
        .collect();

    validate_tasks(&tasks)?;

    Ok(TaskConfiguration {
        tasks,
        default_task,
        cache_dir,
        workers,
        default_timeout,
    })
}

fn add_builtin_variables(variables: &mut HashMap<String, String>) {
    for (key, value) in env::vars() {
        variables.insert(format!("ENV_{}", key), value);
    }

    if let Ok(pwd) = env::current_dir() {
        variables.insert("PWD".to_string(), pwd.to_string_lossy().to_string());
    }
}

fn substitute_variables_in_task(task: &mut Task, variables: &HashMap<String, String>) {
    task.command = substitute_variables(&task.command, variables);

    task.inputs = task
        .inputs
        .iter()
        .map(|path| PathBuf::from(substitute_variables(&path.to_string_lossy(), variables)))
        .collect();

    task.outputs = task
        .outputs
        .iter()
        .map(|path| PathBuf::from(substitute_variables(&path.to_string_lossy(), variables)))
        .collect();
}

fn substitute_variables(text: &str, variables: &HashMap<String, String>) -> String {
    let braced_regex = Regex::new(r"\$\{([A-Za-z_][A-Za-z0-9_]*)\}").unwrap();
    let simple_regex = Regex::new(r"\$([A-Za-z_][A-Za-z0-9_]*)\b").unwrap();

    let mut result = braced_regex
        .replace_all(text, |caps: &regex::Captures| {
            let var_name = &caps[1];
            variables
                .get(var_name)
                .cloned()
                .unwrap_or_else(|| caps[0].to_string())
        })
        .to_string();

    result = simple_regex
        .replace_all(&result, |caps: &regex::Captures| {
            let var_name = &caps[1];
            variables
                .get(var_name)
                .cloned()
                .unwrap_or_else(|| caps[0].to_string())
        })
        .to_string();

    result
}
