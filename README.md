# Compi

![GitHub License](https://img.shields.io/github/license/allyedge/compi)
![Crates.io Total Downloads](https://img.shields.io/crates/d/compi)

A build system written in Rust.

## Features

- **Clean TOML structure**: Simple and declarative task definitions.
- **Incremental Builds**: Tracks file hashes and modification times to skip unnecessary work.
- **Dependencies**: Automatic DAG resolution for complex build chains.
- **Parallel Execution**: Concurrent execution of independent tasks.

## Installation

### From crates.io

```bash
cargo install compi
```

### From GitHub

```bash
cargo install --git https://github.com/allyedge/compi
```

### From Source

```bash
git clone https://github.com/allyedge/compi
cd compi
cargo install --path .
```

### From GitHub Releases

Download the latest binary for your platform from the [GitHub Releases](https://github.com/allyedge/compi/releases) page.

#### Linux

```bash
wget https://github.com/allyedge/compi/releases/latest/download/compi-linux
chmod +x compi-linux
sudo mv compi-linux /usr/local/bin/compi
```

#### macOS

```bash
wget https://github.com/allyedge/compi/releases/latest/download/compi-macos
chmod +x compi-macos
sudo mv compi-macos /usr/local/bin/compi
```

#### Windows

Download the executable from the releases page and add it to your PATH.

## CLI Usage

| Flag | Description |
|------|-------------|
| `-f, --file <FILE>` | Configuration file (default: `compi.toml`) |
| `-j, --workers <N>` | Number of parallel workers (default: CPU cores) |
| `-t, --timeout <DURATION>` | Default timeout (e.g., "30s", "5m") |
| `--output <MODE>` | Output mode: `group` (default) or `stream` |
| `--dry-run` | Preview execution order without running tasks |
| `--rm` | Remove output files after successful execution |
| `-v, --verbose` | Enable verbose logging |

```bash
compi
compi build
compi -j 8 build
compi -t 5m test
compi --rm build
```

## Configuration Reference

Create a `compi.toml` in your project root.

```toml
[config]
default = "build"
cache_dir = ".compi_cache"
workers = 4
default_timeout = "10m"
output = "group"

[variables]
TARGET = "target"
SRC = "src/**/*.rs"
FLAGS = "--release"

[task.prepare]
command = "mkdir -p ${TARGET}"
outputs = ["${TARGET}/"]
aliases = ["p"]

[task.build]
dependencies = ["prepare"]
command = "cargo build ${FLAGS}"
inputs = ["${SRC}", "Cargo.toml"]
outputs = ["${TARGET}/app"]
aliases = ["b"]

[task.test]
dependencies = ["build"]
command = "cargo test"
inputs = ["tests/**/*.rs"]
always_run = true
aliases = ["t"]

[task.clean]
command = "rm -rf ${TARGET}"
```

## Reference

### Task Fields

| Field | Type | Description |
|-------|------|-------------|
| `command` | String | **Required.** Shell command to execute. |
| `dependencies` | [String] | List of task IDs that must complete first. |
| `inputs` | [String] | List of files/globs to track for changes. |
| `outputs` | [String] | List of files/globs this task produces. |
| `aliases` | [String] | Short names for CLI invocation (e.g. `["b"]`). |
| `always_run` | Boolean | If true, ignore cache and always execute. |
| `auto_remove` | Boolean | If true, delete outputs after success (temp files). |
| `timeout` | String | Duration string (e.g. "30s") for this specific task. |

### Caching & Execution Logic

Compi uses a local cache (`compi_cache.json`) to skip tasks that are up-to-date.

A task is **SKIPPED** if:
1. All `outputs` exist.
2. The `inputs` content hash matches the previous run.
3. The `inputs` modification times are older than the `outputs`.

A task **RUNS** if:
1. It has no `inputs` defined.
2. `always_run` is set to `true`.
3. Any output file is missing.
4. Input files have changed (content hash mismatch).
5. Input files are newer than output files.

### Output Cleanup

- **`--rm` flag**: Deletes files listed in `outputs` after the task succeeds.
- **`auto_remove = true`**: Acts like `--rm` is always passed for that specific task.

## License

[MIT](./LICENSE)
