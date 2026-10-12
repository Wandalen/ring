# Why one store made a ring 4x slower, and how we found out

An interactive explainer of the investigation behind #41 (`ring_spsc`'s consumer caches the producer cursor) and #38
(`ring_slot::CopySlot`): cache lines and coherence, the SPSC ring's cursor protocol, how the ring's address turned a
benchmark into a lottery, the chase between producer and consumer and the full-ring lockstep, every measurement at
every stage with its trade-offs, and the comparison with the `rtrb` crate that ends in a regime rather than a code
difference: a consumer with no work per record. #41's two-thread loss on Zen 3 (#55) lives in the same regime.

> **Depends on #41 and #38.** The page quotes `ring_spsc`'s consumer cursor cache and `ring_slot::CopySlot` as source.
> Both exist on those PRs' branches, and this page is stacked after them so that the tree it ships in has the code it
> quotes. Until they merge, `master` does not.

## View it

- Open [`index.html`](index.html) in a browser. One self-contained file: no server, no network, dark and light mode.
- Without cloning: `https://htmlpreview.github.io/?https://raw.githubusercontent.com/Wandalen/ring/master/docs/explainers/cursor_cache/index.html`
  (swap `master` for a branch name to preview a change).

Seven sections. Six have controls the reader drives: a two-core cache-line simulator; an 8-slot ring that counts
which cursor lines each push and pop touches, with the consumer cache on or off; the `Ring` byte map under an
address slider and the environment-variable chart that flips the result; a chase simulator with the fitted lockstep
model and the measured interventions; every table from the records as charts, including 128-byte padding against
both placements and Bench Compare on the rebased heads, then the trade-offs and the per-PR change list; and ring
against `rtrb`, with a stepper through the differences removed one at a time, the kperf counters, and two
consumer-work sliders (the M4 sweep and #55's four CI runners). The seventh holds the open questions and a glossary.

## Where the numbers come from

Every figure is inlined from [`data.json`](data.json), which holds the measurements with their provenance: the
investigation on one Apple M4 Pro (2026-10-09/10); the 128-byte padding, `rtrb` and consumer-work runs on the same
machine (2026-10-11/12, kperf hardware counters on 10-12); and GitHub CI x86 runners (AMD EPYC 7763 = Zen 3, AMD EPYC
9V45 = Zen 5, Intel Xeon Platinum 8573C) on 2026-10-12, with commits and run where known. The records they reproduce
live with the crates:
`ring_spsc/docs/decisions/002_the_consumer_caches_the_producer_cursor.md`,
`ring_slot/docs/benchmarks/001_the_tag_store_against_an_untagged_slot.md` (with its addendum) and
`ring_slot/docs/decisions/002_copy_slot_stores_the_record_bare.md`. Anything inferred rather than measured is
labelled a hypothesis on the page. The kperf counters count misses per record but cannot say which line missed, so the
mechanisms stay inferred.

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
