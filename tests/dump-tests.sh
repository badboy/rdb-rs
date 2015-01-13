#!/bin/bash

formats="json plain nil protocol"

ARG=$1

cargo build $ARG

if [ "$ARG" = "--release" ]; then
  BIN=./target/release/rdb
else
  BIN=./target/rdb
fi

failure=0
for f in $formats; do
  echo "Running $f tests..."
  for dump in $(find dumps -type f -name "*.rdb"); do
    echo "  with $dump"
    if [ "$f" = "json" ]; then
      $BIN --format $f $dump | json >/dev/null
    else
      $BIN --format $f $dump >/dev/null
    fi

    if [ $? -ne 0 ]; then
      echo "Failure with '$dump' (Format: $f)"
      failure=1
    fi
  done
done


if [ $failure = 0 ]; then
  printf "\n\e[1;37;49m\\o/\e[0m \e[1;32;49mAll tests passed without errors!\e[0m\n"
else
  printf "\n\e[1;31;49m!!! WARNING\e[0m Some tests failed.\n"
fi
