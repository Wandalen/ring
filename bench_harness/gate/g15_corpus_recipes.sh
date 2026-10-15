#!/usr/bin/env bash
# G15. Every published recipe still prints what it says it prints.
#
# Executes every ```sh block in the corpus. Instance files quote their output
# and must match it byte for byte. Definition readmes quote nothing by house
# style and must exit 0 instead. Quoting a Module Index's own counts would make
# it go stale the instant a finding is added anywhere in the crate, a worse
# failure than the one quoting catches.
#
# This is the gate that raises the price of a finding. A row that restates the
# paragraph above it satisfies every count G14 takes. It has nothing measured
# behind it to quote, and no recipe here to run.
set -uo pipefail
GATE=G15
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist
run_corpus_checker recipes.py "every recipe reproduces its quoted output"
