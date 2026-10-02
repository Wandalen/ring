#!/usr/bin/env bash
# G12. Every recorded historical defect, reinstated in place, still turns the
# suite red.
#
# An earlier plan's own Next Constraint asked for this gate. That plan closed
# with every feature implemented, tested, documented and manually exercised, and
# named its binding constraint as the thing G1 cannot see. Coverage grades which
# lines ran, never which values were checked. `ring_bench`'s own history is the
# measured instance. It had 100% line coverage over 298 lines and zero defence
# against the crate's own headline defect, because both tests that executed the
# mutated lines asserted on something else. So this gate does not read the
# suite. It breaks the crate and watches.
#
# Non-vacuity, in four parts, because a mutation probe has more ways to pass
# without measuring anything than any other gate here:
#
#   1. The mutant set must be non-empty. A probe over zero defects is a
#      conjunction over an empty set, the vacuous REACHED an earlier review
#      measured, wearing a new costume.
#   2. Every mutant's `from` block must appear EXACTLY once in its target.
#      Zero means the declaration went stale when the code moved, and a
#      mutation that edits nothing leaves the suite green for the most
#      misleading possible reason. More than one means the probe would hit a
#      site nobody recorded, so its verdict would be about a different defect.
#   3. The suite must be GREEN before each mutation. "Red under mutation" says
#      nothing about the mutation if the suite was already red, and that is the
#      failure mode that turns this gate into a rubber stamp for a broken crate.
#   4. Every target file must be byte-identical to its original afterwards.
#      This is the only gate here that writes to `src/`, and a probe that leaves
#      a mutation behind is worse than no probe at all.
#
# What it deliberately does not assert is the NUMBER of failing tests. Each
# mutant records the count its manual round observed and both are printed, so
# erosion stays visible. But a floor would fail on a legitimate test
# consolidation, and the claim worth defending is binary: reinstate the defect,
# the suite notices. B2 and F1 each failed by more than their round predicted,
# the kind of drift a floor would mistake for a defect.
set -uo pipefail
GATE=G12
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist

decl="$DECL/mutant"
[ -d "$decl" ] || fail "no mutant declaration at ${decl#"$REPO"/} — nothing to reinstate"

mapfile -t mutants < <( find "$decl" -maxdepth 1 -name '*.mutant' | sort )
[ ${#mutants[@]} -gt 0 ] || fail "no mutants declared in ${decl#"$REPO"/} — a probe over zero defects grades nothing"

work="$( mktemp -d )" || fail "cannot create a work directory to hold originals in"

# The tree must come back whatever happens next: a failed assertion, a Ctrl-C,
# or a killed test run. `saved.path` exists only while a file is mutated.
restore_now()
{
  local target
  [ -f "$work/saved.path" ] || return 0
  target="$( cat "$work/saved.path" )"
  [ -f "$work/saved" ] && cp -- "$work/saved" "$target"
  rm -f -- "$work/saved.path"
}
# Fix(g12_combined_signal_trap_never_exits)
# Root cause: a single `trap ... EXIT INT TERM HUP` line runs the SAME handler
#   body for every signal in the list, and that body never calls `exit`. Bash
#   does not terminate a script just because a trapped signal arrived and its
#   handler returned. The handler is a callback, not a substitute for the
#   signal's default disposition. So a real Ctrl-C (SIGINT to the whole
#   foreground process group, mid `run_suite`) fired this trap exactly once,
#   correctly restoring the mutated file via `restore_now`. But then execution
#   CONTINUED past line 177's `if run_suite "$crate"` into the `else` branch
#   with `$work` already removed. `failures_in` read a nonexistent `$work/out`
#   and returned empty, and line 201's `[ -n "$fails" ] || fail ...` fired a
#   factually wrong "no longer compiles once mutated" diagnostic for what was an
#   operator-requested interrupt. Verified empirically with a process-group
#   SIGINT (`kill -INT -$PGID`, not a single-PID `kill`, which does not
#   reproduce real terminal Ctrl-C). A combined trap without `exit` lets the
#   script run on. A separate `trap 'exit 1' INT TERM HUP` terminates it
#   immediately, and the EXIT trap alone still fires the cleanup exactly once as
#   a consequence of that `exit` call.
# Pitfall: do not re-merge these two traps into one line. Bash trap registration
#   is process-global and non-stacking (a later `trap ... SIG` replaces, never
#   adds to, an earlier one for the same SIG), so the split only works because
#   EXIT and INT/TERM/HUP are registered as two distinct trap calls, not one
#   list.
trap 'restore_now; rm -rf -- "$work"' EXIT
trap 'exit 1' INT TERM HUP

# One `key: value` line out of a mutant declaration.
#
# Finding(meta_value_keeps_a_trailing_cr_on_crlf_input): the `sed` strip only
# removes leading whitespace after the colon. A `.mutant` file saved with CRLF
# line endings would leave the returned value carrying a trailing `\r` (e.g.
# `crate` becoming `"ring_bench\r"`). That would then fail `crate_dir "$crate"`
# and `family_members | grep -qx "$crate"` below, the same way a CRLF
# `crates.txt` fails common.sh's `decl_lines` consumers. See common.sh's own
# `Finding(decl_lines_keeps_a_trailing_cr_on_crlf_input)` for the fuller
# consumer-chain explanation. This is an independent instance of the same gap
# class in a textually separate helper. No live trigger: every `*.mutant` file
# under `declared/*/mutant/` is confirmed LF-only.
meta()
{
  grep -m1 -E "^$2:[[:space:]]*" "$1" 2>/dev/null | sed -E "s/^$2:[[:space:]]*//"
}

# The verbatim lines between two `--- name` markers.
#
# Fix(g12_block_markers_matched_by_prefix_not_equality): the boundary checks
# used to be `index( $0, s ) == 1` / `index( $0, e ) == 1`. Those are true
# whenever a line STARTS WITH the marker, not only when it IS the marker. A
# `from`/`to` snippet whose own content happened to start with `--- from`,
# `--- to`, or `--- end` would end its block early on a line that was never
# meant as a boundary. That is plausible for a Rust comment using a
# horizontal-rule-style separator, e.g. `// --- to do this differently`.
# Root cause: same bug class as `corpus_lib.py`'s
# `a_deeper_heading_is_not_a_boundary` and `citations.py`'s
# `citations_tests_open_is_a_prefix_not_a_heading`. A prefix test answers
# "is this run of characters present," not "is this the one line meant to open
# or close the section." Pitfall: zero live trigger, measured directly, not
# assumed. Old and new extraction produce byte-identical `from`/`to` text for
# every one of the 17 declared `.mutant` files (51 marker lines total; every
# line matching `^--- ` today is exactly one of the three markers with nothing
# else on the line). One side effect: the prefix check happened to also match a
# marker line saved with a trailing CRLF `\r` (the `\r` trails the match, not
# the marker). So it was accidentally tolerant of the same hazard `meta()`'s own
# disclosed finding above names; exact match is not. Still no live trigger, for
# the same reason that finding gives. Every `.mutant` file here is confirmed
# LF-only.
block()
{
  awk -v s="--- $2" -v e="--- $3" '
    $0 == s { f = 1; next }
    $0 == e { f = 0 }
    f
  ' "$1"
}

# How many times a literal block occurs in a file. Slurped whole, so a block
# spanning several lines is matched as one unit rather than line by line.
occurrences()
{
  FROM="$2" perl -0777 -ne 'BEGIN { $f = $ENV{ FROM } } my $c = () = /\Q$f\E/g; print $c' "$1"
}

# Every declaration is parsed and checked against the tree before a single test
# runs. A stale mutant is cheap to detect and expensive to detect late.
ids=() crates=() files=() froms=() tos=() recorded=() detectors=() rounds=()
probe_crates=()
for m in "${mutants[@]}"; do
  rel="${m#"$decl"/}"
  id="$( meta "$m" id )"
  crate="$( meta "$m" crate )"
  file="$( meta "$m" file )"
  round="$( meta "$m" round )"
  rec="$( meta "$m" round-failures )"
  det="$( meta "$m" detector )"

  [ -n "$id" ]    || fail "$rel declares no id"
  [ -n "$crate" ] || fail "$rel ($id) declares no crate"
  [ -n "$file" ]  || fail "$rel ($id) declares no file"
  [ -n "$round" ] || fail "$rel ($id) carries no round date — an undated probe is a claim, not a record, which is the same bar G4 holds manual runs to"
  [ -n "$rec" ]   || fail "$rel ($id) records no observed failure count — the round that found it is what makes it a defect rather than a guess"
  [ -n "$det" ]   || fail "$rel ($id) names no detector test"

  target="$( crate_dir "$crate" )/$file"
  [ -f "$target" ] || fail "$id targets ${target#"$REPO"/}, which is not in the tree"

  # Membership matters: a mutant against a crate outside the family would be
  # graded by whichever family happened to declare it.
  family_members | grep -qx "$crate" || fail "$id targets '$crate', which family '$GATE_FAMILY' does not declare"

  from="$( block "$m" from to )"
  to="$( block "$m" to end )"
  [ -n "$from" ] || fail "$id declares an empty 'from' block"
  [ -n "$to" ]   || fail "$id declares an empty 'to' block"
  [ "$from" != "$to" ] || fail "$id declares a mutation that changes nothing"

  n="$( occurrences "$target" "$from" )"
  case "$n" in
    1 ) ;;
    0 ) fail "$id no longer matches ${target#"$REPO"/} — the declaration went stale when the code moved, and a mutation that edits nothing would leave the suite green" ;;
    * ) fail "$id matches ${target#"$REPO"/} $n times — the probe would hit a site the round never recorded" ;;
  esac

  ids+=( "$id" ); crates+=( "$crate" ); files+=( "$file" ); rounds+=( "$round" )
  froms+=( "$from" ); tos+=( "$to" ); recorded+=( "$rec" ); detectors+=( "$det" )
  printf '%s\n' "${probe_crates[@]:-}" | grep -qx "$crate" || probe_crates+=( "$crate" )
done

# `--no-fail-fast` is required, not a preference. nextest stops at the first
# failures by default, so the number of failures it reports under a mutation
# depends on scheduling and parallelism, not on the suite. F1's own round
# recorded "16/23 tests run: 12 passed, 4 failed", a run cut short, and a later
# probe of the same mutation on the same code reported 3. The whole point of
# printing the count is to make erosion visible, so it has to be the true count
# of failing tests, not however many happened to run first.
#
# Fix(g12_ran_cargo_from_repo_for_crates_repo_may_not_own)
# Root cause: this ran cargo from $REPO unconditionally, which assumes $REPO is
#   the workspace that declares the crate named by `-p`. This tree can hold more
#   than one workspace root, and a crate the families declare can be a member of
#   a sibling root instead of the root workspace. `-p` selects from the resolve
#   graph, not the member list, so a non-member reachable as a path dependency
#   is accepted without complaint and compiled with its dev-dependencies
#   unresolved. That is the shape that made g9 report fourteen rustc E-codes as
#   clippy findings, and made mutant_survey.sh survey nothing at exit 0.
#   Resolved here through common.sh's crate_workspace_root(), not a second
#   resolver of this file's own.
# Pitfall: was latent when written, now live, not hypothetical. Re-measured, not
#   assumed stale. `ring/Cargo.toml` has since gained its own `[workspace]`
#   table (`cargo locate-project --workspace` from inside any ring_* crate now
#   resolves to `ring/Cargo.toml`, not $REPO's). So the original claim here,
#   "all ten crates those mutants target are members of $REPO, so `cd "$REPO"`
#   is currently the right answer by coincidence", no longer holds.
#   $REPO/Cargo.toml does not list ring_spsc (or any ring_* crate) as a member
#   at all today. `cd "$REPO"; cargo nextest run -p ring_spsc` would now fail
#   outright instead of merely resolving the wrong package graph.
#   `crate_workspace_root()`'s dynamic nearest-`[workspace]`-ancestor walk
#   (correctly finding `ring/Cargo.toml`) prevents that today, and does not only
#   guard against it for some future substrate/ crate. The substrate/ case this
#   paragraph originally warned about remains real for the same reason, on top
#   of this one now being live.
run_suite()
{
  local ws
  ws="$( crate_workspace_root "$1" )" \
    || fail "cannot resolve an owning workspace for $1 — every ancestor up to ${REPO##*/} declares no [workspace], so there is no directory to run cargo from and a mutation verdict for this crate would be graded by a cargo invocation that refused"
  ( cd "$ws" && cargo nextest run -p "$1" --all-features --no-fail-fast ) >"$work/out" 2>&1
}

# Reads nextest's own summary rather than counting FAIL lines, so a run that
# died before reporting yields nothing instead of zero.
failures_in()
{
  grep -oE '[0-9]+ failed' "$work/out" | tail -1 | grep -oE '^[0-9]+'
}

# Non-vacuity 3. Established once per crate, before anything is mutated.
for c in "${probe_crates[@]}"; do
  run_suite "$c" && continue

  # Fix(g12_baseline_blamed_the_suite_for_a_build_that_never_ran)
  # Root cause: this check read run_suite's exit code and nothing else, so it
  #   made one claim, "already red", for the two entirely different conditions
  #   that produce a non-zero exit. `cargo nextest run` exits non-zero both when
  #   tests fail and when the crate's test targets do not compile, and a crate
  #   that never built has a suite with no colour at all. Reported as red, the
  #   crate is blamed for a failure that belongs to the build: a stale
  #   dev-dependency, a poisoned target directory, an unresolvable path dep, or
  #   the workspace-ownership defect the fix below this one addresses. The
  #   reasoning is not new here. The post-mutation branch already carries it as
  #   Fix(g12_compile_failure_counted_as_caught_mutant), and failures_in() was
  #   written to return empty in exactly this case, with a comment saying so.
  #   Both were fourteen lines away and neither reached this call site.
  # Pitfall: the old message rendered its own defect and it still went unread.
  #   `$( failures_in )` substituted empty into "(N failing)", so the line
  #   printed a literal "( failing)" with nothing in the slot. Measured on a
  #   crate whose test target fails to compile: exit 101, no "N failed" summary
  #   line anywhere in the output, failures_in empty, and the two call sites
  #   reaching opposite verdicts from that one byte-identical run. A diagnostic
  #   that cannot name a number for its own headline claim is reporting
  #   something other than what it says it is.
  fails="$( failures_in )"
  if [ -z "$fails" ]; then
    tail -20 "$work/out"
    fail "$c did not build before any mutation — nextest exited non-zero with no failure count to read, so its test suite never ran and 'already red' would be a claim about tests that do not exist yet (cargo's own last lines above)"
  fi
  fail "$c is already red before any mutation — 'red under mutation' would grade nothing (${fails} failing)"
done

green=0 blind=()
for i in "${!ids[@]}"; do
  id="${ids[$i]}"; crate="${crates[$i]}"; target="$( crate_dir "$crate" )/${files[$i]}"

  cp -- "$target" "$work/saved" || fail "$id — cannot save ${target#"$REPO"/} before mutating it"
  printf '%s\n' "$target" >"$work/saved.path"

  FROM="${froms[$i]}" TO="${tos[$i]}" \
    perl -0777 -i -pe 'BEGIN { $f = $ENV{ FROM }; $t = $ENV{ TO } } s/\Q$f\E/$t/' "$target" \
    || fail "$id — the mutation could not be applied to ${target#"$REPO"/}"

  if run_suite "$crate"; then
    blind+=( "$id (${detectors[$i]} recorded ${recorded[$i]} failure(s), now 0)" )
  else
    # Fix(g12_compile_failure_counted_as_caught_mutant)
    # Root cause: `cargo nextest run` exits non-zero both when tests fail AND
    #   when the mutated code fails to compile. run_suite's exit code alone
    #   cannot tell them apart, and this branch used to count either one as
    #   "the suite noticed." A mutant's `from` text can keep matching exactly
    #   once (satisfying the staleness check above) while code drift nearby
    #   changes a type or signature such that the SAME `to` substitution no
    #   longer compiles. When that happens the gate still prints "red" and still
    #   counts it toward the pass, but no test ever ran. The mutant then proves
    #   only that the compiler notices, not that the test suite catches the
    #   original defect. Confirmed empirically: a mutated sandbox crate that
    #   fails to compile exits 101 with no "N failed" summary line anywhere in
    #   its output. So failures_in (already written to return empty in exactly
    #   this case, per its own comment) was sitting unused for this check.
    # Pitfall: this is the same "declaration goes stale when the code moved"
    #   failure mode the from-text occurrence check above already guards
    #   against. It moves the drift from before the mutation (text no longer
    #   matches) to after it (text matches but no longer compiles once
    #   substituted), and nothing was watching that second window.
    fails="$( failures_in )"
    [ -n "$fails" ] \
      || fail "$id — ${target#"$REPO"/} no longer compiles once mutated (round ${rounds[$i]} recorded a real test failure: ${recorded[$i]}); this is drift in the surrounding code, not proof the suite still catches the original defect"
    green=$(( green + 1 ))
    echo "    $id — red, $fails failing (round ${rounds[$i]} recorded ${recorded[$i]})"
  fi

  restore_now
  # Non-vacuity 4. Byte-for-byte, not "looks restored".
  cmp -s "$work/saved" "$target" || fail "$id — ${target#"$REPO"/} did not come back byte-identical; the tree is mutated and must be checked by hand"
done

[ ${#blind[@]} -eq 0 ] \
  || fail "${#blind[@]}/${#ids[@]} reinstated defect(s) left the suite green: ${blind[*]}"

pass "${green}/${#ids[@]} recorded defect(s) reinstated, each turning its crate's suite red, each file restored byte-identically"
