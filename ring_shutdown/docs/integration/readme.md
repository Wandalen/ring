# Integration Doc Definition

### Scope

- **Purpose**: The seam this crate sits on, read from both sides — the four crates beneath it and the crates pointedly absent from that list, then every crate above it that names it.
- **Responsibility**: Per-edge justification, the argument for each absence, and the claims other crates make about this one.
- **In Scope**: `ring_core`, `ring_cursor`, `ring_wait`, `ring_types`; `ring_testkit`'s inbound edge; `ring_poll`, `ring_stats`, `ring_store`, `ring_bench`, `ring_handle`'s edgeless references.
- **Out of Scope**: The family DAG as a whole, beyond this crate's own edges in both directions.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Family Dependency Seam](001_family_dependency_seam.md) | Four edges, each used by a named item, and why `ring_stats` is not a fifth | 🔄 |
| 002 | [What the Family Says About This Crate](002_what_the_family_says_about_this_crate.md) | One inbound edge, five edgeless references, and the three claims among them the code contradicts | 🔄 |

**The split is direction.** `001` reads the seam outward — what this crate
depends on, why, and what it deliberately does not depend on. `002` reads it
inward — what depends on this crate, what merely talks about it, and whether the
talk is true.

They are separate because the two directions have different evidence. Outward is
a manifest this crate owns and a `cargo tree` that either terminates where the
document says or does not. Inward is five other crates' prose, none of which this
crate can compile, and one manifest entry it does not control. Merging them would
give a single document one checkable half and one uncheckable one, and the
uncheckable half is where every error in this definition was found.

The four findings split the same way. `001`'s are about **this crate's own
account of itself**: a crate doc naming three dependencies where the manifest has
four (SD17), and a closure property measured in the one configuration where it
cannot fail (SD18). `002`'s are about **the family's account of this crate**:
three crates documenting a relationship the code does not have (SD19), and this
crate having under-reported its one real consumer for as long as those three
over-reported theirs (SD20).

Every one of the four is a statement about the dependency graph, written as a
sentence, in a family where the graph is five `Cargo.toml` files away. That is
the definition's one recurring defect, and the recipes in both instances are the
proposed fix: spell the claim as a command over the manifests.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/integration
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'outbound edges in the manifest: %s\n' "$( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f&&/^ring_/{ n++ } END{ print n+0 }' ../../Cargo.toml )"
printf 'inbound edges from the family: %s\n' "$( cd ../../..; command grep -rl 'ring_shutdown' ring_*/Cargo.toml | cut -d/ -f1 | command grep -vc '^ring_shutdown$' || true )"
printf 'crates naming it with no edge: %s\n' "$( cd ../../..; command grep -rl 'ring_shutdown' ring_*/src --include='*.rs' | cut -d/ -f1 | sort -u | command grep -vc '^ring_shutdown$\|^ring_testkit$' || true )"
printf 'claims here about other crates: %s\n' "$( command grep -hcoE 'ring_(poll|stats|buffer)' [0-9][0-9][0-9]_*.md | paste -sd+ | bc )"
printf 'tests in this crate naming them: %s\n' "$( cd ../../..; command grep -rlE 'ring_poll|ring_stats|ring_store' ring_shutdown/tests --include='*.rs' | wc -l )"
printf 'checkers that read a manifest: %s\n' "$( cd ../../../bench_harness/gate/corpus; command grep -lc 'Cargo.toml' ./*.py 2>/dev/null | wc -l )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
outbound edges in the manifest: 4
inbound edges from the family: 1
crates naming it with no edge: 5
claims here about other crates: 33
tests in this crate naming them: 0
checkers that read a manifest: 2
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD17 | the crate doc names three dependencies where the manifest has four | **misleading doc** | [`001`](001_family_dependency_seam.md)'s edge table and `src/lib.rs`'s fifth line both say the crate *"Depends on `ring_cursor`, `ring_wait`, `ring_core`"*, while `Cargo.toml`'s `[dependencies]` carries four `ring_*` entries and the omitted one is load-bearing — `use ring_types::{ RingError, WaitKind }` supplies the error every refusal returns and the kind every wait reports, so the list is not merely short but missing the crate the surface's own vocabulary comes from; the defect class is a hand-maintained prose list beside a machine-readable manifest three directories away, the cheapest possible check, and no corpus checker opens a `Cargo.toml` at all (measured: 2 of 11 — see Correction below). |
| SD18 | the closure property is measured in the one configuration where it cannot fail | **misleading doc** | [`001`](001_family_dependency_seam.md) records *zero non-family crates in the dependency closure* and pairs it with a `cargo tree -p ring_shutdown -e normal` command, which is honest as far as it goes; the document then claimed a `--all-features` run *"would find two external crates"*, and it does not — this crate declares no `[features]` section, so `--all-features` here is byte-identical to the default and returns zero, a measurement that cannot distinguish a clean closure from a dirty one; the two crates (`crossbeam-queue v0.3.14`, `crossbeam-utils v0.8.23`) do enter under a workspace-wide `--all-features` build via `ring_core`'s `crossbeam = [ "dep:crossbeam-queue" ]`, which is exactly what `verb/test` runs, so the property is asserted in a configuration that always passes and is false in the configuration the crate is actually tested in. |
| SD19 | three crates document a relationship with this one that the code does not have | **wrong doc** | `ring_poll` declares `pub const PARKING_CRATES : [ &str; 3 ] = [ "ring_barrier", "ring_shutdown", "ring_wait" ];` and asserts its contents in its own doctest — a claim compiled and tested as a claim about a *string* — while this crate states the negation twice ([`api/001`](../api/001_shutdown_surface.md)'s *"Nothing on this surface allocates or parks."* and [`001`](001_family_dependency_seam.md)'s *"the constraint is discharged by not parking"*); separately, `ring_stats::RingStats::reset` is documented *"Used by `ring_shutdown`'s reset"* and `ring_store::Buffer::reset` as *"The reset `ring_shutdown` needs"*, while `ring_shutdown::reset` is five lines that close, discard through the consumer and reopen, names neither crate anywhere in `src/` (measured: 0), and `ring_stats` is listed in [`001`](001_family_dependency_seam.md) as *pointedly absent* with an argument for why the edge would be wrong — so two of the three are doc comments justifying a method's existence by naming a consumer that never arrived, and nothing can catch any of them: no cargo edge, no test in this crate naming them (0), and crate-scoped corpus checkers that never leave one `docs/` tree. |
| SD20 | the one real inbound edge went unrecorded while five prose ones accumulated | n/a — drift | [`001`](001_family_dependency_seam.md) said *"nothing in the family depends on it yet"* and predicted `ring_factory` as the first consumer, while `ring_testkit` has carried a manifest entry and `use ring_shutdown::{ Refusal, Shutdown };` for some time already and `ring_factory` still does not exist as a consumer; the accounting is therefore wrong in both directions at once — this crate under-reported its consumers by one while three others over-reported theirs by one each (→ SD19) — and all four statements are prose about a graph that lives in five `Cargo.toml` files no document reads; the asymmetry is the lesson, since every outbound claim in `001` is tied to a named item and a `cargo tree` command and stayed correct, and the single sentence not derived from a command is the one that went stale, in the same release that added the edge it denies. |

**Correction (2026-09-28):** SD17's parenthetical read "(measured: 0 of 5)".
The corpus has grown from 5 checker scripts to 11 since that count was taken,
and two of the new ones — `addressing_test.py`, `anchors.py` — now contain
the string `Cargo.toml`. Neither is the checker SD17 asks for:
`addressing_test.py` opens crate manifests only as path-resolution fixtures
for testing `addressing.py` itself, and `anchors.py` mentions `Cargo.toml`
only inside a docstring explaining why its static-anchor check skips a
shell-templated path — neither compares a crate's prose dependency claim
against its manifest. The count is 2 of 11, not 0 of 5, and the gap SD17
names is otherwise unchanged.

**Correction (2026-09-28):** SD18's parenthetical named `crossbeam-queue
v0.3.13` and `crossbeam-utils v0.8.22`. Both have since taken a routine patch
bump — `v0.3.14` and `v0.8.23` — per
[`001`](001_family_dependency_seam.md)'s regenerated measurement. The version
numbers moved; which two crates enter the closure, and under which feature,
did not.
