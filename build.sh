#!/bin/sh

# TODO explain me!

DEBUG_=x86_64-unknown-linux-gnu/debug
RELEASE_SUBDIR=x86_64-unknown-linux-gnu/release

SRC_ROOT=/tmp/daiquiri
DEST_ROOT=target

if cargo build --target-dir $SRC_ROOT
then

  SRC_BIN="${SRC_ROOT}/${DEBUG_SUBDIR}/daiquiri"
  if test -f "${SRC_BIN}"
  then
    echo $?
    echo $SRC_BIN
    mkdir -p "${DEST_ROOT}/${DEBUG_SUBDIR}"
    cp "${SRC_BIN}" "${DEST_ROOT}/${DEBUG_SUBDIR}/daiquiri"
  fi

  SRC_BIN="${SRC_ROOT}/${RELEASE_SUBDIR}/daiquiri"
  if test -f "${SRC_BIN}"
  then
    mkdir -p "${DEST_ROOT}/${RELEASE_SUBDIR}"
    cp "${SRC_BIN}" "${DEST_ROOT}/${RELEASE_SUBDIR}/daiquiri"
  fi

fi
