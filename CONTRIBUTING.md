# Contributing to the Unipept API

## Setup
The devcontainer in `.devcontainer/` installs Rust and a SwissProt index. The README section
[Developing the Unipept API](README.md#developing-the-unipept-api) says how to start a server.
The tests do not need the index or OpenSearch.

## Toolchains
- **Build, test and lint on stable.** CI sets `RUSTUP_TOOLCHAIN=stable` for every job except
  formatting. Do the same locally, or rustup uses the nightly from `rust-toolchain.toml`.
- **Format on the pinned nightly.** `.rustfmt.toml` sets `unstable_features = true`. Stable
  rustfmt skips those settings and formats without them, which changes files you did not edit.
  Run `cargo fmt` without `RUSTUP_TOOLCHAIN`, so rustup uses the pin.
- **Change the nightly only in `rust-toolchain.toml`.** Do not set a toolchain in `ci.yml`. After
  a change to the pin, run `cargo fmt --all` and commit both changes together.
- **The MSRV is 1.88.** It is set in the root `Cargo.toml` and in the `msrv` job in `ci.yml`.
  Change both together.

## Checks
A PR to `main` must pass the eight jobs in `.github/workflows/ci.yml`. These commands run the
same checks locally:

| CI job | Command |
| --- | --- |
| Check + test | `RUSTUP_TOOLCHAIN=stable cargo check --workspace --all-targets --locked`<br>`RUSTUP_TOOLCHAIN=stable cargo test --workspace --locked` |
| Release build | `RUSTUP_TOOLCHAIN=stable cargo build --release --locked` |
| MSRV (1.88) | `RUSTUP_TOOLCHAIN=1.88.0 cargo check --workspace --all-targets --all-features --locked` |
| Storage backend aliases | See [Storage backends](#storage-backends) |
| Run Clippy | `RUSTUP_TOOLCHAIN=stable cargo clippy --workspace --all-targets --all-features -- -D warnings` |
| Check formatting | `cargo fmt --all --check` |
| Check the deploy scripts | `shellcheck -x .deploy/lib.sh .deploy/rollout.sh .deploy/loadbalancer/*.sh .deploy/server/*.sh .deploy/tests/*.sh .deploy/tests/*/*.sh` |
| Check documentation | `RUSTUP_TOOLCHAIN=stable RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --document-private-items` |

The MSRV check needs the toolchain installed first: `rustup toolchain install 1.88.0`.

A change to `.deploy/` also has container tests. CI runs them on PRs that change `.deploy/`. They
are not a required check. See [`.deploy/tests/README.md`](.deploy/tests/README.md).

## Storage backends
The storage features select a type for each index structure when the crate compiles. The README
section [Choosing a storage backend](README.md#choosing-a-storage-backend) describes them and the
three builds a release publishes.

`cargo clippy --all-features` checks only one of the nine combinations. To check one
combination:

```
RUSTUP_TOOLCHAIN=stable cargo check --all-targets --locked -p unipept-api --features mmap,preloaded-text
RUSTUP_TOOLCHAIN=stable cargo test --locked -p index --features mmap,preloaded-text
```

The `backends` job in `ci.yml` runs all nine. It runs the endpoint tests only on the `mmap` build:

```
RUSTUP_TOOLCHAIN=stable cargo test --locked -p unipept-api --features mmap
```
