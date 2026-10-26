#!/bin/sh
# Project-local macOS toolchain. Does not alter Homebrew, shell profiles or ~/.cargo.
set -eu
LUMAGLASS_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/.." && pwd)
cd "$LUMAGLASS_ROOT"

if [ "$#" -eq 0 ]; then
  printf '%s\n' 'Usage: sh tools/dev.sh setup | pnpm <command> | cargo <command> | pixi <command> | <executable> [args...]'
  exit 0
fi

export PIXI_HOME="$LUMAGLASS_ROOT/.build/pixi-home"
export PIXI_CACHE_DIR="$LUMAGLASS_ROOT/.build/pixi-cache"
export PIXI_NO_CONFIG=1
export PIXI_NO_TELEMETRY=1
export DO_NOT_TRACK=1
export CARGO_HOME="$LUMAGLASS_ROOT/.build/cargo"
export CARGO_TARGET_DIR="$LUMAGLASS_ROOT/target"
export RUSTUP_HOME="$LUMAGLASS_ROOT/.build/rustup"
export PNPM_HOME="$LUMAGLASS_ROOT/.build/pnpm-home"
export npm_config_cache="$LUMAGLASS_ROOT/.build/npm-cache"
export npm_config_store_dir="$LUMAGLASS_ROOT/.build/pnpm-store"
export npm_config_manage_package_manager_versions=false
export WEBOS_NDK="${WEBOS_NDK:-$LUMAGLASS_ROOT/.build/toolchain/arm-webos-linux-gnueabi_sdk-buildroot}"

case "$(uname -s)-$(uname -m)" in
  Darwin-arm64)
    LUMAGLASS_HOST=aarch64-apple-darwin
    LUMAGLASS_PIXI_SHA=5346cc3465b634c1a901fbe51b2ecbf6a644420316e941ac0ee6032c5e48c191
    ;;
  Darwin-x86_64)
    LUMAGLASS_HOST=x86_64-apple-darwin
    LUMAGLASS_PIXI_SHA=b378db73ac2b2a35de52fe4ee8885aaea5812d0b232aab9154af4d006339f122
    ;;
  *) printf '%s\n' 'This local bootstrap supports macOS; use the documented Nix environment on Linux.' >&2; exit 1 ;;
esac
LUMAGLASS_PIXI="$LUMAGLASS_ROOT/.build/bootstrap/pixi"
mkdir -p "$LUMAGLASS_ROOT/.build/bootstrap"
if [ ! -f "$LUMAGLASS_PIXI" ]; then
  curl --fail --location --retry 3 --silent --show-error --max-time 300 \
    "https://github.com/prefix-dev/pixi/releases/download/v0.83.0/pixi-$LUMAGLASS_HOST" \
    --output "$LUMAGLASS_PIXI.download"
  printf '%s  %s\n' "$LUMAGLASS_PIXI_SHA" "$LUMAGLASS_PIXI.download" | shasum -a 256 -c -
  chmod +x "$LUMAGLASS_PIXI.download"
  mv "$LUMAGLASS_PIXI.download" "$LUMAGLASS_PIXI"
fi
printf '%s  %s\n' "$LUMAGLASS_PIXI_SHA" "$LUMAGLASS_PIXI" | shasum -a 256 -c - >/dev/null

if [ "$1" = pixi ]; then
  shift
  exec "$LUMAGLASS_PIXI" "$@"
fi
exec "$LUMAGLASS_PIXI" run --locked -- sh tools/dev/setup-local.sh "$@"
