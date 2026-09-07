FROM rust:1-bookworm AS builder

WORKDIR /app

COPY Cargo.toml Cargo.lock ./
COPY src ./src

# Cache mount not in image layer -> copy binary out in same RUN
RUN --mount=type=cache,target=/usr/local/cargo/registry \
    --mount=type=cache,target=/app/target \
    cargo build --release \
    && cp target/release/judge-ma-di /app/judge-ma-di

# Must stay root: isolate is setuid and entrypoint delegates cgroup v2
FROM debian:bookworm-slim AS runtime

RUN apt-get update \
    && apt-get install -y --no-install-recommends \
        build-essential \
        ca-certificates \
        git \
        libcap-dev \
        libseccomp-dev \
        libssl-dev \
        libsystemd-dev \
        pkg-config \
        python3 \
        wget \
    && rm -rf /var/lib/apt/lists/*

# master branch required: cgroup v2 not in released tags
RUN wget -qO- https://github.com/ioi/isolate/archive/master.tar.gz | tar -xz -C /tmp \
    && make -C /tmp/isolate-master isolate \
    && make -C /tmp/isolate-master install \
    && rm -rf /tmp/isolate-master

# isolate --cg requires subuid/subgid range
RUN useradd -r isolate \
    && echo "isolate:200000:65536" >> /etc/subuid \
    && echo "isolate:200000:65536" >> /etc/subgid

WORKDIR /user/local/bin

COPY --chmod=755 scripts/checker.sh ./checker.sh
RUN ./checker.sh

COPY config.json ./config.json
COPY --chmod=755 scripts/entrypoint.sh ./entrypoint.sh

COPY --from=builder /app/judge-ma-di ./judge-ma-di

EXPOSE 5000

ENTRYPOINT ["./entrypoint.sh"]
CMD ["./judge-ma-di"]
