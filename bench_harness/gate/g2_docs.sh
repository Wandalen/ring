#!/usr/bin/env bash
# G2 — zero missing_docs diagnostics, every crate exports >= 1 public item,
# no implemented crate's own prose still calls itself a skeleton, and every
# worked example in a doc comment actually compiles and runs.
#
# Every family crate has missing_docs enforced, so a clean build is the first
# half — but by two mechanisms rather than one, and the difference is the
# reason the RUSTFLAGS below is load-bearing rather than decoration. Of the 226
# library crates the eleven declaring families name, fifty-nine carry
# `#![deny(missing_docs)]` in their own crate root, and the other hundred and
# sixty-seven carry `[lints] workspace = true`, inheriting `missing_docs =
# "warn"` from their workspace root. Warn, not deny: for that majority the lint
# only fails a build because this gate passes `RUSTFLAGS=-D warnings`, and
# dropping that flag as redundant would silently downgrade three quarters of
# the tree to a warning nobody reads while this gate kept reporting REACHED.
# (Measured, not assumed: `cargo check -p spatial_bounds -v` shows
# `--warn=missing_docs` on the rustc command line for a crate whose lib.rs
# never mentions the lint.)
#
# The second half is what stops an empty crate from passing: a skeleton with no
# public items has nothing to leave undocumented.
#
# The third half exists because the first two are both blind to the same
# failure. `missing_docs` sees rustdoc comments and nothing else; the public
# item count sees code. Neither reads readme.md, so a crate could be complete,
# tested, fully documented at the item level, and still open with "Skeleton —
# no implementation yet" — which is the first sentence a human reads and the
# only one an automated check never did. Found after S2 with 14 crates in
# exactly that state.
set -uo pipefail
GATE=G2
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist

bare=()
stale=()
crates=()
libs=()
binonly=()
while read -r c; do
  d="$( crate_dir "$c" )"
  crates+=( "$c" )
  # Which crates a doctest run can reach at all, recorded here for the
  # non-vacuity checks after that run. Read out of the manifest rather than
  # assumed from the conventional filename: a `[lib]` section may relocate the
  # library, and a crate with neither a declared path nor `src/lib.rs` has no
  # library for a doctest to live in. Differentially tested against `cargo
  # metadata` over all 259 packages in the tree — no disagreement — which is
  # what makes reading the manifest directly preferable to a second
  # metadata walk beside the one family_bin_sources() already performs.
  libsrc="$( awk '
    /^[[:space:]]*\[/ { in_lib = ( $0 ~ /^[[:space:]]*\[lib\][[:space:]]*$/ ); next }
    in_lib && /^[[:space:]]*path[[:space:]]*=/ && match( $0, /"[^"]*"/ ) {
      print substr( $0, RSTART + 1, RLENGTH - 2 ); exit
    }
  ' "$d/Cargo.toml" 2>/dev/null )"
  [ -n "$libsrc" ] || libsrc="src/lib.rs"
  if [ -f "$d/$libsrc" ]; then libs+=( "$c" ); else binonly+=( "$c" ); fi
  # `pub ` only — `pub(crate)`, `pub(super)` and `pub(in ...)` are crate-private
  # and would let a crate exporting nothing satisfy the non-vacuity check.
  n="$( grep -rhcE '^[[:space:]]*pub[[:space:]]' "$d/src" 2>/dev/null | paste -sd+ | bc )"
  if [ "${n:-0}" -gt 0 ]; then
    # Implemented. Its prose must not still describe a skeleton — in the
    # readme, in the crate docs index, or in the crate root's own module doc.
    #
    # Those three files and not the whole `docs/` tree, which is a narrowing of
    # what this line used to do and is the point rather than an oversight. The
    # boilerplate this detects is a crate's own front-door self-description, and
    # the front door is exactly these three. The rest of `docs/` is analysis,
    # which discusses skeletons in the ordinary course of being about them —
    # `ring_event` documents that the lifecycle marker "cannot distinguish a
    # skeleton from a finished implementation", a sentence that is both true and
    # a substring match. Scanning the corpus made this gate report a finished,
    # 204-line, sixteen-test crate as unimplemented on the strength of a phrase
    # in a findings document about that very confusion.
    grep -rlqE 'Skeleton —|is a skeleton|no implementation yet' \
      "$d/readme.md" "$d/docs/readme.md" "$d/src/lib.rs" 2>/dev/null \
      && stale+=( "$c" )
  else
    bare+=( "$c" )
  fi
done < <( family_crates )

if [ ${#bare[@]} -gt 0 ]; then
  fail "${#bare[@]} crate(s) export no public item: ${bare[*]}"
fi

if [ ${#stale[@]} -gt 0 ]; then
  fail "${#stale[@]} implemented crate(s) still described as a skeleton in their own docs: ${stale[*]}"
fi

work="$( mktemp -d )"
trap 'rm -rf "$work"' EXIT

# The fan-out itself lives in common.sh's cargo_over_workspaces(), because G9
# needs the same one-invocation-per-owning-workspace shape. This gate reads the
# merged text of both streams, so it passes one path for stdout and stderr
# alike; G9 parses stdout as JSON and passes two.
log="$work/check_all_features.log"
if ! cargo_over_workspaces "$log" "$log" "RUSTFLAGS=-D warnings" \
  check --all-features; then
  cat "$log"
  fail "cargo check reports diagnostics (missing_docs denied per crate)"
fi

# And again with every optional feature off. `--all-features` compiles the
# `#[ cfg( feature = ... ) ]` arms and skips the `#[ cfg( not( ... ) ) ]` ones,
# so on its own it leaves half of a feature-gated crate never built by any gate
# — a default build could stop compiling and every gate would still pass.
# `ring_core` (via its tests) and `ring_bench` (via both src and tests) currently
# reach this hole; `ring_factory`'s crossbeam-specific code is purely additive
# (no `cfg(not(...))` counterpart branch that could silently vanish), so it
# doesn't. Either way, the hole is in the gate rather than in any one crate.
log="$work/check_no_default_features.log"
if ! cargo_over_workspaces "$log" "$log" "RUSTFLAGS=-D warnings" \
  check --all-targets --no-default-features; then
  cat "$log"
  fail "cargo check reports diagnostics with default features off"
fi

# The examples have to run, not merely exist. `cargo check` never compiles a
# doctest, so an example that does not build is invisible to everything above:
# `missing_docs` is satisfied by the doc comment's presence, and the public item
# count and the skeleton grep both read the wrong thing entirely.
#
# Found after S11 with two crates in exactly that state, both broken the same
# way. rustdoc infers the `Result`-returning wrapper a `?` needs by checking
# whether the last line literally ends `(())`; this family's codestyle writes
# `( () )`, so the heuristic silently declined and the example had a `?` in a
# function returning unit. Two crates' worth of worked examples had never once
# been compiled, through eleven stages of gates that all reported reached.
log="$work/doctest.log"
if ! cargo_over_workspaces "$log" "$log" "RUSTDOCFLAGS=-D warnings" \
  test --doc --all-features; then
  cat "$log"
  fail "doctests do not compile or do not pass"
fi

# Non-vacuity for the doctest clause, in the same two parts G9 spells out for
# clippy, because a doctest run passes without grading anything in the same two
# ways — and until now the exit code above was the only thing read.
#
#   * Reach. In a selection mixing lib-bearing crates with bin-only ones, cargo
#     silently drops the bin-only members: `cargo test --doc` is a lib-only
#     operation, and a selection containing at least one library gets no warning
#     about the rest. Measured on lang: one `Doc-tests` header for `lang_common`,
#     none for the `smoke_lang_*` binary beside it, exit 0 — while the pass line
#     counted both. (A selection of nothing but bin-only crates does refuse, with
#     `error: no library targets found in packages: …` and exit 101, so this is
#     specific to mixed selections and invisible everywhere else.)
#
#     Bin-only crates are therefore named rather than asserted over. No doctest
#     can ever reach one, so requiring a header for them would be permanently
#     unsatisfiable — one sibling family has over a dozen — and the honest
#     statement is that they are outside what this clause grades, not that they
#     passed it.
#
#   * Vacuity. A family whose crates carry no doc examples at all runs zero
#     doctests and exits 0, and the pass line then reported every crate as
#     documenting its public items "with examples that run": a conjunction over
#     an empty set, which is the shape G9's own header names. Measured across
#     several declaring families: eleven `Doc-tests` headers, every one
#     `running 0 tests`, exit 0. More than one declaring family was in this
#     state.
#
# As in G9, both checks can only turn a REACHED into a NOT REACHED — each adds a
# failure path and removes none — so neither can manufacture a pass, only
# withdraw one that was never earned.
mapfile -t reached < <( awk '$1 == "Doc-tests" { print $2 }' "$log" | sort -u )

unreached=()
for c in "${libs[@]}"; do
  printf '%s\n' "${reached[@]}" | grep -qx "${c//-/_}" || unreached+=( "$c" )
done
if [ ${#unreached[@]} -gt 0 ]; then
  cat "$log"
  fail "doctests passed, but no doctest run reached ${#unreached[@]}/${#libs[@]} library crate(s), so a clean result says nothing about them: ${unreached[*]}"
fi

# `test result: ok. N passed; …` — the harness line each `Doc-tests` header is
# followed by. Summed rather than checked per crate: the bar here is that the
# clause graded something, not that every crate carries an example, which is a
# larger claim than this gate has ever made good on.
ran="$( awk '/^test result:/ { for ( i = 1; i < NF; i++ ) if ( $( i + 1 ) == "passed;" ) total += $i } END { print total + 0 }' "$log" )"
if [ "${ran:-0}" -eq 0 ]; then
  fail "doctests passed over ${#libs[@]} library crate(s), but zero doctests ran — every example-bearing claim below would be a conjunction over an empty set, so this says nothing about whether any documented example compiles"
fi

note=""
if [ ${#binonly[@]} -gt 0 ]; then
  note=" ( ${#binonly[@]} binary-only crate(s) have no library for a doctest to reach and are outside this clause: ${binonly[*]} )"
fi
pass "${#crates[@]} crate(s) export public items and none still calls itself a skeleton; ${ran} doctest(s) ran, reaching all ${#libs[@]} library crate(s)${note}"
