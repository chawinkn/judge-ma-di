# Scripts Reference

Overview of the shell and SQL scripts in [`scripts/`](../scripts/).

---

## 1. `scripts/setup.sh` (Host Environment Provisioning)

* **Purpose**: Bootstraps dependencies on bare-metal Ubuntu / Debian Linux to compile and run the judge without Docker.
* **Execution**: Run once on host setup: `sudo ./scripts/setup.sh`.
* **Actions**:
  1. Installs build tools and system headers (`make`, `libcap-dev`, `libsystemd-dev`, `libssl-dev`, `asciidoc-base`, `pkg-config`, `gcc`, `g++`, `rustc`, `cargo`, `python3`).
  2. Clones, builds, and installs [IOI Isolate](https://github.com/ioi/isolate) to `/usr/local/bin/isolate`.
  3. Invokes `scripts/checker.sh` to compile output checker binaries.

---

## 2. `scripts/checker.sh` (Checker Compilation)

* **Purpose**: Compiles standard competitive programming output checkers into `./checker/`.
* **Execution**: Executed automatically during `docker build` (`Dockerfile`) and host setup (`scripts/setup.sh`).
* **Actions**:
  1. Fetches [testlib.h](https://github.com/MikeMirzayanov/testlib) and standard checker source implementations from [programming-in-th/testlib](https://github.com/programming-in-th/testlib).
  2. Compiles each checker using `g++ -std=c++11 -O2` into `./checker/<name>` (e.g., `lcmp`, `wcmp`, `rcmp`).
  3. Cleans up cloned git repositories and intermediate files.
* **Runtime Usage**: Problems specify their checker via `"checker": "lcmp"` in `manifest.json`. The worker invokes `./checker/<name>` to validate user output against the `.sol` solution file.

---

## 3. `scripts/entrypoint.sh` (Cgroup v2 Subtree Delegation)

* **Purpose**: Configures Linux cgroup v2 subtree delegation inside the container before handing execution to the application.
* **Execution**: Configured as `ENTRYPOINT` in `Dockerfile`.
* **Technical Background**:
  * Isolate requires control groups (`isolate --cg`) for CPU and memory accounting.
  * In cgroups v2, a parent cgroup cannot enable controllers if processes exist directly in that cgroup (the "no internal processes" rule).
* **Actions**:
  1. Moves the shell process (`$$`) into an `/init` leaf child cgroup.
  2. Launches `isolate-cg-keeper` in the root container cgroup to keep controllers active.
  3. Executes the container command via `exec "$@"`.

---

## 4. `scripts/init.sql` (PostgreSQL Schema & Seed Data)

* **Purpose**: Defines relational schema, partial index for queue dispatch, and initial seed fixtures.
* **Execution**:
  * Executed automatically by PostgreSQL on initial container creation via `/docker-entrypoint-initdb.d/init.sql`.
  * Embedded in Rust integration tests via `include_str!("../../scripts/init.sql")` to bootstrap ephemeral test databases.
* **Key Components**:
  * **`task`**: Problem metadata (`id`, `title`, `full_score`, `private`).
  * **`submission`**: Submission records and evaluation results (`id`, `task_id`, `status`, `code`, `language`, `score`, `time`, `memory`, `result`).
  * **`idx_submission_queue`**: Partial index on `(submitted_at ASC, id ASC) WHERE status = 'In Queue'`, optimizing FIFO queue polling with `FOR UPDATE SKIP LOCKED`.
  * **Seed fixture**: Inserts default problem `'a_plus_b'` if not present.
