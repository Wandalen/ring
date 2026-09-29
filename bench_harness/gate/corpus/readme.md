# corpus

The checkers behind G14-G17, G20-G21. Python rather than bash, because all six parse
markdown tables and fenced blocks and one of them diffs multi-line command
output — work bash does badly enough that the shell versions of these grew
their own quoting bugs faster than they found corpus defects.

Each is invoked by its gate script with one or more `docs/` directories and
exits non-zero on any problem. They print what they found and take no verdict:
the REACHED / NOT REACHED line is the gate's, so the tier vocabulary, the count
floors, and which crates are in scope all stay in `declared/` where a person can
change them without opening a checker.

| File | Responsibility |
|------|-----------------|
| `citations.py` | G16 — link resolution, cited-test existence, index/readme agreement |
| `disposition.py` | G20 — every reachable-tier finding acted on or declined, with printed evidence |
| `addressing.py` | G21 — no recipe cites source by absolute line number |
| `addressing_test.py` | Tests `addressing.py`'s crate resolver against every crate the live tree holds |
| `anchors.py` | Static per-clause dead-anchor check for multi-clause awk recipes — no gate wired yet |
| `anchors_test.py` | Tests `anchors.py` against `ring_slot`'s own pre-repair git history |
| `corpus_lib.py` | Fence-aware parsing every checker shares — definitions, instances, findings, both table shapes |
| `recipes.py` | G15 — executes every recipe and diffs it against the output its document quotes |
| `scoreboard.py` | One row per crate: counts, floors, and shortfall — the plan's observable demo, not a gate |
| `shape.py` | G14 — declared definitions, count floors, and agreement across the three places each count appears |
| `vocabulary.py` | G17 — closed tier set, unique and contiguous finding ids |

## Why one shared library

All six answer questions about the same three structures — the definition
directories, the finding rows inside each definition readme, and the Module
Index that restates both. Six private parsers of that markdown would not fail
when they disagreed; they would report different counts for one tree, which is
worse than failing.

The fence-awareness in particular cannot be optional. This corpus publishes
recipes that grep for the row shapes these parsers match: `ring_registry`'s
pitfall readme contains `command grep -n '^| F[0-9] |'` inside a fenced block,
and a parser that reads it as prose finds a findings table in a definition that
does not exist.

## Why the control fixture is not a test

`run_all.sh --control` points each gate at `declared/ring/corpus_control/`, and
under it the polarity inverts: REACHED means the gate found the defect seeded
for it, and a gate reporting the fixture clean has failed.

That is not a test of these checkers, it is half of them. Every one passes by
finding nothing, and the repository's own conventions rulebook names that shape
under *Documentation : Executable Recipes* — an empty result means "the property
holds" and "the command is broken" equally well, and every way one of these rots
expresses itself as silence. Each gate sees only its own fixtures, and each of
those is graded alone against what its name declares, because pointed at every
fixture at once G16 reports a defect whichever of its three assertions is
broken: deleting a definition for G14 also breaks five links, and G17's hybrid
tier is also a disagreement between two tables.
