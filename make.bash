#!/usr/bin/env bash
set -e

build_contract() {
    echo "build for $(uname -m)"
    docker run --rm -v "$(pwd)":/code \
        --mount type=volume,source="$(basename "$(pwd)")_cache",target=/code/target \
        --mount type=volume,source=registry_cache,target=/usr/local/cargo/registry \
        cosmwasm/rust-optimizer:0.12.6
}

usage() {
  cat <<EOUSAGE
Usage: $0 [subcommand]
Available subcommands:
  build           - run docker cargo workspace to build all contracts
EOUSAGE
}

main() {

    case "$1" in
    "-h" | "--help" | "help")
      usage
      exit 0
      ;;
    build) build_contract;;
    *)
      usage
      exit 1
    esac
}

main $@
