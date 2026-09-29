#!/usr/bin/env bash
# G11 — this project's own math library is the only source of math and CG
# types in the repository.
#
# This repository's own graphics-and-math rule requires every
# linear-algebra type, geometry routine, renderer and GPU abstraction to come
# from that one library's module, and forbids "anything offering a parallel CG
# or math surface" beside it. Two ways to acquire one, so two detectors:
#
#   A. a dependency on a third-party math or graphics crate;
#   B. a vector-shaped struct written by hand in this repository.
#
# Why this gate exists at all: it was written after a hand-rolled `{ x, y, z }`
# type with `dot`, `cross`, `normalized`, `norm`, `scale` and
# `impl Add/Sub/Neg` was found to have been a standing violation of that rule
# across twenty crates, undetected by every one of the ten gates before it.
# None of them was looking: G5 grades which crates the family *exports*, not
# what it *imports* or declares, and no other gate reads a type's shape. The
# violation was invisible to the whole ladder, which is the condition this
# gate is written to end rather than the specific type it was found on.
#
# Why repo-wide rather than family-scoped, like G10 and unlike G1-G9: the rule
# is a property of the repository, not of any one family. Both instances found
# so far make the point — one hand-rolled vector type in one family and
# another in a second, neither of which any single family's scope would have
# covered.
#
# Detector B is a shape heuristic, deliberately: it matches a struct carrying
# float fields named `x` and `y`, which is what a hand-rolled vector looks like
# before it grows anything else. It does not attempt to parse Rust. A type that
# is genuinely a record rather than a vector is declared in the allowlist and
# held to the condition that makes it one — see that file.
set -uo pipefail
GATE=G11
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"

assert_declared_crates_exist

# Both trees the rule bounds. The demo-crates root is in scope deliberately,
# not by oversight: the rule's Applicability covers it, its Enforcement recipe
# greps it, and its reference example of a completed port — one demo ported
# off a third-party engine onto this project's own GPU driver — happened
# there. A gate narrower than the rule it enforces reports REACHED about
# ground it never looked at.
SCAN_ROOTS=( "${CRATE_ROOTS[@]}" "$REPO/spike" )
for root in "${SCAN_ROOTS[@]}"; do
  [ -d "$root" ] || fail "scan root ${root#"$REPO"/} does not exist"
done

decl="$DECL/cgtools_allowlist.txt"
[ -f "$decl" ] || fail "no declared cgtools allowlist at ${decl#"$REPO"/}"

mapfile -t allowed < <( decl_lines "$decl" | sort -u )

# --- detector A: a second math or graphics dependency ------------------------
#
# Whole dependency names only, anchored at line start, so this project's own
# GPU driver crate — a legitimate dependency built on top of `wgpu` — is not
# read as `wgpu` itself.
#
# `winit` and `glutin` are deliberately ABSENT. The rule's own Applicability
# permits "windowing, input, and event loops (`winit`)" in as many words — they
# are adjacent concerns, not a CG or math surface — and two spikes declare
# `winit` today. Listing them would have failed the gate on code the rule
# explicitly allows, which is why this list is built from the rule's Forbidden
# bullet rather than from what sounds graphical. `glium` stays: it is a full GL
# wrapper, which is a parallel CG surface, not a window.
FORBIDDEN='glam|nalgebra|nalgebra-glm|cgmath|vek|euclid|ultraviolet|mint|simba'
FORBIDDEN="$FORBIDDEN"'|parry2d|parry3d|ncollide2d|ncollide3d|rapier2d|rapier3d'
FORBIDDEN="$FORBIDDEN"'|kurbo|lyon|palette|three-d|rend3|wgpu|glium'
FORBIDDEN="$FORBIDDEN"'|macroquad|ggez|bevy_render|piston|piston2d-graphics'

# Fix(g11_detector_a_blind_to_deeply_nested_crates)
# Root cause: `-mindepth 2 -maxdepth 2` encoded a single crate depth — a crate
#   directly under a scan root — which was every crate when this gate was
#   written. Two roots have since grown deeper: most crates now sit one level
#   deeper (depth 3, one root per family) and a smaller number deeper still
#   (depth 4). Measured against the live tree the gather read barely a third
#   of all manifests, blind to the rest, and several it did find under the
#   depth-3 root are virtual `[workspace]` manifests carrying no dependency
#   section — so that root contributed no crate at all while still looking
#   non-empty to the guard below.
# Pitfall: `[ ${#manifests[@]} -gt 0 ]` aggregates across every root, so a root
#   that goes dark hides behind its siblings and the gate still reports
#   REACHED — the silent-success shape this whole ladder exists to end. Proving
#   non-vacuity therefore has to be per-root, and has to demand a real
#   `[package]` manifest: the depth-3 root satisfied a bare count on virtual
#   workspace files alone. The scan itself deliberately keeps those virtual
#   files, since `[workspace.dependencies]` is a real way to acquire a
#   forbidden crate. Depth bounds mirror `common.sh`'s `all_crate_names()`;
#   keep the two in step if a root ever lands at a new depth.
manifests=()
for root in "${SCAN_ROOTS[@]}"; do
  while IFS= read -r m; do manifests+=( "$m" ); done \
    < <( find "$root" \( -path '*/target/*' -o -path '*/-target*/*' \) -prune -o \
           -mindepth 2 -maxdepth 4 -name Cargo.toml -print 2>/dev/null )
  crates="$( find "$root" \( -path '*/target/*' -o -path '*/-target*/*' \) -prune -o \
               -mindepth 2 -maxdepth 4 -name Cargo.toml \
               -exec grep -lE '^\[package\]' '{}' '+' 2>/dev/null | wc -l )"
  [ "$crates" -gt 0 ] \
    || fail "detector A found no crate manifest under ${root#"$REPO"/}, so that root went unscanned and a forbidden dependency declared there would be reported as clean"
done

mapfile -t dep_hits < <(
  [ ${#manifests[@]} -gt 0 ] \
    && grep -nE "^($FORBIDDEN)[[:space:]]*=" "${manifests[@]}" 2>/dev/null \
    | sed "s|^$REPO/||"
)

# --- detector B: a hand-rolled vector-shaped struct --------------------------

# Matched on field shape, never on the type's name — the original violation was
# called `V3`, which no sensible blocklist contains. `pub` is deliberately not
# required either: `lang_partition::Coord` spells its fields `x : f64` with
# accessors, the idiomatic Rust encapsulation shape, and a detector keyed to
# `pub x` cannot see it. That is not hypothetical — the rule's own hand recipe
# is keyed that way and misses this file today.
vector_shaped()
{
  # Fix(g11_awk_state_leaks_across_files_and_braceless_structs)
  # Root cause: `inb`/`hx`/`hy`/`nm`/`ln` are awk globals, and this runs as one
  #   `xargs -0 awk` invocation over every scanned file — FNR resets per file
  #   but these do not. A struct with no `{ }` body (a tuple or unit struct,
  #   e.g. `struct Meters(f32);` — an ordinary, common Rust newtype) matches
  #   the opening pattern and sets inb=1, but since it has no brace at all
  #   nothing ever closes it, so `inb` stays stuck open into every line that
  #   follows — including every subsequent file in the same awk invocation.
  #   Confirmed: filea.rs holding only `struct Meters(f32);` followed by
  #   filec.rs holding only a `fn compute() { let x: f32 = ..; let y: f32 =
  #   ..; }` (no struct at all) printed "filec.rs:1:Meters" — a false
  #   violation on a file with no struct whatsoever, attributed to a name and
  #   line number from a different, preceding file.
  # Pitfall: this is exactly the misattribution class this gate's own header
  #   was written to end (the V3 violation invisible to ten prior gates) —
  #   a heuristic that is allowed to miss things per its own design note, but
  #   was never meant to blame innocent files instead.
  # Fix(g11_detector_b_blind_to_declarations_under_tests)
  # Root cause: the scan required `-path '*/src/*'`, so a type declared in a
  #   crate's `tests/` tree was invisible to the gate while remaining visible
  #   to the hand recipe in `rulebook.md`, which globs `*.rs` without a
  #   directory filter. Measured against the live tree that divergence was one
  #   file wide — one sibling crate's own compile test declares a
  #   `component!` Position the recipe returns and the gate did not — so the
  #   rule's own claim that gate and recipe read the same trees was false by a
  #   file nobody had counted.
  # Pitfall: a test-resident declaration is not a lesser violation. A forked
  #   `V3` carrying a `dot` proves the vocabulary exists and is being
  #   exercised, wherever the file sits; the original violation spread from
  #   one crate to twenty precisely because nobody read the declaration as
  #   load-bearing. The `target/` prune is required once `tests/` is in scope
  #   for the same reason detector A needs it — generated sources land under
  #   build directories and would otherwise be scanned as if hand-written.
  #
  # Finding(g11_multiline_tuple_struct_opener_still_leaks_state), not fixed
  # here: the fix above excludes a braceless struct declared and closed on
  # ONE line (`!/;[[:space:]]*$/` catches `struct Meters(f32);`), but a
  # tuple struct whose fields span several lines —
  #
  #     struct Meters(
  #       f32,
  #     );
  #
  # — opens with a line ending in `(`, not `;`, so it is NOT excluded, sets
  # `inb = 1`, and then never contains a `}` at all: the same stuck-open
  # state the fix above already named, for a shape that fix's single-line
  # check does not reach. Everything from that opener to EOF (or the next
  # `}` this file happens to contain, from a function, impl block, or
  # anything else) is then read as this struct's own body — a `}` belonging
  # to unrelated code would close it, attributing whatever `x`/`y` shape
  # appears in between to a struct name that has nothing to do with it,
  # exactly the misattribution class this file's own header says the
  # heuristic was never meant to produce. No live trigger: measured
  # directly, not assumed — a repo-wide search of every scan root's `src/`
  # and `tests/` trees for a `struct NAME(` opener with nothing else on the
  # line returns zero matches; every tuple struct in this corpus today
  # either closes on the same line or uses `{ }` named fields. A full fix
  # needs the same kind of multi-line-open tracking `g1_coverage.sh`'s
  # `phantom_lines()` carries for a different construct, which is a larger
  # change than a drive-by here.
  find "${SCAN_ROOTS[@]}" \( -path '*/target/*' -o -path '*/-target*/*' \) -prune -o \
      -name '*.rs' \( -path '*/src/*' -o -path '*/tests/*' \) -print0 2>/dev/null \
    | xargs -0 awk '
      FNR == 1 { inb = 0; hx = 0; hy = 0 }
      !inb && /struct[[:space:]]+[A-Za-z_]/ && !/^[[:space:]]*(\/\/|\*)/ && !/;[[:space:]]*$/ {
        nm = $0; sub( /.*struct[[:space:]]+/, "", nm ); sub( /[^A-Za-z0-9_].*/, "", nm )
        inb = 1; hx = 0; hy = 0; ln = FNR
      }
      inb {
        if ( $0 ~ /[[:space:]]x[[:space:]]*:[[:space:]]*f(32|64)/ ) hx = 1
        if ( $0 ~ /[[:space:]]y[[:space:]]*:[[:space:]]*f(32|64)/ ) hy = 1
        if ( $0 ~ /}/ ) { if ( hx && hy ) printf "%s:%d:%s\n", FILENAME, ln, nm; inb = 0 }
      }
    ' 2>/dev/null
}

mapfile -t shape_hits < <( vector_shaped | sed "s|^$REPO/||" | sort )

# Vector arithmetic — what turns an allowlisted record back into a vector type.
ARITH='fn (dot|cross|normalize|normalized|try_normalize|mag|mag2|magnitude|length|norm|norm_sq|scale)\b'
ARITH="$ARITH"'|impl[^{]*\b(Add|Sub|Mul|Div|Neg|AddAssign|SubAssign|MulAssign)\b[^{]*for'

violations=() covered=() expired=()
for h in "${shape_hits[@]}"; do
  [ -n "$h" ] || continue
  file="${h%%:*}"
  if [ ${#allowed[@]} -gt 0 ] && printf '%s\n' "${allowed[@]}" | grep -qxF -- "$file"; then
    covered+=( "$file" )
    # The exemption holds only while the type carries no arithmetic.
    if grep -qE "$ARITH" "$REPO/$file" 2>/dev/null; then
      expired+=( "$h" )
    fi
  else
    violations+=( "$h" )
  fi
done

# --- non-vacuity -------------------------------------------------------------
#
# Three halves. Detector B is anchored the way G10 anchors its own: every
# allowlisted file must still match, or the exemption is stale and the detector
# is unproven. Detector A has no allowlist to anchor against — the correct
# steady state is zero third-party math crates — so it is proven against a
# synthetic control instead, which is the only way to distinguish "nothing
# forbidden is declared" from "the pattern matches nothing at all". The third
# checks the gate against the rule it automates, below.

silent=()
for a in "${allowed[@]}"; do
  printf '%s\n' "${covered[@]}" | grep -qxF -- "$a" || silent+=( "$a" )
done

if [ ${#silent[@]} -gt 0 ]; then
  fail "${#silent[@]} allowlisted file(s) no longer define a vector-shaped struct, so detector B proved nothing: ${silent[*]} — either the type was moved to the project's math library (delete the entry) or the detector has stopped matching it"
fi

# Third half, and it guards a different failure than the two above. The rule's
# own § Graphics & Math : cgtools Exclusive section prints this same ARITH
# pattern as the hand triage recipe a reader runs on a fresh hit. If the two drift, the
# hand answer and the gate's answer stop agreeing, and the reader who triages
# by hand gets "clean" on a file the gate would fail — the executable-recipe
# staleness this repository has already been bitten by. So the gate asserts its
# own pattern is the one the rule publishes. Reformatting the recipe fails this
# loudly, which is the point: re-sync it rather than let the two drift quietly.
rule="$REPO/rulebook.md"
[ -f "$rule" ] || fail "no rulebook.md at repository root — the rule this gate automates is missing"
if ! grep -qF -- "$ARITH" "$rule"; then
  fail "the triage pattern in rulebook.md § Graphics & Math : cgtools Exclusive no longer matches this gate's own, so a hand triage and a gate run can now disagree — re-sync the recipe in that section with the ARITH pattern in this file"
fi

control="$( mktemp -d )"
trap 'rm -rf "$control"' EXIT
printf 'glam = "0.30"\n' > "$control/Cargo.toml"
if ! grep -qE "^($FORBIDDEN)[[:space:]]*=" "$control/Cargo.toml"; then
  fail "detector A does not match a known-forbidden dependency in its own control — the pattern is broken, and a clean run would mean nothing"
fi

# --- verdict -----------------------------------------------------------------

if [ ${#dep_hits[@]} -gt 0 ]; then
  printf 'NOT REACHED %s — third-party math or graphics dependency:\n' "$GATE"
  printf '  %s\n' "${dep_hits[@]}"
  printf '  fix: take it from the project'"'"'s math library instead; where that library lacks it, extend it\n'
  printf '  see: rulebook.md \xc2\xa7 Graphics & Math : cgtools Exclusive\n'
  exit 1
fi

if [ ${#violations[@]} -gt 0 ]; then
  printf 'NOT REACHED %s — vector-shaped struct defined outside the project'"'"'s math library:\n' "$GATE"
  printf '  %s\n' "${violations[@]}"
  printf '  fix: use the project'"'"'s math library (its F32x3, F64x3, ... vector types) — or declare the file in %s if it is a record rather than a vector\n' "${decl#"$REPO"/}"
  printf '  see: rulebook.md \xc2\xa7 Graphics & Math : cgtools Exclusive\n'
  exit 1
fi

if [ ${#expired[@]} -gt 0 ]; then
  printf 'NOT REACHED %s — an allowlisted record has grown vector arithmetic, so its exemption no longer holds:\n' "$GATE"
  printf '  %s\n' "${expired[@]}"
  printf '  fix: it is a vector type now — move it to ndarray_cg and drop the allowlist entry\n'
  printf '  see: %s\n' "${decl#"$REPO"/}"
  exit 1
fi

pass "no third-party math dependency, and no vector-shaped struct outside the ${#allowed[@]} declared record(s), each still carrying no arithmetic"
