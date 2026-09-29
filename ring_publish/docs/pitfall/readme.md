# pitfall

Two ways to break this crate, and neither is caught by anything. One is the
caller's — publishing a range that was never claimed, which hangs a producer
forever with no message. The other is the implementer's — advancing publication
when the slot is taken rather than when it is written, which publishes
uninitialised memory and passes every single-threaded test.

They are recorded together because they are the same failure reached from two
sides: the crate's correctness rests entirely on the gap between *claimed* and
*published* staying open for exactly the duration of one payload write, and each
pitfall closes that gap at a different end.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Publishing a Range You Never Claimed](001_publishing_a_range_you_never_claimed.md) | PB35, PB36 — the four break modes, `ring_claim::Claim` as the type that would enforce three of them, and the `Drop` guard the family built twice elsewhere |
| 002 | [Conflating the Two Cursors](002_conflating_the_two_cursors.md) | PB37, PB38 — the founding hazard, the four structs that store cursors, and the three designs' answers to "claimed but unwritten" |

### The Two Sides of One Gap

| | Caller-side (001) | Implementer-side (002) |
|--|-------------------|------------------------|
| The mistake | publish a range you do not own | publish at claim time instead of write time |
| Symptom | a producer spins forever, silently | a consumer reads unwritten memory |
| Severity | a hang — recoverable by restart | undefined behaviour |
| Visible single-threaded | no — the hang needs a peer | no — the write completes before the read |
| Visible under real threads | as a timeout, eventually | only in a nanosecond window |
| Caught by the type system | no — `publish` takes `( Seq, usize )` | no — no type holds both cursors |
| Caught by a test | no — a test of it would hang | **yes** — the loom model, if `§ P1` is run |
| Documented at | `src/lib.rs:183-189`, under `# Panics` | `src/lib.rs:18-27`, the module doc's first section |

The asymmetry in the last two rows is the whole story. The implementer-side
pitfall is the one the crate exists to prevent, so it got a memory model and a
mutation check; the caller-side one is outside what a crate holding a single
cursor can reach, so it got a paragraph.

### Why Neither Has a Runtime Guard

`Publisher` holds one `PaddedCursor` and nothing else. It cannot see:

| To catch | It would need |
|----------|---------------|
| a range never claimed | the claimed cursor — which lives in `ring_claim`, a crate this one does not depend on |
| a double publication | per-claim state — which is what `ring_mpsc`'s per-slot stamps are |
| a slot published unwritten | the slot — which is what `Reserved`/`Reservation` hold and this type does not |

Every one of the three ends at the same boundary, and the boundary is deliberate
([`decisions/001`](../decisions/001_refused_rather_than_reordered.md)): a
publisher that knew about claims, slots or rings would not be a primitive
testable without them.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# every struct in the family that stores a cursor
grep -rnE '^\s*[a-z_]+ *: *(Vec< *)?PaddedCursor' ring_*/src/*.rs

# the guard shape this crate cannot adopt, and who has it
grep -rnE '^impl.*Drop for' ring_*/src/*.rs
grep -c Drop ring_publish/src/lib.rs

# the type that would carry the precondition, and the graph that permits it
grep -nE '^ring_' ring_claim/Cargo.toml ring_publish/Cargo.toml
# zero is the expected answer, and `grep -c` exits 1 when it reports it
grep -c ring_claim ring_publish/tests/publish_test.rs || true
```

| | Value |
|--|------:|
| `PaddedCursor` fields family-wide | 5 |
| Structs holding them | 4 |
| …that hold **two** cursors | 1 — `CursorPair`, and they are producer/consumer |
| …that hold the claimed **and** published pair | **0** |
| `Drop` impls in the family | 4 |
| …in a Tier 5 primitive | **0** |
| `Drop` impls in `ring_publish` | 0 |
| Break modes of the claim precondition | 4 |
| …caught by anything at the point of the mistake | **0** |
| `Claim` accessors taking `self` by value | 7 |
| `ring_claim` uses in `tests/publish_test.rs` | 0 |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB35 | `ring_claim` | n/a — inconsistency | `ring_claim::Claim` is exactly the two values `publish` takes, carries a `must_use` naming this crate's failure from the other side, and is not on the signature; the dependency-cycle objection is false and this crate's own manifest says so |
| PB36 | `ring_claim` | n/a — observation | `Claim` is `Copy`, and all seven of its accessors take `self` by value, so this crate's own tests depend on the derive to compile; making it linear is a signature change to all seven, not just a dropped derive |
| PB37 | family | n/a — observation | Five cursor fields across four structs store a `PaddedCursor`, the only aggregate among them aggregates *consumers*, and `CursorPair` pairs producer with consumer; no type in the family holds the claimed and published pair |
| PB38 | family | n/a — observation | Three designs, three answers: two cursors and caller discipline here, `Claimer` plus per-slot stamps in `ring_mpsc`, and nothing at all in `ring_spsc` where one producer collapses the distinction |
