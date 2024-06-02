#!/bin/bash

red=$(tput setaf 1)
green=$(tput setaf 2)
blue=$(tput setaf 4)
normal=$(tput sgr0)

sudo apt-get update
sudo apt-get install libcap-dev libsystemd-dev libssl-dev asciidoc-base

git clone "https://github.com/ioi/isolate"
git clone "https://github.com/programming-in-th/testlib"

cd isolate
make
sudo make install

cd ..
mkdir -p ./checker

for file in ./testlib/*.cpp
do
  if [ -f "$file" ]; then
    filename_ex=${file##*/}
    filename=${filename_ex%.*}
    
    echo "${blue}Compiling ${filename_ex}${normal}"
    g++ -std=c++11 "${file}" -O2 -o "./checker/${filename}" -I "./testlib"
  fi
done

rm -rf isolate
rm -rf testlib

echo "${green}Setup successfully!${normal}"