# Contributing

Keep each change focused and include any required updates to tests or
documentation. Before opening a pull request, run the same validation used by
continuous integration:

```sh
make ci
```

The pinned Rust toolchain and WASM target are defined in `rust-toolchain.toml`.
Dependency-update pull requests follow the same checks; review their lockfile
changes and wait for the required workflow to pass before merging.

## Linting and security checks

`make ci` runs the full validation suite, which now includes Clippy,
ShellCheck, and a lockfile-based vulnerability scan. You can also run each
check individually while iterating:

### Clippy

Clippy runs across the whole workspace with warnings treated as errors, so any
project warning fails the check. Dependency lint output is excluded so the
check focuses on repository code.

```sh
cargo clippy --workspace --all-targets -- -D warnings
```

### ShellCheck

ShellCheck covers the deployment and build scripts (`deploy.sh` and
`scripts/build.sh`) to catch unsafe quoting, portability, and control-flow
problems. Any exclusions are narrowly scoped and explained inline with a
`# shellcheck disable=...` comment next to the affected line.

```sh
shellcheck deploy.sh scripts/build.sh
```

### Dependency vulnerability scan

The vulnerability scan consumes `Cargo.lock` and fails CI on known advisories.
Any exceptions must be documented and time-bounded. Run it locally with:

```sh
cargo audit
```

If you add a temporary exception, document the advisory ID, the reason, and an
expiry date alongside the configuration so it can be reviewed and removed.

## Branch protection policy for `main`

The `main` branch is protected. The following policy applies to all changes:

- **Pull requests are required.** All changes to `main` must land through a
  pull request; direct pushes to `main` are not permitted.
- **Required status check.** The contract CI workflow must pass before a pull
  request can merge. The required check is the `contract-ci` workflow, which
  includes the `reproducible-build` job. The exact required status check name
  is `contract-ci / reproducible-build`; a pull request cannot be merged until
  this check reports success.
- **Force pushes and deletion are restricted.** Force pushes to `main` are
  blocked, and `main` cannot be deleted. Any exception (for example, an
  emergency revert) requires maintainer approval and must be recorded in the
  pull request or issue that tracks the change.
- **Up-to-date branches.** Pull requests must be up to date with `main` before
  merging so the required check runs against the code that will land.

Maintainers may tighten this policy (for example, by requiring additional
approvals) but must not loosen the required check or the pull-request
requirement without documenting the change here.

## Documentation review

Documentation review is part of pull-request expectations. Any change that
alters a public contract entrypoint, its authorization or input validation
requirements, its storage effects, or the events and errors it emits must
update the contract interface reference in the same pull request. Reviewers
should confirm the reference matches the source before approving.

- Contract interface and event reference: see
  [`docs/contract-interface.md`](docs/contract-interface.md).
- Mainnet deployment checklist: see
  [`docs/mainnet-deployment-checklist.md`](docs/mainnet-deployment-checklist.md).
  Release and change-review discussions for production deployments should
  reference this checklist.

Do not include seed phrases, private keys, funded-account files, or deployment
manifests in commits. Preserve source headers and upstream attribution in the
router and airdrop components. Contract interface or authorization changes need
explicit migration and security-model discussion in the pull request.
