# CI duration

The completed [run on 1334be0](https://github.com/federi95/lumaglass/actions/runs/40508703247)
provides the baseline below. These figures are read from that existing run; no
workflow or test run was started to collect them.

| Stage                                | Observed duration |
| ------------------------------------ | ----------------- |
| Desktop rendering and mocked weather | 601 seconds       |
| Generate native assets               | 99 seconds        |
| Cross-build and verify ARM IPK/feed  | 55 seconds        |
| Desktop/asset system dependencies    | 28 seconds        |
| Unit tests                           | 17 seconds        |
| Set up webOS SDK                     | 19 seconds        |

The main cost is graphics verification. `pnpm test:smoke` runs the native UI,
brand artwork, weather, translations and display-watchdog suites sequentially.
The runner explicitly uses `LIBGL_ALWAYS_SOFTWARE=1` under Xvfb. Many scenarios
restart the native process with isolated preferences, render fixed frame budgets,
and capture real OpenGL output; some also render at 4K. Nine control-center
locales add more captures. This is useful coverage, but software rendering and
repeated startup make it substantially slower than the unit tests.

Assets also compile SVG/vector geometry, fonts and wallpaper derivatives. The ARM
build and package inspection are smaller contributors. Repeated pushes can cancel
an in-progress run and start the checks again, extending the time before one
complete green result is available.

## Implemented pipeline changes

The 28 October revision separates formatting/lint/unit tests, shared runtime
preparation, parallel graphics jobs and the ARM package job. The Home, brand artwork,
mock weather, translation and real display-watchdog suites all remain enabled.
A sixth catalog/inspection suite now also checks the new native app-management
view, disabled global actions and Back focus. Each graphics suite runs on its own runner and Xvfb instance with software GL;
`fail-fast: false` retains diagnostics from the other suites if one fails.

The prepare job builds the desktop binary once and shares a tar archive containing
that exact binary and generated assets within the same workflow run. Tar retains
executable permissions. Graphics jobs opt into the prebuilt binary explicitly;
ordinary standalone smoke commands still build it. The package job reuses those
assets and runs independently of graphics after preparation.

Generated assets use an exact cache key covering the runner OS/architecture,
Rust/Node versions, source assets, generator code, vector crates, Cargo manifests
and lockfile, pnpm lockfile and app manifest. Translation text is compiled into the binary and is not an
input of the asset generator, so changing it does not rebuild fonts/wallpapers.
There are no stale-prefix restores
for generated assets. Only trusted main pushes save asset and Rust caches. A hit
also skips installation of font-generation tools. The privileged PR Test IPK
workflow is unchanged and continues to avoid shared caches.

Each graphics job records `.build/ci-timing-SUITE.json`, adds its duration to the
job summary and uploads its own diagnostic artifact. The original aggregate
check name remains and fails if any required job fails, is cancelled or is
skipped. Release publication still requires that aggregate and the existing
merged-version-PR gate; version 3.0.0 is not published by this optimization.

The expected wall-clock benefit is replacing the sum of graphics-suite durations
with their longest duration, plus runner setup/artifact transfer. Cache hits also
avoid full asset regeneration. Runner minutes can increase because jobs have
separate setup steps. Actual gains must be read from completed runs; the old
601-second observation is not a benchmark of the new UI or pipeline.

Frame budgets and assertions have not been shortened for speed. Replacing fixed
waits with explicit readiness signals remains a possible later improvement and
must preserve animation and 4K coverage.

## First completed optimized run

[Run 40656800577 on 4951da4](https://github.com/federi95/lumaglass/actions/runs/40656800577)
completed all required jobs successfully. From first job start to the aggregate
check completion, elapsed time was 5 minutes 54 seconds. Preparation took 1 minute
48 seconds, Home graphics (including runner setup) 3 minutes 56 seconds, the
translation job 2 minutes 37 seconds and native packaging 1 minute 17 seconds.
Other graphics jobs ran alongside them. Asset reuse was available after the
previous preparation run. These are job wall times, not TV rendering performance;
different revisions and cache states prevent a controlled before/after speedup
claim. The old combined graphics step alone took 601 seconds.
