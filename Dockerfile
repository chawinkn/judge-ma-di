FROM rust:1-bookworm as builder

WORKDIR /app

COPY . .

RUN cargo build --release

FROM debian:bookworm-slim

RUN apt update -y
RUN apt install wget tar gzip git -y

# Install dependecies and initialize isolate sandbox
RUN apt install build-essential libssl-dev libcap-dev libseccomp-dev pkg-config libsystemd-dev python3 -y

# Isolate cgroup v2 (required for memory accounting/limiting)
RUN wget -P /tmp https://github.com/ioi/isolate/archive/master.tar.gz && tar -xzvf /tmp/master.tar.gz -C / > /dev/null
RUN make -C /isolate-master isolate && make -C /isolate-master install && rm -rf /tmp/master.tar.gz /isolate-master

# isolate --cg needs a subuid/subgid range for the "isolate" user to set up its namespaces
RUN useradd -r isolate \
    && echo "isolate:200000:65536" >> /etc/subuid \
    && echo "isolate:200000:65536" >> /etc/subgid

WORKDIR /user/local/bin

COPY --from=builder /app/target/release/judge-ma-di .

COPY config.json /user/local/bin/config.json

COPY scripts/checker.sh /user/local/bin/checker.sh

RUN ./checker.sh

COPY scripts/entrypoint.sh /user/local/bin/entrypoint.sh

RUN chmod +x /user/local/bin/entrypoint.sh

EXPOSE 5000

ENTRYPOINT [ "./entrypoint.sh" ]

CMD [ "./judge-ma-di" ]
