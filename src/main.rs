mod cache;
mod cli;
mod execution;
mod task;
mod util;

use crate::cache::{load_cache, save_cache};
use crate::execution::TaskRunner;
use crate::task::{
    Task, get_required_tasks, load_tasks, show_task_relationships, sort_topologically,
};
use clap::Parser;
use cli::Cli;
use std::process;

fn main() {
    let cli = Cli::parse();

    let (tasks, default_task, cache_dir) = load_tasks(&cli.file);

    show_task_relationships(&tasks, cli.verbose);

    let mut cache = load_cache(cache_dir.as_deref(), &cli.file);

    let sorted_tasks = determine_tasks_to_run(&tasks, &cli.task, &default_task);

    let mut runner = TaskRunner::new(&tasks, &mut cache, cli.rm, cli.verbose);
    let cache_changed = runner.run_tasks(&sorted_tasks);

    if cache_changed {
        save_cache(&cache, cache_dir.as_deref(), &cli.file);
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
