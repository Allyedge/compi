use clap::Parser;
use std::process;

mod cache;
mod cli;
mod error;
mod execution;
mod output;
mod task;
mod util;

use cache::{load_cache, save_cache};
use cli::Cli;
use error::Result;
use execution::TaskRunner;
use output::OutputMode;
use task::{get_required_tasks, load_tasks, show_task_relationships, sort_topologically};

#[tokio::main]
async fn main() -> Result<()> {
    let args = Cli::parse();

    match run_compi(args).await {
        Ok(()) => Ok(()),
        Err(e) => {
            eprintln!("Error: {}", e);
            process::exit(1);
        }
    }
}

async fn run_compi(args: Cli) -> Result<()> {
    let config = load_tasks(&args.file)?;
    let mut tasks = config.tasks;

    show_task_relationships(&tasks, args.verbose);

    let task_list = match &args.task {
        Some(task_id) => get_required_tasks(&tasks, task_id)?,
        None => {
            if let Some(default) = &config.default_task {
                get_required_tasks(&tasks, default)?
            } else {
                sort_topologically(&tasks)
            }
        }
    };

    tasks.retain(|task| task_list.contains(&task.id));

    if args.verbose {
        println!("Task execution order: {}", task_list.join(" -> "));
    }

    if args.dry_run {
        println!("Dry run mode - showing what would be executed:");
        for task_id in &task_list {
            if let Some(task) = tasks.iter().find(|t| t.id == *task_id) {
                println!("  {} would run: {}", task.id, task.command);
            }
        }
        return Ok(());
    }

    let workers = args.workers.or(config.workers);
    let default_timeout = args.timeout.or(config.default_timeout);
    let output_mode = args
        .output
        .clone()
        .or(config.output.clone())
        .unwrap_or(OutputMode::Group);

    let mut cache = load_cache(config.cache_dir.as_deref(), &args.file);
    let mut runner = TaskRunner::new(
        &tasks,
        &mut cache,
        args.rm,
        args.verbose,
        default_timeout,
        workers,
        args.continue_on_failure,
        output_mode,
    );
    let cache_changed = runner.run_tasks(&task_list).await;

    if cache_changed {
        save_cache(&cache, config.cache_dir.as_deref(), &args.file);
    } else if args.verbose {
        println!("No changes detected, cache not saved.");
    }

    Ok(())
}
