# Algorithm: Evaluating a Policy at an Append

### Scope

- **Purpose**: Give the hot-path procedure — the comparison run on every append to decide whether a flush is due — and fix the cost ceiling it must stay under.
- **Responsibility**: State the steps, the per-variant comparison, and the operations forbidden on this path.
- **In Scope**: Policy consultation; the counter `OnBatch` maintains; branch behaviour.
- **Out of Scope**: What happens when the answer is yes (→ [Sequencing Seal, Drain and Reset](002_sequencing_seal_drain_reset.md)); the driver's own cadence (→ [The Driver Surface](../api/002_the_driver_surface.md)).

### Abstract

This procedure runs **once per appended record**, which places it in the same
cost class as the append itself. `ring_tls`'s acceptance criterion requires
that a `TlsBuffer` "accumulates `N` items with zero atomic operations, asserted
by a counting allocator/atomic shim" — a policy consulted on that path and
performing an atomic would fail *another crate's* acceptance test.

**That is the constraint that shapes everything below:** the evaluation is
three comparisons against local state, and its correctness is less interesting
than its cost.

### Algorithm

**Step 1 — Read the policy.** A `Copy` enum held by value; no indirection, no
lock, no atomic (→ [Flush Policy](../type/001_flush_policy.md)'s trait table).

**Step 2 — Dispatch on the variant.**

| Variant | Comparison | State touched |
|---------|-----------|---------------|
| `OnFull` | `buffer.is_full()` | Buffer's own length — already read by the append |
| `OnBatch( n )` | `buffer.len() >= n` | Buffer's own length. **No separate counter** |
| `OnBarrier` | **None.** Always false on this path | — |

**Step 3 — Return the decision.** A `FlushCause` if the policy fired, `None`
if not; the richer [`FlushOutcome`](../type/002_flush_outcome.md) is produced by
the driver, from that cause plus what the claim did.

### Step 3 used to be a counter update, and it was deleted

This procedure originally had four steps. The third incremented a
thread-local `usize` holding records-staged-since-the-last-flush, which
`OnBatch` then compared against `n`. It was written that way because "records
since the last flush" reads like state a policy has to maintain.

**It is not. It is the buffer's own occupancy, on every path.** The driver owns
the buffer privately, only the append adds to it, only a successful flush
empties it, and a rejected flush leaves both untouched together. The counter
could never disagree with `TlsBuffer::len`, so it was duplicated state — and an
increment on the one path this crate is constrained not to make expensive
(→ [`nfr/002`](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md)).

**The redundancy was found by probe, not by reading** — the injection recorded
as `tests/manual/readme.md`'s F3 was aimed at something else entirely and turned
this up as a side effect. Deleting the field left every test green, which is the
behavioural evidence that the two quantities never diverged.

The append path is now a single `push` into the buffer and nothing else. Step 2
runs on the *drive*, not the append, so the append pays no comparison at all.

### `OnBarrier` returns false here, always

**This is the structural consequence of [`pattern/002`](../pattern/002_driven_not_self_firing.md)
and it looks like a bug on first reading.** The barrier is announced through
the driver, not observed during an append, so on the append path there is
nothing to test. An `OnBarrier` buffer therefore accumulates until a drive call
announces the barrier, no matter how full it becomes.

**Which raises the question this procedure must not answer on its own:** what
happens when an `OnBarrier` buffer fills before the barrier arrives? Three
options, none free —

| Option | Cost |
|--------|------|
| Flush anyway | `OnBarrier` silently becomes `OnFull` — [trigger exclusivity](../invariant/001_a_policy_fires_only_at_its_trigger.md)'s V1, and the benchmark compares a policy nobody configured |
| Reject the append | Correct and loud, but the writer must handle it, and no writer expects a staging buffer to refuse |
| Overflow per `ring_overflow`'s policy | Consistent with the family; requires a dependency this crate does not declare |

**The third is almost certainly right and is not this crate's to choose** —
overflow handling belongs to `ring_overflow`, and a separate ruling puts the
discriminant in `ring_types`. Recorded in [`decisions/`](../decisions/readme.md)
rather than settled here, because choosing option 1 by default is how the
family ends up with two policies wearing one name.

**The implementation had to answer anyway, and chose option 2.**
`Flusher::append` returns `Err( RingError::Full )` when the staging buffer
cannot take another record, whatever the policy is. Option 2 is the only one of
the three that is both loud and local: option 1 *is* V1, and option 3 needs a
dependency this crate is not entitled to add on its own.

This is a default, not a ruling. The deferral above stands — if `ring_overflow`
is wired in later, this is the line that changes, and the reason it is safe to
change is that a refusal is the conservative direction to be wrong in. What it
costs is stated rather than hidden: a writer must handle a refusal from a
staging buffer, which is not what a writer expects.
`on_barrier_never_fires_without_an_announcement` drives the buffer past full on
purpose, so the refusal is pinned by a test rather than left incidental.

### Forbidden on this path

| Operation | Why |
|-----------|-----|
| Any atomic | Fails `ring_tls`'s zero-atomic criterion, from another crate's test |
| Any allocation | Same criterion |
| Any lock | The staging design exists to avoid a lock per write |
| Any syscall, any clock read | A timer-based policy would need one; there is deliberately no timer variant |
| Writing to the flush log | The log records flushes, not evaluations (→ [The Flush Log](../data_structure/002_the_flush_log.md)) |
| Any parking or blocking | The family's no-parking-on-the-tick-path rule |

**The clock row is why there is no `OnInterval` variant.** It is the obvious
fourth policy and it would put a clock read on the append path, which no
version of this procedure can afford. A time-based flush is expressible as
`OnBarrier` with a driver called on a timer — the cost moves to the caller's
cadence, where it is already being paid.

**The log row prevents a plausible mistake.** Recording every evaluation would
make the acceptance criterion's log grow per append rather than per flush,
turning a test-only structure into a hot-path allocation
(→ [The Flush Log](../data_structure/002_the_flush_log.md)'s own hazard).

### Algorithms

| File | Relationship |
|------|--------------|
| [002_sequencing_seal_drain_reset.md](002_sequencing_seal_drain_reset.md) | What runs when this returns true |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_policy_enum.md](../data_structure/001_the_policy_enum.md) | The value read in step 1, and why its size matters here |
| [../data_structure/002_the_flush_log.md](../data_structure/002_the_flush_log.md) | Explicitly not written from this path |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md](../non_functional_requirement/002_the_decision_costs_nothing_on_the_append_path.md) | This procedure's cost ceiling, with a measurement method |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_driven_not_self_firing.md](../pattern/002_driven_not_self_firing.md) | Why `OnBarrier`'s row is empty |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_policy_arming_and_firing.md](../lifecycle/004_policy_arming_and_firing.md) | Step 3's counter as the only state any variant carries |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_tls/readme.md`](../../../ring_tls/readme.md) | The zero-atomic accumulation criterion this path must not break |
| [`ring_overflow/readme.md`](../../../ring_overflow/readme.md) | Owns the third option for the `OnBarrier`-fills case |
| [`ring_poll/readme.md`](../../../ring_poll/readme.md) | The no-parking constraint, claimed by `ring_poll` and binding here |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | `on_barrier_never_fires_without_an_announcement` pins it: the buffer is filled to capacity, then driven twenty times without an announcement, and the log must be empty. The open question is settled the way this instance wanted — fullness is not `OnBarrier`'s trigger |

### FL1 — The Refusal This Procedure Defaults To Destroys the Record It Refuses

Option 2 was chosen because "a refusal is the conservative direction to be wrong
in." The refusal path was not read to the bottom:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the pre-fix promise, searched for in the current source (expect 0 -- corrected below) --'
printf '    hits: %s\n' "$( command grep -c 'returned to the caller by never being taken' ring_flush/src/lib.rs )"
echo '  -- its signature, and what it forwards to --'
command grep -E 'pub fn append|self\.buffer\.push' ring_flush/src/lib.rs
echo '  -- and what that forwards into --'
awk '/pub fn push\( &mut self, item : T \)/{ i = 1 } i { print "    " NR ": " $0 } i && /^  \}/{ exit }' \
  ring_tls/src/lib.rs
```

Live output:

```
  -- the pre-fix promise, searched for in the current source (expect 0 -- corrected below) --
    hits: 0
  -- its signature, and what it forwards to --
  pub fn append( &mut self, record : T ) -> Result< (), RingError >
    self.buffer.push( record )
  -- and what that forwards into --
    125:   pub fn push( &mut self, item : T ) -> Result< (), RingError >
    126:   {
    127:     if self.items.len() >= self.limit
    128:     {
    129:       return Err( RingError::Full );
    130:     }
    131:     let reserved = self.items.capacity();
    132:     self.items.push( item );
    133:     debug_assert!
    134:     (
    135:       self.items.capacity() == reserved,
    136:       "push must never grow TlsBuffer's allocation past its with_capacity reservation"
    137:     );
    138:     Ok( () )
    139:   }
```

`Flusher::append` takes `record : T` **by value**, hands it to
`TlsBuffer::push` **by value**, and `push` returns `Err( RingError::Full )` from
a branch taken before `self.items.push( item )` runs. `item` is a local owned
binding at that point, so it drops when `push` returns. `RingError::Full` is a
unit variant carrying nothing, and `append`'s return type is
`Result< (), RingError >` — **there is no channel through which the record could
come back**, so the doc comment's promise is not merely unimplemented, it is
unrepresentable in the signature that states it.

The record is destroyed. Silently. On the refusal path, in the crate whose
entire subject is that records reach the ring.

**This changes the ranking of the three options above.** Option 2 was selected
over option 3 (`ring_overflow`'s policy) partly on the ground that a refusal is
"loud and local" and the conservative direction to be wrong in. A refusal that
drops the payload is neither conservative nor loud: the caller sees an `Err`,
reasonably concludes the record is still theirs to retry or route, and it is
already gone. Option 3 — which routes an unplaceable record through a declared
policy — is the one that actually preserves it.

The defect is `ring_tls`'s to fix; the signature that would fix it is
`push( item : T ) -> Result< (), ( RingError, T ) >`, and this crate's `append`
would have to widen with it. What belongs here is that **this crate's own
documented behaviour is wrong about its own dependency**, and the pending
decision above was ruled on the strength of that wrong description.

`append`'s doc comment no longer makes the promise this finding disproved:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A9 '\[`RingError::Full`\] when the buffer already holds' \
  ring_flush/src/lib.rs
```

Live output:

```
  /// [`RingError::Full`] when the buffer already holds `capacity()` records.
  /// **The record is not returned on refusal — it is dropped.**
  /// `TlsBuffer::push` (`ring_tls`) has no channel to hand it back: its
  /// signature is `Result< (), RingError >` and `RingError::Full` carries
  /// nothing, so `record` drops silently inside `push` before this function
  /// returns. Retrying with the same value is therefore not possible; the fix
  /// is `ring_tls`'s (a widened `push( item : T ) -> Result< (), ( RingError,
  /// T ) >`), which this signature would need to widen with.
  pub fn append( &mut self, record : T ) -> Result< (), RingError >
  {
```

**Disposition:** applied — `Flusher::append`'s doc comment in
`ring_flush/src/lib.rs` no longer claims the refused record is
returned; it now states plainly that the record is dropped, names
`TlsBuffer::push`'s signature as the reason no return channel exists, and
points at the exact widened signature (`ring_tls`'s, out of this crate's
scope to change) that would fix it — the same fix this finding itself
names. This is a documentation correction, not a behavior change: `append`
still drops a refused record exactly as before, but no longer tells a
caller it can retry. The crate's test suite re-verified passing (`cargo test
-p ring_flush --all-features`, 2026-09-04). Now prints: `Retrying with the
same value is therefore not possible; the fix`

### FL2 — The Criterion Shaping This Path Is Not in the Feature It Cites, and Its Owner Records It as Unenforced

The Abstract used to attribute the cost ceiling to an external citation that
did not contain it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the quoted text, in the feature it is attributed to --'
printf '    hits in docs/feature/175: %s\n' \
  "$( command grep -c 'zero atomic' docs/feature/175_thread_local_buffer_and_flush_into.md )"
echo '  -- where it actually lives, and who owns it --'
command grep -n '175 | Thread-local buffer' bench_harness/docs/acceptance/001_feature_reached_tests.md \
  | sed -E 's/^(.{0,150}).*/\1/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and what the owning crate records about enforcing it --'
command grep -n 'TL31' ring_tls/docs/invariant/readme.md | sed -E 's/^(.{0,190}).*/\1/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  -- the quoted text, in the feature it is attributed to --
    hits in docs/feature/175: 0
  -- where it actually lives, and who owns it --
| 175 | Thread-local buffer and flush-into | `ring_tls` | S2 | A `TlsBuffer` accumulates `N` items with zero atomic operations (asserted by a coun
  -- and what the owning crate records about enforcing it --
| TL31 | the test coverage | n/a — coverage | Feature 175's reached-test asserts zero allocations "by a counting allocator", and `ring_tls` carries no `#[ global_allocator ]` to do the as
```

The criterion is real and the quotation is accurate — it is simply somewhere
else. It lives in `bench_harness`'s acceptance register as `ring_tls`'s
reached-test, and the register names **`ring_tls`** as the crate that owns it.
Nothing was fabricated; the external citation this crate's Abstract used to
point at carried a Definition and an If Missing, but no criterion at all, with
no way for a reader to tell whether the constraint existed.

**The owner's own record is the part that matters.** `ring_tls` files TL31
against itself: the reached-test asserts zero allocations "by a counting
allocator", and `ring_tls` carries no `#[ global_allocator ]` to do the
asserting — though four sibling crates now do. So the allocator half of the
criterion is declared and unenforced in the crate that owns it, and this
crate's hot-path design — the deleted counter, the forbidden operations table,
the whole shape of step 2 — defers to it as a hard constraint without recording
that nothing currently checks it.

That deference is still correct. Designing to an unenforced criterion is the
right call when the criterion is right, and it is. What is missing is the
sentence saying so: **this procedure's cost ceiling is currently held by
argument, in two crates, and by measurement in neither.**
