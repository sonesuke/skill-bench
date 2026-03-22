# Skill-Bench Repository

## Language Convention

This repository uses **English only** for all code, documentation, and communication.

- All code comments in English
- All documentation in English
- All commit messages in English
- All variable/function names in English

## Commit Convention

Use **Conventional Commits** format:

```
<type>(<scope>): <description>

[optional body]

[optional footer]
```

**Types:**
- `feat` - New feature
- `fix` - Bug fix
- `docs` - Documentation changes
- `style` - Code style changes (formatting, etc.)
- `refactor` - Code refactoring
- `test` - Adding or updating tests
- `chore` - Build process, tooling, dependencies

**Examples:**
- `feat(assertions): add database query assertion`
- `fix(cli): handle missing pattern argument`
- `docs: update installation instructions`
- `test: add unit tests for assertion checker`

## Project Overview

TOML-based test runner for Claude Code skills. Nextest-style interface with parallel execution, filtering, and failure history management.

## Architecture

- **Rust binary** with embedded harness plugin
- **TOML test definitions** in `cases/` directory
- **Parallel execution** via rayon thread pool
- **18 assertion types** for skills, MCP, tools, files, and databases

## Key Files

- `src/main.rs` - Entry point
- `src/runtime/executor.rs` - Parallel test execution
- `src/assertions/` - Assertion library
- `cases/` - TOML test cases
