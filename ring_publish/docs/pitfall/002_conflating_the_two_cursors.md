# Pitfall: Conflating the Two Cursors

### Scope

- **Purpose**: Record the crate's founding hazard — advancing publication at claim time — and show that the split it depends on is held by no type anywhere in the family.
- **Responsibility**: State what conflation produces, census every cursor in the family, show the one type that pairs cursors pairs a different two, and give the three representations the family actually built.
- **In Scope**: The relationship between `ring_claim::Claimer`'s cursor and `Publisher`'s.
- **Out of Scope**: Caller-side misuse of a correct split — see [`pitfall/001`](001_publishing_a_range_you_never_claimed.md).

### The Hazard, As the Crate States It

`src/lib.rs:18-27` is the module documentation's first section and the reason the
crate exists:

> `ring_claim` advances a cursor when a producer *takes* a range; this crate
> advances a different one when the producer has *finished writing* it. Between
> the two, the slot is claimed and unwritten — and the feature's central
> requirement is that a consumer never sees it.
>
> Conflating the two cursors is not a subtle bug. It publishes uninitialised
> memory, it passes every single-threaded test (where the write completes before
> anything can read), and it fails only under load.

Three properties in that second paragraph, and each is worse than the last:

| Property | Consequence |
|----------|-------------|
| Publishes uninitialised memory | undefined behaviour, not a wrong value |
| Passes every single-threaded test | the whole class of cheap tests is blind to it |
| Fails only under load | the reproduction requires the conditions least available at review |

The field doc repeats the warning at the one place a reader would need it —
`src/lib.rs:71-72`, on the `cursor` field itself:

> Separate from `ring_claim::Claimer`'s cursor on purpose — see the module
> documentation on why conflating them publishes unwritten slots.

### Why Single-Threaded Tests Cannot See It

A conflated implementation would advance publication inside `claim`. On one
thread the sequence is: claim (frontier moves), write the slot, read it back.
The read happens after the write in program order, so it observes the written
value. Nothing is wrong to observe.

Two threads is what separates them, and *only* if the reader looks in the window:

```
producer:   claim(4) ──── frontier=4 ──── write slot 0..4 ────
consumer:                    └─ available() → 0..4 ─ read ─┘
                                    ^^^^^^^^^^^^^^^^^^^^^^^ the window
```

The window is the duration of one payload write —
[`lifecycle/001`](../lifecycle/001_a_slot_from_claim_to_visibility.md)'s 2 → 3
transition. For a `u64` that is nanoseconds. So even a threaded test reproduces
the bug only when the scheduler lands the consumer's read inside a few
nanoseconds, which is why
[`workaround/001`](../workaround/001_the_loom_seam_and_its_only_user.md)'s loom
model exists: it does not wait for that interleaving, it enumerates it.

`tests/manual/readme.md § P1` is the check that the model would actually catch a
conflation — it mutates the source to publish at claim time and records that the
model fails. Without P1, a passing loom run means only that loom ran.

### PB37 — Five Cursor Fields in Four Structs, and the Only Aggregate Among Them Aggregates Consumers

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE '^\s*[a-z_]+ *: *(Vec< *)?PaddedCursor' ring_*/src/*.rs
```

Live output:

```
ring_claim/src/lib.rs:  cursor : PaddedCursor,
ring_cursor/src/lib.rs:  producer : PaddedCursor,
ring_cursor/src/lib.rs:  consumer : PaddedCursor,
ring_cursor/src/lib.rs:      producer : PaddedCursor::new( Seq::ZERO ),
ring_cursor/src/lib.rs:      consumer : PaddedCursor::new( Seq::ZERO ),
ring_cursor/src/lib.rs:      producer : PaddedCursor::new( Seq::ZERO ),
ring_cursor/src/lib.rs:      consumer : PaddedCursor::new( Seq::ZERO ),
ring_gating/src/lib.rs:  cursors : Vec< PaddedCursor >,
ring_publish/src/lib.rs:  cursor : PaddedCursor,
```

Four structs in all 33 crates store a `PaddedCursor`, across five fields — two
hold one cursor each, one holds two, and one holds a collection:

| Field | Crate | Shape | Means | Advanced by |
|-------|-------|-------|-------|-------------|
| `Claimer::cursor` | `ring_claim` | one | claimed — a range was **taken** | `Claimer::claim` |
| `Publisher::cursor` | `ring_publish` | one | published — a range was **written** | `Publisher::try_publish` |
| `CursorPair::producer` | `ring_cursor` | one | produced — one thread's progress | its owner |
| `CursorPair::consumer` | `ring_cursor` | one | consumed — reading progress | its owner |
| `GatingSet::cursors` | `ring_gating` | `Vec` | one commit position **per consumer** | `Consumer::commit` |

The first two are the pair this pitfall is about. They live in **two crates, two
structs, and are both named `cursor`** — the field name carries none of the
distinction the module documentation spends ten lines establishing, in either
crate.

The shape column is the finding. The family has exactly one type for holding
*several* cursors, `GatingSet`, and what it holds is **consumers** — because the
consumer side is genuinely plural and needs a slowest-of. The producer side is
plural too, but along a different axis: not many producers each with a cursor,
but one producer path with **two phases**. Nothing in the family has a type for
that.

The asymmetry has a visible cost elsewhere. `ring_barrier::Barrier::over` takes a
bare `&[ PaddedCursor ]` rather than a `GatingSet`, and
`ring_barrier/src/lib.rs:40-44` records why: there is no way to move an
existing cursor into a set that owns its own, so a publisher's cursor *"could
never be depended on at all"* — a signature changed specifically to let this
crate's cursor be waited on
([`data_structure/002`](../data_structure/002_the_four_cursors_of_the_handshake.md)).

And `CursorPair`, the family's only type that holds two cursors at once, pairs
**producer with consumer** — not claimed with published. So:

| Question | Type that could answer it |
|----------|---------------------------|
| Are the producer and consumer cursors on distinct cache lines? | `CursorPair::on_distinct_lines` |
| Is the published cursor never ahead of the claimed cursor? | **none exists** |

`CursorPair`'s own assertion is about **layout** — false sharing. The
claimed/published relationship is about **ordering**, it is the invariant the
whole feature turns on, and no type in the family can express it, because no
type in the family holds both cursors.

Nothing is wrong with that; it is a consequence of
[`decisions/001`](../decisions/001_refused_rather_than_reordered.md)'s split into
independently testable primitives. It is worth recording because it locates the
invariant precisely: it lives in the *assembly*, and the only assembly that
performs it is `tests/handshake_test.rs`.

These five are not the four that
[`data_structure/002`](../data_structure/002_the_four_cursors_of_the_handshake.md)
enumerates. That file counts the four *roles* the handshake runs on — claimed,
published, consumer position, capacity — of which capacity is not a cursor at
all and consumer position is one element of the `Vec` above. This census counts
*storage*: which structs own a `PaddedCursor`. The two lists overlap on three
entries and answer different questions.

### PB38 — Three Designs in the Family, Three Different Answers to "Claimed but Unwritten"

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*(pub )?[a-z_]+ *: *[A-Z]' ring_mpsc/src/lib.rs | sed -n '1,4p'
grep -E '^\s*(pub )?[a-z_]+ *: *[A-Z]' ring_spsc/src/lib.rs | sed -n '1,3p'
```

Live output:

```
  slots : Buffer< UnsafeCell< S > >,
  stamps : Box< [ AtomicSeq ] >,
  consumers : GatingSet,
      slots : Buffer::new( capacity ),
  slots : Buffer< UnsafeCell< S > >,
  cursors : CursorPair,
      slots : Buffer::new( capacity ),
```

| Design | Claimed state lives in | Published state lives in | Window closed by |
|--------|------------------------|--------------------------|------------------|
| `ring_claim` + `ring_publish` | a cursor in one crate | a cursor in another | the caller, by discipline |
| `ring_mpsc` | `Claimer`'s cursor — **this family's `ring_claim`** | `stamps : Box< [ AtomicSeq ] >`, one per slot | `Reserved`'s `Drop` |
| `ring_spsc` | nothing — one producer | `CursorPair::producer` | `Reservation`'s `Drop` |

Three observations, in order of how much they change the reading:

1. **`ring_mpsc` kept the claim half and replaced the publish half.**
   `Ring< S >` holds `slots`, `stamps` and `consumers` — no published cursor at
   all — while `Ends` holds a `Claimer< 'a >` from `ring_claim`
   (`ring_mpsc/src/lib.rs:655-665`) and documents it as *"the claim cursor that
   every producer shares"*. So the split is not what `ring_mpsc` rejected; the
   *published cursor* is. Per-slot stamps make publication order-free, which is
   the problem [`decisions/001`](../decisions/001_refused_rather_than_reordered.md)
   defers to it.

2. **`ring_spsc` does not have the pitfall.** One producer means claimed and
   published are the same thread's progress, so `cursors.producer()` can be
   advanced at write-completion and there is no second producer to be ordered
   against. `Reservation::drop` stores `self.seq.next()` to that one cursor
   (`ring_spsc/src/lib.rs:792-795`) — the store *is* both operations.

3. **Both of them close the window with `Drop`, and this crate cannot.** The
   guard needs a ring to publish into; `Publisher` holds a cursor and no ring
   ([`pitfall/001`](001_publishing_a_range_you_never_claimed.md) § PB36).

So of the family's three answers, one is *keep two cursors and trust the caller*
— this crate — and it is the only one with no runtime or compile-time mechanism
holding the window shut. That is the honest position, and it is the price of the
primitive being usable without a ring.

### What Actually Checks the Split

| Mechanism | Checks | Scope |
|-----------|--------|-------|
| `tests/handshake_test.rs:77-165` under `loom` | a consumer never observes a claimed-unwritten slot, over every interleaving | one claim, one drain, capacity 2 |
| `tests/manual/readme.md § P1` | that the model above would fail if publication moved to claim time | run by hand |
| `tests/handshake_test.rs:274-328` | the same property under real threads, 20 000 items | probabilistic |
| the type system | **nothing** | — |
| a runtime assertion | **nothing** | — |

The first row is the whole guarantee, and it is worth being exact about its size:
the loom model runs capacity 2 with one claim and one drain
([`lifecycle/002`](../lifecycle/002_the_four_operation_handshake.md) § PB28). It
proves the *mechanism* is sound at the smallest scale that can exhibit the bug,
not that a larger assembly using it is.

The second row is what makes the first row evidence rather than ceremony. A model
that passes and would also pass on a broken implementation proves nothing, and
P1 is the only thing standing between this crate and that state.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_compare_exchange_that_refuses.md](../algorithm/001_the_compare_exchange_that_refuses.md) | The operation that advances the second cursor and only the second |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_padded_cursor_and_nothing_else.md](../data_structure/001_one_padded_cursor_and_nothing_else.md) | The one cursor this crate owns |
| [../data_structure/002_the_four_cursors_of_the_handshake.md](../data_structure/002_the_four_cursors_of_the_handshake.md) | All four, and how the handshake assembles them |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_refused_rather_than_reordered.md](../decisions/001_refused_rather_than_reordered.md) | The split that puts the two cursors in two crates |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_two_crates_that_declined.md](../integration/002_the_two_crates_that_declined.md) | The two designs that answered this differently |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_is_published_is_exclusive_of_the_frontier.md](../invariant/002_is_published_is_exclusive_of_the_frontier.md) | The property a conflation would still satisfy |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_slot_from_claim_to_visibility.md](../lifecycle/001_a_slot_from_claim_to_visibility.md) | The 2 → 3 window conflation erases |
| [../lifecycle/002_the_four_operation_handshake.md](../lifecycle/002_the_four_operation_handshake.md) | How large the checked assembly actually is |

### Pitfalls

| File | Relationship |
|------|--------------|
| [001_publishing_a_range_you_never_claimed.md](001_publishing_a_range_you_never_claimed.md) | The guard shape neither crate can adopt here |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_loom_seam_and_its_only_user.md](../workaround/001_the_loom_seam_and_its_only_user.md) | The only mechanism that checks this at all |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:18-27` | The hazard, stated first in the module documentation |
| `ring_publish/src/lib.rs:69-72` | The same warning on the field it applies to |
| `ring_claim/src/lib.rs:259` | The other cursor, also named `cursor` |
| `ring_cursor/src/lib.rs:247-252,311-336,438-441` | `CursorPair` — producer and consumer, not claimed and published, and the layout assertion over them |
| `ring_gating/src/lib.rs:70-73` | The family's only cursor aggregate, and what it aggregates |
| `ring_barrier/src/lib.rs:40-44` | The signature changed so a publisher's cursor could be depended on at all |
| `ring_mpsc/src/lib.rs:329-337, 655-665` | Stamps instead of a published cursor, `Claimer` kept |
| `ring_spsc/src/lib.rs:261, 792-795` | One `CursorPair`, and a drop that is both operations |

### Tests

| File | Relationship |
|------|--------------|
| `tests/handshake_test.rs:77-165` | The exhaustive check, at capacity 2 |
| `tests/handshake_test.rs:274-328` | The same property under real threads |
| `tests/manual/readme.md § P1` | The mutation that moves publication to claim time |
