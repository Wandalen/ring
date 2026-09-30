# Task runner for the ring workspace; `just` lists the recipes. Mirrors `verb/` one to one
# (verb/readme.md) and depends on none of it, so the scripts can be retired.
#
# Crate-scoped recipes default to the crate you run them from, which replaces the per-crate
# `verb/` wrappers: `cd ring_spsc && just test-only wrap` tests only `ring_spsc`.

set shell := ["bash", "-euo", "pipefail", "-c"]

[private]
_rel := trim_start_match(trim_start_match(invocation_directory(), justfile_directory()), "/")
[private]
_top := replace_regex(_rel, '/.*$', '')
here := if _top == "" { "" } else if path_exists(justfile_directory() / _top / "Cargo.toml") == "true" { _top } else { "" }
[private]
_just := quote(just_executable()) + " --justfile " + quote(justfile())

# List recipes.
default:
    @{{ _just }} --list --unsorted

# Install the nightly toolchain and the cargo tools recipes and gates call. Idempotent.
setup:
    #!/usr/bin/env bash
    set -euo pipefail
    missing="$({{ _just }} _missing || true)"
    if [[ -z "$missing" ]]; then echo "setup: nothing missing"; exit 0; fi
    if grep -qx 'toolchain:nightly' <<<"$missing"; then
        rustup toolchain install nightly --profile minimal --component rustfmt
    fi
    crates="$(sed -n 's/^cargo://p' <<<"$missing" | tr '\n' ' ')"
    if [[ -n "${crates// /}" ]]; then
        if cargo binstall -V >/dev/null 2>&1; then
            cargo binstall -y --locked $crates
        else
            cargo install --locked $crates
        fi
    fi
    sed -n 's/^bin:/setup: install with your system package manager: /p' <<<"$missing" >&2

# List what `setup` would install; fails when anything is missing.
setup-check:
    #!/usr/bin/env bash
    set -euo pipefail
    missing="$({{ _just }} _missing || true)"
    if [[ -z "$missing" ]]; then echo "setup: nothing missing"; exit 0; fi
    sed 's/^/missing /' <<<"$missing"
    exit 1

# nextest: tests; udeps, audit: `test 4`; tarpaulin: G1; mutants: G13; taplo: fmt.
_missing:
    #!/usr/bin/env bash
    rustup run nightly rustfmt --version >/dev/null 2>&1 || echo "toolchain:nightly"
    for t in nextest:cargo-nextest udeps:cargo-udeps audit:cargo-audit \
             tarpaulin:cargo-tarpaulin mutants:cargo-mutants; do
        cargo "${t%%:*}" --version >/dev/null 2>&1 || echo "cargo:${t#*:}"
    done
    for t in taplo:taplo-cli just:just; do
        command -v "${t%%:*}" >/dev/null 2>&1 || echo "cargo:${t#*:}"
    done
    for b in python3 jq perl bc sha256sum; do
        command -v "$b" >/dev/null 2>&1 || echo "bin:$b"
    done

# Format Rust (nightly rustfmt, rustfmt.toml), TOML (taplo, taplo.toml) and this justfile.
fmt:
    cargo +nightly fmt --all
    RUST_LOG=error taplo fmt
    {{ _just }} --fmt --unstable

# Verify formatting without writing.
fmt-check:
    cargo +nightly fmt --all -- --check
    RUST_LOG=error taplo fmt --check
    {{ _just }} --fmt --unstable --check

# Compile; `just build ring_spsc` narrows.
build crate=here:
    cargo build {{ if crate == "" { "--workspace" } else { "-p " + crate } }} --all-features

# Type-check every target.
check crate=here:
    cargo check {{ if crate == "" { "--workspace" } else { "-p " + crate } }} --all-targets --all-features

# Clippy over every target, warnings as errors.
lint crate=here:
    cargo clippy {{ if crate == "" { "--workspace" } else { "-p " + crate } }} --all-targets --all-features -- -D warnings

# Filtered nextest run while iterating: `just test-only wrap`, `just test-only wrap ring_spsc`.
test-only filter="" crate=here:
    cargo nextest run {{ if crate == "" { "--workspace" } else { "-p " + crate } }} --all-features {{ if filter == "" { "" } else { quote(filter) } }}

# nextest, warnings as errors.
nextest crate=here:
    RUSTFLAGS="-D warnings" cargo nextest run {{ if crate == "" { "--workspace" } else { "-p " + crate } }} --all-features

# Doctests (nextest does not run them), warnings as errors.
test-doc crate=here:
    RUSTDOCFLAGS="-D warnings" cargo test {{ if crate == "" { "--workspace" } else { "-p " + crate } }} --doc --all-features

# Unused dependencies (nightly).
udeps:
    cargo +nightly udeps --all-targets --all-features

# RustSec advisories. Needs a Cargo.lock, which is gitignored and exists after any build.
audit:
    if [ -f Cargo.lock ]; then cargo audit; else echo "audit: no Cargo.lock — skipping"; fi

# Leveled verification: 1 nextest, 2 +doctests, 3 +clippy, 4 +udeps +audit, 5 +gates.
test level="3" crate=here:
    #!/usr/bin/env bash
    set -euo pipefail
    case {{ quote(level) }} in
        1|2|3|4|5) ;;
        *) echo "test: level must be 1-5, got '{{ level }}'" >&2; exit 2 ;;
    esac
    level={{ quote(level) }}
    {{ _just }} nextest {{ quote(crate) }}
    if (( level >= 2 )); then {{ _just }} test-doc {{ quote(crate) }}; fi
    if (( level >= 3 )); then {{ _just }} lint {{ quote(crate) }}; fi
    if (( level >= 4 )); then {{ _just }} udeps; {{ _just }} audit; fi
    if (( level >= 5 )); then {{ _just }} gate; fi

# Full pre-push gate: `test 5` over the whole workspace.
verify:
    {{ _just }} test 5 ""

# Every loom model — any tests/*.rs with a `loom::model(` call — in its own target-loom/.
loom crate=here:
    #!/usr/bin/env bash
    set -euo pipefail
    crate={{ quote(crate) }}
    ran=0
    for f in $(grep -lE '^\s*loom::model\(' ./*/tests/*.rs | sort); do
        pkg="$(cut -d/ -f2 <<<"$f")"
        [[ -n "$crate" && "$pkg" != "$crate" ]] && continue
        RUSTFLAGS="--cfg loom" CARGO_TARGET_DIR=target-loom \
            cargo test -p "$pkg" --test "$(basename "$f" .rs)"
        ran=$((ran + 1))
    done
    # An empty run would pass having checked nothing.
    (( ran > 0 )) || { echo "loom: no loom model found${crate:+ in $crate}" >&2; exit 1; }

# Rustdoc from a clean slate: incremental `cargo doc` hides broken links in unchanged crates.
doc *args:
    #!/usr/bin/env bash
    set -euo pipefail
    target="$(cargo metadata --no-deps --format-version 1 \
        | python3 -c 'import json, sys; print(json.load(sys.stdin)["target_directory"])')"
    rm -rf "$target/doc"
    RUSTDOCFLAGS="-D warnings" cargo doc --workspace --no-deps --all-features --keep-going {{ args }}

# Stage gates (`gate g6`, `gate --stage S5`, `gate --every`). Slow; G12 rewrites src/ meanwhile.
gate *args:
    cd bench_harness && bash gate/run_all.sh {{ args }}

# Mutation survey for one crate (slow, mutates src/ in place); keeps G13 fresh.
mutant-survey crate:
    bash bench_harness/gate/mutant_survey.sh {{ quote(crate) }}

# The write-path comparison; `just bench --release` for timings worth recording.
bench *args:
    cargo run -p ring_bench --all-features --example comparison {{ args }}

# Remove build artefacts, the loom and gate target dirs, and scratch logs.
clean:
    cargo clean
    rm -rf target-loom target-gate
    rm -f ./-[0-9][0-9][0-9][0-9]_*.log

# Family manifest info as flat JSON — no single crate is "the" package.
package-info:
    #!/usr/bin/env bash
    set -euo pipefail
    field() { grep -m1 "^$1" Cargo.toml | sed -E "s/^$1 *= *\"([^\"]*)\".*/\1/"; }
    count="$(grep -cE '^\s*"(ring_[a-z_]+|bench_harness)",?\s*$' Cargo.toml)"
    printf '{\n  "family": "ring",\n  "crate_count": %s,\n  "edition": "%s",\n' "$count" "$(field edition)"
    printf '  "license": "%s",\n  "repository": "%s",\n' "$(field license)" "$(field repository)"
    printf '  "language": "rust",\n  "package_manager": "cargo"\n}\n'
