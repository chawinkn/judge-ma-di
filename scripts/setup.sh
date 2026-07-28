#!/bin/bash

red=$(tput setaf 1)
green=$(tput setaf 2)
blue=$(tput setaf 4)
normal=$(tput sgr0)

sudo apt-get update
sudo apt-get install -y \
    make \
    libcap-dev \
    libsystemd-dev \
    libssl-dev \
    asciidoc-base \
    pkg-config \
    rustc \
    cargo \
    python3 \
    gcc \
    g++

git clone "https://github.com/ioi/isolate"

cd isolate
make
sudo make install

cd ..
rm -rf isolate

"$(dirname "$0")/checker.sh"

echo "${green}Setup successfully!${normal}"