# Currently Kaboom

# FROM ubuntu:latest

# RUN apt update
# RUN apt install apt
# RUN apk update && apk add python3 gcc g++
# RUN apk add --update --no-cache git make libcap-dev asciidoc

# RUN git clone https://github.com/ioi/isolate.git

# WORKDIR /usr/src/app/isolate

# RUN make && make install

# FROM rust:1.69

# WORKDIR /usr/src/app

# ARG POSTGRES_URL

# ENV POSTGRES_URL=$POSTGRES_URL

# COPY . .

# RUN cargo build --release
# RUN chmod +x setup.sh && ./setup.sh

# CMD [ "./target/release/judge-ma-di" ]