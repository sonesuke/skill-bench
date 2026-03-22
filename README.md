# skill-bench

TOML-based Claude skill test runner (nextest-style)

## Overview

skill-bench is a Rust-based test runner that executes test cases defined in TOML files. Designed for testing Claude Code skills.

Distributable as a standalone binary - users don't need a Rust environment.

## Features

- **TOML test definitions**: Write tests in TOML without writing Rust code
- **nextest-style interface**: Parallel execution, filtering, failure history management
- **Fast parallel execution**: Multi-threaded execution via rayon
- **Rich assertions**: 18 verification types (skills, MCP, tools, files, databases)
- **Embedded assets**: Harness plugin embedded in binary for standalone distribution

## Installation

```bash
# Build from source
cargo build --release
cargo install --path .

# Or download the binary and add to PATH
cp target/release/skill-bench ~/.local/bin/
```

## Usage

### List tests

```bash
skill-bench list
```

### Run tests

```bash
# Run all tests (uses default pattern: cases)
skill-bench run

# Run tests from specific directory (automatically finds all .toml files)
skill-bench run "cases"
skill-bench run "cases/concept-interviewing"

# Or use explicit glob pattern
skill-bench run "cases/**/*.toml"

# Filter by test name (regex)
skill-bench run --filter "functional-.*"

# Filter by skill name
skill-bench run --skill investigation-recording

# Rerun only failed tests
skill-bench run --rerun-failed

# Specify parallel threads
skill-bench run --threads 4

# Specify plugin directory (path to directory containing .claude-plugin/)
skill-bench run --plugin-dir ./harness-plugin

# Persist Claude session logs to directory
skill-bench run --log logs    # Save logs to logs/ directory
skill-bench run -l .          # Short form: save to current directory
```

### Test Pattern Syntax

The pattern argument accepts either a directory or glob pattern:

- `cases` - All TOML files recursively under `cases/` (directory mode)
- `cases/concept-interviewing` - All TOML files in a specific subdirectory
- `cases/**/*.toml` - Explicit glob pattern for all TOML files recursively
- `cases/*/*.toml` - TOML files only in immediate subdirectories
- `cases/functional-*.toml` - TOML files matching a specific pattern

When a directory is specified, all `.toml` files are automatically found recursively.

### List tests

```bash
# List all tests (uses default pattern: cases)
skill-bench list

# List tests from specific directory
skill-bench list "cases/concept-interviewing"
```

## Directory Structure

```
skill-bench/
├── Cargo.toml              # Rust project config
├── src/                   # Source code
│   ├── main.rs           # Entry point
│   ├── cli/              # CLI argument definitions
│   ├── models/           # Data models
│   ├── runtime/          # Test discovery & execution
│   ├── assertions/       # Assertion library
│   ├── output/           # Result reporting
│   ├── state/            # Failure history management
│   └── assets/           # Embedded assets
│       └── harness-plugin/
├── cases/                # TOML test cases
│   ├── claim-analyzing/
│   ├── concept-interviewing/
│   └── ...
└── target/               # Build output
```

## TOML Test Format

```toml
name = "test-name"
description = "Test description"
timeout = 120  # seconds

test_prompt = """
Test prompt here...
"""

[[setup]]
name = "setup_name"
type = "file"
path = "test.txt"
content = "File content"

[[checks]]
name = "check_name"
command = "skill-invoked"
skill = "skill-name"

[answers]
"question_key" = "answer_value"
```

## Assertion Reference

Assertions use structured TOML format:

### Skill Verification
- `skill-loaded` - Skill was loaded
- `skill-invoked` - Skill was invoked
- `skill-not-invoked` - Skill was NOT invoked

### MCP Verification
- `mcp-loaded` - MCP server was loaded
- `mcp-tool-invoked` - MCP tool was invoked
- `mcp-success` - MCP tool succeeded

### Tool Verification
- `tool-use` - Tool was used
- `param` - Parameter value verification

### File Verification
- `file-content` - Verify file content
- `file-contains` - File contains string
- `workspace-file` - File exists
- `workspace-dir` - Directory exists

### Log Verification
- `output-contains` - Output contains string
- `log-contains` - Log contains pattern
- `text-contains` - Text content search

### Database Verification
- `db-query` - SQL query result verification
  - Numeric comparisons: `">0"`, `">=5"`, `"=10"`, `"<3"`, `"<=2"`

### Negative Assertions

Use `deny = true` on any assertion for negative verification:

```toml
[[checks]]
name = "should-not-contain-error"
command = "file-contains"
file = "output.txt"
contains = "error"
deny = true
```

## Failure History

Failed test history is saved to `.skill-bench/test-history.json`. Use `--rerun-failed` to re-run only tests that failed in the previous run.

## License

MIT
