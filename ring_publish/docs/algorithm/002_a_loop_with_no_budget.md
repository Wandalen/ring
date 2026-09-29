# Algorithm: A Loop With No Budget

### Scope

- **Purpose**: Specify `publish`'s retry loop — the family's only unbounded spin — and establish why unbounded is correct here and would be a defect anywhere else in these 33 crates.
- **Responsibility**: State the loop, prove its termination argument from what it waits on, contrast it with every other waiting construct in the family, and name the one input that makes it hang.
- **In Scope**: `publish` (`src/lib.rs:200-210`).
- **Out of Scope**: The exchange inside it — see [`algorithm/001`](001_the_compare_exchange_that_refuses.md). Whether a `WaitKind` belongs here — see [`decisions/002`](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md).

### The Whole Loop

```rust
pub fn publish( &self, start : Seq, len : usize ) -> Seq
{
  loop
  {
    if let Ok( end ) = self.try_publish( start, len )
    {
      return end;
    }
    core::hint::spin_loop();
  }
}
```

Nine lines. One exit, and it is a success. No budget, no deadline, no attempt
counter, no `WaitKind`, no `Result` — the signature returns `Seq`, not
`Result< Seq, RingError >`, because there is no state from which the function can
report failure.

The failure value from `try_publish` is discarded: the `if let Ok(…)` binds only
the success arm. That is correct rather than lazy — a producer may publish only
the range it claimed, so `start` cannot be adjusted, and the next attempt is
byte-identical to the last (see
[`algorithm/001`](001_the_compare_exchange_that_refuses.md) § PB8).

### PB9 — The Family's Only Unbounded Loop, and Its Termination Argument

```sh
cd "$(git rev-parse --show-toplevel)"
for f in */src/*.rs; do
  n=$( grep -vE '^[[:space:]]*//' "$f" \
       | grep -cE '^[[:space:]]*loop[[:space:]]*\{?[[:space:]]*$' )
  [ "$n" -gt 0 ] && printf "%-40s %s\n" "$f" "$n"
done
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
ring_bench/src/lib.rs               5
ring_publish/src/lib.rs             1
```

Two `ring_*` crates contain a bare `loop` at all:

| Crate | Count | What they are |
|-------|-------|---------------|
| `ring_bench` | 5 | Drain loops in a measurement harness, each bounded by an item count |
| **`ring_publish`** | **1** | This one |

Every other waiting construct in the family is a counted `for` or a `while` with
a computable exit. `ring_wait::wait_until` is `for attempt in 0..spins.max( 1 )`
and returns `Err( RingError::Empty )` on exhaustion. `ring_claim::claim` is
`while count <= headroom( current )` and falls out to `Err( RingError::Full )`.
`ring_poll` open-codes a budgeted loop three times rather than depend on a crate
that blocks. Against that background a bare unbounded `loop` in a Tier 5
primitive is the kind of thing a reviewer should stop at.

It is correct here, and the argument is about **what is being waited for** rather
than about how long the wait is. `src/lib.rs:47-53`:

> Waiting for space is unbounded: it depends on a consumer that may be slow,
> stalled, or gone, so it needs a strategy and a give-up. Waiting for your
> predecessor to publish is bounded by that producer finishing a slot write it
> has already started and cannot abandon — it is not blocked on anything itself.
> A `WaitKind` here would offer a `Park` that can only ever hurt, and a budget
> whose exhaustion has no correct handling.

Stated as a termination argument:

1. The loop exits when `published == start`.
2. `published` advances only through `try_publish`
   ([`invariant/001`](../invariant/001_the_frontier_moves_only_by_compare_exchange.md)),
   and only from the exact current frontier — so it advances through every
   intermediate value and cannot skip `start`.
3. Every sequence below `start` belongs to a producer that has already claimed
   it, and a producer holding a claim is executing a slot write with no blocking
   operation in it.
4. Therefore every predecessor publication is finite, there are finitely many of
   them, and the frontier reaches `start`.

Step 3 is the load-bearing one, and it is a **precondition on the caller, not a
property of this crate**. It holds for every caller that publishes a range it
claimed. It fails for a caller that does not — which is the hang described below.

### The One Input That Hangs

`src/lib.rs:183-189`, under `# Panics`:

> Never. The loop exits when the predecessor publishes, which it is committed to
> doing; a caller that publishes a range it never claimed deadlocks here instead,
> which is a caller bug this crate cannot detect — `try_publish` is the variant
> for a caller that wants to decide for itself.

Two things are being said. The first is that this function has no panic path at
all: there is no `unwrap`, no indexing, no arithmetic that can overflow in a
build where `Seq` is a `u64` counter. The second is that the failure it *does*
have is worse than a panic — it is a hang, with no diagnostic, in a caller-supplied
condition this crate has no way to check.

It cannot check it. `Publisher` holds one cursor and no record of what was
claimed; the claimed cursor is `ring_claim`'s and this crate does not depend on
`ring_claim` ([`integration/001`](../integration/001_ten_crates_name_it_and_none_depends_on_it.md)).
A `Publisher` that could detect the bug would be a `Publisher` that knew about
claims, which is the coupling
[`data_structure/002`](../data_structure/002_the_four_cursors_of_the_handshake.md)
records the four-cursor design as existing to avoid.

What the crate offers instead is the escape hatch named in the last clause:
`try_publish` returns rather than spinning, so a caller that wants a timeout, a
diagnostic, or a give-up writes its own loop.
[`pitfall/001`](../pitfall/001_publishing_a_range_you_never_claimed.md) is the
long form of this failure and how to recognise it.

### Why `spin_loop` and Not an Empty Body

`core::hint::spin_loop()` compiles to `PAUSE` on x86-64 and `YIELD` on aarch64 —
a hint to the core that this is a spin-wait, which lets it de-prioritise the
pipeline and, on hyper-threaded cores, hand issue slots to the sibling thread.
That sibling is very often the producer this loop is waiting for.

It is a hint, not a yield: no scheduler involvement, no syscall, no timing
guarantee, and no upper bound on the loop's iteration count. What it buys is
that the spin costs the *predecessor* less than a bare empty loop would —
measured in [`non_functional_requirement/002`](../non_functional_requirement/002_what_the_spin_costs.md).

The alternative shapes were all considered and rejected in the module
documentation at `:42-53`; the one that matters is `thread::yield_now`, which
would put a scheduler round-trip between the predecessor's publication and this
thread noticing it. On a wait whose expected duration is one slot write, the
round-trip is the dominant cost.

### What the Tests Establish

`tests/publish_test.rs:126-155` is the direct one. It spawns B blocked on
`publish( Seq( 4 ), 4 )` with the frontier at zero, then spins the main thread
1 000 times asserting `published() <= Seq( 4 )` — that B has *not* landed early —
before A publishes and B is joined. `:142-143` states what it is really claiming:
*"Nothing this thread can do makes B's publication land early; the only thing
that unblocks it is A's own publication below."*

`tests/publish_test.rs:157-181` is the stronger one, and it is the loop's actual
stress case: four ranges of 25, handed to four threads in an order deliberately
unrelated to their sequence order (`[ 3W, W, 0, 2W ]`), each calling `publish`.
Whatever the scheduler does, three of the four spin, and the frontier lands
exactly at `4 × WIDTH`. That test would deadlock rather than fail if the
termination argument above were wrong — which is a property worth naming, because
a deadlocking test in CI reads as a hang rather than as a failure.

The unbounded loop is also what keeps
[`workaround/001`](../workaround/001_the_loom_seam_and_its_only_user.md)'s
exhaustive model small: `loom` enumerates every interleaving, and a spin loop
puts no bound on how many iterations one contains. `mod exhaustive` is sized
accordingly — `Capacity::new( 2 )`, one claim of one slot, one drain
(`tests/handshake_test.rs:82,102`) — against the threaded suite's three
producers × 3 000 items at `:503-505`. The file's own header states the trade at
`:37-44`: *"one has scale without coverage, the other coverage without scale."*

### Algorithms

| File | Relationship |
|------|--------------|
| [001_the_compare_exchange_that_refuses.md](001_the_compare_exchange_that_refuses.md) | The operation this loop retries |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_plain_spin_rather_than_a_wait_kind.md](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md) | Why no strategy and no budget, argued against the family's norm |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_two_crates_that_declined.md](../integration/002_the_two_crates_that_declined.md) | The two crates that read this loop and declined it — one as never running, one as never permissible |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_frontier_moves_only_by_compare_exchange.md](../invariant/001_the_frontier_moves_only_by_compare_exchange.md) | Step 2 of the termination argument |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_two_publications.md](../item/002_the_two_publications.md) | `publish` and `try_publish`, contract by contract |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_the_spin_costs.md](../non_functional_requirement/002_what_the_spin_costs.md) | What an iteration costs, and what the wait costs a predecessor |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_publishing_a_range_you_never_claimed.md](../pitfall/001_publishing_a_range_you_never_claimed.md) | The caller bug this loop turns into a silent hang |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_loom_seam_and_its_only_user.md](../workaround/001_the_loom_seam_and_its_only_user.md) | Why an unbounded spin makes the exhaustive model expensive |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:42-53,167-210` | The rejected-strategy argument, and the loop with its `# Panics` contract |
| `ring_wait/src/lib.rs:179-195` | The family's budgeted counterpart — a counted `for` with an exhaustion error |
| `ring_claim/src/lib.rs:436-447` | A retry loop bounded by headroom, exiting to `Full` |
| `ring_bench/src/lib.rs` | The family's only other bare loops, each bounded by an item count |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:116-124` | `publish` returns the end of what it published |
| `tests/publish_test.rs:126-155` | B blocks until A publishes, and nothing else releases it |
| `tests/publish_test.rs:157-181` | Four out-of-order producers end contiguous — the loop's stress case |
| `tests/handshake_test.rs:497-561` | `publish` rather than `try_publish`, and why, under three producers × 3 000 items |
| `tests/manual/readme.md § P4` | The loop retries the same exchange rather than scanning for a reorderable position |
