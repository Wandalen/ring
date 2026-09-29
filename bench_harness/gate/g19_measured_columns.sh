#!/usr/bin/env bash
# G19 — every column the family Overview Table's own Legend calls *measured*
# still equals what the tree says.
#
# The Legend names five: `States`, `LOC`, `Tests`, `Green`, and the built half of
# `Smoke`. It says of them, in the table's own words, that they "read the
# implementation tree rather than the corpus, which makes them the only columns
# here that can contradict an instance's own prose." What it does not say is who
# re-reads them. Nobody did. Each was computed by hand, once, by running the
# recipes the document publishes beneath itself, and has been quoted ever since
# as though the number and the tree were still the same thing.
#
# They are not. One family's instance publishes four implemented modules and
# zero tests against a `Modules` list naming one, which is the shape of the
# failure: not a number that is obviously wrong, a number that is quietly old.
# A reader has no way to tell the two apart, and a figure nobody can check is
# worse than a figure nobody wrote, because it gets cited.
#
# This gate re-runs those recipes and reports every cell where the published
# value and the recomputed one disagree. It is family-independent, like G18: the
# table spans every family this repository could ever declare and belongs to
# none of them individually, so it appears in no `gates.txt`.
#
# It reports and never edits. A gate that corrected the table it grades would
# report clean forever while measuring nothing — the disagreement count would
# track its own last write rather than the tree. Closing a finding here is a
# human edit to `docs/workstream/readme.md`, made deliberately, by someone who
# looked at why the number moved.
#
# ── The two handles ──────────────────────────────────────────────────────────
#
#   G19_TABLE     where the *published* cells are read from. Defaults to the
#                 repository's own table. Overridable because a gate that can
#                 only ever be pointed at one stale document can be observed
#                 failing and never observed discriminating — the fixture that
#                 shows it also reaches has to come from somewhere.
#   G19_VERDICTS  a pre-computed `<crate> <passed> <failed>` rollup for the
#                 `Green` column, so a caller who has already run the suite does
#                 not pay for it twice. Absent, this gate runs the suite itself.
#
# `Modules` and `Smoke` are always read from the real instances under
# `docs/workstream/`, never from beside `G19_TABLE`. The override substitutes a
# set of published claims; it never substitutes the tree those claims are
# measured against, because then a fixture could pass by disagreeing with reality
# consistently.
set -uo pipefail
GATE=G19
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"

WS="$REPO/docs/workstream"
TABLE="${G19_TABLE:-$WS/readme.md}"

[ -f "$TABLE" ] || fail "no Overview Table at ${TABLE#"$REPO"/} — the gate would report clean having read nothing"

# The Overview Table's header, by its own column names. Anchoring on the header
# rather than on the row shape matters: this document carries a second table of
# `| NNN |` rows further down (the Smoke Tests ledger), and a scanner keyed only
# on the row shape silently grades eighteen prose rows as though they were
# measurements.
hdr="$( grep -nE '^\|[[:space:]]*ID[[:space:]]*\|[[:space:]]*Instance[[:space:]]*\|' "$TABLE" | head -1 | cut -d: -f1 )"
[ -n "$hdr" ] || fail "no Overview Table header (\`| ID | Instance | …\`) in ${TABLE#"$REPO"/} — renamed or removed, and a rowless scan reads exactly like a clean one"

# Rows of that table and no other: from the header to the first line that is not
# a table row.
mapfile -t rows < <( awk -v h="$hdr" 'NR > h { if ( $0 !~ /^\|/ ) exit; if ( $0 ~ /^\|[[:space:]]*[0-9][0-9][0-9][[:space:]]*\|/ ) print }' "$TABLE" )
[ ${#rows[@]} -gt 0 ] || fail "Overview Table in ${TABLE#"$REPO"/} has no workstream rows — nothing to compare"

# The leading run of backticked names in a `- **Field:**` line. Stops at the
# first thing that is not a backticked name, which is what separates the list
# from the prose that follows it on the same line. Taken verbatim from the
# recipe the table publishes for its own columns, so this gate measures the
# column as defined rather than as re-invented.
field_names() {
  grep -m1 "^- \*\*$2:\*\*" "$1" 2>/dev/null \
    | sed "s/^- \*\*$2:\*\* *//" \
    | grep -oE '^(`[a-z0-9_]+`(, )?)+' | grep -o '`[a-z0-9_]*`' | tr -d '`'
}

# One module's contribution: its state, its non-blank `src/` lines, and its
# `#[ test ]` attributes. Printed as `state loc tests` for the caller to sum.
#
# Fix(g19_pubs_regex_blind_to_qualified_items): the pattern below used to be
# `'^\s*pub (fn|struct|enum|trait|type|const|static|mod|use)'` — the exact
# pre-fix shape `g22_exemption_expiry.sh`'s own `crate_is_implemented()`
# already found and fixed (`Fix(g22_modifier_qualified_pub_items_invisible)`):
# `pub unsafe fn`, `pub async fn`, `pub unsafe trait` and `pub extern "C" fn`
# all failed to match, so a crate whose entire public surface used a
# qualified form read as `skeleton` here even though it is `implemented`
# there — two separate reimplementations of the same "does this crate export
# anything" question, one fixed and the other not, the identical drift this
# file's own `Fix(a_crashed_test_is_a_failure_rather_than_an_absence)` above
# already names for a different pair of duplicate parsers. Root cause:
# g22's fix was never carried over to this file's own copy of the same
# check. Pitfall: zero live trigger — measured directly, not assumed: no
# crate under any crate root has zero matches under
# the old pattern and more than zero under g22's qualifier-aware one, so
# widening it changes no classification for any tracked module today.
module_reading() {
  local m="$1" dir="$( crate_dir "$1" )" loc=0 tests=0 pubs=0
  [ -d "$dir" ] || { echo "unfiled 0 0"; return; }
  tests="$( find "$dir" -name '*.rs' -exec grep -ho '#\[[[:space:]]*test[[:space:]]*\]' {} + 2>/dev/null | wc -l )"
  [ -d "$dir/src" ] || { echo "skeleton 0 $tests"; return; }
  loc="$( find "$dir/src" -name '*.rs' -exec cat {} + 2>/dev/null | grep -c '[^[:space:]]' )"
  pubs="$( find "$dir/src" -name '*.rs' -exec grep -hcE '^[[:space:]]*pub[[:space:]]+((unsafe|async|extern([[:space:]]+"[^"]*")?)[[:space:]]+)*(fn|struct|enum|trait|type|const|static|mod|use)\b' {} + 2>/dev/null | awk '{ s += $1 } END { print s + 0 }' )"
  if [ "$pubs" -gt 0 ] || [ -f "$dir/src/main.rs" ]; then
    echo "implemented $loc $tests"
  else
    echo "skeleton $loc $tests"
  fi
}

# `Green`, per crate, as `<crate> <passed> <failed>`. Either supplied by the
# caller or produced here by the run the column requires — the Legend calls it
# "the only column that runs anything", so a scan cannot answer it and a gate
# that skipped it would leave a measured column unmeasured while reporting on
# the other four.
verdict_table() {
  if [ -n "${G19_VERDICTS:-}" ]; then
    cat "$G19_VERDICTS"
    return
  fi
  log="$( mktemp )"
  # Fix(g19_verdict_log_leak_on_signal)
  # Root cause: `$log` held the output of a full-workspace
  #   `cargo nextest run` — realistically the single longest-running command
  #   in this whole gate family — with only the manual `rm -f "$log"` at this
  #   function's end and no trap. A SIGINT/TERM/HUP arriving during that run
  #   left `$log` behind with nothing to remove it.
  # Pitfall (x2, both confirmed against the live corpus): (1) this trap
  #   intentionally ALSO names "$VERDICTS" (the outer script's own
  #   EXIT-trapped temp file, set two call frames up before `verdict_table`
  #   ever runs) rather than "$log" alone. Bash trap registration is
  #   process-global and non-stacking — a `trap ... EXIT` here would
  #   silently REPLACE the outer `trap 'rm -f "$VERDICTS"' EXIT` rather than
  #   add to it, dropping $VERDICTS' own cleanup coverage for the rest of
  #   the script's life. Restating both paths keeps the same single EXIT
  #   handler responsible for everything live at this point; the
  #   `G19_VERDICTS`-env-var-set branch above returns before `$VERDICTS` or
  #   `$log` even exist, so it never reaches this line and never needs the
  #   wider trap. (2) `log` was originally declared `local` to this
  #   function — harmless-looking, since it is only read/written in here —
  #   but the trap referencing it is process-global and does not fire until
  #   the WHOLE SCRIPT exits, almost always well after `verdict_table` has
  #   already returned (e.g. when the disagreement-count check far below
  #   calls `fail`). By then the local binding was gone, and under this
  #   file's `set -uo pipefail`, expanding the now-unset "$log" inside the
  #   trap body is itself a hard error that aborts the `rm -f` command
  #   before it runs — so `$VERDICTS` stopped being cleaned too, a
  #   regression on the pre-existing, previously-reliable outer trap.
  #   Caught by actually letting a live run reach a real `fail()` call
  #   after `verdict_table` returned (a synthetic same-function signal test
  #   alone stayed inside the function and never exercised this). Fixed by
  #   dropping `local` — `log` is set exactly once per process here, so a
  #   plain (persistent) variable costs nothing and stays validly bound for
  #   any later EXIT, matching how `$VERDICTS` itself is already scoped.
  trap 'rm -f -- "$log" "$VERDICTS"' EXIT
  # Fix(g19_green_column_measured_one_workspace_only)
  # Root cause: this ran `cargo nextest run --workspace` exactly once, at
  #   `$REPO`. `--workspace` means "every member of the workspace I am
  #   standing in", and that leaves out every crate belonging to a sibling
  #   workspace root instead — each its own `[workspace]` with its own
  #   members — so none of their crates were built, run, or emitted a
  #   nextest status line. A large share of the modules the corpus declares
  #   live in that invisible set.
  # Pitfall: the under-reach is silent, and both of the shapes it produces
  #   read as a plausible measurement rather than as an error. A workstream
  #   whose modules all live outside $REPO contributes no crate to
  #   `$VERDICTS`, so `ran` stays 0 and the row computes `⬜` — indis-
  #   tinguishable from a genuinely untested workstream. A workstream split
  #   across roots contributes only its $REPO crates, so `ran` is 1 and the
  #   row computes a truthful-looking `✅N` over a truncated N. Several work-
  #   streams sat in the first state and several in the second; no inspection of
  #   the gate's own output separates either from a correct reading. Fixed by
  #   fanning out over workspace_roots() — the same enumeration
  #   cargo_over_workspaces() builds its per-family groups from — appending
  #   every root's output to the one `$log`. The parse below is unchanged and
  #   needs to be: it keys off the crate name in each status line, never off
  #   which root produced it.
  #
  # `local` on the three loop variables, deliberately unlike `$log` directly
  # above: the pitfall documented there is specific to a name the EXIT trap
  # expands long after this function returns. These three are read only
  # inside the loop, so scoping them keeps them from colliding with the
  # row loop's own names further down.
  #
  # DEAD_ROOTS is the one name here that must NOT be local. A root emitting no
  # `Summary` line never ran its tests — a build failure, a poisoned target
  # dir — and its crates are then simply absent from `$VERDICTS`, so every
  # workstream owning them computes `⬜`: exactly the silent vacuity this fix
  # removes, reintroduced one root at a time. It is checked at top level
  # rather than here because this function's stdout IS the table, so a `fail`
  # from inside it would be captured into `$VERDICTS` instead of printed.
  local ws rel before
  : >"$log"
  while read -r ws; do
    [ -n "$ws" ] || continue
    rel="${ws#"$REPO"}"; rel="${rel#/}"; [ -n "$rel" ] || rel="<repo root>"
    before="$( wc -l <"$log" )"
    ( cd "$ws" && cargo nextest run --workspace --all-features --no-fail-fast ) >>"$log" 2>&1
    tail -n "+$(( before + 1 ))" "$log" | grep -qE '^[[:space:]]+Summary \[' \
      || DEAD_ROOTS="$DEAD_ROOTS $rel"
  done < <( workspace_roots )
  # `sort -u` is load-bearing: nextest prints a failure twice, inline and again
  # in its closing recap, and counting raw lines doubles every red crate.
  #
  # Fix(a_crashed_test_is_a_failure_rather_than_an_absence): the status
  # alternation was `PASS|FAIL` only, so a test nextest reports as SIGABRT,
  # TIMEOUT or another crash/hang status matched neither branch, produced no
  # grep hit, and never contributed to the tally — not merely undercounted,
  # but able to leave a crate with a real crash reading as a clean, fully
  # passing "N 0", since any co-occurring PASS/LEAK line for that crate still
  # counts. Root cause: ws_verify::probe::result_test carried the identical
  # bug before its own documented fix (probe.rs's own
  # a_test_killed_by_a_signal_is_a_failure_rather_than_an_absence); this bash
  # reimplementation of the same parsing job was never brought in line with
  # it. Pitfall: `cargo nextest` itself exits 100 on an abort, which this
  # gate's caller reads as "ran, something failed", so nothing upstream flags
  # that the per-crate tally undercounted which crate. Extended to the
  # realistic terminal-status vocabulary nextest emits and normalised to
  # PASS/FAIL immediately, graded failure unless PASS or LEAK — mirroring
  # result_test's own allowlist so both implementations agree.
  grep -oE '^ +(PASS|FAIL|TIMEOUT|LEAK|SIGABRT|SIGSEGV|SIGKILL|SIGILL|SIGBUS|SIGFPE|ABORT) \[[^]]*\] +\( *[0-9]+/[0-9]+\) +[^ ]+ +[^ ]+' "$log" \
    | awk '{ v = ( $1 == "PASS" || $1 == "LEAK" ) ? "PASS" : "FAIL"; split( $( NF - 1 ), a, "::" ); print a[ 1 ], $NF, v }' | sort -u -k1,2 \
    | awk '{ if ( $3 == "FAIL" ) f[ $1 ]++; else p[ $1 ]++ }
           END { for ( c in p ) print c, p[ c ], f[ c ] + 0
                 for ( c in f ) if ( !( c in p ) ) print c, 0, f[ c ] }'
  rm -f "$log"
}

[ -z "${G19_VERDICTS:-}" ] || [ -f "$G19_VERDICTS" ] \
  || fail "G19_VERDICTS names no file: $G19_VERDICTS — a missing rollup would read as every crate untested"

VERDICTS="$( mktemp )"
trap 'rm -f "$VERDICTS"' EXIT
# Set before the call, not inside it: on the `G19_VERDICTS`-supplied branch
# `verdict_table` returns before the fan-out loop runs at all, and under
# `set -u` an unset name here would abort the check below rather than pass it.
DEAD_ROOTS=""
verdict_table > "$VERDICTS"

# A root whose tests never ran cannot be distinguished, downstream, from a
# workstream that has no tests — both produce `⬜`. Name it here, while the
# distinction still exists.
[ -z "$DEAD_ROOTS" ] \
  || fail "cargo nextest emitted no Summary line for$DEAD_ROOTS — those workspace roots did not run their tests, so every crate they own is missing from the Green measurement and every workstream owning those crates would read ⬜ (nothing runs) rather than reporting the failure"

# Published cell, normalised: interior spaces dropped so `2 491` and `2491`
# compare equal, since the thousands separator is presentation.
cell() {
  awk -F'|' -v n="$2" '{ v = $n; gsub( /^[[:space:]]+|[[:space:]]+$/, "", v ); gsub( / /, "", v ); print v }' <<<"$1"
}

report=()
compare() {
  [ "$3" = "$4" ] || report+=( "$1 $2: published $3, measured $4" )
}

for row in "${rows[@]}"; do
  id="$( cell "$row" 2 )"
  inst="$( find "$WS" -mindepth 1 -maxdepth 1 -type d -name "${id}_*" | head -1 )"
  [ -n "$inst" ] || { report+=( "$id instance: published a row, measured no docs/workstream/${id}_* directory" ); continue; }

  impl=0 skel=0 unfiled=0 loc=0 tests=0 gp=0 gf=0 ran=0
  while read -r m; do
    [ -n "$m" ] || continue
    set -- $( module_reading "$m" )
    case "$1" in
      implemented ) impl=$(( impl + 1 )) ;;
      skeleton    ) skel=$(( skel + 1 )) ;;
      *           ) unfiled=$(( unfiled + 1 )) ;;
    esac
    loc=$(( loc + $2 )); tests=$(( tests + $3 ))
    v="$( awk -v c="$m" '$1 == c { print $2, $3 }' "$VERDICTS" )"
    [ -n "$v" ] && { set -- $v; gp=$(( gp + $1 )); gf=$(( gf + $2 )); ran=1; }
  done < <( field_names "$inst/readme.md" Modules )

  built=0 total=0
  while read -r s; do
    [ -n "$s" ] || continue
    total=$(( total + 1 ))
    crate_dir "${s#[0-9][0-9][0-9]_}" >/dev/null && built=$(( built + 1 ))
  done < <( field_names "$inst/readme.md" Smoke )

  if   [ "$gf" -gt 0 ]; then green="❌$gf"
  elif [ "$ran" -eq 1 ]; then green="✅$gp"
  else                       green="⬜"; fi

  if   [ "$built" -eq "$total" ] && [ "$total" -gt 0 ]; then smoke="✅$built/$total"
  elif [ "$built" -gt 0 ];                              then smoke="⚠$built/$total"
  else                                                       smoke="⬜$built/$total"; fi

  compare "$id" States "$( cell "$row" 6 )"  "$impl/$skel/$unfiled"
  compare "$id" LOC    "$( cell "$row" 7 )"  "$loc"
  compare "$id" Tests  "$( cell "$row" 8 )"  "$tests"
  compare "$id" Green  "$( cell "$row" 9 )"  "$green"
  compare "$id" Smoke  "$( cell "$row" 11 )" "$smoke"
done

n=${#report[@]}
if [ "$n" -gt 0 ]; then
  printf '  %s\n' "${report[@]}"
  echo "  fix: re-run the recipes published under the Overview Table's Legend and"
  echo "       update docs/workstream/readme.md by hand — this gate never edits it"
  fail "$n disagreement(s) across ${#rows[@]} workstream row(s) in ${TABLE#"$REPO"/}"
fi
pass "${#rows[@]} workstream row(s), every measured column equal to the tree, in ${TABLE#"$REPO"/}"
