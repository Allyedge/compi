use std::{
    collections::{HashMap, HashSet, VecDeque, hash_map::Entry::Occupied},
    fs,
    path::{Path, PathBuf},
    process,
};

use serde::Deserialize;

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
}

#[derive(Debug, Deserialize)]
struct Config {
    #[serde(rename = "task")]
    tasks: HashMap<String, Task>,
    config: Option<ConfigSection>,
}

#[derive(Debug, Deserialize)]
struct ConfigSection {
    default: Option<String>,
    cache_dir: Option<String>,
}

pub fn load_tasks(config_path: &str) -> (Vec<Task>, Option<String>, Option<String>) {
    let contents = fs::read_to_string(config_path).unwrap_or_else(|e| {
        eprintln!("Error reading {}: {}", config_path, e);
        process::exit(1);
    });

    let config: Config = toml::from_str(&contents).unwrap_or_else(|e| {
        eprintln!("Error parsing {}: {}", config_path, e);
        process::exit(1);
    });

    let default_task = config.config.as_ref().and_then(|c| c.default.clone());
    let cache_dir = config.config.as_ref().and_then(|c| c.cache_dir.clone());

    let tasks: Vec<Task> = config
        .tasks
        .into_iter()
        .map(|(name, mut task)| {
            if task.id.is_empty() {
                task.id = name;
            }
            task
        })
        .collect();

    validate_tasks(&tasks).unwrap_or_else(|e| {
        eprintln!("Error: {}", e);
        process::exit(1);
    });

    (tasks, default_task, cache_dir)
}

pub fn sort_topologically(tasks: &[Task]) -> Vec<String> {
    let mut in_degrees: HashMap<String, i32> = HashMap::new();

    for task in tasks {
        in_degrees.insert(task.id.clone(), task.dependencies.len() as i32);
    }

    let mut queue: VecDeque<String> = VecDeque::new();
    for (task_id, in_degree) in &in_degrees {
        if *in_degree == 0 {
            queue.push_back(task_id.clone());
        }
    }

    let mut sorted_tasks: Vec<String> = Vec::new();

    while let Some(task) = queue.pop_front() {
        sorted_tasks.push(task.clone());

        for dependant in tasks {
            if !dependant.dependencies.contains(&task) {
                continue;
            }

            let entry = in_degrees
                .entry(dependant.id.clone())
                .and_modify(|c| *c -= 1);

            if let Occupied(entry) = entry {
                if *entry.get() == 0 {
                    queue.push_back(dependant.id.clone());
                }
            } else {
                eprintln!(
                    "Error: task {} found in dependencies but not in map",
                    dependant.id
                );
                break;
            }
        }
    }

    sorted_tasks
}

fn validate_tasks(tasks: &[Task]) -> Result<(), String> {
    let task_map: HashMap<String, &Task> = tasks.iter().map(|t| (t.id.clone(), t)).collect();

    for task in tasks {
        for dep_id in &task.dependencies {
            if !task_map.contains_key(dep_id) {
                return Err(format!(
                    "Task '{}' depends on '{}' which doesn't exist",
                    task.id, dep_id
                ));
            }
        }
    }

    detect_cycles(tasks)?;
    Ok(())
}

fn detect_cycles(tasks: &[Task]) -> Result<(), String> {
    let task_map: HashMap<String, &Task> = tasks.iter().map(|t| (t.id.clone(), t)).collect();

    for task in tasks {
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        if has_cycle(&task.id, &task_map, &mut visited, &mut path) {
            path.push(task.id.clone());
            return Err(format!("Circular dependency: {}", path.join(" -> ")));
        }
    }

    Ok(())
}

fn has_cycle(
    task_id: &str,
    task_map: &HashMap<String, &Task>,
    visited: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> bool {
    if path.contains(&task_id.to_string()) {
        return true;
    }

    if visited.contains(task_id) {
        return false;
    }

    visited.insert(task_id.to_string());
    path.push(task_id.to_string());

    if let Some(task) = task_map.get(task_id) {
        for dep in &task.dependencies {
            if has_cycle(dep, task_map, visited, path) {
                return true;
            }
        }
    }

    path.pop();
    false
}

pub fn show_task_relationships(tasks: &[Task], verbose: bool) {
    if !verbose {
        return;
    }

    let task_map: HashMap<String, &Task> = tasks.iter().map(|t| (t.id.clone(), t)).collect();

    for task in tasks {
        for dep_id in &task.dependencies {
            if let Some(dep_task) = task_map.get(dep_id) {
                if !has_file_relationship(task, dep_task) {
                    println!(
                        "Info: Task '{}' depends on '{}' for ordering only",
                        task.id, dep_id
                    );
                }
            }
        }
    }
}

fn has_file_relationship(task: &Task, dependency: &Task) -> bool {
    if dependency.outputs.is_empty() || task.inputs.is_empty() {
        return false;
    }

    for dep_output in &dependency.outputs {
        for task_input in &task.inputs {
            if paths_match(dep_output, task_input) {
                return true;
            }
        }
    }

    false
}

fn paths_match(output: &Path, input: &Path) -> bool {
    let output_str = output.to_string_lossy();
    let input_str = input.to_string_lossy();

    if output_str == input_str {
        return true;
    }

    if input_str.contains('*') || input_str.contains('?') || input_str.contains('[') {
        if let Ok(glob_paths) = glob::glob(&input_str) {
            for entry in glob_paths.flatten() {
                if entry == *output {
                    return true;
                }
            }
        }
    }

    if input_str.contains("**") {
        let prefix = input_str.split("**").next().unwrap_or("");
        if !prefix.is_empty() && output_str.starts_with(prefix) {
            return true;
        }
    }

    false
}

pub fn get_required_tasks(tasks: &[Task], target_task_id: &str) -> Result<Vec<String>, String> {
    let task_map: HashMap<String, &Task> = tasks.iter().map(|t| (t.id.clone(), t)).collect();

    if !task_map.contains_key(target_task_id) {
        return Err(format!("Task '{}' not found", target_task_id));
    }

    let mut needed_tasks = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back(target_task_id.to_string());

    while let Some(current_task_id) = queue.pop_front() {
        if needed_tasks.contains(&current_task_id) {
            continue;
        }

        needed_tasks.insert(current_task_id.clone());

        if let Some(task) = task_map.get(&current_task_id) {
            for dep in &task.dependencies {
                if !needed_tasks.contains(dep) {
                    queue.push_back(dep.clone());
                }
            }
        }
    }

    let filtered_tasks: Vec<Task> = tasks
        .iter()
        .filter(|task| needed_tasks.contains(&task.id))
        .cloned()
        .collect();

    Ok(sort_topologically(&filtered_tasks))
}
