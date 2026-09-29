#!/usr/bin/env bash
# G1 — 100% line coverage over the declared family, AND coverable lines > 0.
#
# The non-vacuity half is the point: tarpaulin reports "No coverable lines
# found" on the skeletons and would otherwise be indistinguishable from a
# fully-covered family.
set -uo pipefail
GATE=G1
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist

# `--all-features`, because tarpaulin counts `cfg`-removed lines as uncovered
# rather than omitting them. `ring_core`'s crossbeam backend is behind a cargo
# feature, so a featureless run leaves 26 of its lines in the denominator that
# the binary under test does not contain — measured at 93/119 (78.2%) with the
# feature off against 120/120 with it on, for the identical test suite. The
# featureless figure is a miscount, not a gap, and building every line is what
# makes the 100% threshold below mean what it says.
#
# Safe family-wide: of the ring_* crates, four declare a `crossbeam` feature
# (`ring_core`, `ring_bench`, `ring_factory`, `ring_handle`) and all four
# forward to the same underlying `ring_core/crossbeam` capability rather than
# gating independent ones — see docs/pitfall/002 in ring_core for the full
# account. `loom` is a RUSTFLAGS `cfg` rather than a feature, so `--all-features`
# still turns on exactly one thing.
# Cleared before every run, for the reason `mutant_survey.sh` records at its own
# `rm -rf` and does not need restating here: tarpaulin accumulates one `.profraw`
# per test binary and merges whatever is in this directory, so a run scoped to a
# different family — or one that died before its own merge — leaves profraws that
# the next run silently folds into its reading.
#
# What it looked like: a run reported `Failed to generate report:
# missing section: CoverageFunctions` after every one of its test binaries had
# run and produced a profraw, and the identical command succeeded twenty-eight
# seconds later once the directory had been reduced by an unrelated single-crate
# run. That failure is the benign shape — G1 catches it below and says the gate
# did not run. The shape to fear is a stale profraw that parses cleanly, which
# would move the percentage against a threshold of 100% with nothing to show for
# it.
#
# Same overlap hazard as G12: this must not run concurrently with another
# tarpaulin invocation over this workspace, in either direction.
# Which `target/` cargo will actually use, rather than assuming `$REPO/target`.
# This cleanup and the lcov output below both used to hardcode that path, and a
# run given an isolated `CARGO_TARGET_DIR` — the documented remedy for the
# concurrency hazard named just above, and how this gate is routinely invoked —
# then cleared a directory nothing had written to, leaving the stale-profraw
# hazard the paragraph above describes fully live while reporting it handled.
# Not a deduction: `-target_gate/tarpaulin/profraws` exists on disk from real
# gate runs, so tarpaulin does follow the override.
target_root_for() {
  if [ -n "${CARGO_TARGET_DIR:-}" ]; then printf '%s\n' "${CARGO_TARGET_DIR%/}"
  else printf '%s/target\n' "${1%/}"; fi
}

# Cleared immediately before each invocation rather than once up front, because
# there is now more than one invocation and each merges whatever it finds. That
# also closes a hazard the single-invocation form never had: under a shared
# `CARGO_TARGET_DIR` every workspace's profraws land in one directory, so
# without this the second workspace would fold in the first workspace's.
#
# The leading `/` in the pattern is load-bearing. `*/tarpaulin/profraws` also
# matches `/tarpaulin/profraws` with an empty `*`, so an empty base would pass
# the guard and `rm -rf` a top-level path; `/?*/` requires a non-empty one.
clear_profraws() {
  local d="$1/tarpaulin/profraws"
  case "$d" in
    /?*/tarpaulin/profraws ) rm -rf -- "$d" ;;
    * ) echo "refusing to clear an unexpected profraw path: $d" >&2; exit 2 ;;
  esac
}

# `Lcov` alongside `Stdout` for the `let … else` discount below, which needs
# per-line hit counts. The stdout report gives fractions and a list of uncovered
# lines; neither can distinguish "this line is instrumented and was never hit"
# from "this line was hit" for a line the report does not mention, and the
# discount turns on exactly that difference. `lcov.info`'s `DA:<line>,<hits>`
# records say it outright.
lcov_root="$( target_root_for "$REPO" )/tarpaulin/g1"
case "$lcov_root" in
  /?*/tarpaulin/g1 ) rm -rf -- "$lcov_root" ;;
  * ) echo "refusing to clear an unexpected lcov path: $lcov_root" >&2; exit 2 ;;
esac
mkdir -p "$lcov_root"

# One tarpaulin invocation per owning workspace, not one at `$REPO` for the lot.
# `-p` naming a package outside the invoking workspace is not a scoping request
# cargo honours: with `--all-features` it refuses the whole invocation outright
# — `error: cannot specify features for packages outside of workspace` — and
# that refusal is a property of the selection, not of each package, so one root
# member in the list is enough to mask it for the rest. The eleven families
# split across twelve workspaces, six of them naming no root member at all, and
# this gate graded every one of those six on a command that compiled nothing.
# See common.sh's family_workspace_groups() for the grouping.
#
# Worse here than in G2, and worth naming: tarpaulin swallows cargo's message
# and substitutes `Cargo failed to run! Error: cargo run failed`, so the line
# that says what went wrong is neither the last line nor recognisably an error.
# That is why the failure path prints the whole capture rather than a tail of
# it — the tail is exactly the line that carries no information.
#
# `--output-dir` takes one path, so each workspace writes its own lcov.info and
# they are concatenated afterwards. Concatenation IS the merge for this format:
# records are self-delimiting by `SF:` and the reader below re-anchors at each
# one, and no workspace can report a `SF:` another also reports, since a
# package belongs to exactly one.
out=""
while read -r ws pkgs; do
  [ -n "$ws" ] || continue
  pkg_args=()
  for pkg in $pkgs; do pkg_args+=( -p "$pkg" ); done
  rel="${ws#"$REPO"}"; rel="${rel#/}"; [ -n "$rel" ] || rel="<repo root>"
  slug="$( printf '%s' "$rel" | tr -c 'A-Za-z0-9_.-' '_' )"

  clear_profraws "$( target_root_for "$ws" )"
  ws_out="$( cd "$ws" && cargo tarpaulin "${pkg_args[@]}" --all-features --skip-clean \
    --out Stdout Lcov --output-dir "$lcov_root/$slug" 2>&1 )"

  # Did tarpaulin run at all? This is the one ambiguity an early reading could
  # not resolve: "zero coverable lines" is consistent both with looking and
  # finding nothing and with never looking. Two branches follow, and they are
  # not the same claim. Both ask per
  # invocation, because each answers only for its own packages — another
  # workspace's healthy report is no evidence this one ran.
  #
  # Branch one is tarpaulin's OWN message, so reaching it proves it executed.
  if grep -q 'No coverable lines found' <<<"$ws_out"; then
    fail "tarpaulin ran and reports no coverable lines for $rel — those crates are unimplemented"
  fi

  # Branch two below counts a regex match over tarpaulin's per-file breakdown,
  # and an empty count there could equally mean the regex broke. So before
  # trusting a zero, require positive evidence the tool produced a report: its
  # own coverage summary line. A run that emits neither that line nor the
  # message above did not execute, and that is a broken gate rather than an
  # unimplemented family.
  #
  # Two causes are known to reach here and neither is a coverage regression, so
  # rule both out before treating this as one. They are told apart by reading
  # the printed capture, which is why the whole of it goes to stdout:
  #
  #   * A `-p` package the invoking workspace does not contain — cargo refuses
  #     before compiling anything, in about a second. This is what the
  #     per-workspace loop above exists to prevent, so if it fires anyway the
  #     grouping is wrong; cargo's own `outside of workspace` line will be in
  #     the capture, second from the bottom.
  #   * Another cargo process building in the same target directory. Tarpaulin
  #     exits with `Failed to get test coverage! Error: missing section:
  #     CoverageFunctions`, having read a binary a concurrent build was
  #     mid-write on. Recorded because it happened: two runs eleven seconds
  #     apart over an unchanged tree gave `missing section: CoverageFunctions`
  #     and `100% over 2648 coverable lines`. The distinguishing variable was a
  #     second cargo, not the code. Re-running the gate alone settles this one
  #     — and only this one, which is why it is no longer offered as the remedy
  #     for whatever reaches this branch.
  #
  # Deliberately NOT retried automatically in either case: a gate that silently
  # retries until it likes the answer is how a real intermittent failure gets
  # buried, and both of these are loud, specific and cheap to reproduce by hand.
  if ! grep -qE '^[0-9]+\.[0-9]+% coverage,|\|\| Tested/Total Lines' <<<"$ws_out"; then
    printf '%s\n' "$ws_out"
    fail "tarpaulin produced no coverage report for $rel — the gate did not run (full output above)"
  fi

  out+="$ws_out"$'\n'
done < <( family_workspace_groups )

# The summary percentage tarpaulin prints is workspace-wide: `-p` selects which
# packages' tests to RUN, not which sources to report on, so unrelated crates
# sit in the denominator and hold the figure below
# 100% no matter how complete the family is. Recompute from the per-file
# "Tested/Total Lines" breakdown, restricted to the crates actually in scope —
# the whole family by default, one stage's crates under a stage run.
scoped="$( family_crates | paste -sd'|' )"

# Each surviving line becomes `<crate>/src/<file> <tested>/<total>`.
#
# No `$` anchor, deliberately. Tarpaulin appends a ` +0.00%` delta to every
# per-file line once a previous run exists to compare against, and omits it on
# the very first run in a fresh target dir. An anchored pattern therefore
# matched on the first run and matched nothing on every run after it — a gate
# that passed once and then reported "zero coverable lines" forever, which
# reads as an unimplemented family rather than as a broken gate. `-o` ends the
# match at the fraction, so the delta is discarded either way.
# A crate's path in tarpaulin's report is relative to the workspace the run was
# invoked in, not to `$REPO`, so its shape depends on which workspace owns the
# crate and the crate's own name is the only segment common to both. Measured
# from a crate one directory level below its own workspace root, `-p some_crate`:
#
#     || some_crate/src/lib.rs: 25/28
#
# where the same file reported from `$REPO` reads
# `some_root/some_crate/src/lib.rs`. This pattern used to require a fixed root
# prefix, mandatory — which matched nothing at all once the loop above began
# running each family in its own workspace. Every row was dropped, `per_file`
# came back empty, and the gate reported "zero coverable lines in the crates
# under test": an unimplemented family, in a message about a fully implemented
# one.
#
# So the prefix is optional and names no root, and the crate segment carries the
# restriction instead. Requiring that prefix to end in `/` is what keeps it from
# widening: a hypothetical `legacy_some_crate/src/lib.rs` cannot match, since
# `legacy_` is not a path segment and the alternation is not a substring search.
#
# Stripping it leaves `<crate>/src/<file>` whichever workspace reported the row,
# which is not cosmetic — three things downstream key on that exact shape and on
# both workspaces producing it identically: the max-per-path merge immediately
# below, the binary-entry-point drop, and the phantom discount's own `$rel`.
per_file="$( grep -oE "^\|\| ([^ ]*/)?(${scoped})/src/[^:]+: [0-9]+/[0-9]+" <<<"$out" \
  | sed -E "s#^\|\| ([^ ]*/)?((${scoped})/src/)#\2#; s/: / /" )"

# One row per source file, keeping the highest tested count.
#
# `$out` is now several reports concatenated, and a file can appear in more than
# one of them: tarpaulin instruments the whole path-dependency graph it compiles,
# not just the `-p` packages, so a family crate that another workspace's family
# crate depends on by path is reported by both. Observed in one family whose
# several members all depend by path on a shared crate that is itself a member
# of a different family's workspace, so that shared crate clears the family
# filter above from both reports.
#
# Summing both copies would inflate numerator and denominator together, which
# the 100% threshold mostly survives; the phantom discount below is what makes it
# dangerous. That discount is keyed on the file path, so it applies to every row
# bearing it — a genuine 9/10 counted twice becomes 10/10 twice and passes.
#
# The max is the merge rather than the first or the sum: two invocations run
# different test sets over the same file, and a line covered by either is
# covered. A no-op for a file reported once, which is still the ordinary case.
per_file="$( awk 'NF == 2 {
    split( $2, a, "/" )
    if ( !( $1 in tot ) ) { order[ ++n ] = $1; best[ $1 ] = a[ 1 ]; tot[ $1 ] = a[ 2 ] }
    else if ( a[ 1 ] > best[ $1 ] ) { best[ $1 ] = a[ 1 ] }
  }
  END { for ( i = 1; i <= n; i++ ) print order[ i ], best[ order[ i ] ] "/" tot[ order[ i ] ] }' \
  <<<"$per_file" )"

# ── Binary entry points, and why they are graded elsewhere ──────────────────
#
# Tarpaulin counts a binary's source in the denominator and never executes it.
# Measured directly: `--run-types Bins` and `--run-types AllTargets` both leave
# every binary in the workspace at zero, regardless of which binary or how many
# lines it has. This is a property of the tool, not of the tests: no test
# suite can raise those figures, so a 100% bar including them is unreachable by
# construction rather than demanding.
#
# The lines are dropped from THIS gate's denominator and picked up by two other
# checks, so the entry point stays graded:
#
#   * G7 executes the binary three times per run — debug twice and release once
#     — and byte-compares the output. A `main` that failed to parse its
#     arguments, failed to reach `main_from_args`, or returned the wrong exit
#     code produces no output or different output, and G7 fails.
#   * The shim assertion below bounds how much can live there at all.
#
# That second check is the non-vacuity pairing, and it is the reason this is not
# simply a hole. Excluding a file from coverage is an invitation to park logic
# in it; a bounded shim cannot hold logic. Without the bound, "the entry point is
# only a shim" would be a claim nobody re-checks after the first time it is true.
#
# Which files those are is asked of cargo, not assumed to be `src/main.rs`. Both
# halves above used to name that one path literally, so a `[[bin]]` declaring any
# other `path` was neither dropped nor bounded — it got the strictness of a
# library with none of the reachability. Measured directly: a `[[bin]]` target
# declaring a non-default `path` reported `0/10`, ten lines no test
# can reach, held against a threshold of 100%, and the message the gate printed
# asked for coverage that cannot be written. Its 21 non-comment lines were
# meanwhile outside the budget entirely, which is the same hole from the other
# side. See `family_bin_sources` in `common.sh`.
bin_list="$( family_bin_sources )" \
  || fail "cargo metadata produced no target list, so binary entry points cannot be told from library sources — the gate did not run"
bin_sources=()
[ -n "$bin_list" ] && mapfile -t bin_sources <<<"$bin_list"

SHIM_MAX_LINES=12
oversized=()
for rel in ${bin_sources[@]+"${bin_sources[@]}"}; do
  m="$( crate_dir "${rel%%/*}" )/${rel#*/}"
  [ -f "$m" ] || continue
  n="$( grep -cvE '^[[:space:]]*(//|$)' "$m" )"
  [ "$n" -le "$SHIM_MAX_LINES" ] || oversized+=( "$rel ($n lines, max $SHIM_MAX_LINES)" )
done
if [ ${#oversized[@]} -gt 0 ]; then
  fail "${#oversized[@]} binary entry point(s) are past the shim budget, so excluding them from coverage would hide real logic: ${oversized[*]}"
fi

per_file="$( awk 'NR == FNR { drop[ $0 ] = 1; next } !( $1 in drop )' \
  <( printf '%s\n' ${bin_sources[@]+"${bin_sources[@]}"} ) <( printf '%s\n' "$per_file" ) )"

# ── A bare `else` keyword line, and why it is not always a gap ─────────────
#
# Tarpaulin can report a bare `else` keyword line as instrumented-but-never-hit
# while the block that line guards is provably executed. Measured 2026-08-31,
# `lang_common/src/flow_record.rs` under its own test suite:
#
#     line 199  `let Some( colon ) = rest.find( ':' )`   417 hits
#     line 200  `else`                                     0 hits
#     line 202  `return Err( … )`, inside that block        5 hits
#
# Line 202 is reachable only by the pattern on 199 failing, which is the `else`
# on 200 being taken. Five executions of 202 is five executions of 200. The
# zero is an artifact of the instrumentation, not a fact about the tests, and no
# test can be written to fix it: this is `main.rs` again, and it gets the same
# treatment for the same stated reason — a 100% bar including such lines is
# unreachable by construction rather than demanding.
#
# The same thing happens at an ordinary `if`/`else`. Measured 2026-09-05,
# `session_ledger/src/replay.rs` under the whole slice family's tests:
#
#     line 204  `if actual == terminal.hash()`            822 hits
#     line 206  `Outcome::Match`, the taken branch        382 hits
#     line 208  `else`                                      0 hits
#     line 210  `Outcome::HashMismatch { … }`, inside it   58 hits
#
# and the argument reads identically: line 210 executes only when the `else` on
# 208 is taken, so fifty-eight executions of 210 is fifty-eight of 208.
#
# What makes the discount safe is that it proves itself per site rather than
# trusting a rule about the construct — which is exactly why it does not need to
# tell the two constructs apart, and no longer tries to. It applies to one line
# only when that line reports zero hits AND some line inside the block it opens
# reports more than zero. Rust guarantees the second implies the first was
# taken: an `else` block's contents are reachable by no other route, whether the
# `else` belongs to a `let` or to an `if`. Every other case is left alone and
# still fails the gate:
#
#   * An `else` whose path no test takes — the whole block reads zero, nothing
#     inside it is positive, no discount, and the gate fails on the `else` and
#     the body alike. Measured 2026-08-31: `exact_decimal/src/lib.rs` lines 237,
#     238 and 240 all read zero together.
#   * A site where tarpaulin instruments the `else` correctly — the line already
#     reads positive, so there is nothing to discount. Measured in the same run:
#     all four `let … else` in `smoke_lang_stack/src/lib.rs`, whose `else` lines
#     are hit normally. Which sites land which way is not settled here; the
#     discount does not depend on knowing.
#
# So this cannot excuse an untested branch. The statement the block is required
# to contain — the diverging one in a `let … else`, the value in an `if`/`else`
# — stays in the denominator, and it is the line that actually evidences the
# path was taken. The keyword line is the only thing ever discounted, and only
# against evidence from inside its own block.
#
# The narrower earlier form of this rule fired only inside a `let … else`, told
# apart by a `let` being open with the preceding line not ending in `}`. That
# discrimination is dropped rather than kept as a belt: it never carried the
# safety argument — the per-site evidence did — and keeping it meant an
# `if`/`else` phantom had no route to a verdict except rewriting working source
# to dodge a tool artifact.
phantom_lines()
{
  # Brace placement below is awk's, not the house's: a top-level pattern and its
  # `{` must share a line, or the pattern silently becomes a bare print rule and
  # the block runs unconditionally. Allman applies normally inside the actions.
  #
  # Fix(brace_depth_miscounted_braces_inside_string_and_char_literals)
  # Root cause: the depth counter below used to gsub `{`/`}` over each line's
  #   raw text, so a string literal such as
  #   `format!( "{noun} value has no matching '}}'" )`
  #   (flow_record.rs:332) contributed its own unmatched braces to the count —
  #   one open, three close on that single line — closing the tracked block
  #   one line early. Demonstrated live at that exact site (and its two
  #   siblings at lines 528 and 571); it happened not to flip any verdict
  #   there only because each block's sole content line was also its only
  #   possible evidence line, so the truncated search window still covered it
  #   by coincidence. A block whose real evidence line sits after a brace-
  #   bearing literal would have that evidence silently excluded, which is the
  #   exact phantom-failure shape this whole discount exists to prevent.
  # Pitfall: a lifetime (`'a`) opens with the same character as a char literal
  #   but never closes with a second one on the same line — treating every
  #   `'` as a char-literal opener strips real braces from any code that
  #   follows a lifetime on that line. `strip_literals` below only recognizes
  #   a `'` as a char literal when a closing `'` actually follows (one or two
  #   characters later, allowing for a `\`-escape); a lone `'a` is passed
  #   through unchanged. Line comments are truncated too, for the same
  #   reason — block comments (`/* */`) are not, since none exist in this
  #   crate and the multi-line state they would need is not worth carrying
  #   for a residual, unconfirmed-live case.
  awk -v src="$1" -v sq="'" '
    function trim( x ) { sub( /^[[:space:]]+/, "", x ); sub( /[[:space:]]+$/, "", x ); return x }

    function strip_literals( s,    out, i, n, c ) {
      out = ""; n = length( s ); i = 1
      while ( i <= n ) {
        c = substr( s, i, 1 )
        if ( c == "\"" ) {
          i++
          while ( i <= n ) {
            if ( substr( s, i, 1 ) == "\\" ) { i += 2; continue }
            i++
            if ( substr( s, i - 1, 1 ) == "\"" ) break
          }
          continue
        }
        if ( c == sq ) {
          if ( substr( s, i + 1, 1 ) == "\\" && substr( s, i + 3, 1 ) == sq ) { i += 4; continue }
          if ( substr( s, i + 1, 1 ) != "" && substr( s, i + 2, 1 ) == sq ) { i += 3; continue }
          out = out c; i++; continue
        }
        if ( c == "/" && substr( s, i + 1, 1 ) == "/" ) break
        out = out c; i++
      }
      return out
    }

    NR == FNR {
      if ( $0 ~ /^SF:/ ) { inf = ( substr( $0, 4 ) == src ); next }
      if ( inf && $0 ~ /^DA:/ )
      {
        split( substr( $0, 4 ), a, "," )
        hits[ a[ 1 ] ] = a[ 2 ] + 0
        known[ a[ 1 ] ] = 1
      }
      next
    }

    { line[ FNR ] = $0 }

    END {
      n = FNR
      for ( i = 1; i <= n; i++ )
      {
        t = trim( line[ i ] )
        if ( t == "" || t ~ /^\/\// ) continue

        if ( t == "else" )
        {
          # Already hit, or not instrumented at all — nothing to discount.
          if ( !( i in known ) || hits[ i ] > 0 ) continue

          # Extent of the block this `else` opens, by brace depth.
          depth = 0; started = 0; last = 0
          for ( j = i + 1; j <= n; j++ )
          {
            o = strip_literals( line[ j ] ); c = o
            n_open = gsub( /\{/, "{", o )
            n_close = gsub( /\}/, "}", c )
            depth += n_open - n_close
            if ( n_open > 0 ) started = 1
            if ( started && depth <= 0 ) { last = j; break }
          }
          if ( last == 0 ) continue

          # Positive evidence from inside the block is the whole argument.
          for ( j = i + 1; j <= last; j++ )
            if ( ( j in known ) && hits[ j ] > 0 ) { print i; break }
          continue
        }
      }
    }
  ' "$LCOV" "$1"
}

# One lcov.info per workspace, concatenated into one — valid for this format for
# the reason given at the invocation loop above, and read back by the awk that
# re-anchors at every `SF:` record.
#
# `-mindepth 2` is what keeps the destination out of its own input: the merged
# file sits at the root of `$lcov_root`, each workspace's under a slug directory
# one level further down.
LCOV="$lcov_root/lcov.info"
: >"$LCOV"
merged=0
while IFS= read -r part; do
  cat -- "$part" >>"$LCOV"
  merged=$(( merged + 1 ))
done < <( find "$lcov_root" -mindepth 2 -name lcov.info 2>/dev/null | LC_ALL=C sort )
[ "$merged" -gt 0 ] || fail "tarpaulin produced no lcov.info under $lcov_root, so the bare \`else\` discount cannot be evidenced"

phantom=0
phantom_map=""
while read -r c; do
  cdir="$( crate_dir "$c" )"
  while IFS= read -r src; do
    # Same set the denominator dropped. A discount credited to a file no longer
    # in `per_file` changes no arithmetic, but it would still be counted into
    # the note below — an evidence line reporting work the verdict did not rest
    # on.
    rel="$c/${src#"$cdir/"}"
    grep -qxF "$rel" <<<"$bin_list" && continue
    n_hit="$( phantom_lines "$src" | grep -c . )"
    [ "$n_hit" -gt 0 ] || continue
    phantom=$(( phantom + n_hit ))
    phantom_map+="$rel $n_hit"$'\n'
  done < <( find "$cdir/src" -name '*.rs' 2>/dev/null )
done < <( family_crates )

# Fold the discount into the fractions themselves rather than into the totals,
# so the shortfall list below names only files that are genuinely short.
if [ "$phantom" -gt 0 ]; then
  per_file="$( awk 'NR == FNR { add[ $1 ] = $2; next }
    { split( $2, a, "/" ); print $1, ( a[ 1 ] + add[ $1 ] ) "/" a[ 2 ] }' \
    <( printf '%s' "$phantom_map" ) <( printf '%s\n' "$per_file" ) )"
fi

read -r lines total <<<"$( awk '{ split( $2, a, "/" ); t += a[ 1 ]; n += a[ 2 ] } END { print t+0, n+0 }' <<<"$per_file" )"

[ "${total:-0}" -gt 0 ] || fail "zero coverable lines in the crates under test"

note=""
[ "$phantom" -gt 0 ] && note=", ${phantom} uninstrumentable bare \`else\` keyword line(s) discounted, each against a hit line inside its own block"

if [ "$lines" -lt "$total" ]; then
  short="$( awk '{ split( $2, a, "/" ); if ( a[ 1 ] < a[ 2 ] ) printf "%s %s ", $1, $2 }' <<<"$per_file" )"
  pct="$( awk "BEGIN{ printf \"%.2f\", 100 * $lines / $total }" )"
  fail "line coverage ${pct}% (${lines}/${total}), need 100%${note} — short: ${short}"
fi

pass "100% line coverage over ${total} coverable lines${note}"
