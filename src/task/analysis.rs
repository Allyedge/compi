use std::{collections::HashMap, path::Path};

use super::Task;

pub fn show_task_relationships(tasks: &[Task], verbose: bool) {
    if !verbose {
        return;
    }

    let task_map: HashMap<&str, &Task> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();

    for task in tasks {
        for dep_id in &task.dependencies {
            if let Some(dep_task) = task_map.get(dep_id.as_str()) {
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

    if is_glob_pattern(&input_str) {
        if let Ok(glob_paths) = glob::glob(&input_str) {
            for entry in glob_paths.flatten() {
                if entry == *output {
                    return true;
                }
            }
        }
    }

    if input_str.contains("**") {
        if let Some(prefix) = input_str.split("**").next() {
            if !prefix.is_empty() && output_str.starts_with(prefix) {
                return true;
            }
        }
    }

    false
}

fn is_glob_pattern(path: &str) -> bool {
    path.contains('*') || path.contains('?') || path.contains('[')
}
