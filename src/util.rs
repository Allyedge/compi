use blake3::Hash;
use glob::{GlobError, PatternError, glob};
use std::{
    collections::HashSet,
    fmt, fs,
    path::{Path, PathBuf},
};
use std::{
    io::Error,
    process::{Command, ExitStatus},
};

#[derive(Debug)]
pub enum FileError {
    GlobPattern(PatternError),
    GlobExpansion(GlobError),
    Io(Error),
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

impl From<Error> for FileError {
    fn from(err: Error) -> Self {
        FileError::Io(err)
    }
}

pub fn expand_globs(paths: &[PathBuf]) -> Result<Vec<PathBuf>, FileError> {
    let mut result = Vec::new();
    let mut seen = HashSet::new();

    for path in paths {
        let path_str = path.to_string_lossy();

        if is_glob_pattern(&path_str) {
            expand_single_glob(&path_str, &mut result, &mut seen)?;
        } else {
            add_if_exists(path, &mut result, &mut seen);
        }
    }

    Ok(result)
}

fn is_glob_pattern(path: &str) -> bool {
    path.contains('*') || path.contains('?') || path.contains('[')
}

fn expand_single_glob(
    pattern: &str,
    result: &mut Vec<PathBuf>,
    seen: &mut HashSet<PathBuf>,
) -> Result<(), FileError> {
    let glob_paths = glob(pattern)?;

    for entry in glob_paths {
        let expanded_path = entry?;
        if expanded_path.is_file() && seen.insert(expanded_path.clone()) {
            result.push(expanded_path);
        }
    }

    Ok(())
}

fn add_if_exists(path: &Path, result: &mut Vec<PathBuf>, seen: &mut HashSet<PathBuf>) {
    if path.exists() && seen.insert(path.to_path_buf()) {
        result.push(path.to_path_buf());
    } else if !path.exists() {
        eprintln!("Warning: Input file '{}' does not exist", path.display());
    }
}

pub fn hash_files(inputs: Vec<PathBuf>) -> Result<Hash, FileError> {
    let expanded_files = match expand_globs(&inputs) {
        Ok(files) => files,
        Err(FileError::GlobPattern(e)) => {
            eprintln!("Error: Invalid glob pattern: {}", e);
            return Err(FileError::GlobPattern(e));
        }
        Err(FileError::GlobExpansion(e)) => {
            eprintln!("Error: Failed to expand glob pattern: {}", e);
            return Err(FileError::GlobExpansion(e));
        }
        Err(e) => return Err(e),
    };

    if expanded_files.is_empty() {
        return Ok(blake3::hash(b""));
    }

    let mut sorted_files = expanded_files;
    sorted_files.sort();

    let mut hashes: Vec<Hash> = Vec::new();

    for file_path in sorted_files {
        match fs::read(&file_path) {
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
    for hash in hashes {
        combined_hash_data.extend_from_slice(hash.as_bytes());
    }

    Ok(blake3::hash(&combined_hash_data))
}

pub fn run_command(command: &str) -> Result<ExitStatus, Error> {
    let mut cmd = if cfg!(target_os = "windows") {
        let mut c = Command::new("cmd");
        c.args(["/C", command]);
        c
    } else {
        let mut c = Command::new("sh");
        c.args(["-c", command]);
        c
    };

    cmd.status()
}

pub fn cleanup_outputs(outputs: &[PathBuf], verbose: bool) -> Result<(), FileError> {
    if outputs.is_empty() {
        return Ok(());
    }

    let expanded_outputs = expand_globs(outputs)?;

    for output_path in expanded_outputs {
        if output_path.exists() {
            let result = if output_path.is_dir() {
                fs::remove_dir_all(&output_path)
            } else {
                fs::remove_file(&output_path)
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
