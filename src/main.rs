mod cache;
mod task;
mod util;

use crate::cache::{load_cache, save_cache};
use crate::task::{
    Task, get_required_tasks, load_tasks, show_task_relationships, sort_topologically,
};
use clap::Parser;
use std::collections::HashMap;
use std::path::PathBuf;
use std::process;
use std::time::SystemTime;
use util::{expand_globs, hash_files, run_command};

#[derive(Parser)]
#[command(author, version, about, long_about = None)]
struct Cli {
    /// Configuration file to use
    #[arg(short = 'f', long = "file", default_value = "compi.toml")]
    file: String,

    /// Enable verbose output
    #[arg(short = 'v', long = "verbose")]
    verbose: bool,

    /// Task to run
    task: Option<String>,
}

fn main() {
    let cli = Cli::parse();

    let (tasks, default_task, cache_dir) = load_tasks(&cli.file);

    show_task_relationships(&tasks, cli.verbose);

    let mut cache = load_cache(cache_dir.as_deref());
    let mut cache_changed = false;

    let mut task_map: HashMap<String, &Task> = HashMap::new();
    for task in &tasks {
        task_map.insert(task.id.clone(), task);
    }

    let sorted_tasks = determine_tasks_to_run(&tasks, &cli.task, &default_task);

    for task_id in sorted_tasks {
        let task = match task_map.get(&task_id) {
            Some(task) => task,
            None => {
                eprintln!("Error: task {} found in queue but not in task map", task_id);
                break;
            }
        };

        if should_run_task(task, &cache, cli.verbose) {
            if cli.verbose {
                println!("Running task: {}", task.id);
            }

            if execute_task(task, &mut cache) {
                cache_changed = true;
            } else {
                break;
            }
        }
    }

    if cache_changed {
        save_cache(&cache, cache_dir.as_deref());
    } else if cli.verbose {
        println!("No changes detected, cache not saved.");
    }
}

fn determine_tasks_to_run(
    tasks: &[Task],
    target_task: &Option<String>,
    default_task: &Option<String>,
) -> Vec<String> {
    if let Some(target) = target_task {
        match get_required_tasks(tasks, target) {
            Ok(task_ids) => task_ids,
            Err(e) => {
                eprintln!("Error: {}", e);
                process::exit(1);
            }
        }
    } else if let Some(default) = default_task {
        match get_required_tasks(tasks, default) {
            Ok(task_ids) => task_ids,
            Err(e) => {
                eprintln!("Error with default task '{}': {}", default, e);
                process::exit(1);
            }
        }
    } else {
        sort_topologically(tasks)
    }
}

fn should_run_task(task: &Task, cache: &cache::Cache, verbose: bool) -> bool {
    if task.inputs.is_empty() {
        if verbose {
            println!("Task '{}': no inputs, always run", task.id);
        }
        return true;
    }

    if !outputs_exist(task) {
        if verbose {
            println!("Task '{}': outputs missing, must run", task.id);
        }
        return true;
    }

    if outputs_outdated(task) {
        if verbose {
            println!("Task '{}': outputs older than inputs, must run", task.id);
        }
        return true;
    }

    match hash_files(task.inputs.clone()) {
        Ok(hash) => {
            let hash_key = hash.to_hex();
            if !cache.contains_key(&hash_key) {
                if verbose {
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

    if verbose {
        println!("Task '{}': outputs up-to-date, skipping", task.id);
    }
    false
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
    let mut newest: Option<SystemTime> = None;

    let expanded_paths = match expand_globs(paths) {
        Ok(paths) => paths,
        Err(_) => return None,
    };

    for path in expanded_paths {
        if let Ok(metadata) = path.metadata() {
            if let Ok(modified) = metadata.modified() {
                newest = Some(newest.map_or(modified, |n: SystemTime| n.max(modified)));
            }
        }
    }

    newest
}

fn oldest_timestamp(paths: &[PathBuf]) -> Option<SystemTime> {
    let mut oldest: Option<SystemTime> = None;

    for path in paths {
        if let Ok(metadata) = path.metadata() {
            if let Ok(modified) = metadata.modified() {
                oldest = Some(oldest.map_or(modified, |o: SystemTime| o.min(modified)));
            }
        }
    }

    oldest
}

fn execute_task(task: &Task, cache: &mut cache::Cache) -> bool {
    match run_command(&task.command) {
        Ok(status) if status.success() => {
            if !task.inputs.is_empty() {
                if let Ok(hash) = hash_files(task.inputs.clone()) {
                    cache.insert(hash.to_hex().to_string());
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
