# The workspace's build, test and check recipes. `just` lists them, `just --usage <recipe>`
# shows a recipe's options, and `just --dry-run <recipe>` prints its commands without running them.
# Recipes run from this directory wherever `just` is invoked, so no recipe reads the caller's location.
#
# `--crate <name>` narrows a recipe to one package. `prepend(" -p ", crate)` is ` -p <name>` when it is
# set and nothing when it is not.

set shell := ["bash", "-euo", "pipefail", "-c"]

# The nightly `fmt` runs under. CI installs the same one by reading it with `just --evaluate nightly`.
nightly := "nightly-2026-09-29"

[private]
default:
    @{{ quote(just_executable()) }} --list --unsorted

# Run the tests with nextest, warnings as errors
[arg("crate", long, help="narrow to one package")]
nextest crate="":
    RUSTFLAGS="-D warnings" cargo nextest run{{ prepend(" -p ", crate) }} --all-features

# Nextest skips doctests, and `ring_spsc` and `ring_mpsc` keep their `compile_fail` soundness guards there.

# Run the doctests, rustdoc warnings as errors
[arg("crate", long, help="narrow to one package")]
doctest crate="":
    RUSTDOCFLAGS="-D warnings" cargo test --doc{{ prepend(" -p ", crate) }} --all-features

# Run clippy over every target, warnings as errors
[arg("crate", long, help="narrow to one package")]
lint crate="":
    cargo clippy{{ prepend(" -p ", crate) }} --all-targets --all-features -- -D warnings

# Final verification: nextest, doctests and clippy
[arg("crate", long, help="narrow to one package")]
test crate="": (nextest crate) (doctest crate) (lint crate)

# Runs nextest alone, so it skips the doctests and clippy that `test` adds.

# Filtered nextest run, for iterating during development
[arg("crate", long, help="narrow to one package")]
[arg("filter", long, help="run only the tests whose name contains this")]
test_only crate="" filter="":
    cargo nextest run{{ prepend(" -p ", crate) }} --all-features{{ prepend(" ", filter) }}

# Find unused dependencies with cargo-udeps
udeps:
    cargo +nightly udeps --all-targets --all-features

# Check Cargo.lock against the RustSec advisory database
audit:
    if [ -f Cargo.lock ]; then cargo +nightly audit; else echo "no Cargo.lock — skipping audit"; fi

# `run_all.sh` takes the family, then the stage, then gate ids, the order its own usage examples fix.
# Its declarations live under `bench_harness/gate/declared/`.

# Run the family's declared gate suite
[arg("family", long, help="family whose declared gates run")]
[arg("stage", long, help="scope to one stage, such as S4")]
[arg("gates", help="gate ids to run, such as g3. Every declared gate when none")]
gate family="ring" stage="" *gates:
    cd bench_harness && bash gate/run_all.sh --family {{ family }}{{ prepend(" --stage ", stage) }}{{ if gates == "" { "" } else { " " + gates } }}

# Full pre-push check: test, udeps, audit and the gate suite
verify: test udeps audit gate

# Compile the workspace
[arg("crate", long, help="narrow to one package")]
build crate="":
    cargo build{{ prepend(" -p ", crate) }} --all-features

# `rustfmt.toml`'s unstable options (`group_imports`, `imports_layout`) take effect only on nightly.
# Stable ignores them with a warning.

# Apply the repo's rustfmt style under the pinned nightly
[arg("check", long, value="true", help="verify formatting without writing")]
fmt check="false":
    cargo +{{ nightly }} fmt --all{{ if check == "true" { " -- --check" } else { "" } }}

# Incremental `cargo doc` skips re-checking unchanged crates, which hides broken intra-doc links in them,
# so the old output goes first.

# Rebuild rustdoc from a clean slate, warnings as errors
doc:
    rm -rf target/doc
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --keep-going

# Run ring_bench's comparison example
bench:
    cargo run -p ring_bench --all-features --example comparison

# Packages each crate and builds it from the package, as crates.io would. Siblings not yet on crates.io
# resolve locally.

# Dry-run `cargo publish` for every publishable crate, uploading nothing
[arg("crate", long, help="narrow to one package")]
publish_check crate="":
    cargo publish --dry-run --allow-dirty {{ if crate == "" { "--workspace" } else { "-p " + crate } }}

# Remove target/ and the gate scratch logs
clean:
    cargo clean
    rm -f ./-[0-9][0-9][0-9][0-9]_*.log
