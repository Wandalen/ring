# Lifecycle: The Consumer Over a Ring's Life

### Scope

**Purpose:** Trace a `Consumer` from construction through steady state to the
end of its borrow, and establish what it carries across each boundary.

**Responsibility:** The `Consumer` value's own lifecycle — construction, the
poll loop, reconstruction, and destruction.

**In Scope:** `Consumer::new`'s non-initialisation; the steady-state loop; what
happens when the `Consumer` is dropped and rebuilt; the wrap point.

**Out of Scope:** A sequence's lifecycle — that is
[`001`](001_a_sequence_from_published_to_committed.md). The lifetime parameter's
type-level meaning, which is
[`type/002`](../type/002_the_lifetime_on_consumer.md).

---

## Four Phases, and Only One Has State

| Phase | Operation | State changed |
|-------|-----------|---------------|
| Construct | `Consumer::new( &cursor, barrier )` | **none** |
| Poll | `available()` / `available_up_to()` | none |
| Advance | `commit()` / `commit_available()` | the borrowed cursor |
| Drop | — | **none** |

A `Consumer` has no state of its own at any point in its life
([`data_structure/002`](../data_structure/002_two_borrows_and_no_owned_state.md)).
The only durable state is the cursor, which it borrows and does not own.

### CN32 — A `Consumer` Is Disposable, and the Test Suite Proves It by Relying on It

Because construction initialises nothing and destruction releases nothing, a
`Consumer` can be created and dropped freely around a cursor that persists
across all of them. Dropping one loses no position, no buffered state, and no
claim; the next one built over the same cursor resumes exactly where the last
left off.

This is not incidental — the test suite depends on it structurally. The
exhaustive sweep at `consume_test.rs:253` builds a **fresh `Consumer` per
(position, candidate) pair**, inside the inner loop:

```rust
let position_cursor = PaddedCursor::default();
let consumer = Consumer::new( &position_cursor, Barrier::over( &published ) );
consumer.commit( Seq( position ) ).expect( "reachable in one step from zero" );

let accepted = consumer.commit( Seq( candidate ) ).is_ok();
```

And `tests/manual/readme.md § N6` exists specifically to keep it that way,
recording that the first draft reused one consumer across the inner loop and was
therefore silently testing different cases than it asserted — because an
accepted commit moves the position.

So disposability is load-bearing in two opposite ways at once. The sweep needs a
fresh `Consumer` per case for isolation, and it needs the cursor to be *fresh
too* — which is why each iteration also builds a new `PaddedCursor`. A
`Consumer` alone is not enough to reset; the state lives one level down.

That asymmetry is exactly what makes `Consumer::new` not resetting the cursor
correct ([`item/002`](../item/002_the_eight_of_a_consumer.md)). If it reset,
every reconstruction would tell the producer the consumer had read nothing, and
the disposability the suite relies on would become a hazard instead of a
property.

**Cost:** none. Recorded because the property is used by the strongest test in
the crate, guarded by a manual check, and stated as a property nowhere.

---

### CN33 — The Wrap Point Is Unreachable, and Nothing Here Depends on Believing That

`Seq` is a `u64` and every arithmetic operation on it is plain `+` or a
saturating subtraction ([`item/001`](../item/001_the_six_of_a_run.md) CN27). At
one billion publications per second — far beyond any real ring — `u64::MAX` is
about 585 years away.

The interesting part is not the number but what the crate does about it, which
is nothing, and why that is right:

| Approach | Cost | Chosen |
|----------|------|:------:|
| plain `+`, ignore the wrap | none | ✔ |
| `checked_add` and propagate | a `Result` on every accessor including `end()` | ✘ |
| `wrapping_add` and compare modularly | every comparison in the family becomes a distance check | ✘ |
| a `u128` sequence | doubles every cursor and kills the atomic | ✘ |

The second and third are the ones worth naming because both appear in
production ring buffers. Modular comparison in particular is how a `u32`-sequence
ring must work, and it complicates every ordering comparison in `ring_seqno`,
`ring_gating` and `ring_barrier` — for a wrap that a `u64` puts past the
lifetime of the software.

`ring_claim`'s corpus reached the same conclusion for the producer cursor and
recorded it in the same place — its `lifecycle/002`. Two crates, two cursors,
same arithmetic, same reasoning, arrived at independently. Neither states it in
the source.

What *is* worth noting is where the assumption is visible from. `ring_seqno`'s
`distance_to` documents itself as "saturating rather than signed", which is a
deliberate choice that only makes sense on the assumption that the sequences
never wrap — a wrapped pair would produce a saturated `0` that means "not later"
when the truth is "much later". So the no-wrap assumption is load-bearing in
`ring_seqno`, one crate down, and is recorded there as a sentence about signedness
rather than as an assumption about range.

**Cost:** none reachable. Recorded because the assumption is shared by at least
four crates, argued in none of them, and the one place its consequence is
visible describes it as something else.

---

## The Steady State

What a running consumer actually does, per iteration, with costs from
[`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md):

| Step | Atomics | Allocations |
|------|--------:|------------:|
| `available()` | 1 + `n` loads | 0 — was 1 |
| read the caller's buffer | — | — |
| `commit( run.end() )` | 1 + `n` loads, 1 store | 0 — was 1 |
| **per iteration** | **2 + 2`n` loads, 1 store** | **0 — was 2** |

The run is computed twice per drained batch, because both `available` and
`commit` recompute it independently. A caller that uses `commit_available`
instead computes it twice as well — that call recomputes internally, and the
caller had to call `available()` itself to know what to read. What each of those
two computations used to cost was one allocation, `ring_cursor::slowest`'s `Vec`;
`b7e075ca` removed it, so the double computation is now loads only.

For a consumer draining large batches the loads are amortised to nothing. For
one polling an idle ring, both of that poll's costs have since gone: the
allocation with `b7e075ca`, and the store with the emptiness guard on
`commit_available`
([`pitfall/001`](../pitfall/001_commit_available_does_not_call_commit.md) CN44).
An idle poll through `commit_available` is `1 + n` loads and nothing else.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| lifecycle | [001](001_a_sequence_from_published_to_committed.md) | a sequence's states rather than the consumer's |
| data_structure | [002](../data_structure/002_two_borrows_and_no_owned_state.md) | why construction and drop are free |
| item | [002](../item/002_the_eight_of_a_consumer.md) | `new` not resetting the cursor |
| invariant | [002](../invariant/002_the_cursor_only_moves_forward.md) | `§ N6`, the check guarding CN32's property |
| non_functional_requirement | [001](../non_functional_requirement/001_what_the_read_path_costs.md) | the per-iteration costs |

### Sources

| What | Where |
|------|-------|
| `Consumer::new` | `ring_consume/src/lib.rs:236-240` |
| The fresh-per-case sweep | `ring_consume/tests/consume_test.rs:265-271` |
| `§ N6`'s rationale | `ring_consume/tests/manual/readme.md:109-124` |
| `distance_to`'s saturating note | `ring_types/src/id.rs:73-74` |

### Tests

| Claim | Verified by |
|-------|-------------|
| A fresh `Consumer` resumes where the last left off | `consume_test.rs:265-271`, and the sweep's correctness depends on it |
| `new` changes no state | `§ N2`'s grep — no store or load in `new` |
| Reuse across the sweep was a real defect | `§ N6`'s own record of the first draft |
| Two run computations per drained batch, once costing an allocation each | the counting probe, frozen in [`non_functional_requirement/001`](../non_functional_requirement/001_what_the_read_path_costs.md) — 1 each for `available` and `commit`, both now 0 |
