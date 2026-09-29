#!/usr/bin/env bash
# G16 — every citation resolves, and the two spellings of each finding agree.
#
# Relative links resolve to files on disk; every test named in a `### Tests`
# table exists in the crate's own tests/; and each Module Index findings row
# states the same Subject and Tier as the definition readme that owns it.
#
# The last is the load-bearing one, and the column order is why. The index
# writes `| ID | Finding | Subject | Reachable | Where |` and the definition
# readme writes `| ID | Subject | Tier | Finding |` — the same two facts in
# different positions, so nothing keeps them aligned by copy-paste and drift
# shows up as a mismatch rather than as two copies of one mistake.
set -uo pipefail
GATE=G16
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist
run_corpus_checker citations.py "links, cited tests, and index/readme agreement"
