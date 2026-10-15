#!/usr/bin/env bash
# G20. Every finding on a reachable tier has been acted on or declined, with a
# reason, inside its own section.
#
# G14-G17 between them hold the corpus' shape, its recipes, its citations and
# its vocabulary. A family can pass all four while every hazard it names is
# still there, because none of them asks whether anything was *done*. That is
# the only question a findings layer exists to answer.
#
# The `Now prints:` clause keeps this from being a Proxy Trap. A disposition
# column that takes the author's word measures that someone typed `applied`,
# never that a correction landed. Requiring `applied` to quote a literal out of
# a `Live output:` block in the same file chains it to the source, because G15
# independently holds every such block equal to what its recipe prints today.
# Source -> recipe -> quoted block -> disposition, with G15 holding the middle
# link and this gate the last.
#
# It reports every reachable tier at once. Passing `--tier` narrows it to one,
# which is what S7a-S7d do while working through them a tier at a time. The
# family verdict is the unnarrowed run.
set -uo pipefail
GATE=G20
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist
run_corpus_checker disposition.py "every reachable-tier finding acted on or declined" "$( corpus_standard )"
