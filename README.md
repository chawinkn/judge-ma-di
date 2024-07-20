# Programming Judge System

Judge Ma Di (จัดมาดิ๊)

## With Docker

setup the services environment or other settings in `docker-compose.yml`

you can change the isolate version (cgroup v1 or v2) in `Dockerfile`

```bash
$ docker compose up -d
```

## Without Docker

### Setup env

```bash
$ cp .env.example .env
$ vim .env
```

### Install isolate and teslib

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

## PostgreSQL (Optional)

you can edit the db services in `docker-compose.yml`

## Ngrok (Optional)

you can edit the ngrok command in `docker-compose.yml`
