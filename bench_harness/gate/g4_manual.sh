#!/usr/bin/env bash
# G4. Every crate carries tests/manual/readme.md with an executed run record.
#
# "Manually tested" is only a claim until someone records the run. The gate
# requires a `## Run Record` section carrying an ISO date, so an empty plan
# with no execution behind it does not count.
set -uo pipefail
GATE=G4
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist

# Extract the Run Record section's own body, from its heading to the next
# heading at the same or shallower level, or EOF. The function prints it so the
# caller can grep *this* text for a date, not the whole file.
#
# The date check used to run against the whole file, not this section. A
# heading-only stub ("## Run Record" with nothing under it yet) would still pass
# as long as some unrelated ISO-shaped date sat anywhere else in the document. A
# plan with a "Created 2026-01-01" line but no executed run would satisfy both
# independent greps without the date ever describing a run. Real manual readmes
# routinely nest `###`-level detail under Run Record (e.g.
# "### Defect found by M1"). So the boundary has to compare heading depth, not
# stop at the next `#` line of any level, or a dated sub-section would be cut
# off along with a genuine record. Latent, not exploited: checked against all
# 127 live manual readmes and none currently relies on a date outside its own
# Run Record section.
#
# Fix(g4_open_heading_matched_by_prefix_not_equality): the open pattern used to
# be `/^##+ Run Record/` with no trailing anchor. It matched any heading
# STARTING WITH "Run Record", not only a heading that IS "Run Record". Suppose a
# crate's manual readme headed a section "## Run Record Draft" or
# "## Run Recorded By Hand" ahead of its real "## Run Record". That earlier
# heading would become the one this function reads, with no warning. It would
# take everything up to the real heading as the recorded run's own body, and the
# close check would then read the real heading as ending the wrong section.
# Root cause: same bug class as `corpus_lib.py`'s
# `a_deeper_heading_is_not_a_boundary` and `g12_mutation.sh`'s
# `g12_block_markers_matched_by_prefix_not_equality`. The open and close
# boundaries both need "is this the one heading," and a prefix test answers a
# different question. Pitfall: zero live trigger, measured directly, not
# assumed. Every "## Run Record"-shaped heading across every live
# `tests/manual/readme.md` under any crate root is exactly
# hash-run-space-"Run Record" with nothing trailing (no extra words, no trailing
# whitespace, no CR). So anchoring the pattern changes which lines it matches
# for zero files today.
run_record_section() {
  awk '
    /^##+ Run Record$/ && !found {
      found = 1
      line = $0; level = 0
      while ( substr( line, level + 1, 1 ) == "#" ) level++
      print
      next
    }
    found && /^#+ / {
      line = $0; lvl = 0
      while ( substr( line, lvl + 1, 1 ) == "#" ) lvl++
      if ( lvl <= level ) exit
    }
    found { print }
  ' "$1"
}

absent=() unrun=()
while read -r c; do
  f="$( crate_dir "$c" )/tests/manual/readme.md"
  if [ ! -f "$f" ]; then
    absent+=( "$c" )
  elif ! run_record_section "$f" | grep -qE '[0-9]{4}-[0-9]{2}-[0-9]{2}'; then
    unrun+=( "$c" )
  fi
done < <( family_crates )

n="$( family_crates | wc -l )"
if [ ${#absent[@]} -gt 0 ] || [ ${#unrun[@]} -gt 0 ]; then
  fail "${#absent[@]}/${n} without a manual plan, ${#unrun[@]} with a plan but no dated run record"
fi
pass "all ${n} crates carry a dated manual run record"
