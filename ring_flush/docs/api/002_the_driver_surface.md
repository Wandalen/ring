# API: The Driver Surface

### Scope

- **Purpose**: Fix the call a consumer must make and this crate cannot make for itself — the entry point [`pattern/002`](../pattern/002_driven_not_self_firing.md) forces into existence — and account for the one argument this crate can neither supply nor verify.
- **Responsibility**: State the operations, their error behaviour, and the compatibility guarantees.
- **In Scope**: The drive call; barrier announcement; the outcome returned.
- **Out of Scope**: What the drive does internally (→ [Sequencing Seal, Drain and Reset](../algorithm/002_sequencing_seal_drain_reset.md)); the consumer's own cadence, which no crate in this family owns.

### Abstract

**This crate has no thread, no timer, and no callback registration.** It cannot
flush unless called. That is a deliberate commitment
(→ [Driven, Not Self-Firing](../pattern/002_driven_not_self_firing.md)) and this
surface is its consequence: one call the consumer makes at a cadence the
consumer chooses.

The surface is small and the smallness is the design. Everything interesting
about *when* a flush happens is either in the bound policy or in the caller's
schedule; nothing is in this API's shape.

### Operations

| # | Operation | Signature shape | Notes |
|---|-----------|-----------------|-------|
| D1 | Drive without a barrier | `fn drive( &mut self ) -> FlushOutcome` | Evaluates `OnFull` and `OnBatch`; `OnBarrier` returns `NotTriggered` |
| D2 | Drive announcing a barrier | `fn drive_at_barrier( &mut self ) -> FlushOutcome` | The only route by which `OnBarrier` can fire |
| D3 | Final drain | `fn drain_final( &mut self ) -> FlushOutcome` | Ignores the policy; used once at teardown (→ [`lifecycle/002`](../lifecycle/002_from_configuration_to_the_final_drain.md)) |
| D4 | Stage a record | `fn append( &mut self, record : T ) -> Result< (), RingError >` | **Added.** The hot path — one `push`, no policy consultation |
| D5 | Opt into the flush log | `fn with_log( self ) -> Self` | **Added.** Builder-style; a driver has no log unless asked |
| D6 | Read / clear the log | `fn log( &self ) -> Option< &FlushLog >`, `fn clear_log( &mut self )` | **Added.** `None` when D5 was not called |

**D4 was not in this instance's original table, and the surface does not work
without it.** `Flusher::new` takes the buffer by value, so once bound, the
caller cannot reach it — every record has to arrive through the driver. This
was not an oversight in reasoning so much as a consequence of a decision made
later: the instance was written assuming the buffer stayed with the caller.

**D4 is where this crate's cost constraint actually lands.** It is one `push`
into the staging buffer and nothing else — no policy consultation, no counter,
no branch (→ [`nfr/002`](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md)).
The policy is evaluated on D1/D2, which run at the caller's cadence rather than
per record. `appending_never_publishes` asserts the whole of that: thirty-two
appends, nothing in the ring, empty log.

**D5 and D6 are the flush log made opt-in**, which is how
[`decisions/`](../decisions/readme.md)'s Pending 2 dissolved. They are on the
export surface deliberately: a test in another crate must be able to build an
observed driver, and a `cfg`-gated log could not be observed from an integration
test at all.

**D1 and D2 as separate calls rather than `drive( at_barrier: bool )`.** A
boolean parameter at a call site reads as `drive( false )` and communicates
nothing; two named methods make the announcement visible in the code that makes
it, which matters because that announcement is the one fact this crate cannot
check. A reader auditing whether barriers are announced correctly can grep for
`drive_at_barrier`; they cannot grep for `true`.

**D3 is separate from both and is not a general escape hatch.** It ignores the
policy, which is exactly the capability
[`api/001`](001_the_policy_surface.md) refuses to expose as `flush_now()`. The
difference is that `drain_final` names a lifecycle phase — it is called once,
at teardown, by the owner — rather than offering an unconditional flush to any
caller. That distinction lives in documentation and naming, not in the type
system, and is therefore a convention that can erode.

### The unverifiable argument

**D2 asserts a fact this crate has no way to check: that the stage barrier has
actually been reached.**

| What this crate knows | What it does not |
|-----------------------|------------------|
| That `drive_at_barrier` was called | Whether a barrier was genuinely crossed |
| Its own policy and the buffer's occupancy | Which barrier the ring's consumer is gated on |
| The buffer's occupancy | Whether the caller holds the right barrier at all |

`ring_barrier` is not in the dependency closure, and adding it would supply
none of this either (→ [the pitfall](../pitfall/001_on_barrier_cannot_see_the_barrier.md))
— a `[dependencies]` line buys the type; the *instance* the consumer waits on
is what D2 needs and is not something an edge can carry.

**So D2's contract is: "the caller states the barrier is reached, and this
crate believes it."** Every consequence of a wrong announcement — a flush at
the wrong point, publication order becoming timing-dependent — is the caller's,
and no mechanism here detects it. Stating that plainly is the only mitigation
available.

### Error Handling

| Condition | Result |
|-----------|--------|
| Policy did not fire | `FlushOutcome::NotTriggered` — **not an error** |
| Fired, buffer empty | `FlushOutcome::TriggeredEmpty` |
| Fired, ring full | `FlushOutcome::Rejected { staged }` — records **retained**, safe to retry ([`algorithm/002`](../algorithm/002_sequencing_seal_drain_reset.md)'s O3) |
| D4 (`append`), staging buffer already at capacity | `Err( RingError::Full )` — the one condition on this surface that is an error, not a `FlushOutcome` |
| Called from a thread other than the buffer's owner | **Undefined by this surface.** The buffer is thread-local; this is `ring_tls`'s precondition, not this crate's to check |
| Called after `drain_final` | Open — see [`decisions/`](../decisions/readme.md) |

**D1, D2 and D3 return `FlushOutcome`, never `Result`,** because three of four
outcomes on those three operations are ordinary. `Rejected` is backpressure the
design expects, not a failure (→ [Flush Outcome](../type/002_flush_outcome.md)).
D4 is the exception — see `FL7` below.

**No operation blocks, parks, retries, or sleeps.** A family-wide rule constrains
everything on the tick path; a `drive` that waited for ring space
would violate it, and the violating test lives in `ring_poll` rather than here
(→ [`ring_handle`'s record of the same non-local obligation](../../../ring_handle/docs/invariant/002_no_parking_operation_is_reachable.md)).

### Compatibility Guarantees

1. **`drive` never blocks.** Changing this breaks the tick path's central
   property and would not be caught by this crate's tests.
2. **`Rejected` retains the records.** A caller may retry; that is the whole
   value of the variant.
3. **D1 and D2 stay separate.** Collapsing them into a boolean parameter loses
   the greppability the unverifiable argument makes necessary.
4. **All three return `FlushOutcome`,** never `bool` or `()` — the
   `NotTriggered` / `TriggeredEmpty` distinction is what makes a misconfigured
   `OnBarrier` observable at all.
5. **No public signature names a crate outside the Contract's five.**

**Guarantee 4 is the one most likely to be eroded by convenience.** A caller
who does not care about the distinction will want `fn drive( &mut self )`
returning nothing, and adding that overload costs nothing visible — while
removing the only runtime signal that the crate's most likely misconfiguration
is present.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_sequencing_seal_drain_reset.md](../algorithm/002_sequencing_seal_drain_reset.md) | What a firing drive runs; its O3 is guarantee 2 |

### APIs

| File | Relationship |
|------|--------------|
| [001_the_policy_surface.md](001_the_policy_surface.md) | Configuration; D3 is where its refused `flush_now()` case is served |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_from_configuration_to_the_final_drain.md](../lifecycle/002_from_configuration_to_the_final_drain.md) | D3's phase, and the open question about calls after it |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_driven_not_self_firing.md](../pattern/002_driven_not_self_firing.md) | The commitment this surface implements; its R6 is the unverifiable argument |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_on_barrier_cannot_see_the_barrier.md](../pitfall/001_on_barrier_cannot_see_the_barrier.md) | Why D2 exists rather than observation |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_flush_outcome.md](../type/002_flush_outcome.md) | Guarantee 4's return type |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_poll/readme.md`](../../../ring_poll/readme.md) | Guarantee 1's origin — constrains this crate, claimed by `ring_poll` |
| [`ring_barrier/readme.md`](../../../ring_barrier/readme.md) | The barrier variant D2 exists to serve |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | The observers — `staged_follows_the_buffer` walks `staged()` from 0 through 2 and back to 0 across a flush; `every_type_can_be_printed` pins the `Debug` obligation on all six exported types plus `Flusher` itself, which is what lets a failing assertion name what it held |
| `tests/flush_test.rs` | `on_barrier_never_fires_without_an_announcement` calls D1 against `OnBarrier` twenty-four times and asserts `NotTriggered` every time. Guarantee 4's distinction is pinned separately by `an_empty_trigger_is_recorded_and_is_not_a_non_trigger`, which is where `NotTriggered` and `TriggeredEmpty` are actually told apart |

### FL7 — "No Operation Returns `Result`" Is Written Seventy Lines Below the Operation That Does

The claim and its counterexample are in the same document:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the claim, and the row of this same instance that contradicts it --'
echo '     ( offsets, not line numbers — this block is quoted into the file it reads )'
awk '/^### FL/{ exit }
     /No operation returns|^\| D4 \|/ { if ( !anchor ) anchor = NR
       printf "    %+4d  %s\n", NR - anchor, substr( $0, 1, 96 ) }' \
  ring_flush/docs/api/002_the_driver_surface.md
echo '  -- every public signature on the driver, by return type --'
command grep -E '^  pub fn (append|drive|drive_at_barrier|drain_final|with_log|log|clear_log)' \
  ring_flush/src/lib.rs
echo '  -- and which conditions the Error Handling table enumerates --'
awk '/^### Error Handling/{ i = 1; next } i && /^\| /{ print "    " $0 } i && /^###/{ exit }' \
  ring_flush/docs/api/002_the_driver_surface.md | sed -E 's/^(.{0,105}).*/\1/'
```

Live output:

```
  -- the claim, and the row of this same instance that contradicts it --
     ( offsets, not line numbers — this block is quoted into the file it reads )
      +0  | D4 | Stage a record | `fn append( &mut self, record : T ) -> Result< (), RingError >` | **Adde
  -- every public signature on the driver, by return type --
  pub fn with_log( mut self ) -> Self
  pub fn log( &self ) -> Option< &FlushLog >
  pub fn clear_log( &mut self )
  pub fn append( &mut self, record : T ) -> Result< (), RingError >
  pub fn drive( &mut self ) -> FlushOutcome
  pub fn drive_at_barrier( &mut self ) -> FlushOutcome
  pub fn drain_final( &mut self ) -> FlushOutcome
  -- and which conditions the Error Handling table enumerates --
    | Condition | Result |
    | Policy did not fire | `FlushOutcome::NotTriggered` — **not an error** |
    | Fired, buffer empty | `FlushOutcome::TriggeredEmpty` |
    | Fired, ring full | `FlushOutcome::Rejected { staged }` — records **retained**, safe to retry ([`alg
    | D4 (`append`), staging buffer already at capacity | `Err( RingError::Full )` — the one condition on
    | Called from a thread other than the buffer's owner | **Undefined by this surface.** The buffer is t
    | Called after `drain_final` | Open — see [`decisions/`](../decisions/readme.md) |
```

`append` returns `Result< (), RingError >`. It is listed in this instance's own
operations table as D4, and the Error Handling section three tables later opens
with the sentence "No operation returns `Result`."

**The sentence is defensible about what it meant and wrong about what it says.**
Its argument is that a `FlushOutcome` is not a failure — `Rejected` is
backpressure, `NotTriggered` is ordinary — and that argument concerns D1, D2 and
D3, which were the whole surface when the sentence was written. D4 was added
later, from `api/001`, when ownership moved the buffer into the driver, and it
carries its own **Added** marker to say so.

**The consequence is that the Error Handling table enumerates conditions for the
three operations that cannot fail and none for the one that can.** Every row
lists a `FlushOutcome` variant or a deferral; there is no row for a full staging
buffer, which is the only condition on this surface that produces an `Err`. And
that condition is the one where the record is destroyed rather than returned
(→ [`algorithm/001`](../algorithm/001_evaluating_a_policy_at_an_append.md)'s
FL1), so the surface's error documentation is silent precisely where a consumer
most needs it.

The shape worth keeping: **a section that opens by asserting a category is empty
will not be revisited when something is added to it.** The addition is marked as
an addition, in the operations table at the top of this instance, seventy lines
above the sentence it falsified — and seventy lines is far enough that the
sentence is not on screen while the row is being written. The assertion is also
phrased as a property of the whole surface rather than of the rows it summarises,
so nothing about editing one row suggests re-reading it.

```sh
cd "$(git rev-parse --show-toplevel)"
# -m2 and no -n: this file is its own subject, so an unbounded match also finds
# this command line and every copy of its own output below; two is the number of
# real prose occurrences above this block
command grep -m2 'D4 is the exception\|staging buffer already at capacity' \
  ring_flush/docs/api/002_the_driver_surface.md
```

Live output:

```
| D4 (`append`), staging buffer already at capacity | `Err( RingError::Full )` — the one condition on this surface that is an error, not a `FlushOutcome` |
D4 is the exception — see `FL7` below.
```

**Disposition:** applied — the blanket sentence now scopes itself to D1/D2/D3
and names D4 as the exception, and the Error Handling table gained the missing
row for D4's own failure condition (`RingError::Full` when the staging buffer
is already at capacity). The row states only what is verified — the error
variant and its trigger — and takes no position on whether the record is
destroyed or preserved on that path, which is `algorithm/001`'s own `FL1` to
settle, not this finding's. Now prints: `D4 is the exception`

### FL8 — The Row Deferring to an Open Decision Points at One That Closed

The last Error Handling row and the register it defers to disagree:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the deferral ( the prose below quotes it too, so stop at the findings ) --'
awk '/^### FL/{ exit } /Called after /{ print "    " substr( $0, 1, 96 ) }' \
  ring_flush/docs/api/002_the_driver_surface.md
echo '  -- the register it points at --'
command grep -E '^\| P5 \|' ring_flush/docs/decisions/readme.md
echo '  -- and the tests that closed it, on both readings --'
command grep -E '^fn (a_driver_still_works_after_a_final_drain|a_second_final_drain_is_an_empty_trigger)' \
  ring_flush/tests/flush_test.rs
```

Live output:

```
  -- the deferral ( the prose below quotes it too, so stop at the findings ) --
    | Called after `drain_final` | Open — see [`decisions/`](../decisions/readme.md) |
  -- the register it points at --
| P5 | Appends after `drain_final` | **Answered by measurement** — the flusher is reusable, and it is now a test rather than a discovery |
  -- and the tests that closed it, on both readings --
fn a_driver_still_works_after_a_final_drain()
fn a_second_final_drain_is_an_empty_trigger()
```

The row says Open. P5 says answered by measurement, and the answer is pinned by
tests: a driver still works after a final drain, and a second final drain is an
empty trigger.

**Both readings of the row are closed.** "Called after `drain_final`" is
ambiguous between a further *drive* and a further *append* — this is a table
about D1, D2 and D3, so the drive reading is the natural one, while P5's own
subject is appends. It does not matter which was meant: the drive case is
`a_driver_still_works_after_a_final_drain` and the append case is P5's own
measurement, and both landed.

That ambiguity is the reason to record this rather than simply correct it. A
deferral written as a table cell has no room to say *which* question it defers,
so it points at a register by name and inherits whatever that register says
next. When P5 closed, the person closing it had no way to discover that a cell
in a different directory was reading their entry as an answer to a slightly
different question. **A cross-document deferral survives its own resolution
because resolution is recorded at the destination and staleness is created at
the source** — the same failure `decisions/002`'s FL16 records from the other
end, where closing one half of a pair silently raised the other half's price.
