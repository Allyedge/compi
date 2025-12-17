use std::collections::{hash_map::Entry::Occupied, HashMap, HashSet, VecDeque};

use super::Task;
use crate::error::{CompiError, Result};

pub fn sort_topologically(tasks: &[Task]) -> Vec<String> {
    let mut in_degrees: HashMap<&str, usize> = HashMap::new();

    for task in tasks {
        in_degrees.insert(&task.id, task.dependencies.len());
    }

    let mut queue: VecDeque<&str> = VecDeque::new();
    for (task_id, &in_degree) in &in_degrees {
        if in_degree == 0 {
            queue.push_back(task_id);
        }
    }

    let mut sorted_tasks: Vec<String> = Vec::new();

    while let Some(task_id) = queue.pop_front() {
        sorted_tasks.push(task_id.to_string());

        for dependent in tasks {
            if !dependent.dependencies.iter().any(|dep| dep == task_id) {
                continue;
            }

            let entry = in_degrees.entry(&dependent.id).and_modify(|c| *c -= 1);

            if let Occupied(entry) = entry {
                if *entry.get() == 0 {
                    queue.push_back(&dependent.id);
                }
            }
        }
    }

    sorted_tasks
}

pub fn validate_tasks(tasks: &[Task]) -> Result<()> {
    let task_ids: HashSet<&str> = tasks.iter().map(|t| t.id.as_str()).collect();
    let mut aliases: HashMap<&str, &str> = HashMap::new();

    for task in tasks {
        for dep_id in &task.dependencies {
            if dep_id == &task.id {
                return Err(CompiError::Dependency(format!(
                    "Task '{}' depends on itself",
                    task.id
                )));
            }
            if !task_ids.contains(dep_id.as_str()) {
                return Err(CompiError::Dependency(format!(
                    "Task '{}' depends on '{}' which doesn't exist",
                    task.id, dep_id
                )));
            }
        }

        for alias in &task.aliases {
            if task_ids.contains(alias.as_str()) {
                return Err(CompiError::Dependency(format!(
                    "Task '{}' defines alias '{}' which conflicts with task ID '{}'",
                    task.id, alias, alias
                )));
            }

            if let Some(existing_task) = aliases.get(alias.as_str()) {
                return Err(CompiError::Dependency(format!(
                    "Task '{}' defines alias '{}' which is already used by task '{}'",
                    task.id, alias, existing_task
                )));
            }

            aliases.insert(alias.as_str(), &task.id);
        }
    }

    detect_cycles(tasks)?;
    Ok(())
}

pub fn get_required_tasks(tasks: &[Task], target_task_id: &str) -> Result<Vec<String>> {
    let task_map: HashMap<&str, &Task> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();

    let mut resolved_id = target_task_id;

    if !task_map.contains_key(resolved_id) {
        let alias_match = tasks
            .iter()
            .find(|t| t.aliases.iter().any(|a| a == target_task_id));

        match alias_match {
            Some(task) => {
                resolved_id = &task.id;
            }
            None => {
                return Err(CompiError::Task(format!(
                    "Task '{}' not found",
                    target_task_id
                )));
            }
        }
    }

    let mut needed_tasks = HashSet::new();
    let mut queue = VecDeque::new();

    queue.push_back(resolved_id);

    while let Some(current_task_id) = queue.pop_front() {
        if needed_tasks.contains(current_task_id) {
            continue;
        }

        needed_tasks.insert(current_task_id);

        if let Some(task) = task_map.get(current_task_id) {
            for dep in &task.dependencies {
                if !needed_tasks.contains(dep.as_str()) {
                    queue.push_back(dep);
                }
            }
        }
    }

    let filtered_tasks: Vec<Task> = tasks
        .iter()
        .filter(|task| needed_tasks.contains(task.id.as_str()))
        .cloned()
        .collect();

    Ok(sort_topologically(&filtered_tasks))
}

fn detect_cycles(tasks: &[Task]) -> Result<()> {
    let task_map: HashMap<&str, &Task> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();

    for task in tasks {
        let mut visited = HashSet::new();
        let mut path = Vec::new();

        if has_cycle(&task.id, &task_map, &mut visited, &mut path) {
            path.push(task.id.clone());
            return Err(CompiError::Dependency(format!(
                "Circular dependency: {}",
                path.join(" -> ")
            )));
        }
    }

    Ok(())
}

fn has_cycle(
    task_id: &str,
    task_map: &HashMap<&str, &Task>,
    visited: &mut HashSet<String>,
    path: &mut Vec<String>,
) -> bool {
    if path.iter().any(|id| id == task_id) {
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
