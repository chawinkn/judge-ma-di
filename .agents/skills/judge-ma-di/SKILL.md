```markdown
# judge-ma-di Development Patterns

> Auto-generated skill from repository analysis

## Overview

This skill teaches the core development patterns and workflows for the `judge-ma-di` Rust codebase. It covers coding conventions, architectural refactoring, script organization, and testing practices. By following these guidelines, contributors can maintain consistency, reliability, and clarity throughout the project.

## Coding Conventions

### File Naming

- Use **camelCase** for file names.
  - Example: `queueManager.rs`, `submissionHandler.rs`

### Import Style

- Use **relative imports** within modules.
  - Example:
    ```rust
    mod queue;
    use crate::queue::QueueManager;
    ```

### Export Style

- Use **named exports** for modules and functions.
  - Example:
    ```rust
    pub mod queue;
    pub fn process_submission() { ... }
    ```

### Commit Messages

- Follow **conventional commit** patterns.
- Allowed prefixes: `chore`, `refactor`, `test`, `feat`
- Keep commit messages concise (average ~36 characters).
  - Example:
    ```
    feat: add async support to queue manager
    refactor: modularize submission routes
    ```

## Workflows

### Refactor Core Architecture

**Trigger:** When you need to restructure core modules or replace major infrastructure (e.g., switch queue system, modularize routes).

**Command:** `/refactor-core`

1. **Update environment and config files**  
   Edit `.env.example` and `config.json` to reflect new settings or variables.
2. **Modify dependencies**  
   Update `Cargo.toml` and `Cargo.lock` to add, remove, or update dependencies.
3. **Edit or add core source files**  
   Refactor or create files such as:
   - `src/queue.rs`
   - `src/lib.rs`
   - `src/main.rs`
   - `src/routes/mod.rs`
   - `src/routes/submission.rs`
   - `src/worker.rs`
4. **Update documentation**  
   Revise `README.md` to document architectural changes and usage.
5. **Update or add tests**  
   Ensure tests in `tests/queue.rs`, `tests/submission.rs`, and `tests/worker.rs` are updated or added to cover new/changed functionality.

**Example:**
```rust
// src/queue.rs
pub struct QueueManager { /* ... */ }

impl QueueManager {
    pub fn new() -> Self { /* ... */ }
    pub fn enqueue(&self, item: Submission) { /* ... */ }
}
```

### Script File Reorganization

**Trigger:** When you want to move, rename, or reorganize script files for better structure or clarity.

**Command:** `/move-scripts`

1. **Move or rename script files**  
   Organize scripts such as:
   - `scripts/checker.sh`
   - `scripts/entrypoint.sh`
   - `scripts/setup.sh`
2. **Update Dockerfile and documentation**  
   Revise `Dockerfile` and `README.md` to reflect new script paths or names.

**Example:**
```dockerfile
# Dockerfile
COPY scripts/entrypoint.sh /usr/local/bin/entrypoint.sh
ENTRYPOINT ["/usr/local/bin/entrypoint.sh"]
```

## Testing Patterns

- Test files are written in Rust and placed in the `tests/` directory.
- File naming pattern: `*.rs` (e.g., `tests/queue.rs`, `tests/submission.rs`)
- Each test file targets a specific module or feature.
- Use Rust's built-in testing framework:
  ```rust
  #[cfg(test)]
  mod tests {
      use super::*;

      #[test]
      fn test_enqueue() {
          // test logic here
      }
  }
  ```

## Commands

| Command         | Purpose                                                      |
|-----------------|-------------------------------------------------------------|
| /refactor-core  | Refactor core architecture or replace major infrastructure  |
| /move-scripts   | Move, rename, or reorganize script files                    |
```