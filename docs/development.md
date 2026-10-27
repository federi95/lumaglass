# Lumaglass development

## Toolchain and commands

```sh
nix develop
pnpm install --frozen-lockfile
pnpm assets
pnpm dev
```

The flake supplies Rust 1.93, Cargo/rustfmt/Clippy, Node.js 24, pnpm 10.34.9, SDL2, pkg-config,
woff2, Python/fontTools, CMake and Git. Linux includes Mesa and Xvfb. Supported Nix hosts are
Apple Silicon macOS and ARM64/x86-64 Linux. macOS also has a project-local setup described
below. Other environments must supply these tools and Rust 1.88+, including
`armv7-unknown-linux-gnueabi` for TV builds. No browser installation is needed.

### Local macOS setup without Nix

Apple Silicon and Intel Macs can use the checked-in Pixi environment. Xcode or Apple's
Command Line Tools must already be available (`xcode-select -p`); the bootstrap does not
install Apple's system SDK. Only macOS's existing shell, curl, shasum and tar are needed
to start. Node, pnpm, Rust and Homebrew are not prerequisites.

```sh
sh tools/dev.sh setup
sh tools/dev.sh pnpm assets
sh tools/dev.sh pnpm dev
```

`setup` installs dependencies with `--frozen-lockfile`, Rust 1.93.0 with rustfmt/Clippy and
the `armv7-unknown-linux-gnueabi` standard library, and the exact pnpm from `package.json`.
It does not execute tests. `pnpm assets` generates fonts and artwork and compiles the
host SVG tool; it does not run tests or open a TV connection either.

Pixi 0.83.0 and Rustup 1.28.2 bootstraps are SHA-256 checked before execution. `pixi.lock`
pins Node 24, desktop SDL2, pkg-config, woff2, Python/fontTools and CMake for both Mac
architectures. The environment is stored in `.pixi/`; bootstrap binaries, Rust/pnpm
installations and download/package caches stay in `.build/`; pnpm dependencies stay
in `node_modules/`, and Cargo builds in `target/`. Shell profiles, Homebrew and global
toolchain installations are not modified. The wrapper changes environment variables
only for its child process, so invoke all local project commands through it:

```sh
sh tools/dev.sh pnpm sdk
sh tools/dev.sh pnpm build
sh tools/dev.sh pnpm run package
sh tools/dev.sh pnpm check
sh tools/dev.sh pnpm format:check
```

The wrapper sets `WEBOS_NDK` to `.build/toolchain/arm-webos-linux-gnueabi_sdk-buildroot`
unless explicitly overridden. The native SDK and SDL-webOS remain separate from the
desktop SDL2; host libraries are never used as ARM target libraries.

### VS Code and compatible editors

Open the repository root as a trusted workspace after running the local setup.
The checked-in `.vscode/settings.json` selects the installed Oxc package entry
points directly, bypassing pnpm's shell launchers. `oxc.useExecPath` runs them
with the editor's bundled Node runtime, so the extension does not need Node or
Oxc on the global `PATH`. Command-line scripts continue to use the pinned local
toolchain through `sh tools/dev.sh`.

Biome is disabled only in this workspace with `biome.enabled: false`; user
settings and other projects are untouched. Oxfmt handles the configured tooling
and documentation languages; Rust uses the rust-analyzer formatter. Repository
formatting/linting uses rustfmt/Clippy plus Oxfmt/Oxlint:

```sh
sh tools/dev.sh pnpm format
sh tools/dev.sh pnpm format:check
sh tools/dev.sh pnpm check
```

If the editor was open before dependencies were installed, use **Oxc: Restart
oxlint Server** and **Oxc: Restart oxfmt Server**, or **Developer: Reload Window**.
The extension re-reads the workspace paths; no global installation is required.
The paths and runtime option are documented by the
[Oxc extension](https://github.com/oxc-project/oxc-vscode#configuration).

### SDK archive integrity

`tools/webos/sdk-sources.json` pins each supported host archive URL and SHA-256.
`pnpm sdk` verifies the archive before extraction or executing its relocation
script, stages the installation and restores the previous SDK if replacement
fails. Relocation runs at the final path. A matching local installation marker,
compiler and sysroot header allow reuse; an older unmarked install is replaced.
These hashes were computed from the fixed upstream release archives, whose
GitHub asset metadata did not provide digests; they are repository integrity
pins, not publisher signatures. Recheck the archive bytes when updating them.

To update Node dependencies, use `sh tools/dev.sh pnpm update --latest`; update Rust
lockfile versions with `sh tools/dev.sh cargo update`. A new incompatible crate range
also requires updating its `Cargo.toml` and checking the affected code. Run `setup` again
after changing `packageManager`. Update the tarball hash in `flake.nix` together with the
pnpm pin. After updating `pixi.toml`, regenerate `pixi.lock` with
`sh tools/dev.sh pixi update`; ordinary wrapper commands
use `--locked` and never silently solve a different host environment.

The full local environment is disposable: removing `.build/`, `.pixi/`, `node_modules/`
and `target/` removes its installed tools, caches and build outputs. Re-run `setup` and
asset generation afterward. Moving the checkout can require recreating these directories
because native environments and the webOS SDK contain absolute paths.

### Command reference

| Command                   | Purpose                                                    |
| ------------------------- | ---------------------------------------------------------- |
| `pnpm dev [--options]`    | Run the desktop preview                                    |
| `pnpm assets [--4k]`      | Prepare fonts, vector artwork and raster backgrounds       |
| `pnpm screenshots`        | Capture native Home, Apps, General and Inputs into `docs/` |
| `pnpm assets:vector`      | Recompile bundled SVG icon/logo meshes                     |
| `pnpm assets:fonts`       | Restore checksum-pinned Noto font source if needed         |
| `pnpm sdk`                | Download/relocate a pinned host webOS SDK                  |
| `pnpm runtime`            | Prepare checksum-pinned SDL-webOS                          |
| `pnpm build`              | Release ARM cross-build using `WEBOS_NDK`                  |
| `pnpm run package`        | Assets, cross-build, IPK verification and Homebrew feed    |
| `pnpm deploy --device tv` | Build/install/launch through an ares device profile        |
| `pnpm check`              | Oxlint and workspace Clippy, without executing tests       |
| `pnpm format:check`       | Rust and tooling/document formatting checks                |
| `pnpm test`               | Release-tooling and Rust workspace unit tests              |
| `pnpm test:icons`         | Mocked runtime icon reads and bundled vector checks        |
| `pnpm test:smoke`         | Desktop rendering, mocked weather and localization checks  |
| `pnpm changeset`          | Add a release note and requested version bump to a PR      |
| `pnpm version:check`      | Check synchronized app release versions                    |

The TV target is ARMv7 **soft-float calling convention**, `gnueabi`, not `gnueabihf`. Codegen
uses the NDK's Cortex-A9/VFP baseline with NEON disabled for Rust code. Hardware floating point
is enabled without changing the calling convention; the pinned compiler emits unstable-feature
warnings for those target switches. `src/platform/webos/compat.rs` implements `getauxval` for the SDK's
older libc by reading real kernel auxiliary values, used by Rust startup and TLS CPU detection.

The NDK's SDL is a link-time stub with Wayland disabled. `tools/webos/webos-runtime.mjs` instead pins
SDL-webOS `release-2.30.12-webos.5`, standard SDL ABI, with archive/library SHA-256 verification.
It is linked and packaged with runtime name `libSDL2-2.0.so.0`, loaded from `$ORIGIN/lib`.
Desktop pkg-config paths are isolated from the ARM build. `WEBOS_NATIVE_LIBS` can point to
explicitly selected ARM webOS shared libraries; SDL overrides must have a real webOS/Wayland
backend. Avoid copying the whole SDK library directory or the TV's graphics drivers/libc.

## Manual extra-app preview

```sh
pnpm dev --test-extra-apps 120
```

On the TV, add `--test-extra-apps 120` after the JSON argument in the jailed command above.
This explicitly appends up to 300 labeled non-launchable cards to a successful real inventory,
using its apps as artwork/category templates. It uses `scroll-preview-preferences.json`,
initialized from normal choices once. Favoriting test cards exercises long Home strips without
changing normal favorites. Ctrl+C then a normal launch restores real inventory/preferences.
The placeholder cards are never a fallback for an inventory failure.

## Checks and CI

Rust behavior tests cover navigation, preferences, localization, rendering geometry and weather
state without a TV. `tests/` scripts exercise actual desktop framebuffers and mock HTTP responses
for navigation, persistence, pointer input, translations/CJK, weather limits/cache/races and
profiling. They contact no live weather APIs or television. On Linux:

```sh
pnpm format:check
pnpm check
pnpm test
pnpm assets
xvfb-run -a pnpm test:smoke
```

CI uses this native-only flow, then cross-links with a pinned Linux SDK, inspects/normalizes the
actual ARM IPK and generates checksums/feed. It uploads release/rendering artifacts on PRs,
main-branch pushes and manual runs. `pnpm check` also validates synchronized release versions,
and `pnpm test:release` covers version synchronization and the publish gate, including the real
Changesets version command in an isolated fixture. Host CI cannot prove hardware compatibility,
small-text quality or presentation frame rate.

### PR test IPKs

`.github/workflows/pr-test-ipk.yml` runs **PR Test IPK** whenever a PR is opened, reopened or
receives new commits, including fork PRs and drafts. It automatically builds the exact PR head
commit and uploads the verified ARM IPK and `sha256sums`. One bot comment is created or updated
with the full commit hash, workflow run and artifact download link. Failed or cancelled builds
update the same comment with their status; superseded commits cannot overwrite the latest head's
comment. Downloads are GitHub artifact ZIPs, require signing in and expire after 14 days.

The workflow uses only the `opened`, `synchronize` and `reopened` PR events, without a deployment
environment or required reviewers. Each new commit cancels the previous run. It becomes active
after it is merged into the PR's base branch; reopen an existing PR to request its first build,
or rerun a workflow to rebuild the same commit.

Only the build job checks out PR code. It has read-only repository access, no persisted
checkout credentials and no shared build caches. A separate job with comment-writing permission
uses the trusted base-branch workflow and GitHub's artifact API without checking out PR code.
The package keeps the normal `org.lumaglass.app` identity and version, so installing it replaces
the existing Lumaglass app. These builds are for manual TV verification; the ordinary CI checks still
run independently.

## Translation style

Write translated labels in sentence case: capitalize the first word, with normal
language-specific grammar. Preserve proper names and established acronyms, such
as Lumaglass, LG, HDMI and MSAA. German nouns keep their grammatical capitals.
Do not automatically lowercase runtime strings: installed app names, greeting
names, input labels and other user content must remain unchanged. A new sentence
may start with a capital; sentence case does not mean forcing the entire label
to lowercase after its first character.
