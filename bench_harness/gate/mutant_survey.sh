#!/usr/bin/env bash
# Mutant survey. Propose defect candidates mechanically, instead of waiting for
# someone to find one by hand.
#
# An earlier plan closed with a constraint. G12 replays a list, and nothing
# generates the list. G12 is sound. It reinstates each recorded defect and
# checks the suite still notices, but it can only ever defend against defects
# somebody already tripped over. A crate whose blind spot nobody has hit yet
# reads exactly like a crate with no blind spot.
#
# This runs the other direction. It breaks the crate in every way
# `cargo mutants` knows how and reports the mutations the suite did NOT notice.
# Each survivor is a line that runs under test with nothing asserting on what it
# produced. That is the B5 pathology, found before it costs anyone a wrong
# answer instead of after.
#
# Survivors are candidates, not verdicts. Recording one as a `.mutant`
# declaration is a judgement about whether the defect is worth defending, and
# that judgement stays human. What this removes is the need to have been
# unlucky first.
#
# ACCEPTED SURVIVORS, AND WHY THE LIST IS SUBTRACTED RATHER THAN JUST FILED
#
# Some survivors are decisions, not gaps. `ring_bench` refuses to assert on a
# clock, because a flaky assertion inside a benchmark discredits the very
# measurement it exists to produce. So every timing-only mutation survives it
# permanently and correctly. This tool cannot see that. It reports
# "the suite did not notice" and stops.
#
# Left alone, that makes the survey worse the more it is used. Every sweep
# re-presents the same settled decisions as fresh findings, and the real ones
# get read past. So a family may declare accepted survivors, this subtracts
# them, and the report becomes *new survivors since the list was written*.
#
# The subtraction is checked in both directions. An acceptance matching nothing
# is reported as stale rather than ignored. It means the code moved under the
# entry or the survivor was fixed, and either way the file now grants a
# permission for something that no longer exists. That is G12's own stale-`from`
# rule, applied to the other half of the same problem.
#
# WHY --in-place, WHICH IS OTHERWISE THE WRONG CHOICE
#
# `cargo mutants` normally copies the tree to a scratch directory and mutates
# the copy, which is strictly safer. That is unavailable in general. A path
# dependency that escapes the workspace root breaks under relocation. Relocate
# the tree and that path resolves against the new parent instead, which need not
# exist there, so the baseline build fails before a single mutant is tested. Any
# tree-relocating tool hits this, not just this one.
#
# So the survey mutates the real tree, and pays for it with the same discipline
# G12 uses: hash every target before, restore-check after, and treat a tree that
# did not come back as an emergency, not a test result. A killed run is the
# residual risk. `cargo mutants` restores on its own signal handling, but nobody
# can catch a `kill -9`. The hash check on the next run catches that, which is
# why it runs before the survey and not only after.
#
# WHY --all-features, WHICH THE FIRST VERSION OF THIS SCRIPT OMITTED
#
# `cargo mutants` runs the suite with default features unless told otherwise,
# and it mutates source text, not compiled code. Put those together and any
# `#[ cfg( feature = ... ) ]` body that is off by default gets mutated, compiles
# as dead code, and passes. That happens every single time, for every mutation,
# no matter how good the tests are. It reports as a survivor and looks exactly
# like a line the suite failed to defend.
#
# The first ring_bench survey returned 22 survivors and 18 of them were one
# `#[ cfg( feature = "crossbeam" ) ]` function that the default build never
# compiled. The suite does cover it. The tests iterate `Candidate::ALL`, which
# includes that candidate once the feature is on, so all 18 were noise, and they
# buried the two findings that were real.
#
# That is this crate's own baseline pathology one level down. A gate that cannot
# tell "not started" from "finished" measures nothing, and a survey that cannot
# tell "not compiled" from "not defended" reports nothing. Surveying with every
# feature on makes a survivor mean what the output says it means.
#
# Usage:
#   mutant_survey.sh <crate>            survey one crate
#   mutant_survey.sh <crate> --list     count candidate mutations, run nothing
#   mutant_survey.sh --family <name>    survey every crate in a family, in turn
#   mutant_survey.sh --family <name> --list
#
# The family sweep runs crates one at a time because `--in-place` refuses
# `--jobs`. It does not stop at the first crate with survivors, because a sweep
# that aborts on its first finding reports the first crate, not the family. It
# re-invokes this script per crate instead of looping inside one run, so every
# crate gets its own output clear, its own hash guard, and its own restore
# check, exactly as a single-crate run would.
#
# Exit codes are distinct on purpose. "found problems", "left the tree broken",
# and "the acceptance list has rotted" are not the same news:
#   0  survey ran, no survivors the family has not already accepted
#   1  survey ran, new survivors reported
#   2  the run is not trustworthy. STOP AND READ THE OUTPUT. Either the tree
#      did not come back byte-identical, or the survey tested zero mutants and
#      its silence is absence rather than a clean result
#   3  no new survivors, but an acceptance no longer matches anything
#
# Exit 0 is the only exit that writes anything outside `-mutants_out`. It
# records `declared/<family>/surveyed/<crate>.surveyed`, holding the date and a
# digest of the src/ and tests/ that were swept, which `g13_survey_freshness.sh`
# grades. The other three deliberately leave no record. 1 and 3 mean the sweep
# found something nobody has ruled on yet, and 2 means the run cannot be
# believed at all. A freshness record from any of them would certify a sweep
# that never finished.
set -uo pipefail

# Fix(a_family_sweep_ran_its_own_name_as_a_command_and_surveyed_nothing)
#
# Root cause: `${BASH_SOURCE[0]}` is whatever string named the script, not a
# path to it. Invoked as `bash mutant_survey.sh` from this directory, the
# ordinary way to run it from here, that string is the bare `mutant_survey.sh`
# with no slash in it. So bash resolved the self-invocation in the --family arm
# below through `$PATH`, which does not contain `.`. Every member died with
# `mutant_survey.sh: command not found`. Measured: one family's `--list`
# produced 32 identical not-found lines and surveyed nothing; another family's
# produced 11.
#
# Resolved once, by `cd`+`pwd`, and shared by all four sites that reach for this
# script's own location: REPO, the declared/ lookup, the self-invocation, and
# common.sh. That replaces four separate copies of a string that is only
# conditionally a path.
#
# Pitfall: the other three sites all wrap it in `dirname`, and
# `dirname mutant_survey.sh` is `.`. That is the right answer whenever the cwd
# is this directory, and it has to be for bash to have found the script by bare
# name in the first place. So REPO, the declared/ list and common.sh every one
# of them resolved correctly in precisely the run where the self-invocation
# could not. Nothing looked wrong until the one line that *executed* the string
# instead of merely taking its dirname. Executing is what distinguishes a path
# from a command name; no amount of `dirname`ing it will show the difference.
SELF_DIR="$( cd "$( dirname "${BASH_SOURCE[0]}" )" && pwd )" || exit 2
SELF="$SELF_DIR/$( basename "${BASH_SOURCE[0]}" )"

REPO="$( cd "$SELF_DIR/../../.." && pwd )"
CRATE="${1:-}"
MODE="${2:-run}"

[ -n "$CRATE" ] || { echo "usage: mutant_survey.sh <crate> [--list] | --family <name> [--list]" >&2; exit 2; }

if [ "$CRATE" = "--family" ]; then
  family="${2:-}"
  list_file="$SELF_DIR/declared/$family/crates.txt"
  [ -f "$list_file" ] || { echo "no family declaration at ${list_file}" >&2; exit 2; }

  sweep_mode="${3:-run}"
  swept=0 clean=0 found=0 broke=0 rotted=0
  dirty_crates="" broken_crates="" rotted_crates=""

  # Fix(word_splitting_a_declared_line_can_fragment_one_member_into_several):
  # was `for member in $( grep -vE '^\s*(#|$)' "$list_file" )`. Unquoted command
  # substitution word-splits on IFS, so any crates.txt line carrying more than
  # its bare name would iterate as several bogus members sharing one line
  # instead of the one real crate. Each would then fail crate_dir lookup and
  # count toward "not trustworthy" instead of being skipped or read whole.
  # decl_lines-style filtering (`grep -vE '^\s*(#|$)'`, used identically here)
  # only strips whole-line comments. An inline `# note` trailing a real name,
  # the single most ordinary way a two-word line would arise, survives it
  # untouched. Root cause: this was the one iteration of a `declared/<family>/`
  # list anywhere in gate/ that word-split instead of reading line-by-line.
  # common.sh's `assert_declared_crates_exist` and `corpus_docs` both
  # `while read` over `family_crates`, run_all.sh's `--every` branch uses
  # `mapfile`, and g22_exemption_expiry.sh reads exempt.txt the same way.
  # Pitfall: zero live trigger today. Every line in all 11 declared crates.txt
  # files is confirmed a single bare token (checked via `awk '{print NF}'`, max
  # 1 everywhere). It stays dormant until any crates.txt line ever carries more
  # than its bare crate name. Switched to the same line-based `while read` idiom
  # used everywhere else in this file family.
  while read -r member
  do
    "$SELF" "$member" "$sweep_mode"
    case $? in
      0 ) clean=$(( clean + 1 )) ;;
      1 ) found=$(( found + 1 )); dirty_crates="$dirty_crates $member" ;;
      3 ) rotted=$(( rotted + 1 )); rotted_crates="$rotted_crates $member" ;;
      * ) broke=$(( broke + 1 )); broken_crates="$broken_crates $member" ;;
    esac
    swept=$(( swept + 1 ))
  done < <( grep -vE '^\s*(#|$)' "$list_file" )

  # Fix(a_list_sweep_reported_success_however_many_members_had_failed)
  #
  # Was an unconditional `[ "$sweep_mode" = "--list" ] && exit 0`. Half of that
  # is right and worth keeping. In --list mode the per-crate arm exits 0 on
  # every success, so `clean`, `found` and `rotted` carry no signal and the
  # survivor summary below would be a report about nothing. `broke` is the one
  # counter that still means something, and it was discarded with the rest.
  #
  # Root cause: the early return was written as a check on the *mode*, when what
  # it needed to express was "this mode produces no findings to summarise". That
  # is a narrower claim, true of three counters and false of the fourth.
  # Measured with the self-invocation defect above still live: all members of
  # both families used in that measurement failed, and both sweeps exited 0.
  #
  # Pitfall: this is the same vacuous-success shape the single-crate --list
  # arm's own refusal guard was written against, one level up, and the two
  # compounded. The arm printed a plausible `0 candidate mutation(s)`, the sweep
  # swallowed the exit codes, and a family-wide survey that surveyed nothing
  # reported as a clean sweep. A mode that cannot produce a finding can still
  # produce a failure. Only the finding half is mode-dependent, and reading
  # "no findings here" as "nothing to report here" is what let a total failure
  # exit zero.
  if [ "$sweep_mode" = "--list" ]; then
    [ "$broke" -eq 0 ] || {
      echo >&2
      echo "════ list: $family — ${broke} of ${swept} crate(s) could not be listed at all:${broken_crates}" >&2
      echo "    A listing that did not run is not a listing of zero." >&2
      exit 2
    }
    exit 0
  fi

  echo
  echo "════ sweep: $family — $swept crate(s) surveyed"
  echo "    $clean clean, $found with new survivors, $rotted with a stale acceptance, $broke not trustworthy"
  [ -n "$dirty_crates" ]  && echo "    new survivors:$dirty_crates"
  [ -n "$rotted_crates" ] && echo "    stale acceptance:$rotted_crates"
  [ -n "$broken_crates" ] && echo "    NOT TRUSTWORTHY:$broken_crates"

  # A sweep's exit reports its worst crate, worst-first: an untrustworthy run
  # outranks a finding, because a finding is a result and the other is not.
  [ "$broke"  -gt 0 ] && exit 2
  [ "$found"  -gt 0 ] && exit 1
  [ "$rotted" -gt 0 ] && exit 3
  exit 0
fi

# Sourced for `crate_dir`, `decl_lines` and `DECL_ROOT` only, never for
# `pass`/`fail`, whose REACHED/NOT REACHED vocabulary is the one thing this
# script must not speak. The declaration comment convention lives in one place
# and this honours it instead of re-deriving the same grep. It is sourced here,
# not after the crate is resolved, because resolving the crate is now one of the
# things it is sourced for.
. "$SELF_DIR/common.sh"

# Which root a crate lives under is not fixed. A literal old-root prefix here
# did not merely mislabel an error message. It made this script unrunnable for
# all 33 ring crates after they moved to their current root, dying on its first
# line of real work with `no crate at <old-root>/ring_seqno`. That is the same
# population G13 reports as never surveyed. The gate names this script as the
# remedy, and the remedy could not run.
#
# CRATE_REL carries the answer down to the foreign-result check below rather
# than letting it widen to accept both roots. A result prefixed by the
# old root must still read as foreign, because it would mean the
# survey mutated a tree this crate no longer occupies.
CRATE_DIR="$( crate_dir "$CRATE" )" \
  || { echo "no crate named $CRATE under module/ or ring/" >&2; exit 2; }

# Fix(mutant_survey_ran_cargo_from_a_workspace_that_does_not_own_the_crate)
#
# Root cause: the relocation fix above generalized where a crate's *directory*
# is found and left the cargo invocation's *cwd* hardcoded to `$REPO`. Of the
# many crates the declaring families named, most were not members of the root
# workspace at all; several whole families had no member in it. Every
# non-member crate was listed, and surveyed, from a workspace that does not
# contain it. The same defect was already fixed in g1/g7, g2, g9, g19 and
# `verb/test`; this script was the last consumer still carrying it.
#
# Pitfall: `-p` is not the discriminator, and reaching for it will mislead.
# `cargo pkgid -p <non-member crate>` run from `$REPO` *succeeds*. cargo's own
# `-p` selects from the resolve graph and accepts a non-member that is reachable
# as a path dependency. `cargo mutants` then applies its own, separate
# source-tree check, which refuses. It refuses as a stderr
# `WARN Package "…" not found in source tree` with exit 0, not as an error. So
# every ordinary cargo reflex reports the crate as selectable right up to the
# point the tool declines, without a word, to do anything with it. Measured:
# `--list` for one non-member crate yields 0 candidates from `$REPO` and dozens
# from its own owning workspace.
#
# Falls back to `$REPO` when the owning workspace cannot be resolved, for
# family_workspace_groups()'s reason. An unresolvable crate should still reach
# cargo and fail with cargo's own diagnostic, not be dropped here.
WS_ROOT="$( crate_workspace_root "$CRATE" 2>/dev/null )" || WS_ROOT=""
[ -n "$WS_ROOT" ] || WS_ROOT="$REPO"
WS_REL="${WS_ROOT#"$REPO"}"; WS_REL="${WS_REL#/}"; [ -n "$WS_REL" ] || WS_REL="<repo root>"

# CRATE_REL carries the answer down to the foreign-result check below rather
# than letting it widen to accept both roots. A result prefixed by the
# old root must still read as foreign, because it would mean the
# survey mutated a tree this crate no longer occupies.
#
# Relative to WS_ROOT, not to `$REPO`, because that is the form `cargo mutants`
# reports in. Run from a sibling family's own workspace root it writes
# `some_crate/src/lib.rs:170:5`; run from `$REPO` it writes
# `ring/ring_seqno/src/lib.rs:52:3`. For crates whose owning workspace is
# `$REPO` the two forms are byte-identical, so this changes nothing for them.
# For crates owned by another workspace it is the difference between a foreign
# check that reads real paths and one that would reject every line the survey
# produced.
CRATE_REL="${CRATE_DIR#"$WS_ROOT"/}"

cd "$WS_ROOT" || exit 2

# The accepted list lives with its family's other declarations, so the crate has
# to be resolved to a family first. A crate belonging to no declared family has
# no list, and that is not an error. It is a crate no family has ruled on yet,
# and every survivor it reports is new by definition.
#
# Finding(accepted_file_resolution_shares_crate_dir_tie_break_property): this
# loop takes the first family whose crates.txt names $CRATE, in glob order. That
# is the same first-match-wins shape common.sh's crate_dir() documents at length
# for resolving a crate's directory. A crate declared into two families at once
# would have its survivors checked against only the alphabetically-earlier
# family's accepted/ list, without a word. The later family's own acceptances
# would never apply to it, and a survivor it already ruled on could be
# re-reported as new. No live trigger: checked directly, not assumed. No crate
# name appears in more than one family's crates.txt anywhere under declared/
# today (verified by comparing every declared crate name across all families for
# a duplicate). Left undisclosed-as-a-defect rather than fixed for the same
# reason crate_dir()'s tie-break is kept rather than resolved. Nothing currently
# breaks the tie, and a family sweep is a rare enough operation that resolving
# this properly (recording every family a crate is declared into, not only the
# first) is not worth doing ahead of an actual need.
accepted_file=""
decl_dir=""
for crates_txt in "$DECL_ROOT"/*/crates.txt
do
  [ -e "$crates_txt" ] || continue
  if decl_lines "$crates_txt" | grep -qxF "$CRATE"; then
    decl_dir="$( dirname "$crates_txt" )"
    accepted_file="$decl_dir/accepted/$CRATE.accepted"
    break
  fi
done

if [ "$MODE" = "--list" ]; then
  # stderr is captured, not discarded, and the reason is the bug this arm was
  # found with. Run from a workspace that does not own the crate,
  # `cargo mutants` does not refuse. It prints
  # `WARN Package "<name>" not found in source tree` to stderr, writes nothing
  # to stdout, and exits 0. `grep -c .` turns that empty stdout into a `0`, and
  # this arm printed `<crate> — 0 candidate mutation(s), none tested` and exited
  # 0. That is indistinguishable from a real zero, and real zeroes exist: a
  # 24-line crate can genuinely have none. One family's `--list` printed that
  # line for all 32 members and exited 0, a family-wide survey that surveyed
  # nothing, reported as a clean sweep.
  #
  # The cwd fix above stops it happening. This check stops it being silent if it
  # ever happens again. Checked independently of the count, because a refusal is
  # a refusal whatever number accompanies it.
  list_err="$( mktemp )" || exit 2
  trap 'rm -f -- "$list_err"' EXIT
  count="$( cargo mutants -p "$CRATE" --all-features --list 2>"$list_err" | grep -c . )"
  if grep -q 'not found in source tree' "$list_err"; then
    cat "$list_err" >&2
    echo "LIST DID NOT RUN — cargo mutants refused ${CRATE} as outside the source tree at ${WS_REL}." >&2
    echo "A count of ${count} here measures the refusal, not the crate. (cargo's own stderr above)" >&2
    exit 2
  fi
  [ ! -s "$list_err" ] || cat "$list_err" >&2
  echo "$CRATE — $count candidate mutation(s), none tested"
  exit 0
fi

# The guard covers tests/ as well as src/. `cargo mutants` has no reason to
# touch a test file, which is exactly why a changed one would go unnoticed.
guard="$( mktemp )" || exit 2
trap 'rm -f -- "$guard"' EXIT

hash_targets()
{
  crate_rs_listing "$CRATE_DIR"
}

# Written only on a fully clean run, and that condition is the whole value of
# the record. A crate reporting new survivors has findings nobody has ruled on
# yet; a crate reporting a stale acceptance has declarations that no longer
# describe it. Both are unfinished sweeps. A record written for either would let
# G13 report freshness for a crate whose triage never happened. That is the same
# "arrives carrying evidence" failure the output-clearing above exists to stop.
#
# The digest comes from the guard rather than from a second walk of the tree,
# because the guard is the exact state the survey ran against and has already
# been checked byte-for-byte against the tree it left behind.
record_sweep()
{
  [ -n "$decl_dir" ] || return 0
  local dir="$decl_dir/surveyed" rec
  mkdir -p "$dir" || return 0
  rec="$dir/$CRATE.surveyed"
  {
    printf '# Written by mutant_survey.sh on a clean run — never edited by hand.\n'
    printf '# Regenerate by re-surveying the crate; G13 reads `digest` and fails\n'
    printf '# once it stops matching the crate on disk.\n'
    printf 'crate: %s\n' "$CRATE"
    printf 'swept: %s\n' "$( date +%F )"
    printf 'digest: %s\n' "$( sha256sum < "$guard" | awk '{ print $1 }' )"
  } > "$rec"
  echo "swept — recorded in ${rec#"$REPO"/}"
}

hash_targets > "$guard"
[ -s "$guard" ] || { echo "no .rs files under $CRATE_REL to survey" >&2; exit 2; }

out="$REPO/-mutants_out"

# Cleared before every run, and this is required, not tidiness. `cargo mutants`
# rotates its previous output to `mutants.out.old` only once it gets far enough
# to produce new output. A run that dies on the baseline build never gets there,
# and leaves the *previous crate's* `mutants.out` sitting untouched. The report
# below then reads another crate's results and prints them under this crate's
# name, in zero seconds, with a confident exit 1.
#
# Not hypothetical: a deliberately broken `ring_types` baseline reported
# `ring_bench`'s three survivors, and every check in this script passed while it
# did. The tree hash matched (nothing had moved), and the mutant count was
# healthy (it was counting the previous run). A stale-read is worse than a
# vacuous one, because it arrives carrying evidence.
case "$out" in
  */-mutants_out ) rm -rf -- "$out" ;;
  * ) echo "refusing to clear an unexpected output path: $out" >&2; exit 2 ;;
esac

cargo mutants -p "$CRATE" --all-features --in-place -o "$out"
survey_status=$?

# Checked before the findings are reported, and reported even when the survey
# itself failed. A broken tree outranks whatever the survey was going to say.
if ! ( cd "$CRATE_DIR" && sha256sum -c --status - ) < "$guard"; then
  echo
  echo "EMERGENCY — ${CRATE} did not come back byte-identical after the survey."
  echo "The tree is mutated. Recover it before running anything else:"
  echo
  ( cd "$CRATE_DIR" && sha256sum -c - ) < "$guard" 2>&1 | grep -v ': OK$'
  exit 2
fi

# The same failure one degree weaker, and the one that got through.
#
# `cargo mutants` files a mutant "unviable" when the mutated crate does not
# build, and it cannot tell a mutation that broke the build from a build that
# was already broken. So when the workspace stops compiling for a reason no
# mutation caused, every remaining mutant lands in the unviable pile, `tested`
# stays healthy, the non-vacuity check below passes, and the survey reports a
# handful of survivors for a crate it never graded.
#
# Not hypothetical. A concurrent session added a workspace member and left its
# manifest half-written mid-run. This crate came back
# "24 missed, 219 caught, 1646 unviable", 87% unbuildable, and read as very
# nearly clean. Surveyed again once the workspace built, the same crate reported
# 128 missed and 1252 caught. The undercount was five-fold and it arrived
# looking like good news, which is the shape of failure this whole directory
# exists to refuse.
#
# So build the unmutated crate once more, now. If it does not build here, the
# unviable pile is not attributable to the mutations and the run means nothing.
if ! cargo build -p "$CRATE" --all-targets --all-features > "$out/post_baseline.log" 2>&1; then
  echo
  echo "SURVEY NOT TRUSTWORTHY — ${CRATE} does not build now that the run is over."
  echo "Every mutant filed 'unviable' may be that failure rather than its own"
  echo "mutation, so the survivor count is a floor of unknown depth, not a result."
  echo "Fix the build and survey again:"
  echo "  ${out#"$REPO"/}/post_baseline.log"
  exit 2
fi

# `-o DIR` writes into `DIR/mutants.out`, rotating any previous run to
# `mutants.out.old`. The nesting is the tool's, not ours.
results="$out/mutants.out"

count_lines()
{
  [ -f "$1" ] || { echo 0; return; }
  grep -c . "$1" 2>/dev/null || true
}

# The survey's own non-vacuity check, and the reason `survey_status` is captured
# at all. If the baseline build dies, `cargo mutants` writes no `missed.txt`, so
# `$missed` comes back empty and every line below reads it as
# "nothing survived". The escaping-path-dependency risk above is one way that
# can happen. A survey that never ran would report the cleanest possible result.
#
# That is the exact failure this whole gate directory was built against, and it
# would have landed here in the one script whose job is to find it. So count
# what was tested, and refuse to report on a run that tested nothing.
tested=$((
  $( count_lines "$results/caught.txt" )
  + $( count_lines "$results/missed.txt" )
  + $( count_lines "$results/timeout.txt" )
  + $( count_lines "$results/unviable.txt" )
))

if [ "$tested" -eq 0 ]; then
  echo
  echo "SURVEY DID NOT RUN — ${CRATE} tested 0 mutants (cargo mutants exited ${survey_status})."
  echo "This is not a clean result; it is the absence of one. Check the baseline"
  echo "build before reading anything into it:"
  echo "  ${results#"$REPO"/}/log/baseline.log"
  exit 2
fi
missed="$( cat "$results/missed.txt" 2>/dev/null )"
timed_out="$( cat "$results/timeout.txt" 2>/dev/null )"

# Defence in depth on the same failure. Every descriptor `cargo mutants` emits
# is prefixed with the path it mutated, so results belonging to some other crate
# are mechanically detectable rather than merely unlikely. The clear above
# should make this unreachable; it is here because the failure it catches is one
# that reports success, and those are worth catching twice.
foreign="$( printf '%s\n%s\n' "$missed" "$timed_out" | grep . | grep -v "^$CRATE_REL/" || true )"
if [ -n "$foreign" ]; then
  echo
  echo "SURVEY RESULTS DO NOT BELONG TO ${CRATE} — refusing to report them:"
  echo "$foreign" | sed 's/^/    /'
  exit 2
fi

echo
echo "── survey: $CRATE — tree restored byte-identically"

# Reported apart from survivors because a hang is not a blind spot. The suite
# does fail. It fails by never finishing, which is a real detection and a
# useless diagnostic. Loop-counter mutations land here almost every time.
if [ -n "$timed_out" ]; then
  echo "timed out — noticed by hanging rather than by any assertion:"
  echo "$timed_out" | sed 's/^/    /'
  echo
fi

accepted=""
if [ -n "$accepted_file" ] && [ -f "$accepted_file" ]; then
  accepted="$( decl_lines "$accepted_file" )"
fi

# Subtracted by whole-line exact match, and checked in both directions: what
# survived and is not accepted, and what is accepted and did not survive.
new_survivors="$missed"
stale=""
if [ -n "$accepted" ]; then
  new_survivors="$( grep -Fxv -f <( echo "$accepted" ) <<< "$missed" )" || true
  stale="$( grep -Fxv -f <( echo "$missed" ) <<< "$accepted" )" || true
  echo "accepted — $( echo "$accepted" | grep -c . ) survivor(s) already ruled on, per ${accepted_file#"$REPO"/}"
fi

if [ -n "$stale" ]; then
  echo
  echo "STALE — accepted, but no longer a survivor:"
  echo "$stale" | sed 's/^/    /'
  echo
  echo "Either the code moved under the entry or the suite grew an assertion that"
  echo "now catches it. As written the entry grants a permission for something"
  echo "that is not there — drop it or repoint it."
fi

if [ -z "$new_survivors" ]; then
  [ -n "$stale" ] && exit 3
  echo "no new survivors — every viable mutation was noticed by the suite or already ruled on"
  record_sweep
  exit 0
fi

echo
echo "new survivors — each is a mutation the suite did not notice:"
echo "$new_survivors" | sed 's/^/    /'
echo
echo "Record the ones worth defending as declarations under"
echo "  gate/declared/<family>/mutant/*.mutant"
echo "so G12 replays them from here on. Record the ones that are decisions rather"
echo "than gaps, with the reason, under"
echo "  gate/declared/<family>/accepted/<crate>.accepted"
echo "so the next sweep reports what is new instead of what is settled. Leaving"
echo "one in neither file still decides it — it just decides it silently."
exit 1
