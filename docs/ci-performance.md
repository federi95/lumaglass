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

Potential follow-up changes, requiring a separately authorized CI run:

- Run independent graphics suites in separate jobs with a shared generated-asset
  artifact, keeping the ARM build independent where possible.
- Reuse native build/asset artifacts within the same commit and cache asset outputs
  with keys covering their sources, compiler, fonts and locked dependencies.
- Replace conservative frame budgets with explicit settled/ready conditions where
  animations are not the subject of a scenario. Keep separate motion/4K coverage.
- Retain per-suite timings so duration changes can be attributed to a suite rather
  than inferred from the combined step.

These are proposed improvements. This revision leaves the workflow triggers and
test coverage unchanged and does not push commits or dispatch Actions workflows.
