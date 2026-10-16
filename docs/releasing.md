# Automated releases

External publication is initiated only by a request tag. Normal pushes and
pull requests never publish a crate, a GitHub Release, or a Docker image.

## Start a release

Before creating a request tag, run **Actions → Release preflight → Run
workflow** and enter the target version, for example `0.14.0`. It performs the
same version planning, test, manifest materialization, and a `cargo publish
--dry-run` against the first dependency-safe crate as a real release. It also
verifies the crates.io, Docker Hub, GitHub Release, and release-bot
credentials. It uses only a disposable runner: it does not push a commit or
tag, publish a crate, or publish an image. Dependent crates are not dry-run:
their new dependency versions do not exist on crates.io until the real release
has published the preceding crates.

Push a request tag that points to the current `master` tip:

```bash
git switch master
git pull --ff-only
git tag release-request/v0.14.0
git push origin release-request/v0.14.0
```

The version in the request tag is the desired xiu version. The tag must be
greater than the version in `confs/release/xiu.Cargo.toml`.

`release-request.yml` then:

1. compares the request commit with the preceding formal `v*` tag;
2. finds changed crates from `confs/release-packages.toml`, including every
   reverse dependency, and increments their patch versions;
3. updates the affected release manifests and their internal dependencies;
4. generates a versioned changelog entry for every affected crate, tests and
   dry-runs every publish;
5. publishes crates in dependency order;
6. creates a release commit on `master`, then creates the formal `vX.Y.Z`
   tag at that commit.

The formal tag triggers `release.yml`, which builds binaries, creates the
GitHub Release, and publishes Docker images. Request tags are retained as an
audit record and never trigger that artifact workflow.

Repository `Cargo.toml` files are local-development manifests and keep their
path dependencies. A normal clone needs only `make` (or `make build`) to build
the local source tree. The release workflow is the only place that temporarily
copies the release manifests over those files for `cargo publish`; those copies
are never committed.

## Required repository configuration

Create the protected GitHub Environment `crates-publish` and place these
secrets there:

- `CARGO_REGISTRY_TOKEN`: crates.io token with publish permission.
- `RELEASE_BOT_TOKEN`: a GitHub App installation token or fine-grained PAT
  limited to this repository, with **Contents: Read and write**. It creates
  the release commit and the formal tag.
- `ACCESS_TOKEN`: token with repository contents write permission, used by the
  GitHub Release upload action.
- `DOCKERHUB_USERNAME` and `DOCKERHUB_TOKEN`: Docker Hub credentials used to
  publish the multi-architecture image.

`RELEASE_BOT_TOKEN` is required because a tag pushed with the default
`GITHUB_TOKEN` does not trigger another GitHub Actions workflow. The bot token
lets the generated formal `v*` tag start `release.yml` normally.

Protect `master` and avoid merging other changes while a release is running.
The workflow verifies that `master` is unchanged before it pushes the release
commit; it stops rather than tagging a release built from stale code.

## Adding a publishable crate

Add one entry to `confs/release-packages.toml` with its crate name, source
directory, release manifest, and in-tree manifest. The release planner derives
the dependency order from the `[dependencies]` sections of the release
manifests; do not hand-maintain an order in a workflow.
