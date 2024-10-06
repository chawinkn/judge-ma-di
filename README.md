# Programming Judge System

Judge Ma Di (จัดมาดิ๊)

# Stack

- Rust
- Axum (Rust API Framework)
- RabbitMQ (Queue)
- IOI Isolate (Sandbox Environment)
- PostgreSQL (Database)

# [:link:Setup (with frontend)](https://gist.github.com/chawinkn/f1c7dae8bc4b0b8f489d0f775c715bcd)

- Docker (Containerization)

# Env

- `MAX_WORKER`: Maximum number of concurrent workers (Default = 1)

## With Docker

Setup the services environment or other settings in [`docker-compose.yml`](https://github.com/chawinkn/judge-ma-di/blob/master/docker-compose.yml)

You can change the isolate version (cgroup v1 or v2) in [`Dockerfile`](https://github.com/chawinkn/judge-ma-di/blob/master/Dockerfile#L17)

```bash
$ docker compose up -d
```

## Without Docker

### Setup env

```bash
$ cp .env.example .env
$ vim .env
```

### Install isolate and testlib

```bash
$ bash setup.sh
```

### Start RabbitMQ

enable only rabbitmq in `docker-compose.yml`

```bash
$ docker compose up -d
```

### Start

```bash
$ cargo run
```
