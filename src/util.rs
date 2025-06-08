use blake3::Hash;
use glob::{GlobError, PatternError, glob};
use std::process::{Output, Stdio};
use std::{
    collections::HashSet,
    fmt, fs,
    io::Error as IoError,
    path::{Path, PathBuf},
    time::Duration,
};
use tokio::io::AsyncReadExt;
use tokio::process::Command as TokioCommand;

#[derive(Debug)]
pub enum FileError {
    GlobPattern(PatternError),
    GlobExpansion(GlobError),
    Io(IoError),
}

#[derive(Debug)]
pub enum CommandError {
    Io(IoError),
    Timeout,
}

impl fmt::Display for FileError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            FileError::GlobPattern(e) => write!(f, "Invalid glob pattern: {}", e),
            FileError::GlobExpansion(e) => write!(f, "Failed to expand glob: {}", e),
            FileError::Io(e) => write!(f, "IO error: {}", e),
        }
    }
}

impl std::error::Error for FileError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FileError::GlobPattern(e) => Some(e),
            FileError::GlobExpansion(e) => Some(e),
            FileError::Io(e) => Some(e),
        }
    }
}

impl From<PatternError> for FileError {
    fn from(err: PatternError) -> Self {
        FileError::GlobPattern(err)
    }
}

impl From<GlobError> for FileError {
    fn from(err: GlobError) -> Self {
        FileError::GlobExpansion(err)
    }
}

impl From<IoError> for FileError {
    fn from(err: IoError) -> Self {
        FileError::Io(err)
    }
}

impl fmt::Display for CommandError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            CommandError::Io(e) => write!(f, "Command execution error: {}", e),
            CommandError::Timeout => write!(f, "Command timed out"),
        }
    }
}

impl std::error::Error for CommandError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            CommandError::Io(e) => Some(e),
            CommandError::Timeout => None,
        }
    }
}

pub fn parse_timeout(timeout_str: Option<&str>, default_timeout: Option<&str>) -> Option<Duration> {
    let timeout_to_parse = timeout_str.or(default_timeout)?;

    if timeout_to_parse == "0" || timeout_to_parse.is_empty() {
        return None;
    }

    match timeout_to_parse.parse::<humantime::Duration>() {
        Ok(duration) => Some(duration.into()),
        Err(e) => {
            eprintln!(
                "Warning: Invalid timeout format '{}': {}",
                timeout_to_parse, e
            );
            eprintln!("Use duration format like '5m', '30s', '1h30m'");
            None
        }
    }
}

pub fn expand_globs(paths: &[PathBuf]) -> Result<Vec<PathBuf>, FileError> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();

    for path in paths {
        let path_str = path.to_string_lossy();

        if is_glob_pattern(&path_str) {
            let expanded_paths = expand_single_glob(&path_str)?;
            for expanded_path in expanded_paths {
                if expanded_path.is_file() && seen.insert(expanded_path.clone()) {
                    result.push(expanded_path);
                }
            }
        } else {
            add_if_exists(path, &mut result, &mut seen);
        }
    }

    Ok(result)
}

fn is_glob_pattern(path: &str) -> bool {
    path.contains('*') || path.contains('?') || path.contains('[')
}

fn expand_single_glob(pattern: &str) -> Result<Vec<PathBuf>, FileError> {
    let glob_paths = glob(pattern)?;
    glob_paths
        .collect::<Result<Vec<_>, _>>()
        .map_err(FileError::from)
}

fn add_if_exists(path: &Path, result: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    if path.exists() && seen.insert(path.to_path_buf()) {
        result.push(path.to_path_buf());
    } else if !path.exists() {
        eprintln!("Warning: Input file '{}' does not exist", path.display());
    }
}

pub fn hash_files(inputs: Vec<PathBuf>) -> Result<Hash, FileError> {
    let expanded_files = expand_globs(&inputs)?;

    if expanded_files.is_empty() {
        return Ok(blake3::hash(b""));
    }

    let mut sorted_files = expanded_files;
    sorted_files.sort();

    let mut hashes = Vec::new();

    for file_path in &sorted_files {
        match fs::read(file_path) {
            Ok(contents) => {
                let path_str = file_path.to_string_lossy();
                let combined = format!("{}:{}", path_str.len(), path_str);
                let mut combined_bytes = combined.into_bytes();
                combined_bytes.extend_from_slice(&contents);

                hashes.push(blake3::hash(&combined_bytes));
            }
            Err(e) => {
                eprintln!(
                    "Warning: Could not read file '{}': {}",
                    file_path.display(),
                    e
                );
            }
        }
    }

    if hashes.is_empty() {
        return Ok(blake3::hash(b""));
    }

    let mut combined_hash_data = Vec::new();
    for hash in &hashes {
        combined_hash_data.extend_from_slice(hash.as_bytes());
    }

    Ok(blake3::hash(&combined_hash_data))
}

pub async fn run_command_with_timeout(
    command: &str,
    timeout: Option<Duration>,
) -> Result<std::process::Output, CommandError> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = TokioCommand::new("cmd");
        c.args(["/C", command]);
        c
    } else {
        let mut c = TokioCommand::new("sh");
        c.args(["-c", command]);
        c
    };

    cmd.stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .stdin(Stdio::null());

    let mut child = cmd.spawn().map_err(CommandError::Io)?;

    match timeout {
        Some(duration) => {
            tokio::select! {
                result = child.wait() => {
                    let status = result.map_err(CommandError::Io)?;

                    let mut stdout = Vec::new();
                    let mut stderr = Vec::new();

                    if let Some(mut stdout_pipe) = child.stdout.take() {
                        stdout_pipe.read_to_end(&mut stdout).await.map_err(CommandError::Io)?;
                    }

                    if let Some(mut stderr_pipe) = child.stderr.take() {
                        stderr_pipe.read_to_end(&mut stderr).await.map_err(CommandError::Io)?;
                    }

                    Ok(Output {
                        status,
                        stdout,
                        stderr,
                    })
                }
                _ = tokio::time::sleep(duration) => {
                    if let Err(kill_err) = child.kill().await {
                        eprintln!("Warning: Failed to kill timed-out process: {}", kill_err);
                    }
                    let _ = child.wait().await;
                    Err(CommandError::Timeout)
                }
            }
        }
        None => child.wait_with_output().await.map_err(CommandError::Io),
    }
}

pub fn cleanup_outputs(outputs: &[PathBuf], verbose: bool) -> Result<(), FileError> {
    if outputs.is_empty() {
        return Ok(());
    }

    let expanded_outputs = expand_globs(outputs)?;

    for output_path in &expanded_outputs {
        if output_path.exists() {
            let result = if output_path.is_dir() {
                fs::remove_dir_all(output_path)
            } else {
                fs::remove_file(output_path)
            };

            match result {
                Ok(()) => {
                    if verbose {
                        println!("Removed: {}", output_path.display());
                    }
                }
                Err(e) => {
                    eprintln!(
                        "Warning: Failed to remove '{}': {}",
                        output_path.display(),
                        e
                    );
                }
            }
        }
    }

    Ok(())
}
