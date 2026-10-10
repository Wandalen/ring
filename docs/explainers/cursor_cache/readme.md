# Why one store made a ring 4x slower, and how we found out

An interactive explainer of the investigation behind #41 (`ring_spsc`'s consumer caches the producer cursor) and #38
(`ring_slot::CopySlot`): cache lines and coherence, the SPSC ring's cursor protocol, how the ring's address turned a
benchmark into a lottery, the chase between producer and consumer and the full-ring lockstep, and every measurement
at every stage with its trade-offs.

## View it

- Open [`index.html`](index.html) in a browser. One self-contained file: no server, no network, dark and light mode.
- Without cloning: `https://htmlpreview.github.io/?https://raw.githubusercontent.com/Wandalen/ring/master/docs/explainers/cursor_cache/index.html`
  (swap `master` for a branch name to preview a change).

Five sections, each with controls the reader drives: a two-core cache-line simulator; an 8-slot ring that counts
which cursor lines each push and pop touches, with the consumer cache on or off; the `Ring` byte map under an
address slider and the environment-variable chart that flips the result; a chase simulator with the fitted lockstep
model and the measured interventions; and every table from the records as charts, then the trade-offs, the per-PR
change list, open questions and a glossary.

## Where the numbers come from

Every figure is inlined from [`data.json`](data.json), which holds the measurements from the investigation on one
Apple M4 Pro (2026-10-09/10) with their provenance. The records they reproduce live with the crates:
`ring_spsc/docs/decisions/002_the_consumer_caches_the_producer_cursor.md`,
`ring_slot/docs/benchmarks/001_the_tag_store_against_an_untagged_slot.md` (with its addendum) and
`ring_slot/docs/decisions/002_copy_slot_stores_the_record_bare.md`. Anything inferred rather than measured is
labelled a hypothesis on the page; no hardware performance counters were available on that machine.

## Edit and rebuild

| File | Responsibility |
|------|-----------------|
| `index.html` | The built page. Regenerate, never edit by hand |
| `sections/NN-slug.html` | One `<section>` per chapter: scoped `<style>`, content, one IIFE `<script>`, data inlined as a constant |
| `shell.html` | Design tokens, type scale, navigation, the `<!-- NAV -->` and `<!-- SECTIONS -->` slots |
| `build.py` | `python3 build.py [--only NN]` concatenates the sections into `index.html` and prints sanity checks |
| `data.json` | The measurements; a section copies what it needs rather than fetching, so the page opens from `file://` |
| `snap.swift` | `swift snap.swift index.html <outdir> 1440 1100 dark\|light <sectionId,...>` renders WebKit screenshots and reports JavaScript errors, for checking a change without a browser at hand |

Vanilla HTML, CSS and JavaScript only. A new number goes into `data.json` first, then into the section that shows it.
