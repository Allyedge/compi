use super::Task;
use serde::Deserialize;
use std::{collections::HashMap, env, fs, path::PathBuf, process};

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
}

#[derive(Debug)]
pub struct TaskConfiguration {
    pub tasks: Vec<Task>,
    pub default_task: Option<String>,
    pub cache_dir: Option<String>,
}

pub fn load_tasks(config_path: &str) -> (Vec<Task>, Option<String>, Option<String>) {
    let config = load_and_parse_config(config_path);
    let task_config = process_config(config);
    (
        task_config.tasks,
        task_config.default_task,
        task_config.cache_dir,
    )
}

fn load_and_parse_config(config_path: &str) -> Config {
    let contents = fs::read_to_string(config_path).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", config_path, e);
        process::exit(1);
    });

    toml::from_str(&contents).unwrap_or_else(|e| {
        eprintln!("Error parsing {}: {}", config_path, e);
        process::exit(1);
    })
}

fn process_config(config: Config) -> TaskConfiguration {
    let default_task = config.config.as_ref().and_then(|c| c.default.clone());
    let cache_dir = config.config.as_ref().and_then(|c| c.cache_dir.clone());

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

    super::analysis::validate_tasks(&tasks).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        process::exit(1);
    });

    TaskConfiguration {
        tasks,
        default_task,
        cache_dir,
    }
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
    let mut result = text.to_string();

    for (key, value) in variables {
        let pattern1 = format!("${{{}}}", key);
        let pattern2 = format!("${}", key);
        result = result.replace(&pattern1, value);
        result = result.replace(&pattern2, value);
    }

    result
}
