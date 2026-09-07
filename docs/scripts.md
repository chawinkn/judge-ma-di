# Scripts Reference

This document describes the utility scripts located in the [`scripts/`](../scripts/) directory.

---

## 1. `scripts/setup.sh` (Bare-Metal Setup)

* **Purpose**: Bootstraps host environment dependencies for running the judge outside Docker.
* **When to use**: Initial setup on bare-metal Ubuntu/Debian Linux systems.
* **Actions**:
  1. Installs build tools and kernel development libraries (`make`, `libcap-dev`, `libsystemd-dev`, `libssl-dev`, `asciidoc-base`, `pkg-config`, `gcc`, `g++`, `rustc`, `cargo`, `python3`).
  2. Clones, compiles, and installs [IOI Isolate](https://github.com/ioi/isolate) (`isolate`) to `/usr/local/bin/isolate`.
  3. Executes `scripts/checker.sh` to compile standard testlib checkers.

---

## 2. `scripts/checker.sh` (Checker Binaries Compilation)

* **Purpose**: Compiles standard competitive programming output checkers into `./checker/`.
* **When to use**: Automatically executed during Docker build (`Dockerfile`) and host setup (`scripts/setup.sh`).
* **Actions**:
  1. Fetches [testlib.h](https://github.com/MikeMirzayanov/testlib) and standard checker source implementations from [programming-in-th/testlib](https://github.com/programming-in-th/testlib).
  2. Compiles each checker using `g++ -std=c++11 -O2` into `./checker/<name>` (e.g. `lcmp`, `wcmp`, `rcmp`).
  3. Cleans up cloned git repositories and temporary files.
* **Usage in Judge**: Tasks reference these compiled binaries via `checker` in `tasks/<task_id>/manifest.json`.

---

## 3. `scripts/entrypoint.sh` (Docker Cgroup v2 Delegation)

* **Purpose**: Sets up Linux cgroup v2 subtree delegation inside the Docker container before starting the judge processes.
* **When to use**: Defined as the `ENTRYPOINT` for the runtime container in `Dockerfile`.
* **Why it's needed**:
  * Isolate requires control group delegation to manage memory limits and accounting (`isolate --cg`).
  * In cgroups v2, a parent cgroup cannot enable controllers if processes exist directly in that cgroup ("no internal process" rule).
* **Actions**:
  1. Moves the current process (`$$`) into a leaf subgroup (`/init`).
  2. Launches `isolate-cg-keeper` in the root container cgroup.
  3. Hands over execution to the container command via `exec "$@"`.

---

## 4. `scripts/init.sql` (Database Initialization)

* **Purpose**: Defines schema and seeds default data for PostgreSQL.
* **When to use**:
  * Automatically executed by Postgres on first startup via `/docker-entrypoint-initdb.d/init.sql` (mounted in `compose.yml`).
  * Embedded in integration tests via `include_str!("../../scripts/init.sql")` for automated schema provisioning.
* **Contents**:
  * **`task` table**: Problem definitions (`id`, `title`, `full_score`, `private`).
  * **`submission` table**: Code submissions (`id`, `task_id`, `status`, `code`, `language`, `score`, `time`, `memory`, `result`, etc.).
  * **`idx_submission_queue` index**: Partial index on `(submitted_at ASC, id ASC) WHERE status = 'In Queue'` optimizing atomic FIFO queue polling (`FOR UPDATE SKIP LOCKED`).
  * **Default seed fixture**: Inserts sample problem `'a_plus_b'` if not already present.
