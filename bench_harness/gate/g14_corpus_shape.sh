#!/usr/bin/env bash
# G14 — every crate carries the declared doc definitions, clears the instance
# and finding floors, and states the same three counts in all three places they
# appear: the tree, each definition readme's own findings table, and the Module
# Index that claims to enumerate both.
#
# The third assertion is the one that does the work. Counting directories is a
# check a fresh `mkdir` satisfies, and G2 already demonstrated what a
# presence-only doc gate is worth — it read fourteen finished crates as
# unimplemented and, before that, passed fourteen skeletons. Requiring a count
# to agree with two independently written restatements of itself fails the
# moment a finding lands in a document and not in the index, which is this
# corpus' actual failure mode rather than a hypothesised one.
set -uo pipefail
GATE=G14
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist

# Fix(g14_hardcoded_floor_message): the REACHED/NOT REACHED message used to
# spell the floors as the literal string "13 definitions, 26 instances, 52
# findings" — ring's own numbers on the day this gate was written — regardless
# of what corpus_standard.txt actually declares. Root cause: every sibling
# corpus gate (g15-g17, g20, g21) describes its check qualitatively for
# exactly this reason, but g14 alone restated the standard's own numbers as a
# literal, duplicating a value common.sh's corpus_standard() comment already
# names as living in exactly one place ("the standard and the instrument are
# one artifact and neither can contradict the other visibly"). Pitfall:
# raising min_findings from 52 to 53 — the file's own comment names this as an
# expected future edit — changes what shape.py enforces but not what this
# message prints, so a passing run keeps reporting "52 findings" after the
# real floor moved to 53, silently misreporting what was actually graded.
# Confirmed by sandboxed run: pointing this same script at a standard
# declaring min_definitions 2 / min_instances 4 / min_findings 4 still printed
# "13 definitions, 26 instances, 52 findings" on REACHED.
std="$( corpus_standard )"
min_defs="$( decl_lines "$std" | awk '$1 == "min_definitions" { print $2 }' )"
min_inst="$( decl_lines "$std" | awk '$1 == "min_instances" { print $2 }' )"
min_find="$( decl_lines "$std" | awk '$1 == "min_findings" { print $2 }' )"
run_corpus_checker shape.py \
  "${min_defs} definitions, ${min_inst} instances, ${min_find} findings, counts agreeing" \
  "$std"
