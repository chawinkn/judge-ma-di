---
name: script-file-reorganization
description: Workflow command scaffold for script-file-reorganization in judge-ma-di.
allowed_tools: ["Bash", "Read", "Write", "Grep", "Glob"]
---

# /script-file-reorganization

Use this workflow when working on **script-file-reorganization** in `judge-ma-di`.

## Goal

Move, rename, or reorganize script files and update references in documentation.

## Common Files

- `Dockerfile`
- `README.md`
- `scripts/checker.sh`
- `scripts/entrypoint.sh`
- `scripts/setup.sh`

## Suggested Sequence

1. Understand the current state and failure mode before editing.
2. Make the smallest coherent change that satisfies the workflow goal.
3. Run the most relevant verification for touched files.
4. Summarize what changed and what still needs review.

## Typical Commit Signals

- Move or rename script files (checker.sh, entrypoint.sh, setup.sh)
- Update Dockerfile and README.md to reflect new script locations

## Notes

- Treat this as a scaffold, not a hard-coded script.
- Update the command if the workflow evolves materially.