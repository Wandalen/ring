# Item: The Two Publications

### Scope

- **Purpose**: Take `try_publish` and `publish` one at a time — signature, contract, documented failure mode, and every call site — and record what the pair is for when one is a two-line loop around the other.
- **Responsibility**: Give each its contract as written, count how its return value is actually used, place its documented failure section against the family's, and state the division of labour the pair encodes.
- **In Scope**: `Publisher::try_publish` (`src/lib.rs:138-165`) and `Publisher::publish` (`:167-210`).
- **Out of Scope**: The three that read — see [`item/001`](001_the_three_readings_of_the_cursor.md).

### The Two, Side by Side

| | `try_publish` | `publish` |
|--|---------------|-----------|
| Signature | `( &self, start : Seq, len : usize ) -> Result< Seq, Seq >` | `( &self, start : Seq, len : usize ) -> Seq` |
| Blocks | never | until its turn comes |
| Body | one `compare_exchange`, one `map` | `loop { if let Ok(..) { return } spin_loop() }` |
| Documented section | `# Errors` | `# Panics` |
| Documented failure | *"not a failure … `compare_exchange`'s 'try again'"* | *"Never. … a caller … deadlocks here instead"* |
| `#[ must_use ]` | inherited from `Result` | **no** |
| Call sites in tests | 11 | 21 |
| Call sites in doctests | 6 | 2 |
| Return value branched on | **once, repo-wide** | never |

Same two arguments, same effect on the cursor, opposite answers to one question:
*who decides what to do when it is not your turn.*

### `try_publish` — The Whole Mechanism, in Two Lines

```rust
pub fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >
{
  let end = start.advanced_by( len as u64 );
  self.cursor.compare_exchange( start, end, PUBLISH, GATING ).map( | _ | end )
}
```

Line one computes the destination; line two attempts the move and rewrites the
success value. `compare_exchange` returns the *previous* value on success, which
here is `start` — information the caller already had — so `.map( | _ | end )`
replaces it with the only thing worth returning.

Its `# Errors` section (`:140-146`) is the crate's most carefully worded
paragraph, because the section header is misleading and it says so:

> The current published position, when it is not `start` — meaning some earlier
> claim has not been published yet. Deliberately not a `RingError`: this is not a
> failure, it is `compare_exchange`'s "try again", and the value returned is what
> to try against next.

The last clause is the part that turns out not to matter — see PB24 below and
[`algorithm/001`](../algorithm/001_the_compare_exchange_that_refuses.md) § PB8.

**Every one of its 11 test call sites is inside an `assert_eq!`.** Not one uses
the result to decide anything:

| Where | Asserting |
|-------|-----------|
| `tests/publish_test.rs:67,68` | `Ok( Seq( 4 ) )`, `Ok( Seq( 5 ) )` — the exact frontier |
| `tests/publish_test.rs:80,83,84` | a refusal, then both turns taken in order |
| `tests/publish_test.rs:96,97` | `Err( Seq( 8 ) )` twice — *"not even re-publishing"* |
| `tests/publish_test.rs:110` | `Ok( Seq( 5 ) )` for a zero-length publication |
| `tests/handshake_test.rs:369,378,379` | out-of-order refused, then both accepted |

### `publish` — The Loop, and the Word "Never"

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

Nine lines, no state, no budget, and the crate's only `loop`. It adds exactly one
thing to `try_publish`: the decision that the correct response to a refusal is to
try again unchanged.

Its return value is discarded at **15 of 21** call sites, and correctly so — `end`
is `start.advanced_by( len )`, both of which the caller passed in.
[`api/001`](../api/001_six_methods_and_no_caller.md) § PB11 records the split;
the residue is four `assert_eq!` sites (`tests/publish_test.rs:121-123,150`) and
two closure tails (`:139`, `:176`) where the value is returned rather than used.

### PB23 — The Family Has Two `# Panics` Sections Reading "Never", and Only One Names a Deadlock

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rc '# Panics' ring_*/src/*.rs | grep -v ':0'
grep -rc '# Errors' ring_*/src/*.rs | grep -v ':0'
```

Live output:

```
ring_batch/src/lib.rs:1
ring_bench/src/lib.rs:3
ring_store/src/lib.rs:2
ring_index/src/lib.rs:1
ring_publish/src/lib.rs:1
ring_wait/src/lib.rs:1
ring_atomic/src/lib.rs:1
ring_barrier/src/lib.rs:1
ring_batch/src/lib.rs:1
ring_bench/src/lib.rs:5
ring_claim/src/lib.rs:2
ring_config/src/lib.rs:1
ring_consume/src/lib.rs:1
ring_core/src/lib.rs:3
ring_debug/src/lib.rs:4
ring_event/src/lib.rs:2
ring_factory/src/lib.rs:3
ring_flush/src/lib.rs:2
ring_gating/src/lib.rs:1
ring_handle/src/lib.rs:1
ring_mpsc/src/lib.rs:2
ring_overflow/src/lib.rs:1
ring_poll/src/lib.rs:1
ring_publish/src/lib.rs:1
ring_registry/src/lib.rs:1
ring_shutdown/src/lib.rs:5
ring_slot/src/lib.rs:1
ring_spsc/src/lib.rs:3
ring_testkit/src/lib.rs:3
ring_tls/src/lib.rs:1
ring_types/src/capacity.rs:1
ring_wait/src/lib.rs:4
```

Across 33 crates: **52 `# Errors` sections in 26 files, and 9 `# Panics`
sections in 6 files** — of which 4, in 3 files, are discussed below. This crate
has one of each, on adjacent methods.

Two of the four `# Panics` sections say "Never", and they belong to the family's
two waiting loops:

| | `ring_wait::wait_until` (`:176-178`) | `ring_publish::publish` (`:183-188`) |
|--|--------------------------------------|--------------------------------------|
| Text | *"Never. The budget is a `usize` count and the loop is bounded by it."* | *"Never. The loop exits when the predecessor publishes, which it is committed to doing; a caller that publishes a range it never claimed deadlocks here instead …"* |
| Why never | the loop is **bounded** | the loop is **unbounded but guaranteed to exit** |
| Worse outcome named | none — there is none | **deadlock** |
| Sentences | 1 | 3 |

The asymmetry is the finding. `ring_wait`'s "Never" is complete in one sentence
because a bounded loop cannot do anything worse than return early.
`ring_publish`'s needs three, because the honest answer to *"does this panic?"*
is *"no — it does something worse, under a precondition the type system does not
express."*

The other two `# Panics` sections, both in `ring_store` (`:167`, `:182`), do the
opposite thing: they document a panic that *can* happen and argue it is
unreachable — *"a `SlotIndex` reaching this crate came from `ring_index::of`,
which cannot produce one out of range"*. Three sections, three different
relationships between a documented panic and reality, in a family that mostly
does not write the section at all.

`publish`'s precondition — *publish only what you claimed* — is stated in this
section and nowhere else in the signature.
[`pitfall/001`](../pitfall/001_publishing_a_range_you_never_claimed.md) records
what breaking it costs and why no check catches it.

### PB24 — One Branch, Repo-Wide, on the `Result` That Exists To Be Branched On

`try_publish` returns `Result< Seq, Seq >` specifically so a caller can decide
for itself — `:189` calls it *"the variant for a caller that wants to decide"*.
Counting the deciders:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE '(if let|match|\.is_ok\(|\.is_err\(|\?)' --include='*.rs' . \
  | grep 'try_publish'
```

Live output:

```
ring_publish/src/lib.rs:      if let Ok( end ) = self.try_publish( start, len )
```

**One hit, repository-wide:**

```
ring_publish/src/lib.rs:204:      if let Ok( end ) = self.try_publish( start, len )
```

The only caller in the repository that branches on `try_publish`'s result is
`publish` — the method whose entire purpose is to make branching unnecessary.
Every other site, in tests and doctests alike, either asserts the value
(15 sites — 11 in tests, 4 in doctests at `:81`, `:155`, `:158`, `:159`) or
discards it (`:129`, `:211`, two `let _ =` setup lines in the doctests of
`published` and `is_published`). Eighteen sites, one decision.

Three things follow, in decreasing certainty:

1. **The `Err( Seq )` payload is unused.** It is asserted five times and consumed
   zero — recorded as [`algorithm/001`](../algorithm/001_the_compare_exchange_that_refuses.md)
   § PB8. `publish` ignores it (`if let Ok`, no `Err` arm) because a producer may
   only publish what it claimed, so the frontier's current value is not an input
   to anything it will do next.
2. **`try_publish`'s justification is entirely forward-looking.** It exists for a
   caller that wants a deadline, a diagnostic, or a give-up
   ([`decisions/002`](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md)),
   and no such caller exists — consistent with the crate having no callers at all
   ([`api/001`](../api/001_six_methods_and_no_caller.md) § PB10).
3. **The pair is not redundant, but its evidence is thin.** `publish` genuinely
   needs `try_publish` as its body; nothing needs `try_publish` on the public
   surface. Making it private would cost the eleven direct tests that pin the
   refusal semantics — which is a real cost, since those tests are how
   [`decisions/001`](../decisions/001_refused_rather_than_reordered.md) is
   checked behaviourally at all.

Point 3 is why the surface keeps both. The public `try_publish` is, in practice,
a testability seam that is *also* documented as a caller affordance; only the
first of those two roles has ever been exercised.

### The Division of Labour

| Question | `try_publish` | `publish` |
|----------|---------------|-----------|
| Did the publication happen? | the caller reads the `Result` | always, by the time it returns |
| What if it is not my turn? | **caller decides** | spin, unconditionally |
| Can it not return? | no | yes, if the precondition is broken |
| Can it be called from a tick path? | yes | no — no bound, no deadline |
| Is it in the family's waiting-primitive census? | no | yes — the crate's one hit ([`decisions/002`](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md) § PB18) |

The pair is the standard `try_*` / blocking split, with the unusual property that
the blocking variant's wait is *guaranteed to end* rather than merely *likely
to* — which is what lets it drop the budget, the strategy, and the `Result` all
at once, and is the whole content of
[`algorithm/002`](../algorithm/002_a_loop_with_no_budget.md)'s termination
argument.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_compare_exchange_that_refuses.md](../algorithm/001_the_compare_exchange_that_refuses.md) | `try_publish`'s two lines, and the unconsumed `Err` payload |
| [../algorithm/002_a_loop_with_no_budget.md](../algorithm/002_a_loop_with_no_budget.md) | `publish`'s loop, and why it terminates |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_six_methods_and_no_caller.md](../api/001_six_methods_and_no_caller.md) | The full surface, and the 15-of-21 discard split |
| [../api/002_a_result_whose_error_is_not_an_error.md](../api/002_a_result_whose_error_is_not_an_error.md) | Why the error type is `Seq` and not `RingError` |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_refused_rather_than_reordered.md](../decisions/001_refused_rather_than_reordered.md) | The refusal both methods implement |
| [../decisions/002_a_plain_spin_rather_than_a_wait_kind.md](../decisions/002_a_plain_spin_rather_than_a_wait_kind.md) | Why `publish` takes neither a strategy nor a budget |

### Items

| File | Relationship |
|------|--------------|
| [001_the_three_readings_of_the_cursor.md](001_the_three_readings_of_the_cursor.md) | The three that observe what these two move |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_publishing_a_range_you_never_claimed.md](../pitfall/001_publishing_a_range_you_never_claimed.md) | The precondition `# Panics` states and nothing enforces |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:138-210` | Both methods, both documented sections, both doctests |
| `ring_wait/src/lib.rs:176-179` | The family's other "Never." — bounded, one sentence |
| `ring_store/src/lib.rs:195-202, 211-213` | The family's other two `# Panics` — reachable, argued unreachable |
| `ring_types/src/id.rs:65-68` | `advanced_by`, which makes `end` derivable from the arguments |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:62-112` | `try_publish` at all four boundaries: exact, past, behind, zero-length |
| `tests/publish_test.rs:116-124` | `publish` returning the end of what it published |
| `tests/publish_test.rs:126-155` | `publish` waiting rather than reordering |
| `tests/handshake_test.rs:356-380` | Both methods on the same publisher, in the same test |
| `tests/handshake_test.rs:409-432` | `publish` under real backpressure, one lap deep |
