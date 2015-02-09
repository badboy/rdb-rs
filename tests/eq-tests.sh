#!/bin/bash

SCRIPTPATH=$(cd "$(dirname "$0")"; pwd)
SCRIPT="$SCRIPTPATH/$(basename "$0")"


DUMP_DIRECTORY="${SCRIPTPATH}/dumps"

ARG=$1

if [ ! -d "$DUMP_DIRECTORY" ]; then
  printf "\n\e[1;31;49m!!! ERROR\e[0m Missing dump directory. Aborting.\n"
  exit 2
fi

cargo build $ARG

if [ "$ARG" = "--release" ]; then
  BIN=./target/release/rdb
else
  BIN=./target/rdb
fi

failure=0
for dump in $(find "$DUMP_DIRECTORY" -type f -name "*.rdb"); do
  file=$(basename $dump)
  echo "  with $file"

  json=$(basename $dump) 
  json=${json/.rdb/.json}
  diff -q  \
    <($BIN --format json $dump 2>/dev/null) \
    $DUMP_DIRECTORY/json/$json >/dev/null 2>&1

  if [ $? -ne 0 ]; then
    echo "Failure with '$file'"
    failure=1
  fi
done


if [ $failure = 0 ]; then
  printf "\n\e[1;37;49m\\o/\e[0m \e[1;32;49mAll tests passed without errors!\e[0m\n"
else
  printf "\n\e[1;31;49m!!! WARNING\e[0m Some tests failed.\n"
  exit 1
fi
