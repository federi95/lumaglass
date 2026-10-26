# Getting started

On macOS, an existing Xcode or Command Line Tools installation supplies Apple's
compiler and SDK. From the repository root:

```sh
sh tools/dev.sh setup
sh tools/dev.sh pnpm assets
sh tools/dev.sh pnpm dev
```

The wrapper installs the pinned tools inside the checkout. Use it before pnpm,
Cargo or Pixi commands so global tools, caches and shell profiles stay unchanged.
Setup and asset generation do not execute tests. Preview uses desktop demo
inventory and never calls a TV; weather requests require explicit configuration.

For a TV package:

```sh
sh tools/dev.sh pnpm sdk
sh tools/dev.sh pnpm run package
```

The wrapper sets `WEBOS_NDK` to the SDK in `.build/toolchain/` unless overridden.
Packaging creates the ARM IPK, checksum, license notices and Homebrew feed in
`dist/`. See [TV integration](tv-integration.md) before installing or enabling
takeover, especially if Luma or Glasshouse Home is already enabled.

Linux users can use the existing `nix develop` environment and the commands in
[development.md](development.md). No browser is needed to run Lumaglass.
