#!/usr/bin/env bash
# G9 — clippy reports nothing across the family, over every target.
#
# Added from a finding rather than from a wish. The eight gates before this
# one all reported REACHED while `cargo clippy -D warnings` failed on three
# crates with twelve findings, and none of the eight could have seen it: G2
# grades whether documentation exists, G6 grades one specific lint
# (`unsafe_code`) by grepping for its opt-out, and no gate had ever invoked the
# lint driver at all.
#
# One of those twelve was not a style nit. A classifier function guarded with
# `!( e >= 0.0 )`, which rejects NaN by relying on comparisons against NaN
# being false — correct for NaN, and it admits `+INF`, which then cleared the
# parabolic band by an infinite margin and returned `Ok( Hyperbolic )` with
# `is_propagable` true, while both the validator and the hyperbolic solver
# refused the same input. That is a classifier naming a branch whose solver will
# not run: precisely the disagreement this gate exists to eliminate, alive
# inside the crate whose job is to prevent it, past a full review pass.
# `clippy::neg_cmp_op_on_partial_ord` flagged the idiom for readability and was
# pointing at the defect.
#
# Non-vacuity, in two parts, because a lint gate has two distinct ways to pass
# without checking anything:
#
#   * A crate clippy never examined contributes no findings, and a conjunction
#     over crates that were skipped is a conjunction over an empty set. So this
#     counts the compiler artifacts clippy actually emitted and requires one for
#     every crate in scope. A cached run still emits them (`"fresh": true`), so
#     this survives a warm target directory without weakening.
#
#   * `--all-targets` is what makes the gate reach tests, benches and binaries.
#     Ten of the twelve findings that motivated this gate were in test or demo
#     targets, not in library code, so a run that quietly graded libraries only
#     would have reported REACHED over the defect that prompted it. Requiring at
#     least one non-lib target proves the flag is doing its job rather than
#     trusting that it was typed.
#
# A third failure mode: the two checks above answer the
# wrong question when cargo did not run at all. Both are phrased as scope gaps
# — "clippy examined no target for N/M crate(s)" — and a build that fails before
# emitting any artifact satisfies that description exactly, so an I/O error, an
# unresolvable dependency or a poisoned target directory all reported as though
# the gate had merely graded a narrow slice. The exit code said otherwise and
# nothing read it; stderr said why and it went to /dev/null.
#
# Measured, not argued: with CARGO_TARGET_DIR pointed inside a read-only
# directory over two real members, cargo exits 101, writes `error: Permission
# denied (os error 13)` to stderr, and emits zero JSON lines — so zero
# error-level messages, an empty findings count, and every crate unexamined.
# The gate printed its scope-gap message about 2/2 crates. The same two crates
# with a writable target directory reported REACHED, which is what attributes
# the red arm to the poison rather than to the crates or to the gate at large.
#
# The check below can only turn a REACHED into a NOT REACHED — it adds a failure
# path and removes none — so it cannot manufacture a pass, only withdraw one
# that was never earned.
#
# A fourth failure mode, and the one that made the third's message unreachable:
# this gate ran `cd "$REPO" && cargo clippy -p <every family crate>` whatever
# workspace actually owned those crates. After a relocation to per-family
# workspace roots, only one of one family's twenty declared crates remained a
# member of the root workspace, and twelve of another family's twenty-seven.
#
# Cargo still ran, which is why this survived. `-p` selects from the resolve
# graph, not from the member list, and the first family's nineteen
# non-members were reachable as path dependencies of its own demo binary,
# which is a member — so `cargo metadata --no-deps` naming one crate and `-p`
# accepting twenty are both true at once, and non-membership never produced
# the clean refusal that would have been obvious. (That refusal does exist:
# selecting only non-members gets `error: cannot specify features for
# packages outside of workspace`. One member in the selection is enough to
# suppress it.)
#
# What cargo does not do is resolve a non-member's dev-dependencies. Under
# --all-targets every test target of those nineteen crates then failed on its
# own imports — measured: eight E0433 and six E0432, with one crate's test
# unable to see a sibling crate its own manifest declares. Those arrive as
# error-level compiler messages, so they were counted as clippy findings, and
# the third-failure-mode guard — which speaks only when the findings count is zero — was never
# reached. G9 reported "14 clippy finding(s)" about crates whose lints it had
# never run, permanently, for both families that declare it.
#
# Two defects, so two fixes. The run is fanned out one invocation per owning
# workspace, so every crate is compiled by the workspace that declares it and
# its dev-deps resolve. And the findings count now separates rustc's own error
# codes from lint names, so a build that did not compile is named a build
# failure whether or not it emitted an error-level message on the way down.
#
# A fifth failure mode: the two checks above between them assume every
# error-level message carries a code, either an `E`-prefixed one from rustc or
# a lint name otherwise, and read anything that is not the former as the
# latter. A message can carry neither — `compile_error!` and the diagnostics a
# proc-macro raises through `syn::Error` surface as `level: "error"` with
# `.message.code` absent, not present-and-empty, and this family declares
# proc-macro targets (its own demo binary is one, per G9's own kind list). The
# `// "unnamed"` fallback below was catching those and counting them into the
# clippy-findings branch — a hard compile failure with no code at all read out
# as "N clippy finding(s)", the same misattribution the fourth failure mode
# named, for the one shape its `^E[0-9]{4}$` check does not match. Never a
# status change either way: an "unnamed" entry was already a non-empty line in
# `codes`, so it was already failing the gate through the second check below
# before this fix — moving it into the first, correctly-worded branch changes
# which message a family sees, never whether it was REACHED. Not exploited
# today: measured by re-running this gate immediately after the fix over the
# one family whose kind list already proves it carries a proc-macro target —
# it reports REACHED, so no crate anywhere in it currently emits a codeless
# error.
set -uo pipefail
GATE=G9
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist

command -v jq >/dev/null || fail "jq is not on PATH — this gate parses cargo's JSON message stream"

mapfile -t crates < <( family_crates )

work="$( mktemp -d )"
trap 'rm -rf "$work"' EXIT

# stderr goes to a file rather than to /dev/null, and the exit code is kept.
# Both were discarded until this fix; see the third failure mode above.
#
# One invocation per owning workspace, per the fourth failure mode. The `-p`
# list is built by cargo_over_workspaces() from family_workspace_groups()
# rather than here, because which crates travel together is now a property of
# which workspace declares them. Two destinations, not one: stdout is the JSON
# message stream every `jq` below parses, and cargo's own prose — the
# per-workspace labels, `error: could not compile …` — has to stay out of it.
#
# Empty third argument: this gate sets no RUSTFLAGS. `-- -D warnings` already
# denies at the clippy-driver level, and adding a RUSTFLAGS the gate never had
# would change what gets rebuilt and what it reports.
cargo_over_workspaces "$work/clippy.json" "$work/clippy_stderr.log" "" \
  clippy --all-targets --all-features --message-format=json -- -D warnings
cargo_rc=$?
stream="$( cat "$work/clippy.json" )"

# Which crates clippy actually looked at, and with which target kinds.
mapfile -t examined < <(
  jq -r 'select( .reason == "compiler-artifact" ) | .target.name' <<<"$stream" 2>/dev/null | sort -u
)
kinds="$( jq -r 'select( .reason == "compiler-artifact" ) | .target.kind[]' <<<"$stream" 2>/dev/null | sort -u )"

# Findings first, non-vacuity second — an ordering this gate's own baseline run
# corrected. Checked the other way round, a single lint error in one crate
# aborts the build, the crates queued behind it never emit an artifact, and the
# gate reports "clippy examined no target for 9/20 crates" about what is
# actually one readable lint error. The vacuity checks exist to stop a false
# REACHED, so they are only load-bearing once the findings count is zero; asking
# them first buries the answer in a diagnostic about build topology.
#
# Within the findings region, rustc's own error codes are separated from lint
# names, per the fourth failure mode. Clippy's lints carry `clippy::<name>`,
# rustc's lints carry a bare name, and rustc's build errors carry an `E<dddd>`
# — so an E-code in this stream is a crate that did not compile, never a lint
# that was denied. Counting the two together let fourteen unresolved imports be
# reported as fourteen clippy findings. Nor does a lint denial cascade into
# E-codes downstream: when one crate fails, cargo never schedules its
# dependents, so they emit nothing at all rather than errors of their own.
codes="$( jq -r 'select( .reason == "compiler-message" and .message.level == "error" ) | .message.code.code // "unnamed"' <<<"$stream" 2>/dev/null )"
findings="$( grep . <<<"$codes" | sort | uniq -c | sort -rn )"
n_findings="$( grep -c . <<<"$codes" )"
n_rustc="$( grep -cE '^(E[0-9]{4}|unnamed)$' <<<"$codes" )"

if [ "${n_rustc:-0}" -gt 0 ]; then
  cat "$work/clippy_stderr.log"
  fail "${n_rustc} of ${n_findings} error-level message(s) carry a rustc error code or no code at all (never a lint, which always carries one), so this is a build failure and not a lint result — whatever did not compile was never linted at all: $( tr '\n' ';' <<<"$findings" | sed 's/  */ /g' ) (cargo's own stderr above)"
fi

if [ "${n_findings:-0}" -gt 0 ]; then
  fail "${n_findings} clippy finding(s) across ${#crates[@]} crate(s) in scope: $( tr '\n' ';' <<<"$findings" | sed 's/  */ /g' )"
fi

# The run itself succeeded. Checked after the findings count and before the two
# vacuity checks, for the same reason the findings count is checked first: a
# lint error also exits non-zero, and answering that with "cargo exited 101"
# instead of naming the lint codes would undo the ordering the block above
# fixed. Once the findings count is zero, a non-zero exit is a failure clippy
# never expressed as a finding, and the two checks below would read it as a
# scope gap.
if [ "$cargo_rc" -ne 0 ]; then
  cat "$work/clippy_stderr.log"
  fail "cargo exited ${cargo_rc} with no clippy finding to explain it — the run failed before it could grade anything, so this is a build failure and not a lint result (cargo's own stderr above)"
fi

# Part one: every crate in scope was examined. Cargo reports a target's name
# with hyphens normalised to underscores, which is how the crates are named
# here anyway; compare on that form so a future hyphenated member does not
# silently read as unexamined.
unexamined=()
for c in "${crates[@]}"; do
  printf '%s\n' "${examined[@]}" | grep -qx "${c//-/_}" || unexamined+=( "$c" )
done
[ ${#unexamined[@]} -eq 0 ] \
  || fail "no findings, but clippy examined no target for ${#unexamined[@]}/${#crates[@]} crate(s) in scope, so a clean result says nothing about them: ${unexamined[*]}"

# Part two: the run reached test targets specifically.
#
# Named as `test` rather than as "something other than lib", which is what this
# check said first and what its own baseline run disproved: dropping
# --all-targets still leaves `bin` behind, so an any-non-lib test reported
# REACHED over a run that had stopped examining every test target in the family.
# `test` is the kind --all-targets uniquely adds here, and the kind 10 of the 12
# findings that motivated this gate were in, so it is the kind worth asserting.
grep -qx 'test' <<<"$kinds" \
  || fail "no findings, but clippy examined no test target ( kinds seen: $( tr '\n' ' ' <<<"$kinds" )) — a clean result here says nothing about test code, where 10 of the 12 findings that motivated this gate lived"

pass "clippy clean over ${#crates[@]} crate(s), $( wc -l <<<"$kinds" ) target kind(s) ($( tr '\n' ' ' <<<"$kinds" ))"
