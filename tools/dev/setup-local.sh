#!/bin/sh
# Called inside Pixi by dev.sh, with all writable tool homes scoped to .build/.
set -eu
LUMAGLASS_ROOT=$(CDPATH= cd -- "$(dirname -- "$0")/../.." && pwd)
cd "$LUMAGLASS_ROOT"
export PATH="$CARGO_HOME/bin:$LUMAGLASS_ROOT/.build/pnpm/node_modules/.bin:$PATH"
export RUSTUP_TOOLCHAIN=1.93.0

# Conda dylibs use @rpath; pkg-config supplies -L but not a runtime search path.
# Scope this to macOS so the webOS target's sysroot/CPU/rpath flags stay isolated.
# A Cargo config array also preserves checkouts containing spaces for desktop use.
node --input-type=module -e '
  import fs from "node:fs";
  import path from "node:path";
  const directory = process.env.CARGO_HOME;
  const flag = `link-arg=-Wl,-rpath,${path.join(process.env.CONDA_PREFIX, "lib")}`;
  const triple = process.arch === "arm64" ? "aarch64-apple-darwin" : "x86_64-apple-darwin";
  const config = `[target.${triple}]\nrustflags = ["-C", ${JSON.stringify(flag)}]\n`;
  fs.mkdirSync(directory, {recursive: true});
  const destination = path.join(directory, "config.toml");
  if (!fs.existsSync(destination) || fs.readFileSync(destination, "utf8") !== config)
    fs.writeFileSync(destination, config);
'

if [ "${1:-}" != setup ]; then
  if [ ! -x "$CARGO_HOME/bin/cargo" ] || [ ! -x "$LUMAGLASS_ROOT/.build/pnpm/node_modules/.bin/pnpm" ]; then
    printf '%s\n' 'Run sh tools/dev.sh setup first.' >&2
    exit 1
  fi
  exec "$@"
fi

if ! xcrun --find clang >/dev/null 2>&1; then
  printf '%s\n' 'Apple Xcode or Command Line Tools must already be installed and selected (xcode-select -p).' >&2
  exit 1
fi

if [ ! -x "$CARGO_HOME/bin/rustup" ]; then
  case "$(uname -m)" in
    arm64) LUMAGLASS_HOST=aarch64-apple-darwin; LUMAGLASS_RUSTUP_SHA=20ef5516c31b1ac2290084199ba77dbbcaa1406c45c1d978ca68558ef5964ef5 ;;
    x86_64) LUMAGLASS_HOST=x86_64-apple-darwin; LUMAGLASS_RUSTUP_SHA=9c331076f62b4d0edeae63d9d1c9442d5fe39b37b05025ec8d41c5ed35486496 ;;
    *) printf '%s\n' 'Unsupported macOS architecture.' >&2; exit 1 ;;
  esac
  LUMAGLASS_INSTALLER="$LUMAGLASS_ROOT/.build/bootstrap/rustup-init"
  curl --fail --location --retry 3 --silent --show-error --max-time 300 \
    "https://static.rust-lang.org/rustup/archive/1.28.2/$LUMAGLASS_HOST/rustup-init" \
    --output "$LUMAGLASS_INSTALLER.download"
  printf '%s  %s\n' "$LUMAGLASS_RUSTUP_SHA" "$LUMAGLASS_INSTALLER.download" | shasum -a 256 -c -
  chmod +x "$LUMAGLASS_INSTALLER.download"
  mv "$LUMAGLASS_INSTALLER.download" "$LUMAGLASS_INSTALLER"
  # Rust elsewhere on PATH is allowed: this install lives entirely under .build/.
  RUSTUP_INIT_SKIP_PATH_CHECK=yes "$LUMAGLASS_INSTALLER" -y --no-modify-path --profile minimal --default-toolchain none
fi
rustup set auto-self-update disable
rustup toolchain install "$RUSTUP_TOOLCHAIN" --profile minimal --component rustfmt,clippy \
  --target armv7-unknown-linux-gnueabi

LUMAGLASS_PNPM_VERSION=$(node -p "JSON.parse(require('fs').readFileSync('package.json')).packageManager.replace('pnpm@', '')")
if [ ! -x "$LUMAGLASS_ROOT/.build/pnpm/node_modules/.bin/pnpm" ] || \
  [ "$("$LUMAGLASS_ROOT/.build/pnpm/node_modules/.bin/pnpm" --version)" != "$LUMAGLASS_PNPM_VERSION" ]; then
  (
    unset npm_config_store_dir npm_config_manage_package_manager_versions
    npm install --prefix "$LUMAGLASS_ROOT/.build/pnpm" --ignore-scripts --no-audit --no-fund \
      --package-lock=false "pnpm@$LUMAGLASS_PNPM_VERSION"
  )
fi
pnpm install --frozen-lockfile
printf '%s\n' 'Local toolchain ready. Use sh tools/dev.sh pnpm assets, then sh tools/dev.sh pnpm dev.'
