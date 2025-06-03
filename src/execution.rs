use crate::{
    cache,
    task::Task,
    util::{cleanup_outputs, expand_globs, hash_files, run_command},
};
use std::{path::PathBuf, time::SystemTime};

pub struct TaskRunner<'a> {
    tasks: &'a [Task],
    cache: &'a mut cache::Cache,
    rm: bool,
    verbose: bool,
}

impl<'a> TaskRunner<'a> {
    pub fn new(tasks: &'a [Task], cache: &'a mut cache::Cache, rm: bool, verbose: bool) -> Self {
        Self {
            tasks,
            cache,
            rm,
            verbose,
        }
    }

    pub fn run_tasks(&mut self, task_ids: &[String]) -> bool {
        for task_id in task_ids {
            let task = match self.tasks.iter().find(|t| &t.id == task_id) {
                Some(task) => task,
                None => {
                    eprintln!("Error: task {} found in queue but not in task map", task_id);
                    return false;
                }
            };

            if self.should_run_task(task) {
                if self.verbose {
                    println!("Running task: {}", task.id);
                }

                if !self.execute_task(task) {
                    return false;
                }
            }
        }

        true
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
                let hash_key = hash.to_hex();
                if !self.cache.contains_key(&hash_key) {
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

    fn execute_task(&mut self, task: &Task) -> bool {
        match run_command(&task.command) {
            Ok(status) if status.success() => {
                if !task.inputs.is_empty() {
                    if let Ok(hash) = hash_files(task.inputs.clone()) {
                        self.cache.insert(hash.to_hex().to_string());
                    }
                }

                if (self.rm || task.auto_remove) && !task.outputs.is_empty() {
                    if let Err(e) = cleanup_outputs(&task.outputs, self.verbose) {
                        eprintln!("Warning: Cleanup failed for task '{}': {}", task.id, e);
                    }
                }

                true
            }
            Ok(status) => {
                eprintln!("Error: Task '{}' failed with status: {}", task.id, status);
                false
            }
            Err(e) => {
                eprintln!("Error: Task '{}' failed to execute: {}", task.id, e);
                false
            }
        }
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
