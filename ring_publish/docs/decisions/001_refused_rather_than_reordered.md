# Decision: Refused Rather Than Reordered

### Scope

- **Purpose**: Record the choice to advance the published cursor only from the exact frontier, the two alternatives rejected, and the condition under which the rejection would be revisited.
- **Responsibility**: State the ruling, quantify what each alternative would have bought and cost, show the check that keeps them out of the code, and trace what happened when the crate that needed them arrived.
- **In Scope**: `try_publish`'s all-or-nothing advance.
- **Out of Scope**: The spin built over the refusal — see [`decisions/002`](002_a_plain_spin_rather_than_a_wait_kind.md).

### The Ruling

`try_publish` advances only when the published cursor reads *exactly* `start`. A
producer whose predecessor has not published is refused and must try again. The
frontier is therefore always a contiguous prefix: everything below it is written,
and there is no per-slot state anywhere in the crate.

`src/lib.rs:29-40` states it and rejects the alternatives in the same breath:

> The alternative — publishing to the highest contiguous point, or tracking
> per-slot availability in a bitmap — would let producer B's publication proceed
> while producer A is still writing. That works, and it is what a high-contention
> multi-producer ring eventually needs; it is deliberately not here, because it
> is `ring_mpsc`'s problem at S5 and putting it in the primitive would make the
> primitive untestable without a second producer.

Two reasons, and the second is the decisive one. The first is scope (someone
else's problem). The second is about *testability*: a mechanism that only
differs from the simple one under contention cannot be validated without
contention, and a primitive whose correctness argument requires two threads is a
primitive whose correctness argument is a scheduling accident.

### The Three Designs

| | Chosen — exact-frontier exchange | Highest-contiguous scan | Per-slot bitmap |
|--|--------------------------------|-------------------------|-----------------|
| State | one `Seq` | one `Seq` + a record of published-but-not-contiguous ranges | one bit (or stamp) per slot |
| B publishes while A writes | no — refused | yes, recorded; frontier moves later | yes, recorded per slot |
| Producer waits on a peer | **yes** | no | no |
| Consumer's read | one atomic load | one load + a scan | a scan from the frontier |
| Testable single-threaded | **fully** | no — the interesting path needs two producers | no — same |
| Memory | 64 bytes total | 64 bytes + unbounded bookkeeping | capacity × stamp width |
| Who took it | this crate | — | `ring_mpsc`, as stamps |

The row that decided it is *"producer waits on a peer"*. It is the chosen
design's one genuine cost, and it is the reason `ring_mpsc` declined the crate
([`integration/002`](../integration/002_the_two_crates_that_declined.md)) — the
coupling `ring_mpsc`'s contended-claim design exists to remove. Everything else
on the chosen column is better.

### PB17 — The Rejection Is Checked By Grep, Not Trusted

`tests/manual/readme.md § P4` exists because a rejection recorded only in prose
is a rejection that can be quietly undone:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -iE "bitmap|contiguous|while|for |max\(|highest" \
  || echo '(no matches — neither rejected alternative appears in the code)'
# control: the identical expression over ring_claim, which does loop
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs \
  | grep -iE "bitmap|contiguous|while|for |max\(|highest"
```

Live output:

```
(no matches — neither rejected alternative appears in the code)
    while count <= self.consumers.headroom( current )
    while let granted @ 1.. = max.min( self.consumers.headroom( current ) )
```

**Expected: no match in this crate.** The control below it runs the identical
expression over `ring_claim` and returns two `while` lines, so the empty result
above is a measured absence rather than a grep that silently matched nothing.
The check's own note at
`:110-114`:

> One `loop` in `publish` is the crate's only repetition, and it retries the
> *same* exchange rather than scanning for a reorderable position. Any hit here
> means the primitive grew `ring_mpsc`'s problem, which would make it untestable
> without a second producer — the exact reason it was kept out.

The comment-stripping filter is load-bearing and the plan says why at `:15-18`:
this crate's module documentation *argues about* bitmaps and contiguity at
length, so an unfiltered grep would report the rejected designs as
implementations. The filter drops `///`, `//!` and `//` alike.

This is the family's characteristic check shape — assert an *absence*
structurally rather than assert a behaviour — and it is the only kind available
here, because the difference between the three designs is invisible to any test a
single producer can run.

### What Happened When the Crate That Needed the Alternative Arrived

`ring_mpsc` needed exactly the rejected mechanism. It did not
extend this crate; it implemented the alternative itself and dropped the
dependency: with publication tracked in per-slot stamps, there is no published
cursor for either `ring_publish` or `ring_consume` to act on.

So the deferral worked exactly as written. The primitive stayed simple and
single-thread-testable; the crate that needed contention-tolerant publication
built it where the contention is; and the boundary held under contact rather than
being renegotiated. `ring_mpsc:23-25` describes what it built instead:

> So publication is a `Release` store into `stamps[ seq & mask ]`, and a slot is
> published exactly when its stamp equals the sequence addressing it. No producer
> waits for another to publish; the consumer pays a scan instead.

That last clause is the trade, stated plainly: this crate makes the *producer*
wait; the alternative makes the *consumer* scan. Neither is free, and which is
cheaper depends on producer count — which is precisely the parameter this crate
does not know and `ring_mpsc` does.

### The Cost Nobody Pays Yet

The refusal has one consequence a future consumer would feel: publication
latency for producer B is bounded below by producer A's slot write, however
large that write is. A ring carrying 4 KiB records has a B that waits for a
4 KiB `memcpy` it has no interest in.

Nothing measures this, because nothing outside this crate's tests publishes
anything ([`api/001`](../api/001_six_methods_and_no_caller.md) § PB10).
`tests/handshake_test.rs:497-561` runs three producers × 3 000 items with a
one-word payload, which is the smallest possible write and therefore the
friendliest possible case for the refusal.
[`non_functional_requirement/002`](../non_functional_requirement/002_what_the_spin_costs.md)
records what that measures and what it does not.

### Reversal Conditions

| Would reverse this if | Because | Currently |
|-----------------------|---------|-----------|
| A consumer needs contention-tolerant publication **and** cannot own its own stamp array | the deferral assumed the needing crate would build it locally, and `ring_mpsc` did | no such consumer exists |
| The single-producer case stopped mattering | the whole testability argument is that the simple path is fully exercisable | `ring_spsc` does not even use this crate ([`integration/002`](../integration/002_the_two_crates_that_declined.md)) |
| Producer payloads grew large enough that B's wait dominated | the latency coupling above | unmeasured; no production caller |

None holds. The decision is stable, and the evidence that it is stable is that
the one crate positioned to overturn it examined it in writing and chose to build
its own mechanism instead.

### PB49 — The Deferral Named a Bitmap; the Crate It Deferred To Built Stamps

```sh
cd "$(git rev-parse --show-toplevel)"
# the mechanism this crate's rejection names
grep 'bitmap' ring_publish/src/lib.rs
# the mechanism ring_mpsc actually built
grep -c 'bitmap' ring_mpsc/src/lib.rs || true
grep -c 'stamp' ring_mpsc/src/lib.rs
grep '^//! # Publication' ring_mpsc/src/lib.rs
```

Live output:

```
//! per-slot availability in a bitmap — would let producer B's publication
0
52
//! # Publication is a per-slot stamp, and that is this crate's whole addition
```

The rejection names two alternatives — publishing to the highest contiguous
point, or "tracking per-slot availability in a bitmap" — and defers both:
*"it is `ring_mpsc`'s problem at S5"*.

`ring_mpsc` exists now. It says `bitmap` zero times and `stamp` fifty-two,
and it puts the mechanism in a module-documentation heading: *"Publication is a
per-slot stamp, and that is this crate's whole addition"*. A stamp is not a
bitmap. It holds the sequence occupying the slot rather than one bit of
occupancy, which is what lets `ring_mpsc` drop the sentinel a bitmap design
needs — the difference is the reason it works, not a spelling.

**Correction (2026-09-28):** this paragraph read "stamp forty-seven" against an
already-drifted count; PB49's own recipe above now prints 52. The conclusion is
unchanged — `ring_mpsc` uses stamps, not a bitmap — the count moves as
`ring_mpsc/src/lib.rs` grows and was never the point being made.

The decision is not wrong where it matters. It rejected reordering, it deferred
the general mechanism, and the deferral worked exactly as written: the crate
that needed it built it locally, which is what
[`integration/002`](../integration/002_the_two_crates_that_declined.md) records
from the other end. What is stale is the single concrete noun. A reader
following this sentence into `ring_mpsc` goes looking for a bitmap, finds a
stamp array, and has nothing connecting the two.

This is the second of this crate's forecasts overtaken by the crate it named.
[`integration/002`](../integration/002_the_two_crates_that_declined.md) records
the open question naming `ring_core` as the next possible
adopter — `ring_core` has since been implemented and does not adopt. Both
sentences were written before their subject existed; neither was revisited
after it did, and nothing in the corpus would notice either way.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_compare_exchange_that_refuses.md](../algorithm/001_the_compare_exchange_that_refuses.md) | The operation this decision shapes |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_six_methods_and_no_caller.md](../api/001_six_methods_and_no_caller.md) | The `publish_up_to` this decision keeps off the surface |

### Decisions

| File | Relationship |
|------|--------------|
| [002_a_plain_spin_rather_than_a_wait_kind.md](002_a_plain_spin_rather_than_a_wait_kind.md) | What the refusal forces the caller to do, and how |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_two_crates_that_declined.md](../integration/002_the_two_crates_that_declined.md) | The ruling that settled it, and both declining rationales |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_frontier_moves_only_by_compare_exchange.md](../invariant/001_the_frontier_moves_only_by_compare_exchange.md) | The contiguous-prefix property this decision establishes |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_the_spin_costs.md](../non_functional_requirement/002_what_the_spin_costs.md) | What the coupling costs, and at what payload size |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_conflating_the_two_cursors.md](../pitfall/002_conflating_the_two_cursors.md) | The failure the refusal is guarding against |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:29-40,161-165` | The rejection with both alternatives named, and the operation |
| `ring_mpsc/src/lib.rs:13-28, 30-40` | The alternative, implemented where the contention is |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:72-85` | A start past the frontier is refused, and nothing moves |
| `tests/publish_test.rs:157-181` | Out-of-order producers still end contiguous |
| `tests/handshake_test.rs:357-380` | Out-of-order publication refused rather than advancing past a gap |
| `tests/manual/readme.md § P4` | Neither rejected alternative is present in the code |
