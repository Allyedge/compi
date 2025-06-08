use std::{
    collections::HashSet,
    fs::{self, File},
    io::{BufReader, BufWriter},
    path::{Path, PathBuf},
};

const DEFAULT_CACHE_DIR: &str = ".";
const CACHE_FILENAME: &str = "compi_cache.json";

pub type Cache = HashSet<String>;

pub fn load_cache(cache_dir: Option<&str>, config_path: &str) -> Cache {
    let cache_path = get_cache_path(cache_dir, config_path);

    let file = match File::open(&cache_path) {
        Ok(file) => file,
        Err(_) => return Cache::default(),
    };

    let reader = BufReader::new(file);
    serde_json::from_reader(reader).unwrap_or_default()
}

pub fn save_cache(cache: &Cache, cache_dir: Option<&str>, config_path: &str) {
    let cache_path = get_cache_path(cache_dir, config_path);

    if let Some(parent) = cache_path.parent() {
        if let Err(e) = fs::create_dir_all(parent) {
            eprintln!("Warning: Failed to create cache directory: {}", e);
            return;
        }
    }

    match File::create(&cache_path) {
        Ok(file) => {
            let writer = BufWriter::new(file);
            if let Err(e) = serde_json::to_writer_pretty(writer, cache) {
                eprintln!("Warning: Failed to write cache file: {}", e);
            }
        }
        Err(e) => {
            eprintln!("Warning: Failed to open cache file for writing: {}", e);
        }
    }
}

fn get_cache_path(cache_dir: Option<&str>, config_path: &str) -> PathBuf {
    let config_parent = Path::new(config_path)
        .parent()
        .unwrap_or_else(|| Path::new("."));

    let cache_dir = cache_dir.unwrap_or(DEFAULT_CACHE_DIR);

    let cache_dir_path = if Path::new(cache_dir).is_absolute() {
        PathBuf::from(cache_dir)
    } else {
        config_parent.join(cache_dir)
    };

    cache_dir_path.join(CACHE_FILENAME)
}
