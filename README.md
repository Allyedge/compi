# Compi

![GitHub License](https://img.shields.io/github/license/allyedge/compi)

![Crates.io Total Downloads](https://img.shields.io/crates/d/compi)

A build system written in Rust.

## Features

- **Clean TOML structure**: Compi uses a clean TOML structure to define tasks
- **Safety**: Compi uses dependencies, inputs and outputs defined for the tasks to warn you of potential issues
- **Dependencies**: By using dependencies, Compi allows for a clean representation of very complex command chains

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

```bash
# Run default task
compi

# Run specific task
compi build

# Use a different config file
compi -f my-config.toml

# Verbose output
compi -v build

# Remove outputs after successful execution
compi --rm build

# Combine flags
compi --rm -v test

# Show help
compi --help
```

### Command Options

- `-f, --file <FILE>`: Configuration file (default: `compi.toml`)
- `-v, --verbose`: Enable verbose output
- `--rm`: Remove outputs after successful task execution
- `TASK`: Task to run

## Configuration Format

Create a `compi.toml` file in your project root:

### Basic Structure

```toml
[config]
default = "build"           # Default task to run
cache_dir = "cache"         # Cache directory

[task.task_name]
command = "shell command"   # Command to execute
dependencies = ["dep1"]     # Tasks that must run before this one
inputs = ["src/*.rs"]       # Input files
outputs = ["target/app"]    # Output files
auto_remove = false         # Automatically remove outputs after successful execution
```

### Example Configuration

```toml
[config]
default = "build"
cache_dir = ".build-cache"

[variables]
TARGET_DIR = "target"
APP_NAME = "myapp"
SOURCE_PATTERN = "src/**/*.rs"
TEST_PATTERN = "tests/**/*.rs"

[task.prepare]
id = "prep"
command = "mkdir -p ${TARGET_DIR}"
outputs = ["${TARGET_DIR}/"]

[task.build]
command = "cargo build"
dependencies = ["prep"]
inputs = [
    "${SOURCE_PATTERN}",
    "Cargo.toml"
]
outputs = ["${TARGET_DIR}/debug/${APP_NAME}"]

[task.test]
command = "cargo test"
dependencies = ["build"]
inputs = ["${SOURCE_PATTERN}", "${TEST_PATTERN}"]

[task.clean]
command = "rm -rf ${TARGET_DIR}/"
```

## Variables

Compi supports variables for reducing duplication and making configurations more maintainable.

### Variable Definition

Define variables in the `[variables]` section:

```toml
[variables]
TARGET_DIR = "target"
BUILD_TYPE = "debug"
BINARY_NAME = "myapp"
SOURCE_PATTERN = "src/**/*.rs"
COMPILE_FLAGS = "--release --target x86_64-unknown-linux-gnu"
```

### Variable Usage

Use variables anywhere in your configuration with `${VAR_NAME}` or `$VAR_NAME` syntax:

```toml
[task.build]
command = "cargo build ${COMPILE_FLAGS}"
inputs = ["${SOURCE_PATTERN}", "Cargo.toml"]
outputs = ["${TARGET_DIR}/${BUILD_TYPE}/${BINARY_NAME}"]
```

### Built-in Variables

Compi provides several built-in variables:

- `$PWD`: Current working directory
- `$ENV_*`: All environment variables prefixed with `ENV_` (e.g., `$ENV_HOME`, `$ENV_USER`)

### Environment Variables

Access environment variables using the `ENV_` prefix:

```toml
[variables]
HOME_DIR = "${ENV_HOME}"
USER_NAME = "${ENV_USER}"

[task.deploy]
command = "scp app ${ENV_USER}@server:/home/${ENV_USER}/bin/"
```

## Task Fields

### Required Fields

- `command`: Shell command to execute

### Optional Fields

- `id`: Override the task name (defaults to `[task.name]`)
- `dependencies`: Array of task names that must run first
- `inputs`: Array of input files/patterns (supports globs)
- `outputs`: Array of output files this task produces
- `auto_remove`: Automatically remove outputs after successful execution (default: `false`)

## Build Logic

Compi uses a 4-tier system to determine if a task should run:

1. **No inputs**: Always run (e.g., cleanup tasks)
2. **Missing outputs**: Must run if any output file doesn't exist
3. **Outdated outputs**: Run if any input is newer than any output
4. **Content changed**: Run if file content hash changed since last run

## Glob Patterns

Input files support standard glob patterns:

- `*.rs`: All Rust files in current directory
- `src/**/*.rs`: All Rust files in src and subdirectories
- `test/*.{rs,toml}`: Rust and TOML files in test directory

## Dependency Management

- Tasks run in topological order based on dependencies
- Circular dependencies are detected and reported as errors
- Missing dependencies cause build failure

## Output Cleanup

Compi provides two ways to automatically clean up outputs after successful task execution:

### CLI Flag

Use the `--rm` flag to remove outputs after running a task:

```bash
# Remove outputs after building
compi --rm build

# Remove outputs with verbose logging
compi --rm -v test
```

### Auto-Remove Field

Set `auto_remove = true` in task configuration for automatic cleanup:

```toml
[task.temp_build]
command = "gcc -o temp_app *.c"
outputs = ["temp_app", "*.o"]
auto_remove = true  # Always clean up after successful execution

[task.generate_docs]
command = "doxygen"
outputs = ["docs/"]
# Only removed if --rm flag is used
```

### Behavior

- **Only on success**: Outputs are only removed if the task exits with code 0
- **Glob expansion**: Patterns like `*.o` are expanded to actual files before removal
- **Safe deletion**: Only the files/directories explicitly listed in outputs are deleted

## Cache System

- Stores file content hashes to detect changes
- Configurable location via `cache_dir` in config
- Cache location is relative to the config file

## Error Handling

- **Errors**: Stop execution (missing tasks, circular deps, command failures)
- **Warnings**: Continue execution (missing files, glob errors)
- **Info**: Verbose mode only (dependency relationships)

## Examples

### Simple Build Pipeline

```toml
[config]
default = "deploy"

[task.compile]
command = "gcc *.c -o app"
inputs = ["*.c", "*.h"]
outputs = ["app"]

[task.test]
command = "./app --test"
dependencies = ["compile"]
inputs = ["app"]

[task.deploy]
command = "scp app server:/"
dependencies = ["test"]
inputs = ["app"]
```

### Multi-Language Project

```toml
[config]
default = "all"

[task.js]
command = "npm run build"
inputs = ["src/**/*.js", "package.json"]
outputs = ["dist/app.js"]

[task.css]
command = "sass src/style.scss dist/style.css"
inputs = ["src/**/*.scss"]
outputs = ["dist/style.css"]

[task.all]
dependencies = ["js", "css"]
command = "echo 'Build complete'"
```

### Build with Cleanup

```toml
[config]
default = "package"

[task.compile]
command = "gcc *.c -o app"
inputs = ["*.c", "*.h"]
outputs = ["app", "*.o"]

[task.test]
command = "./app --test > test.log"
dependencies = ["compile"]
inputs = ["app"]
outputs = ["test.log"]
auto_remove = true  # Always clean up test logs

[task.package]
command = "tar -czf app.tar.gz app"
dependencies = ["test"]
inputs = ["app"]
outputs = ["app.tar.gz"]

# Usage:
# compi --rm compile  # Removes app and *.o files after compilation
# compi test          # Automatically removes test.log (auto_remove = true)
# compi package       # Keeps app.tar.gz (no cleanup)
```

## License

[MIT](./LICENSE)
