FROM rust:1.69-buster as builder

WORKDIR /app

COPY . .

RUN cargo build --release

FROM ubuntu:22.04

RUN apt update -y
RUN apt install wget tar gzip git -y

# Install dependecies and initialize isolate sandbox
RUN apt install build-essential libssl-dev libcap-dev pkg-config libsystemd-dev python3 -y

# Isolate cgroup v2
# RUN wget -P /tmp https://github.com/ioi/isolate/archive/master.tar.gz && tar -xzvf /tmp/master.tar.gz -C / > /dev/null
# RUN make -C /isolate-master isolate && make -C /isolate-master install && rm -rf /tmp/master.tar.gz /isolate-master
# ENV PATH="/isolate-master:$PATH"

# Isolate cgroup v1
RUN wget -P /tmp https://github.com/ioi/isolate/archive/refs/tags/v1.10.1.tar.gz && tar -xzvf /tmp/v1.10.1.tar.gz -C / > /dev/null
RUN make -C /isolate-1.10.1 isolate && make -C /isolate-1.10.1 install && rm -rf /tmp/v1.10.1.tar.gz /isolate-1.10.1
ENV PATH="/isolate-1.10.1:$PATH"

# OpenSSL
# RUN wget http://nz2.archive.ubuntu.com/ubuntu/pool/main/o/openssl/libssl1.1_1.1.1f-1ubuntu2.22_amd64.deb
RUN wget http://launchpadlibrarian.net/715615335/libssl1.1_1.1.1f-1ubuntu2.22_amd64.deb
RUN dpkg -i libssl1.1_1.1.1f-1ubuntu2.22_amd64.deb

WORKDIR /user/local/bin

COPY --from=builder /app/target/release/judge-ma-di .

# COPY checker /user/local/bin/checker

# COPY tasks /user/local/bin/tasks

COPY config.json /user/local/bin/config.json

COPY checker.sh /user/local/bin/checker.sh

RUN ./checker.sh

EXPOSE 5000

ENV PORT=5000

ENV HOSTNAME="0.0.0.0"

CMD [ "./judge-ma-di" ]
