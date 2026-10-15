#!/usr/bin/env bash
# G3. Every feature the family declares is claimed by at least one test.
#
# A test claims a feature by citing it as `docs/feature/NNN` in a family crate's
# tests/. The pairing that stops a stage run from reading vacuously is
# `want > 0`. Without it, a stage declaring no features would satisfy
# "every declared feature is claimed" with nothing to claim. So a stage under
# test must name at least one feature to be gradeable.
#
# Fix(g3_header_claimed_an_uniqueness_check_that_was_never_built): this header
# used to add a second clause, "AND the count of claiming test modules equals
# the count of declared features". That clause describes a bijection (at most
# one claiming file per feature, not just at least one). `found`/`want` below
# have never checked that. `found` increments once per feature on a boolean
# `grep -rq` (first match wins), so a feature cited by two or three files still
# counts as exactly one toward `found`. Root cause: the header named an
# invariant the implementation never carried, instead of describing the code in
# front of it. Pitfall: not hypothetical. Measured live against `ring`
# (`GATE_FAMILY=ring bash ./g3_features.sh`), which reports REACHED
# ("all 22 features claimed by a test") while 18 of those 22 are each claimed by
# 2 or more distinct test files, not 1. That is the multiplicity the old second
# clause claimed to reject. Building that check, instead of removing the claim,
# was deliberately left undone here. Adding it would flip `ring`'s G3 from
# REACHED to NOT REACHED today. That is a live regression in a currently-passing
# family across every family that declares G3, not a doc-only correction. It is
# a new-behavior decision left for the user to make, not something to change
# unannounced while fixing a header.
set -uo pipefail
GATE=G3
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist

# Which crates' tests may claim a feature: the whole family by default, or only
# the stage's own crates under a stage run. That way a stage cannot get credit
# for a feature some later stage's crate claims.
search=()
while read -r c; do
  d="$( crate_dir "$c" )" && [ -d "$d/tests" ] && search+=( "$d/tests" )
done < <( family_crates )

missing=()
found=0
want=0
for id in $( feature_ids ); do
  want=$(( want + 1 ))
  if [ ${#search[@]} -gt 0 ] && grep -rqE "docs/feature/0*${id}_" "${search[@]}" 2>/dev/null; then
    found=$(( found + 1 ))
  else
    missing+=( "$id" )
  fi
done

[ "$want" -gt 0 ] || fail "no features in scope — nothing to claim"
if [ "$found" -ne "$want" ]; then
  fail "${found}/${want} features claimed by a test; unclaimed: ${missing[*]}"
fi
pass "all ${want} features claimed by a test"
