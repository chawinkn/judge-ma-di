# Programming Judge System

Judge Ma Di (จัดมาดิ๊)

# Stack

- Rust
- Axum (Rust API Framework)
- IOI Isolate (Sandbox Environment)
- PostgreSQL (Database)

# Architecture

```mermaid
flowchart LR
    Client["Client / Frontend"]
    DB[("PostgreSQL\n(Submissions Queue)")]

    subgraph Judge["Judge Ma Di"]
        API["HTTP API\n(Task Management)"]
        Storage[("Task Storage\n(Manifest & Testcases)")]
        Worker["Judge Worker\n(Evaluation & Checker)"]
        Sandbox["IOI Isolate\n(Sandbox)"]
    end

    %% Problem Setup
    Client -->|"Upload task"| API
    API -->|"Extract testcases & manifest"| Storage

    %% Submission Lifecycle
    Client -->|"Submit submission\n(status: 'In Queue')"| DB
    DB -->|"Poll submission\n(status -> 'Judging')"| Worker
    Storage -->|"Load testcases & limits"| Worker
    Worker <-->|"Execution\n"| Sandbox
    Worker -->|"Update verdict & score\n(status -> 'Completed')"| DB
```

# [:link:Setup (with frontend)](https://gist.github.com/chawinkn/f1c7dae8bc4b0b8f489d0f775c715bcd)

- Docker (Containerization)

## With Docker

Setup the services environment or other settings in `compose.yml`

```bash
$ docker compose -f compose.yml up -d
```

## Without Docker

### Setup env

```bash
$ cp .env.example .env
$ vim .env
```

### Install isolate and testlib

```bash
$ bash scripts/setup.sh
```

### Start

```bash
$ cargo run
```

### Git hooks

Runs `cargo fmt` + `cargo clippy` on commit. One-time setup per clone:

```bash
$ git config core.hooksPath .githooks
```

### Testing

```bash
$ cargo test                        # unit tests
$ cargo test --features integration # integration tests
```
