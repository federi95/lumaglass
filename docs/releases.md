# Releases

> The fork retains the 2.9.0 source version until the Changesets version PR creates its first release. Entries inherited from Luma are historical. Release artifacts use `org.lumaglass.app`, independently of Luma's package ID.

The first stable Lumaglass release targets **3.0.0** when the planned product
scope is ready. The pending major Changeset already expresses the new identity
and installation boundary. Keep the generated Version Packages PR unmerged
until release readiness; ordinary feature work does not bump source versions
manually. Any intermediate prerelease needs its own explicit release decision.

## Releases

Add a changeset to feature/fix PRs that should be included in the next app release:

```sh
pnpm changeset
```

Select `lumaglass`, choose patch/minor/major and describe the change. Commit the generated file
under `.changeset/` with the PR. Documentation-only and maintenance PRs that do not need a release
can omit it. Keep the existing app version in ordinary PRs.

`.github/workflows/changeset-status.yml` adds a Changesets status check and maintains one comment
per PR, including fork PRs. It reports the requested release types or notes that a PR without a
changeset will not request an app version bump. For code or behavior changes, add a changeset
using semantic versioning: **patch** for backward-compatible fixes, **minor** for backward-compatible
features, or **major** for breaking changes. The comment is informational, so documentation-only
and maintenance PRs can still merge without a release changeset. It updates when a PR is opened,
reopened, edited, marked ready or receives new commits, and skips the generated Version Packages
PR. The workflow becomes active once it is merged into the PR's base branch. It uses
`pull_request_target` with a read-only status job and a separate comment-writing job, without
installing dependencies or running code from the PR.

After changesets merge into `main`, `.github/workflows/release-pr.yml` creates or updates the
**Version Packages** PR on `changeset-release/main`. Changesets combines the pending notes using
the highest requested bump. The custom `pnpm version:release` command updates `package.json`
and `CHANGELOG.md`, then synchronizes `appinfo.json`, the root `Cargo.toml`, the `lumaglass` entry in
`Cargo.lock`, and the versioned IPK examples in `README.md`. Dependency pins and supporting Rust
crate versions are preserved. The package stays private and is never published to npm.

The generated PR receives the same CI build as ordinary PRs. Since PRs created with GitHub's
built-in token do not trigger `pull_request` workflows, the release workflow explicitly dispatches
`ci.yml` for the version branch. Enable **Settings → Actions → General → Allow GitHub Actions to
create and approve pull requests** in the repository. No additional token or npm secret is needed.

Review and merge the version PR when ready to release. Only a main-branch push associated with
that merged, same-repository PR and a numeric version increase can enter the publish job. It must
also pass the existing checks, desktop smoke tests and native packaging build. Ordinary merges,
unmerged PRs and manual CI runs do not publish. To retry a failed publication, rerun the original
CI run for the merged version PR's main-branch commit.

Publication retains the existing guards: an already-published version is skipped, a draft can be
resumed, and a tag pointing at another commit requires a new app version. `pnpm version:check`
checks synchronization without writing files. `pnpm version:sync` repairs metadata from the
current `package.json` version without calculating a bump. `pnpm version:release` is reserved for
the automated version PR.

Homebrew Channel's details page reads `packages[].shortDescription` (or `fullDescriptionUrl`),
not `manifest.appDescription`. The feed generator writes the root app metadata description into
`shortDescription` and `manifest.appDescription`. It also copies `assets/homebrew-description.html`
to `dist/description.html` and sets the package's `fullDescriptionUrl` to that version's release
asset. Homebrew Channel prefers this HTML description when the URL is present. CI uploads the
HTML alongside the feed and IPK, so the feed and its description must be published together.

Key modules: `state.rs` focus/preferences, `view.rs` geometry, `renderer.rs` GPU caches/draws,
`font.rs` lazy metrics/outlines, `vector.rs` GPU mesh upload, `platform.rs` worker/inventory,
`luna.rs` LS2, `weather*.rs` scheduling/HTTPS/cache, `storage.rs` atomic JSON, and `main.rs` SDL loop.
