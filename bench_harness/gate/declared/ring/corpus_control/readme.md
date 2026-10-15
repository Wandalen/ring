# corpus_control

The control arm for the corpus gates. Every one of G14–G17, G20–G21 passes by finding
nothing, and an empty result means "the property holds" and "the checker is
broken" equally well. So each gate is also run against a fixture here carrying
one seeded defect, and **a gate that reports clean against its own fixture has
failed** regardless of what it reports against the family.

One fixture per defect, never one fixture with several. A single fixture
carrying four defects at once would let any gate failing for any reason read as
all four working — that proves something fires, never that each detector fires.
G21 carries three for the same reason G20 carries four: a line address has more
than one spelling, and one fixture holding all of them would let the `sed` half
alone passing read as the whole rule enforced. That is not hypothetical, and it
has now happened twice. G21 shipped knowing only `sed`, reported the family
clean, and was believed until G15 turned up four recipes that had slid onto
different code through the `awk` spelling it could not see. Its `sed` detector
then turned out to require a closing quote right after the command letter, so a
program of several commands — `sed -n '161,166p;177,182p;199,204p'` — matched
nothing, and 148 of those lived across 15 crates while the gate reported REACHED
for all 33.

**Separate fixtures are only half the mechanism; grading them separately is the
other half.** Every fixture for a gate is run on its own and graded against what
its own name declares — a `_clean` fixture must report nothing, every other
fixture must report something. Run together, the verdict collapses to *did any
of these fire*, which the gate can satisfy from the one detector that already
works: `g21_line_addressed` held this control green on the multi-command form's
behalf for its whole life, and `g20_clean` — documented above as the proof G20
can pass — was never actually graded, because one non-empty result anywhere in
the set was the entire test.

G20 has four distinct labels and so carries four fixtures plus a fifth that is
clean, because a gate proven only able to fail is as untrustworthy as one proven
only able to pass: at the moment G20 was written every finding in the family
lacked a disposition, and without `g20_clean` there was no evidence the gate
could ever report otherwise.

G21 carried the identical gap. Its three fixtures are all defect fixtures —
each proves the detector catches one spelling of a line address — and none of
them is a `_clean` case proving the detector leaves a legitimate,
content-anchored recipe alone. Without one, `run_all.sh --control g21` could
never clear the `saw_clean` check in `common.sh`'s `run_corpus_checker`, so the
control run reported NOT REACHED regardless of whether the detector itself was
correct. `g21_content_addressed_clean/` closes it, carrying a content-anchored
`sed` range, a computed-offset `awk` call, and a non-numbering `grep` — the same
forms `g21_awk_addressed` and `g21_multi_addressed` already carry beside their
own seeded defect to prove G21 must spare them.

| Directory | Responsibility |
|-----------|-----------------|
| `g14_missing_definition/` | G14 must name the absent definition, not merely exit non-zero |
| `g14_definition_present_clean/` | G14 must report zero when every declared definition is present |
| `g14_agree_orphan_heading/` | G14 must catch a finding heading with no definition-readme row and no Module Index row |
| `g14_thin_instances/` | G14 must catch one definition under the per-definition instance floor even while the corpus' aggregate totals clear every floor |
| `g14_below_floor/` | G14 must catch the total finding count under the declared floor even while every definition and instance count is clean |
| `g15_stale_quoted_output/` | G15 must print the diff between a recipe's output and the block quoting it |
| `g15_missing_item_kind_readme/` | G15 must name a missing `item/<kind>/readme.md`, not stop at `item/`'s own |
| `g15_item_kind_present_clean/` | G15 must report zero when `item/<kind>/readme.md` is present and correct |
| `g15_crate_readme_recipe_fails/` | G15 must name a crate-level `docs/readme.md` recipe that exits non-zero, not skip the file entirely |
| `g15_crate_readme_present_clean/` | G15 must report zero when the crate-level `docs/readme.md`'s own recipe is present and exits 0 |
| `g16_tier_disagreement/` | G16 must catch a master row whose Tier alone disagrees with its owner's |
| `g16_tier_agreement_clean/` | G16 must report zero when every Tier agrees with its owner's |
| `g17_hybrid_tier/` | G17 must refuse a tier string outside the declared set |
| `g17_tier_in_declared_set_clean/` | G17 must report zero when every tier string is in the declared set |
| `g20_missing_disposition/` | G20 must report a reachable finding carrying no disposition at all |
| `g20_unevidenced_application/` | G20 must refuse `applied` whose quoted literal no Live output block prints |
| `g20_shrugged_decline/` | G20 must refuse a decline too short to be a ruling |
| `g20_double_disposition/` | G20 must refuse a finding ruled on twice |
| `g20_clean/` | G20 must report zero here — the proof it can pass, not only fail |
| `g21_line_addressed/` | G21 must flag a line address into a file and spare a pipeline pager |
| `g21_awk_addressed/` | G21 must flag the same address in the awk spelling, not just the sed one |
| `g21_multi_addressed/` | G21 must flag an address in a multi-command sed program, and spare what replaces it |
| `g21_content_addressed_clean/` | G21 must report zero when every address is content-anchored, not by line |
| `standard.txt` | The floors the fixtures are graded against, deliberately lower than the family's |
