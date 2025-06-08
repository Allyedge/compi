use std::{collections::HashMap, path::PathBuf, sync::Arc, thread, time::SystemTime};
use tokio::sync::Semaphore;

use crate::{
    cache,
    task::Task,
    util::{
        CommandError, cleanup_outputs, expand_globs, hash_files, parse_timeout,
        run_command_with_timeout,
    },
};

fn default_workers() -> usize {
    thread::available_parallelism()
        .map(|n| n.get())
        .unwrap_or(1)
}

#[derive(Debug)]
pub struct ExecutionLevel {
    pub level: usize,
    pub task_ids: Vec<String>,
}

pub fn calculate_dependency_levels(tasks: &[Task]) -> Vec<ExecutionLevel> {
    let task_map: HashMap<&str, &Task> = tasks.iter().map(|t| (t.id.as_str(), t)).collect();
    let mut levels: HashMap<String, usize> = HashMap::new();

    for task in tasks {
        calculate_task_level(&task.id, &task_map, &mut levels);
    }

    let mut level_groups: HashMap<usize, Vec<String>> = HashMap::new();
    for (task_id, level) in levels {
        level_groups.entry(level).or_default().push(task_id);
    }

    let mut execution_levels: Vec<ExecutionLevel> = level_groups
        .into_iter()
        .map(|(level, task_ids)| ExecutionLevel { level, task_ids })
        .collect();

    execution_levels.sort_by_key(|el| el.level);
    execution_levels
}

fn calculate_task_level(
    task_id: &str,
    task_map: &HashMap<&str, &Task>,
    levels: &mut HashMap<String, usize>,
) -> usize {
    if let Some(&level) = levels.get(task_id) {
        return level;
    }

    let task = match task_map.get(task_id) {
        Some(task) => task,
        None => {
            levels.insert(task_id.to_string(), 0);
            return 0;
        }
    };

    if task.dependencies.is_empty() {
        levels.insert(task_id.to_string(), 0);
        return 0;
    }

    let max_dep_level = task
        .dependencies
        .iter()
        .map(|dep| calculate_task_level(dep, task_map, levels))
        .max()
        .unwrap_or(0);

    let level = max_dep_level + 1;
    levels.insert(task_id.to_string(), level);
    level
}

pub struct TaskRunner<'a> {
    tasks: &'a [Task],
    cache: &'a mut cache::Cache,
    rm: bool,
    verbose: bool,
    default_timeout: Option<String>,
    workers: usize,
    continue_on_failure: bool,
}

impl<'a> TaskRunner<'a> {
    pub fn new(
        tasks: &'a [Task],
        cache: &'a mut cache::Cache,
        rm: bool,
        verbose: bool,
        default_timeout: Option<String>,
        workers: Option<usize>,
        continue_on_failure: bool,
    ) -> Self {
        let workers = workers.unwrap_or_else(default_workers);
        Self {
            tasks,
            cache,
            rm,
            verbose,
            default_timeout,
            workers,
            continue_on_failure,
        }
    }

    pub async fn run_tasks(&mut self, task_ids: &[String]) -> bool {
        let tasks_to_run: Vec<Task> = task_ids
            .iter()
            .filter_map(|task_id| self.tasks.iter().find(|t| &t.id == task_id))
            .cloned()
            .collect();

        if tasks_to_run.is_empty() {
            return false;
        }

        let execution_levels = calculate_dependency_levels(&tasks_to_run);

        if self.verbose {
            println!(
                "Executing {} levels with up to {} workers:",
                execution_levels.len(),
                self.workers
            );
            for level in &execution_levels {
                println!("  Level {}: {} tasks", level.level, level.task_ids.len());
            }
        }

        let mut any_cache_updated = false;

        for level in execution_levels {
            if self.verbose {
                println!(
                    "Level {}: Running {} tasks in parallel",
                    level.level,
                    level.task_ids.len()
                );
            }

            let level_result = self.execute_level_parallel(&level.task_ids).await;

            match level_result {
                Ok(cache_updated) => {
                    if cache_updated {
                        any_cache_updated = true;
                    }
                }
                Err(_) => {
                    if self.continue_on_failure {
                        eprintln!(
                            "Level {} had failures, but continuing due to --continue-on-failure",
                            level.level
                        );
                    } else {
                        eprintln!("Level {} failed, stopping execution", level.level);
                        return false;
                    }
                }
            }
        }

        any_cache_updated
    }

    async fn execute_level_parallel(&mut self, task_ids: &[String]) -> Result<bool, ()> {
        if task_ids.is_empty() {
            return Ok(false);
        }

        let semaphore = Arc::new(Semaphore::new(self.workers));
        let mut handles = Vec::new();
        let mut any_cache_updated = false;

        for task_id in task_ids {
            let task = match self.tasks.iter().find(|t| &t.id == task_id) {
                Some(task) => task,
                None => {
                    eprintln!("Error: task {} not found", task_id);
                    return Err(());
                }
            };

            if !self.should_run_task(task) {
                if self.verbose {
                    println!("Task '{}': outputs up-to-date, skipping", task.id);
                }
                continue;
            }

            let task_clone = task.clone();
            let semaphore_clone = Arc::clone(&semaphore);
            let default_timeout = self.default_timeout.clone();
            let rm = self.rm;
            let verbose = self.verbose;

            let handle = tokio::spawn(async move {
                let _permit = semaphore_clone.acquire().await.unwrap();

                if verbose {
                    println!("Running task: {}", task_clone.id);
                }

                Self::execute_single_task(&task_clone, default_timeout, rm, verbose).await
            });

            handles.push((task.id.clone(), handle));
        }

        for (task_id, handle) in handles {
            match handle.await {
                Ok(Ok(cache_updated)) => {
                    if cache_updated {
                        any_cache_updated = true;
                        if let Some(task) = self.tasks.iter().find(|t| t.id == task_id) {
                            if !task.inputs.is_empty() {
                                if let Ok(hash) = hash_files(task.inputs.clone()) {
                                    self.cache.insert(hash.to_hex().to_string());
                                }
                            }
                        }
                    }
                }
                Ok(Err(_)) => {
                    eprintln!("Task '{}' failed", task_id);
                    if !self.continue_on_failure {
                        return Err(());
                    }
                }
                Err(e) => {
                    eprintln!("Task '{}' panicked: {}", task_id, e);
                    if !self.continue_on_failure {
                        return Err(());
                    }
                }
            }
        }

        Ok(any_cache_updated)
    }

    async fn execute_single_task(
        task: &Task,
        default_timeout: Option<String>,
        rm: bool,
        verbose: bool,
    ) -> Result<bool, ()> {
        let timeout = parse_timeout(task.timeout.as_deref(), default_timeout.as_deref());

        match run_command_with_timeout(&task.command, timeout).await {
            Ok(status) if status.success() => {
                let cache_updated = !task.inputs.is_empty();

                if (rm || task.auto_remove) && !task.outputs.is_empty() {
                    if let Err(e) = cleanup_outputs(&task.outputs, verbose) {
                        eprintln!("Warning: Cleanup failed for task '{}': {}", task.id, e);
                    }
                }

                Ok(cache_updated)
            }
            Ok(status) => {
                eprintln!("Error: Task '{}' failed with status: {}", task.id, status);
                Err(())
            }
            Err(CommandError::Timeout) => {
                eprintln!("Error: Task '{}' timed out", task.id);
                Err(())
            }
            Err(CommandError::Io(e)) => {
                eprintln!("Error: Task '{}' failed to execute: {}", task.id, e);
                Err(())
            }
        }
    }

    fn should_run_task(&self, task: &Task) -> bool {
        if task.inputs.is_empty() {
            if self.verbose {
                println!("Task '{}': no inputs, always run", task.id);
            }
            return true;
        }

        if !outputs_exist(task) {
            if self.verbose {
                println!("Task '{}': outputs missing, must run", task.id);
            }
            return true;
        }

        if outputs_outdated(task) {
            if self.verbose {
                println!("Task '{}': outputs older than inputs, must run", task.id);
            }
            return true;
        }

        match hash_files(task.inputs.clone()) {
            Ok(hash) => {
                let hash_key = hash.to_hex().to_string();
                if !self.cache.contains(&hash_key) {
                    if self.verbose {
                        println!("Task '{}': input content changed, must run", task.id);
                    }
                    return true;
                }
            }
            Err(e) => {
                eprintln!(
                    "Error: Could not process inputs for task '{}': {}",
                    task.id, e
                );
                return true;
            }
        }

        if self.verbose {
            println!("Task '{}': outputs up-to-date, skipping", task.id);
        }
        false
    }
}

fn outputs_exist(task: &Task) -> bool {
    if task.outputs.is_empty() {
        return true;
    }

    task.outputs.iter().all(|output| output.exists())
}

fn outputs_outdated(task: &Task) -> bool {
    if task.outputs.is_empty() || task.inputs.is_empty() {
        return false;
    }

    let newest_input_time = match newest_timestamp(&task.inputs) {
        Some(time) => time,
        None => return true,
    };

    let oldest_output_time = match oldest_timestamp(&task.outputs) {
        Some(time) => time,
        None => return true,
    };

    newest_input_time > oldest_output_time
}

fn newest_timestamp(paths: &[PathBuf]) -> Option<SystemTime> {
    let expanded_paths = expand_globs(paths).ok()?;

    expanded_paths
        .iter()
        .filter_map(|path| {
            path.metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
        })
        .max()
}

fn oldest_timestamp(paths: &[PathBuf]) -> Option<SystemTime> {
    paths
        .iter()
        .filter_map(|path| {
            path.metadata()
                .ok()
                .and_then(|metadata| metadata.modified().ok())
        })
        .min()
}
