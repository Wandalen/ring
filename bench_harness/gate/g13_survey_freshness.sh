#!/usr/bin/env bash
# G13. Every crate's survey findings still describe the crate as it stands.
#
# G12 replays defects somebody recorded, and `accepted/` subtracts survivors
# somebody ruled on. Both are claims about a crate at the moment it was swept,
# and both decay without saying so. A suite that grows an assertion, or a source
# file that gains a branch, changes what a survey would find while changing
# nothing either directory can see. The checks that catch this exist. A stale
# `from` block fails G12, and an acceptance matching no survivor exits 3. But
# both run only when a person chooses to sweep. On the gate board a crate last
# surveyed in March reads the same as one surveyed this morning.
#
# This gate is the mechanical half of that question and deliberately only the
# mechanical half. Whether a survivor is a defect or a decision is a judgement
# and stays off the board, which is why `mutant_survey.sh` is not a gate.
# Whether that judgement is still about the current code is a digest comparison,
# and there is no reason for a person to be the one making it.
set -uo pipefail
GATE=G13
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist

REC_DIR="$DECL/surveyed"

# The same listing `mutant_survey.sh` takes as its restoration guard, reduced to
# one line. Both scripts share it through `crate_rs_listing` instead of
# rewalking the tree here, so the two cannot drift apart and start hashing
# different file sets. Both halves of the listing matter, for different reasons.
# A changed test may now kill what survived, and a changed source may carry
# mutations no survey has ever run.
crate_digest() {
  crate_rs_listing "$( crate_dir "$1" )" | sha256sum | awk '{ print $1 }'
}

never=() moved=() n=0
while read -r c; do
  n=$(( n + 1 ))
  rec="$REC_DIR/$c.surveyed"
  if [ ! -f "$rec" ]; then
    never+=( "$c" )
    continue
  fi
  want="$( grep -E '^digest:' "$rec" | awk '{ print $2 }' )"
  # A record with no digest counts as moved, not absent. The file claims a sweep
  # happened, so reporting it as never-surveyed would understate what is wrong
  # with it.
  [ -n "$want" ] && [ "$want" = "$( crate_digest "$c" )" ] || moved+=( "$c" )
done < <( family_crates )

if [ ${#never[@]} -gt 0 ] || [ ${#moved[@]} -gt 0 ]; then
  if [ ${#never[@]} -gt 0 ]; then
    echo "    never surveyed — no record under ${REC_DIR#"$REPO"/}:"
    printf '        %s\n' "${never[@]}"
  fi
  if [ ${#moved[@]} -gt 0 ]; then
    echo "    surveyed, then changed — the recorded findings are about older code:"
    printf '        %s\n' "${moved[@]}"
  fi
  echo "    re-survey each, then re-run this gate:"
  echo "        bash ring/bench_harness/gate/mutant_survey.sh <crate>"
  fail "${#never[@]}/${n} never surveyed, ${#moved[@]}/${n} changed since their last clean sweep"
fi
pass "all ${n} crates surveyed clean against the source and tests they carry now"
