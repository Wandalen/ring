#!/usr/bin/env bash
# G17. The tier vocabulary is closed and the finding ids are a clean set.
#
# corpus_standard.txt declares thirteen tier strings, taken from the corpus, not
# composed for it. The bolding is part of the string. A reachable tier is
# bolded, a record tier is not, and a hybrid such as `n/a — latent hazard`
# claims both at once. Four are live in ring_publish's Module Index, so this
# gate's first real hit is an existing defect, not a seeded one.
#
# Ids must also be unique and contiguous from 1. A gap means a finding was
# deleted without renumbering, so every cross-reference past it now points one
# place off. A duplicate means two findings answer to one name.
set -uo pipefail
GATE=G17
source "$( dirname "${BASH_SOURCE[0]}" )/common.sh"
assert_declared_crates_exist
run_corpus_checker vocabulary.py "tiers from the declared set, ids unique and contiguous" "$( corpus_standard )"
