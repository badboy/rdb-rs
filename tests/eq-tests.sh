#!/bin/bash

SCRIPTPATH=$(cd "$(dirname "$0")"; pwd)
SCRIPT="$SCRIPTPATH/$(basename "$0")"


DUMP_DIRECTORY="${SCRIPTPATH}/dumps"

BUILD=1
if [ "$1" = "--no-build" ]; then
  BUILD=0
  shift
fi
ARG=$1

if [ ! -d "$DUMP_DIRECTORY" ]; then
  printf "\n\e[1;31;49m!!! ERROR\e[0m Missing dump directory. Aborting.\n"
  exit 2
fi

if [ $BUILD -eq 1 ]; then
  cargo build $ARG
fi

if [ "$ARG" = "--release" ]; then
  BIN=./target/release/rdb
else
  BIN=./target/debug/rdb
fi

failure=0
for dump in $(find "$DUMP_DIRECTORY" -type f -name "*.rdb"); do
  file=$(basename $dump)
  if [ "$file" = "regular_sorted_set.rdb" ]; then
    echo "  [skipping $file]"
    continue
  fi

  echo "  with $file"

  json=$(basename $dump) 
  json=${json/.rdb/.json}
  diffout=$(diff -u \
    <($BIN --format json $dump 2>/dev/null | json) \
    <(json <$DUMP_DIRECTORY/json/$json 2>&1))

  if [ $? -ne 0 ]; then
    echo "Failure with '$file'"
    echo
    echo "---------"
    echo "$diffout"
    echo "---------"
    failure=1
  fi
done


if [ $failure = 0 ]; then
  printf "\n\e[1;37;49m\\o/\e[0m \e[1;32;49mAll tests passed without errors!\e[0m\n"
else
  printf "\n\e[1;31;49m!!! WARNING\e[0m Some tests failed.\n"
  exit 1
fi
