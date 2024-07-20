#!/bin/bash

git clone "https://github.com/programming-in-th/testlib"

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

rm -rf testlib