---
name: refactor-core-architecture
description: Workflow command scaffold for refactor-core-architecture in judge-ma-di.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /refactor-core-architecture

Use this workflow when working on **refactor-core-architecture** in `judge-ma-di`.

## Goal

Restructure core modules or replace major infrastructure (e.g., queue system), updating related configs, documentation, and tests.

## Common Files

- `.env.example`
- `Cargo.toml`
- `Cargo.lock`
- `README.md`
- `src/queue.rs`
- `src/lib.rs`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Update environment and config files (.env.example, config.json)
- Modify Cargo.toml and Cargo.lock for dependencies
- Edit or add core source files (src/queue.rs, src/lib.rs, src/main.rs, src/routes/*, src/worker.rs, etc.)
- Update or add documentation (README.md)
- Update or add tests (tests/queue.rs, tests/submission.rs, tests/worker.rs)

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.