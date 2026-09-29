# Algorithm: Sequencing Seal, Drain and Reset

### Scope

- **Purpose**: Give the cold-path procedure this crate owns — the three-primitive cycle `ring_tls` deliberately exposed separately so that this crate could sequence it — and fix the ordering obligations the outcome type depends on.
- **Responsibility**: State the steps, the failure handling at each, and what must not be reordered or fused.
- **In Scope**: The seal/drain/reset sequence; abort behaviour; the window between steps.
- **Out of Scope**: The primitives' own mechanics (→ [`ring_tls`'s consolidator surface](../../../ring_tls/docs/api/002_consolidator_read_surface.md)); the decision to run this at all (→ [Evaluating a Policy at an Append](001_evaluating_a_policy_at_an_append.md)).

### Abstract

`ring_tls` exposes three operations rather than one:

> "the seam between them is where `ring_flush` and the …"
> — [its consolidator read surface](../../../ring_tls/docs/api/002_consolidator_read_surface.md)

**This procedure is the reason that split exists.** A fused `consolidate()`
call would be simpler for every caller except the one that needs to decide
between the steps — and that caller is this crate. The cost of the split is
that the ordering obligation moves here, where it can be got wrong.

The sequence runs only when [policy evaluation](001_evaluating_a_policy_at_an_append.md)
has already returned "fire." It is the cold path: it happens once per flush,
not once per append, and is permitted to be as expensive as correctness
requires.

### Algorithm

**Step 1 — Seal.** Close the buffer to further appends and take ownership of
its current contents.

- After this step the buffer's writer sees a fresh region; the sealed contents are this procedure's alone.
- The mechanism is `ring_tls`'s epoch swap (→ [Buffer Epoch Cycle](../../../ring_tls/docs/lifecycle/003_buffer_epoch_cycle.md)).
- **Must be first.** Draining an unsealed buffer races the writer.

**Step 2 — Claim.** Reserve contiguous space in the ring for the sealed record
count.

- One claim, not one per record — this is what [`ring_batch`](../../../ring_batch/readme.md) is for and what `ring_tls`'s "single contiguous claim" requirement expects.
- **This is the step that can fail.** The ring may not have room.
- On failure: **abort without resetting.** Go to step 5.

**Step 3 — Drain.** Copy the sealed contents into the claimed space and publish.

- Cannot fail once step 2 succeeded — the space is already reserved.
- Publication order within the batch is the buffer's append order, which is what preserves a system's own writes' relative order through the merge.

**Step 4 — Reset.** Return the sealed region to the buffer as writable space.

- **Must be last, and must not run if step 2 failed** — that is [`FlushOutcome`](../type/002_flush_outcome.md)'s M3, the guarantee that makes `Rejected` retryable rather than lossy.

**Step 5 — Report.** Return the outcome: `Flushed { count }` after step 4,
`Rejected { staged }` if step 2 failed, `TriggeredEmpty` if the seal found
nothing.

**There is no step 6.** In particular there is no retry, no backoff, and no
wait — this crate does not park
(the family-wide no-parking-on-the-tick-path constraint). A `Rejected`
outcome goes to the caller, which owns the decision to retry on its next drive.

### What was built is three steps, and two of the five collapsed

`Flusher::run` is the whole of this procedure. Set beside the five steps above:

| Step | As specified | As built |
|------|--------------|----------|
| 1 Seal | `ring_tls`'s epoch swap | **No swap.** `Flusher` owns the buffer by value, so `&mut self.buffer` *is* the seal — there is no other writer to close the buffer against |
| 2 Claim | After the seal; the step that can fail | **Moved to first**, before the buffer is touched at all: `if self.producer.free_capacity() < staged { return Rejected }` |
| 3 Drain | Copy into the claimed space | `self.producer.try_push_batch( &mut self.buffer.drain() )` |
| 4 Reset | Return the region as writable | **Not a step.** `TlsBuffer::drain` returns a `std::vec::Drain`, which empties the buffer as it drops — inside the same expression as step 3 |
| 5 Report | Return the outcome | `self.record( cause, outcome )`, which also writes the log entry |

**Step 1 dissolved because a decision made elsewhere removed the writer it was
protecting against.** This instance assumed the staging buffer stays with its
owning thread while a consolidator reads it — hence the epoch swap, hence a
"sealed region" distinct from a "fresh region." What
[`api/002`](../api/002_the_driver_surface.md)'s D4 records is that
`Flusher::new` takes the buffer **by value**: after binding, the only path to
the buffer is through the driver, and the driver holds `&mut self` for the
whole of `run`. The race step 1 exists to prevent is not merely prevented, it
is unrepresentable.

**Step 4 dissolved because the borrow checker owns the ordering.** `Drain`
empties on drop, and its drop point is the end of the statement that created
it. There is no reset statement, so there is no reset statement to move.

### The ordering obligations became structural rather than obeyed

| # | How the implementation satisfies it |
|---|------------------------------------|
| O1 | Vacuous — no unsealed window exists; see step 1 above |
| O2 | The capacity check precedes the push in the same function, and `try_push_batch` does its own claim internally |
| O3 | **Structural.** The rejection path `return`s before `self.buffer` is read, so the drain — and with it the reset — is unreachable from it |
| O4 | Same statement as O3; they were never two obligations |
| O5 | `run` is called once per drive and `drain()` appears once in it |
| O6 | Still inherited, still untested here — the check is `free_capacity() < staged`, which returns rather than waits |

**O3 and O4 were called the pair that produces silent data loss, and the
implementation removed the possibility instead of complying with it.** The
prose above asked for reset to be "reachable only through the success path —
structurally, not by a conditional a later edit can invert." What was built is
stronger: reset is not reachable at all, because it is not written. An edit
that inverted the capacity check would return `Flushed` for a full ring, which
`a_rejected_flush_leaves_the_policy_armed` catches; an edit that moved the
reset cannot be made, because there is nothing to move.

**Moving the claim to the front is the change that did all of this.** The
specified order — seal, then discover the ring is full — is what creates a
sealed-and-unreset window in the first place. Checking capacity while the
buffer is still untouched means the failure case has no cleanup, and a failure
case with no cleanup cannot get its cleanup wrong.

### The one thing the implementation did not do

`count < staged` is not asserted after the push. The reasoning is in `run`'s
own comment: the shortfall is reachable only by violating this crate's
single-producer contract, and by the time it is observable
`ring_core::try_push_batch` has already destroyed the records it could not
place. A `debug_assert!` would promise a guarantee that vanishes in release —
the build where the race is likelier — and no runtime check can undo the loss.
What is reported is what landed.

That destructive behaviour in `ring_core` is a family-level defect, recorded
against that crate rather than worked around here.

### Ordering obligations

| # | Obligation | If violated |
|---|-----------|-------------|
| O1 | Seal strictly before drain | The drain races the writer; records are torn or lost |
| O2 | Claim before drain | The drain writes into unreserved space, corrupting another producer's slots |
| O3 | Reset strictly after a successful drain | Records are discarded while the caller is told they were rejected — silent loss, reported as backpressure |
| O4 | Reset not at all on claim failure | Same as O3 |
| O5 | Exactly one seal per drive call | Two seals leave a region owned by nobody |
| O6 | No step may park | The family's tick path must not block |

**O3 and O4 are the same obligation and they are the one that produces silent
data loss.** Every other violation in this table corrupts loudly or races
detectably; this one returns a plausible-looking `Rejected` to a caller who
will reasonably retry, while the records it names are already gone. The
sequence must be written so that reset is reachable only through the success
path — structurally, not by a conditional a later edit can invert.

**O6 is inherited rather than local.** Nothing in this sequence obviously
wants to block, but a claim that "waits for space" is the natural
implementation of backpressure and would violate it. The constraint's own test
lives in `ring_poll`, so this crate's suite stays green if it is broken — the
same non-local obligation
[`ring_handle` records](../../../ring_handle/docs/invariant/002_no_parking_operation_is_reachable.md).

### Failure handling

| Failure at | Outcome | Buffer state afterwards, as specified | As built |
|-----------|---------|------------------------|----------|
| Seal, nothing staged | `TriggeredEmpty` | Unchanged, writable | Same — the `staged == 0` early return |
| Claim, ring full | `Rejected { staged }` | **Sealed, not reset** — contents intact, buffer not accepting appends | **Unchanged and writable.** The check precedes the drain, so the buffer is never sealed on this path |
| Drain | Cannot fail after a successful claim | — | Same |
| Reset | Cannot fail | Writable | Not a step |

**The `Rejected` row's second column describes a state the implementation never
enters, and its consequence went with it.** The concern was that a rejected
flush leaves the buffer sealed and unavailable, so a writer appending during
that window has nowhere to go. Because the capacity check runs before the
buffer is touched, there is no window: after a rejection the buffer holds
exactly what it held before, and `append` behaves exactly as it did.

**What the rejection *does* leave is a buffer closer to full**, which is the
surviving half of the question. A rejected `OnBarrier` driver keeps staging
until `append` returns `RingError::Full`, and that is
[`decisions/`](../decisions/readme.md)'s Pending 1, defaulted to option 2 —
refuse the append and let the caller see it. The "second region" and
"reset-on-reject" options in that pending both addressed the sealed window and
are moot; only "refuse, visibly" remained, which is what
`buffer_capacity()` exists to let a caller anticipate.

**This row is why that pending could be defaulted rather than deferred again.**
The question looked like it needed `ring_tls`'s buffer shape and
`ring_overflow`'s policy to answer. It needed neither: reordering two steps
inside this crate removed two of the three candidate answers.

### Algorithms

| File | Relationship |
|------|--------------|
| [001_evaluating_a_policy_at_an_append.md](001_evaluating_a_policy_at_an_append.md) | The hot-path decision that gates this procedure |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md](../integration/001_two_dependencies_and_the_barrier_it_cannot_see.md) | Where `ring_batch`'s claim reaches this crate from |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_publication_point_is_designed_not_inherited.md](../invariant/002_publication_point_is_designed_not_inherited.md) | Its P1 — this sequence is the only route that respects the invariant |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_two_batch_sizes_that_must_not_diverge.md](../pitfall/002_two_batch_sizes_that_must_not_diverge.md) | Step 2's claim width against `OnBatch( n )` |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_buffer_state_through_a_flush.md](../lifecycle/003_buffer_state_through_a_flush.md) | This sequence as states, including the sealed-and-unreset window |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_flush_outcome.md](../type/002_flush_outcome.md) | Step 5's product; M3 is O3 restated as a type guarantee |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_tls/docs/api/002_consolidator_read_surface.md`](../../../ring_tls/docs/api/002_consolidator_read_surface.md) | The three primitives, exposed separately for this caller |
| [`ring_tls/docs/lifecycle/002_consolidation_cycle.md`](../../../ring_tls/docs/lifecycle/002_consolidation_cycle.md) | The cycle named there and deferred to this crate; C2–C4 are steps 1, 3 and 4 |
| [`ring_batch/readme.md`](../../../ring_batch/readme.md) | Step 2's "single contiguous claim" requirement |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | O3/O4 — `a_rejected_flush_leaves_the_policy_armed` is that case, and it goes past what this row asked for: the records survive two refusals, then land unchanged when room appears. `a_refused_final_drain_keeps_the_records` is the same guarantee on the shutdown path |
| `tests/append_cost_test.rs` | Step 2's early return, from the other side — `a_drive_that_does_not_fire_changes_nothing` runs a thousand non-firing drives and asserts the staging buffer's capacity, occupancy, the ring, and the log all stand still |

### FL3 — O6 Is Not Merely Untested Here; the Guard It Defers To Does Not Name This Crate

O6's row says the constraint's test lives in `ring_poll`, "so this crate's suite
stays green if it is broken." Both suites do:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the roster of crates permitted to park --'
command grep 'pub const PARKING_CRATES' ring_poll/src/lib.rs
echo '  -- and the crates the guard asserts are on the tick path --'
command grep 'for tick_path in' ring_poll/tests/poll_test.rs
echo '  -- is this crate in either list? --'
printf '    ring_flush in PARKING_CRATES:      %s\n' \
  "$( command grep -c 'PARKING_CRATES.*ring_flush' ring_poll/src/lib.rs )"
printf '    ring_flush in the tick-path guard: %s\n' \
  "$( command grep -c 'tick_path in.*ring_flush' ring_poll/tests/poll_test.rs )"
```

Live output:

```
  -- the roster of crates permitted to park --
pub const PARKING_CRATES : [ &str; 3 ] = [ "ring_barrier", "ring_shutdown", "ring_wait" ];
  -- and the crates the guard asserts are on the tick path --
  for tick_path in [ "ring_poll", "ring_handle", "ring_core" ]
  for tick_path in [ "ring_poll", "ring_handle", "ring_core" ]
  -- is this crate in either list? --
    ring_flush in PARKING_CRATES:      0
    ring_flush in the tick-path guard: 0
```

The roster names three crates that *may* park, and the guard names three crates
that must not reach one — `ring_poll`, `ring_handle`, `ring_core`. `ring_flush`
appears in neither.

**So the deferral has no destination.** O6's row assumes a division of labour in
which this crate declines to test the constraint because another crate tests it
family-wide. `ring_poll`'s guard is not family-wide: it is a three-name
allowlist compared against a three-name denylist, and every crate outside both
— this one included — is unexamined. A `free_capacity()` replaced by a spinning
wait-for-space would turn `run` into a parking operation on the tick path, and
the entire family's test suite would stay green.

The gap is cheap to close and the closing does not belong here: the guard's
`for tick_path in [ … ]` list is three literals, and the honest form is every
crate the tick path can reach. That is `ring_poll`'s roster to widen, and the
crate has already filed the roster's hand-maintenance as its own open decision.
What belongs here is that **this instance's O6 row states a coverage claim that
is not true**, and the row is the only thing standing between the obligation and
nobody checking it.

### FL4 — `run` Says the Shortfall Requires Violating a Contract That Is Stated Only Inside `run`

The comment declining the `count < staged` assertion rests on a contract that
appears exactly once and is breakable by a safe public method:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every statement of the single-producer contract in this crate --'
command grep -r 'sole producer\|single.producer\|only producer' \
  ring_flush/src ring_flush/docs/api ring_flush/docs/invariant \
  ring_flush/docs/integration 2>/dev/null | sed -E 's/^(.{0,130}).*/\1/'
echo '  -- what the family hands out to break it --'
command grep 'pub fn try_clone' ring_core/src/lib.rs
echo '  -- and what this crate binds --'
command grep 'producer : Producer' ring_flush/src/lib.rs
```

Live output:

```
  -- every statement of the single-producer contract in this crate --
ring_flush/src/lib.rs:    // `Flusher` is the sole producer of it can only grow between here and the
  -- what the family hands out to break it --
  pub fn try_clone( &self ) -> Option< Producer< 'a, T > >
  -- and what this crate binds --
  producer : Producer< 'a, T >,
    producer : Producer< 'a, T >,
```

One line. It is a `//` comment in the body of a private function, and it is the
sole place in this crate — source, `api/`, `invariant/`, `integration/` — where
the single-producer requirement is written down. A consumer reading the public
surface has no way to learn it exists.

**`ring_core::Producer::try_clone` is the counterpart.** It is safe, public, and
hands back a second `Producer` on the same ring. Nothing about the type
`Flusher::new` accepts distinguishes a producer that is sole from one that is
not, so the configuration `run`'s comment calls a contract violation is
constructible from the family's own API in two lines, with no `unsafe` and no
warning.

The comment's reasoning about the assertion is right and should stand: by the
time a shortfall is observable the records are already destroyed by
`try_push_batch`, so a `debug_assert!` would promise what it cannot deliver.
**The defect is the contract's location, not the decision it justifies.** A
requirement that a caller must satisfy belongs on the constructor that binds the
producer, where a consumer will read it — and stating it there is also what
would make `try_clone`'s interaction with this crate a documented hazard rather
than a discoverable one.
