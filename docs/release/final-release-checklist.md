# Final Release Checklist

Use this checklist for the actual public release.

This file is the final operator checklist. It does not replace:

- `docs/release/release-quickstart.md`
- `docs/release/release-guide.md`
- `.github/workflows/release-dryrun.yml`
- `.github/workflows/release-publish.yml`
- `scripts/release.bat`
- `scripts/release.sh`

## Release Gate

The workspace must be published in this exact order:

1. `canlink-hal`
2. `canlink-tscan-sys`
3. `canlink-mock`
4. `canlink-tscan`
5. `canlink-cli`

Mandatory rules:

- publish one crate at a time
- wait until the just-published version is indexed on crates.io
- only then continue to the next crate
- do not queue all `cargo publish` commands at once

## Operator Inputs

Fill these in before starting:

- Target version: `____________`
- Release date: `____________`
- Release operator: `____________`
- Repository visibility at publish time: `private / public`
- Publish path: `GitHub Actions / local script / manual`

## Repository Preconditions

- [ ] Working tree is clean on `main`
- [ ] `origin/main` is up to date locally
- [ ] `repository` metadata points to `https://github.com/iamsevens/canlink-rs`
- [ ] `README.md` and crate README files reflect the current public scope
- [ ] No private names, emails, internal IPs, local paths, or hardware identifiers remain in public files

## Release Metadata

- [ ] Workspace version in `Cargo.toml` is updated to the target version
- [ ] `CHANGELOG.md` contains the target version entry
- [ ] Release notes are ready
- [ ] The publish order is confirmed again: `canlink-hal -> canlink-tscan-sys -> canlink-mock -> canlink-tscan -> canlink-cli`

## Token And GitHub Setup

- [ ] crates.io account is ready
- [ ] Local `cargo login` works if using local publish
- [ ] GitHub secret `CARGO_REGISTRY_TOKEN` is configured if using GitHub Actions
- [ ] GitHub environment `crates-io` exists if using GitHub Actions approval gates
- [ ] The operator knows whether the repository should be made public before or after the crates.io publish

## Verification Before Publish

- [ ] `cargo test --all-features --workspace` passes
- [ ] `scripts/check.bat` or `./scripts/check.sh` passes
- [ ] `cargo doc --no-deps --all-features --workspace` passes
- [ ] Release dry-run passes for all 5 crates
- [ ] Dry-run verification uses the required `patch.crates-io.*.path=...` overrides for unpublished internal crates

## Publish Execution

- [ ] Commit the release preparation changes on `main`
- [ ] Create annotated tag `v<version>`
- [ ] Push `main`
- [ ] Push tag `v<version>`
- [ ] Start publish using exactly one path: GitHub Actions `Release Publish`, `scripts/release.bat`, `scripts/release.sh`, or fully manual

Per-crate execution:

- [ ] Publish `canlink-hal`
- [ ] Wait until `canlink-hal = "<version>"` is indexed
- [ ] Publish `canlink-tscan-sys`
- [ ] Wait until `canlink-tscan-sys = "<version>"` is indexed
- [ ] Publish `canlink-mock`
- [ ] Wait until `canlink-mock = "<version>"` is indexed
- [ ] Publish `canlink-tscan`
- [ ] Wait until `canlink-tscan = "<version>"` is indexed
- [ ] Publish `canlink-cli`
- [ ] Wait until `canlink-cli = "<version>"` is indexed

## Post-Publish Verification

- [ ] crates.io page opens for `canlink-hal`
- [ ] crates.io page opens for `canlink-tscan-sys`
- [ ] crates.io page opens for `canlink-mock`
- [ ] crates.io page opens for `canlink-tscan`
- [ ] crates.io page opens for `canlink-cli`
- [ ] `cargo install canlink-cli` succeeds in a clean environment
- [ ] `canlink --version` reports the expected version
- [ ] docs.rs pages start building as expected

## Optional Public Release Tasks

- [ ] Repository visibility is switched to `public` at the chosen time
- [ ] GitHub release entry is created if needed
- [ ] Release announcement is posted if needed

## Stop Conditions

Stop and fix the issue before continuing if any of the following happens:

- a dry-run fails
- a real publish fails
- crates.io indexing does not advance for the current crate
- the next crate would be published before the previous one is indexed
- metadata, README, or changelog no longer matches the release contents

## Sign-Off

- Final result: `ready / published / blocked`
- Notes: `____________________________________________________________`
