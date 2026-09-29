#!/usr/bin/env bash
# G10 — no unpinned transcendental reaches the deterministic simulation path.
#
# Every transcendental reaching the deterministic simulation path must go
# through the pinned kernel: `pinned::sin( x )`, never `x.sin()`. IEEE-754
# pins only six operations, so two conforming libms may differ in the last
# bit — and an iterative solver turns that bit into a branch two clients
# answer differently.
#
# Why a grep rather than a run: G7 grades cross-target determinism and
# STRUCTURALLY CANNOT catch this class. An early 70-point probe found
# glibc-on-aarch64 and musl-on-x86-64 agreeing bitwise on all 70 inputs, so a
# raw libm call produces byte-identical output on the very pair G7 compares
# and passes. No dependency audit sees it either: a leak adds no Cargo.toml
# line and `cargo tree` prints clean. The call shape is the only signal.
#
# Why repo-wide rather than family-scoped, unlike every other gate here:
# determinism is a whole-pipeline property. A broadphase that disagrees across
# machines about which bodies are near each other hands a different body set to
# a byte-perfect propagator, and the propagator's guarantee buys nothing. Both
# leaks this gate was written for prove the point — one was in a sibling
# family's own demo, the other in a crate no scope limited to that family
# would have looked at.
#
# `sqrt` is excluded on purpose: IEEE-754 requires it correctly rounded, so
# `x.sqrt()` is already bit-reproducible. Including it would bury real findings
# under every vector-magnitude call in the repository.
#
# Two detectors, because there are two ways to reach the platform's libm and
# only one of them looks like it.
#
# Detector A greps the call shape `x.sin()`. Note `sin_cos` in its list: for a
# while it was absent, and `\.(sin)\(` cannot match `.sin_cos(`, so the single
# likeliest leak in this repository was the one shape the gate could not see.
# `sin_cos` is what the deterministic path actually calls, and a sibling
# crate's own pitfall doc names `a.sin_cos()` in as many words as the
# keystroke that loses the property. A gate written to catch that keystroke
# has to spell it.
#
# Detector B greps for this project's own math library's unpinned helpers —
# `mat3x3::from_angle_x`, `Quat::from_euler_xyz`, `vec3::to_spherical` — which
# reach `.sin_cos()` internally and so are invisible to any pattern written in
# method-call syntax. It exists because this repository's own rules can be
# obeyed both at once and still produce a leak: the project names that
# library the only permitted source of mathematics, so a developer who needs
# a rotation matrix is pointed straight at `mat3x3::from_angle_x( a )`. It
# compiles, it obeys that rule, and it is unpinned. The deterministic path has
# already stood on this spot once — one function needed a rotation and
# assembled it by hand from `pinned::sin_cos` instead. That was the right
# answer reached without a guard, which is not a thing to rely on twice.
#
# Non-vacuity, detector A, and the half that matters: an allowlist entry has to
# still be doing the thing it was excused for. The one entry currently
# allowlisted is excused precisely because it compares
# `pinned::sin( x ).to_bits()` against `x.sin().to_bits()` to prove a
# substitution is detectable. If that call ever stops matching, either the
# control arm was deleted — a real regression in the guard — or the allowlist
# is stale. Checking only for violations would report REACHED in both cases,
# having verified nothing: the pattern would be proven to match zero lines
# anywhere, which is also what a broken pattern looks like.
#
# Non-vacuity, detector B, which needs a different answer: the repository
# calls none of these helpers today, so a clean run is the expected result
# and cannot distinguish itself from a misspelled pattern. The control is
# that math library itself. Every name in detector B's list must still
# resolve to a `pub fn` in one of its files that still reaches libm — the
# premise the detector rests on, and one nothing in this repository would
# notice going stale. A rename fails here loudly. So does that library pinning
# these helpers itself, and that failure is the correct one: it means the
# gate and the rule behind it want rewriting, not re-running.
set -uo pipefail
GATE=G10
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"

# This gate is repo-wide (see header above), so its scan has to be too.
# common.sh's CRATE_ROOTS deliberately excludes a separate demo-crates root —
# left out there so G18/G22's census would not silently widen (see that
# array's own comment) — but the same exclusion is a regression for G10
# specifically: an earlier relocation moved every demo_* crate to that
# excluded root, and this gate's own allowlist excuses one of them by name as
# its control arm. Since that move, this gate has scanned zero files under
# that root and its non-vacuity check has had nothing live to find that entry
# with — silently, because a stale allowlist entry and a moved-away one fail
# the same way.
#
# Widening the array here, after sourcing, reaches every consumer this file
# calls — the SRC glob below, the unscanned-files safety net, and
# strip_crate_root() — from one line instead of three separately-patched call
# sites, and never reaches another gate: each gate runs as its own bash
# process, so this mutation dies with this script. G11 solved the same
# "also scan the demo-crates root" problem with its own separately-named
# SCAN_ROOTS instead, but its detectors never call strip_crate_root() — they
# normalize hits with a plain repo-root strip. This gate's allowlist is
# crate-relative (no tree prefix at all), a spelling only strip_crate_root()
# produces, so mutating the array it actually reads is what keeps the
# allowlist's existing entries valid, not merely what finds more files.
CRATE_ROOTS+=( "$REPO/spike" )

decl="$DECL/pinned_allowlist.txt"
[ -f "$decl" ] || fail "no declared pinned allowlist at ${decl#"$REPO"/}"

mapfile -t allowed < <( decl_lines "$decl" | sort -u )

# The file set both detectors read, expanded once so the two cannot drift
# apart. `**` is not recursive here — `globstar` is unset, so bash reads it as
# `*` — making this `<dir>/*.rs` plus exactly one level below, which is every
# depth either tree has today.
#
# `tests/` is scanned alongside `src/`, and was not always. It came in when the
# repository-wide rule moved every test out of `src/` into its own directory:
# an inline `#[ cfg( test ) ]` block used to be read as part of the file it sat
# in, so before that move tests were scanned incidentally, and after it no test
# was scanned anywhere. The rule is right and the coverage loss was a side
# effect of it, not an argument against it.
#
# What that reaches is narrower than what detector A guards in `src/`. A test
# is not shipped into the simulation, so a raw libm call there cannot make the
# *product* platform-dependent. It can make the *instrument* platform-dependent,
# which is worse in one specific shape: a test that builds its expected value
# from raw libm is checking the pinned kernel against an oracle that drifts
# across targets exactly the way the thing under test must not, and it fails on
# the machine where the kernel is correct.
#
# `examples/` and `benches/` stay out deliberately. Neither decides whether
# anything is correct, so a raw call in one is not a wrong answer waiting to
# happen — and listing them would mean allowlist entries for files that are not
# hazards. This allowlist is only worth reading because every line in it had to
# be argued for; entries that excuse non-problems are what stops that being true.
# Crate/src/tests globbed at all three depths a crate is reachable at, because
# this tree has never had a single uniform one (see common.sh's crate_dir(),
# which spans the same range):
#
#   flat   `<root>/<crate>/…`                — an ungrouped root
#   one    `<root>/<family>/<crate>/…`       — a family-grouped root
#   two    `<root>/<group>/<name>/<crate>/…` — a doubly-grouped root
#
# A root that has no crates at a given depth contributes zero matches from that
# set under nullglob, not a phantom literal path.
#
# The middle depth was missing until it was measured: one grouped root used to
# sit nested one level deeper under another root, which the two-deep set
# matched, and when it was promoted to a repository root of its own it lost
# exactly one level and stopped matching anything at all. Every one of its
# sources went unread while the two surviving sets kept the aggregate
# non-empty, so neither the total-emptiness check below nor the deeper-than-
# reach net after it could notice — hence the per-root assertion rather than
# one across all roots.
shopt -s nullglob
SRC=()
for _root in "${CRATE_ROOTS[@]}"; do
  _root_src=( "$_root"/*/src/*.rs "$_root"/*/src/*/*.rs
              "$_root"/*/tests/*.rs "$_root"/*/tests/*/*.rs
              "$_root"/*/*/src/*.rs "$_root"/*/*/src/*/*.rs
              "$_root"/*/*/tests/*.rs "$_root"/*/*/tests/*/*.rs
              "$_root"/*/*/*/src/*.rs "$_root"/*/*/*/src/*/*.rs
              "$_root"/*/*/*/tests/*.rs "$_root"/*/*/*/tests/*/*.rs )
  [ ${#_root_src[@]} -gt 0 ] || fail "no source matched any scan glob under ${_root#"$REPO"/}, so every crate in that root is unread while the other roots keep this gate looking busy — the tree moved to a depth the globs do not cover"
  SRC+=( "${_root_src[@]}" )
done
shopt -u nullglob
[ ${#SRC[@]} -gt 0 ] || fail "the scan globs match no sources under any of ${CRATE_ROOTS[*]#"$REPO"/} at */{src,tests}, so both detectors would report clean having read nothing"

# A file deeper than those globs reach would be skipped in silence, which is
# the one failure a grep gate cannot notice on its own. Make it loud instead.
# Same flat-plus-nested pairing as the SRC globs above: the base paths this
# walks from have to reach a nested crate's src/tests too, or a file sitting
# there is invisible to the loud check exactly the way it was invisible to
# the silent scan — the one shape this safety net exists to catch.
#
# A hyphen-prefixed path segment is excluded from the result, not from the
# search: rulebook.md's own storage-tier convention makes `-*` disposable,
# gitignored scratch (build output, caches, temp state) at any directory
# depth, and a trybuild/tarpaulin scratch tree left behind by a normal
# `cargo test` run (e.g. `ring/bench_harness/gate/-target_gate/tests/...`)
# reaches the same nested depth as a real workstream-numbered crate by
# accident, not by being one. Filtering the convention this gate already
# assumes elsewhere is narrower than filtering by name: a differently-named
# scratch dir is still caught as long as it keeps the `-` prefix, and a real
# crate is never accidentally excused, since no permanent crate is allowed
# to live under a hyphen-prefixed directory in the first place.
mapfile -t unscanned < <(
  for _root in "${CRATE_ROOTS[@]}"; do
    find "$_root"/*/src "$_root"/*/tests -mindepth 3 -name '*.rs' 2>/dev/null
    find "$_root"/*/*/src "$_root"/*/*/tests -mindepth 3 -name '*.rs' 2>/dev/null
    find "$_root"/*/*/*/src "$_root"/*/*/*/tests -mindepth 3 -name '*.rs' 2>/dev/null
  done | sed "s|^$REPO/||" | grep -vE '(^|/)-'
)
if [ ${#unscanned[@]} -gt 0 ]; then
  fail "${#unscanned[@]} source file(s) sit deeper than this gate's scan globs reach and were never read: ${unscanned[*]} — add a level to SRC rather than let a clean run mean less than it says"
fi

# --- detector A: the platform's libm, called directly ------------------------
#
# Every f64 transcendental except sqrt. Kept on one line so the pattern in
# rulebook.md § Determinism : Pinned Transcendentals and the pattern here are
# comparable by eye.
FNS='sin|cos|tan|asin|acos|atan|atan2|sinh|cosh|tanh|asinh|acosh|atanh'
FNS="$FNS"'|exp|exp2|exp_m1|ln|ln_1p|log2|log10|powf|powi|cbrt|hypot|mul_add'
FNS="$FNS"'|sin_cos'

# `log` is deliberately absent from FNS and matched separately below. It is the
# same class of error as the `sin_cos` note in the rulebook, arriving from the
# other side: there the regex was too tight to see a real call, here a bare
# `log` alternative is loose enough to see calls that are not libm at all.
# `f64::log` takes the base as an argument — `x.log( 10.0 )` — so a bare
# `.log()` can never be it, while a domain method of that name is matched every
# time. One such method exists inside the tree this gate scans (`ring_flush`'s
# `log()` accessor, `ring_flush/src/lib.rs`), and its own tests call it 26
# times. Requiring the argument is what separates the two spellings.
#
# A sweep of all 28 names found `log` to be the only collision. `log2`/`log10`
# take no argument and so cannot be disambiguated this way, but neither has a
# domain namesake to disambiguate from.
LOG_WITH_BASE='\.log\([[:space:]]*[^)[:space:]]'

# UFCS/free-function spelling: `f64::sin( x )` reaches the same libm as
# `x.sin()` but carries none of the receiver-dot shape `\.($FNS)\(` anchors
# on, so it was invisible to this detector until now — the same class of gap
# `sin_cos` closed above, arriving from the opposite direction (a real call
# shape the pattern never covered, not a loose pattern matching too much). A
# repository-wide sweep (every root, every Cargo.toml) found zero
# existing calls of this form and zero `libm`/`num-traits` dependency
# anywhere, so this closes a blind spot rather than a live leak. Unlike the
# bare `.log(` collision above, `f32::`/`f64::`/`libm::` as a path qualifier
# has no domain namesake in this repository to disambiguate from, so `log`
# needs no argument-count check on this side.
FREEFN_QUAL='f32|f64|libm'

# Hits as `path:line:text`, doc comments excluded — a `///` line is prose about
# the code, not code, and rustdoc examples are compiled and run by G2 rather
# than shipped into the simulation.
mapfile -t hits_a < <(
  grep -nHE "\.($FNS)\(|$LOG_WITH_BASE|\b($FREEFN_QUAL)::($FNS)\(" "${SRC[@]}" 2>/dev/null \
    | grep -vE '^[^:]+:[0-9]+:\s*(///|//!|//)' \
    | strip_crate_root
)

# --- detector B: the platform's libm, reached through the project's math library
#
# Names distinctive enough to match bare. `rot` is not — bare, it would hit any
# local of that name — so it has to carry its module qualifier, which is how it
# is spelled at a call site anyway.
CGFNS='from_angle_x|from_angle_y|from_angle_z|from_axis_angle|from_euler_xyz'
CGFNS="$CGFNS"'|to_euler_xyz|from_spherical|to_spherical|from_scale_rotation_translation'
CGFNS="$CGFNS"'|rot_around_point|perspective_rh|perspective_rh_gl|slerp'
CGROT='(mat2x2|mat2x2h|mat3x3h)::rot'

mapfile -t hits_b < <(
  grep -nHE "\b($CGFNS)\b|$CGROT" "${SRC[@]}" 2>/dev/null \
    | grep -vE '^[^:]+:[0-9]+:\s*(///|//!|//)' \
    | strip_crate_root
)

# Both detectors classify against the same allowlist: a file excused from
# calling libm directly is excused from reaching it through the math library
# too. One concept, one declaration, rather than two lists free to disagree.
violations_a=() violations_b=() covered=()
for tagged in "${hits_a[@]/#/A:}" "${hits_b[@]/#/B:}"; do
  h="${tagged#?:}"
  [ -n "$h" ] || continue
  file="${h%%:*}"
  if [ ${#allowed[@]} -gt 0 ] && printf '%s\n' "${allowed[@]}" | grep -qxF -- "$file"; then
    covered+=( "$file" )
  elif [ "${tagged:0:1}" = A ]; then
    violations_a+=( "$h" )
  else
    violations_b+=( "$h" )
  fi
done

# Non-vacuity, detector A: every allowlisted file must still contain a match,
# or the exemption is stale and the detector is unproven.
silent=()
for a in "${allowed[@]}"; do
  printf '%s\n' "${covered[@]}" | grep -qxF -- "$a" || silent+=( "$a" )
done

if [ ${#silent[@]} -gt 0 ]; then
  fail "${#silent[@]} allowlisted file(s) no longer reach libm by either route, so this gate proved nothing: ${silent[*]} — either the control arm was removed (a regression in the guard) or the entry is stale and should be deleted"
fi

# Non-vacuity, detector B: the math library resolved from the manifest that
# declares it rather than a path written here, so a moved checkout follows
# instead of the control quietly going stale.
manifest="$( crate_dir pinned_math_facade )/Cargo.toml"
[ -f "$manifest" ] || fail "no manifest at ${manifest#"$REPO"/} to resolve the math library from, so detector B cannot check its own premise"
cg_rel="$( sed -nE 's/^[[:space:]]*math_backend[[:space:]]*=.*path[[:space:]]*=[[:space:]]*"([^"]+)".*/\1/p' "$manifest" | head -1 )"
[ -n "$cg_rel" ] || fail "${manifest#"$REPO"/} declares no path dependency on the math library, so detector B has nothing to check its premise against"
CG="$( cd "$( dirname "$manifest" )/$cg_rel" 2>/dev/null && pwd )"
[ -n "${CG:-}" ] && [ -d "$CG/src" ] || fail "the math library path '$cg_rel' declared by ${manifest#"$REPO"/} does not resolve to a source tree, so detector B's premise is unchecked"

# Each name must still be a `pub fn` in the math library, in a file that still reaches libm.
# `pinned.rs` is excluded from the search deliberately: it defines the pinned
# replacements, so a name found only there is a name that has been fixed.
cg_syms=( ${CGFNS//|/ } rot )
gone=() now_pinned=()
for sym in "${cg_syms[@]}"; do
  mapfile -t defs < <( grep -rlE "pub fn $sym\b" "$CG/src" 2>/dev/null | grep -v '/pinned\.rs$' )
  if [ ${#defs[@]} -eq 0 ]; then gone+=( "$sym" ); continue; fi
  reaches=0
  for d in "${defs[@]}"; do
    if grep -qE "\.($FNS)\(" "$d"; then reaches=1; break; fi
  done
  [ $reaches -eq 1 ] || now_pinned+=( "$sym" )
done

if [ ${#gone[@]} -gt 0 ]; then
  fail "${#gone[@]} name(s) in detector B's pattern no longer exist in the math library: ${gone[*]} — the pattern is stale and now guards nothing; re-derive it from that library's current API"
fi

if [ ${#now_pinned[@]} -gt 0 ]; then
  fail "${#now_pinned[@]} math-library helper(s) no longer reach libm: ${now_pinned[*]} — if the library pinned them, detector B is guarding a hazard that no longer exists, and this gate and the Determinism : Pinned Transcendentals rule both want rewriting rather than re-running"
fi

# --- the recipe and the gate must stay the same instrument -------------------
#
# rulebook.md § Determinism : Pinned Transcendentals prints both of these lists
# as the hand triage a reader runs on a fresh hit. If the two drift, the hand
# answer and the gate's answer stop agreeing, and whoever triages by hand reads
# "clean" on a file this gate would fail — the executable-recipe staleness this
# repository has already been bitten by, and which G11 guards the same way for
# its own pattern.
#
# This is not hypothetical for G10 specifically. `sin_cos` was missing from the
# gate and from the rule's recipe simultaneously, because both were written by
# listing function names and neither was re-read against how those names are
# spelled at a call site. Two copies of one list drift silently; two copies
# checked against each other cannot.
rule="$REPO/rulebook.md"
[ -f "$rule" ] || fail "no rulebook.md at repository root — the rule this gate automates is missing"

if ! grep -qF -- "$FNS" "$rule"; then
  fail "detector A's pattern no longer appears verbatim in rulebook.md's Determinism : Pinned Transcendentals recipe, so a hand triage and a gate run can now disagree — re-sync the recipe there with the FNS pattern in this file"
fi

undocumented=()
for sym in "${cg_syms[@]}"; do
  grep -qF -- "$sym" "$rule" || undocumented+=( "$sym" )
done
if [ ${#undocumented[@]} -gt 0 ]; then
  fail "${#undocumented[@]} name(s) detector B rejects are named nowhere in rulebook.md: ${undocumented[*]} — a gate that fails a call site the rule never mentions is a gate nobody can act on; add them to the Applicability bullet or drop them from CGFNS"
fi

# --- verdict -----------------------------------------------------------------

if [ ${#violations_a[@]} -gt 0 ]; then
  printf 'NOT REACHED %s — bare libm transcendental on the deterministic path:\n' "$GATE"
  printf '  %s\n' "${violations_a[@]}"
  printf '  fix: call the pinned kernel instead — `pinned::sin( x )`, never `x.sin()`\n'
  printf '  see: rulebook.md \xc2\xa7 Determinism : Pinned Transcendentals\n'
  exit 1
fi

if [ ${#violations_b[@]} -gt 0 ]; then
  printf 'NOT REACHED %s — math-library helper that is unpinned inside, on the deterministic path:\n' "$GATE"
  printf '  %s\n' "${violations_b[@]}"
  printf '  fix: assemble it from the pinned kernel instead — `pinned::sin_cos( a )`, then build the matrix by hand\n'
  printf '  see: rulebook.md \xc2\xa7 Determinism : Pinned Transcendentals\n'
  exit 1
fi

pass "no unpinned transcendental by either route across ${#SRC[@]} sources, outside the ${#allowed[@]} declared call site(s) each still exercising the pattern; detector B's ${#cg_syms[@]} math-library names all still resolve to unpinned helpers"
