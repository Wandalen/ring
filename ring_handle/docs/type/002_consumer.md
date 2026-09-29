# Type: Consumer

### Scope

- **Purpose**: Define the draining handle as a value, and state the property that makes it unlike its counterpart — its *location* in the program is a correctness parameter, and no rule about the type itself constrains that.
- **Responsibility**: The definition and the validation rules governing it.
- **In Scope**: `Consumer< T >`'s identity, representation, auto-trait status, and the placement question the type cannot answer.
- **Out of Scope**: Its methods (→ [Consumer Surface](../api/002_consumer_surface.md)); where it should be placed (→ [The Barrier Holds the Consumer](../lifecycle/002_the_barrier_holds_the_consumer.md)).

### Definition

**A `Consumer< T >` is the exclusive right to drain one ring, expressed as an
owned value — and, because it is exclusive, it is also the ring's consume
point.**

The second clause has no counterpart on the producer side and it is the reason
this type is worth documenting separately rather than as "the same thing,
mirrored." The design requires that "the consumer runs at a defined point — end
of stage, end of tick — and nowhere else," and this type is the mechanism:
there is one of it, so wherever it is held *is* that point, by construction.

| Property | Value |
|----------|-------|
| Kind | Struct, generic over the item type `T` |
| Fields | One — a reference to the shared backend |
| Size | One pointer |
| Constructed by | The split, and nothing else |
| Constructed how many times per ring | Exactly once |
| Exported | **Yes** — reached through `ring_handle`, one of the five Contract names |

| Trait | Status | Why |
|-------|--------|-----|
| `Send` | **Required** | The barrier that holds it may not be the thread that created it — N2 |
| `Sync` | **Deliberately not granted** | Two threads draining concurrently is two consume points |
| `Clone` | **Forbidden** | A second consumer does not corrupt anything — it destroys replayability, which is worse in the specific sense that nothing fails |
| `Copy` | **Forbidden** | Same |
| `Debug` | Permitted | Must not drain to produce its output, which would make debugging destructive |
| `Deref` | **Forbidden** | Restores publish capability alongside drain |
| `Default` | **Meaningless** | No ring to drain |
| `Drop` | Permitted, must not drain | A draining `Drop` discards outstanding items silently; `drain_all()`, `ring_shutdown`'s explicit operation, is what exists for that |

**The `Clone` row's reasoning differs from `Producer`'s and the difference is
worth stating.** A cloned `Producer` races: two writers, one slot, corruption
that a sanitizer can find. A cloned `Consumer` races nothing — each drains a
well-formed disjoint subset, no memory is corrupted, every functional test
passes, and two runs of identical inputs now consume different prefixes.
This carries the consequence: "replay diverges and a test that passes proves
nothing about the next run."

**`Debug`'s constraint is unusual enough to be easy to get wrong.** The obvious
implementation of `Debug` for a queue handle prints the pending items — which
requires reading them, which for a ring means advancing past them or borrowing
across slots the producer may reuse. A `Debug` that drains is a debugger that
changes the program.

### Validation

| # | Rule | Enforced by | Detected when |
|---|------|-------------|---------------|
| C1 | Exactly one `Consumer` exists per ring | The split returning it by value, and `!Clone` | Compile time, for the duplication route |
| C2 | No publish-shaped method exists on it | Absence, plus `tests/ui/consumer_publishes.rs` | Compile time — the acceptance criterion's second case |
| C3 | It is `Send` | The backend field's bounds | Compile time, via static assertion |
| C4 | It is not `Sync` | `tests/ui/producer_shared_across_threads.rs`, structurally | Compile time for `Producer`. The `Consumer` shares the cause — the same `PhantomData< Cell< () > >` inherited from `ring_spsc` — so one case covers both in practice and neither in principle; a `Consumer`-only regression would pass |
| C5 | It is not `Clone` | `tests/ui/consumer_clones.rs` | Compile time. Written for symmetry with the producer case, because "it's only reading" is the argument that makes a second consumer sound harmless — it advances the same read cursor |
| C6 | It is exactly the size of the `ring_core` handle it wraps (16 bytes — a discriminant and a reference, no policy field) | `the_wrapper_costs_nothing` | A `size_of` equality catches an added field |
| C7 | `Debug` does not consume items | Nothing | Not detected by any mechanism short of review |
| C8 | **It is held at a barrier** | **Nothing whatsoever** | **Not detected, not detectable, and not this crate's to enforce** |

**C8 is the entry that matters and the only one with no path to enforcement.**
Every other rule in this table is at least in principle checkable. C8 is a
claim about where in a program a value is stored, and a value that is `Send` and
owned can be stored anywhere. A `Consumer` moved into a mid-tick system
satisfies C1 through C7 perfectly and destroys the determinism the whole
arrangement exists to buy.

**This is the type's honest boundary.** This crate's contribution is making it
so "the barrier can hold the only thing capable of draining" — *can*, not
*does*. Concentrating the capability into one
movable value is necessary for the barrier to hold it exclusively, and it is not
sufficient for the barrier to be where it ends up. Whatever enforces C8 lives in
the scheduler or the system-registration layer, above this family entirely
(→ [The Barrier Holds the Consumer](../lifecycle/002_the_barrier_holds_the_consumer.md)).

**C7 is worth its row despite being unenforceable**, because the wrong `Debug`
impl is the natural one and its failure is silent: a log line added during
debugging changes which items the barrier later sees.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | What this value can do, and why `drain()`'s bound is fixed at call time |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_splitting_a_ring_into_two_ends.md](../algorithm/001_splitting_a_ring_into_two_ends.md) | The only construction site — C1's mechanism |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) | The single field determining C3, C4 and C6 |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | C2 as its V2, and the explicit statement that this crate does not enforce C8 |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_barrier_holds_the_consumer.md](../lifecycle/002_the_barrier_holds_the_consumer.md) | C8 worked out — what is needed above this family for it to hold |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_send_without_sync.md](../non_functional_requirement/002_send_without_sync.md) | C3 and C4 as criteria |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_ring_liveness_through_a_handle.md](../lifecycle/004_ring_liveness_through_a_handle.md) | The closed-and-drained state `Drop` must not silently produce |

### Types

| File | Relationship |
|------|--------------|
| [001_producer.md](001_producer.md) | The complementary right, whose `Clone` row fails differently from this one's |

### Sources

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | "A consumer handle can drain and cannot publish"; and "the barrier can hold the only thing capable of draining" — C8's *can* |
| [../lifecycle/002_the_barrier_holds_the_consumer.md](../lifecycle/002_the_barrier_holds_the_consumer.md) | Why C1 and C8 are determinism requirements rather than API preferences |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `both_handles_are_send` | `assert_send::< Consumer< '_, u32 > >()` — C3 |
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `every_handle_can_be_printed` | Formatting a `Consumer` with `Debug` leaves `len()` unchanged — C7, the one unenforceable-by-types rule that a test can still catch. Extended to `Drain` too, where the temptation is stronger: a `Debug` that enumerated pending records to print them would drain the ring as a side effect of a log line |
| [`tests/ui/consumer_publishes.rs`](../../tests/ui/consumer_publishes.rs) | C2 — the acceptance criterion's second compile-fail case |

### HD43 — The Crate Claims Four Rules Have No Detector, and Is Wrong About Exactly the Two Somebody Wrote a Test For

C7's row says `Debug` non-consumption is enforced by "Nothing" and "Not
detected by any mechanism short of review." Eighty rows later the same file
names the mechanism:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every rule row in this crate claiming no detector --'
command grep -rE '^\| (U|C|M)[0-9] \|.*(Nothing|Not detected|no compile-time detector)' \
  ring_handle/docs | cut -c1-121 | sed 's|^|    |'
echo '  -- and the rows in the same files that cite a detector for two of them --'
command grep -H -E '^\| \[?`tests/' \
  ring_handle/docs/type/002_consumer.md \
  ring_handle/docs/lifecycle/003_handle_ownership.md \
  | command grep -E 'C7|M9' \
  | sed 's|ring_handle/docs/||; s|(\.\./[^)]*)||' | sed -E 's/^(.{0,152}).*/\1/' | sed 's|^|    |'
```

Live output:

```
  -- every rule row in this crate claiming no detector --
    ring_handle/docs/type/001_producer.md:| U7 | No public route reaches the backend | Absence | **Not detected** — no
    ring_handle/docs/type/002_consumer.md:| C7 | `Debug` does not consume items | Nothing | Not detected by any mechanis
    ring_handle/docs/type/002_consumer.md:| C8 | **It is held at a barrier** | **Nothing whatsoever** | **Not detected, 
    ring_handle/docs/lifecycle/003_handle_ownership.md:| M9 | any | H1 by cloning | — | **Forbidden**, and this is the
  -- and the rows in the same files that cite a detector for two of them --
    type/002_consumer.md:| [`tests/handle_test.rs`] · `every_handle_can_be_printed` | Formatting a `Consumer` with `Debug` leaves `len()` unchanged — C7, th
    lifecycle/003_handle_ownership.md:| `tests/ui/producer_clones.rs` | M9 — no longer without a detector; the case is outside this crate's stated criterio
    lifecycle/003_handle_ownership.md:| `tests/ui/producer_clones.rs` | M9 — no longer without a detector; the case is outside this crate's stated crit
```

Four rows in this crate declare a rule undetectable. Two of them — C7 here and
M9 in [`lifecycle/003`](../lifecycle/003_handle_ownership.md), recorded there as
HD31 — are contradicted by a Tests row in their own file. The prose reinforces
the stale half: "C7 is worth its row despite being unenforceable" sits three
paragraphs above the row citing the test that enforces it.

**The pattern is what makes this worth a second finding rather than a second
correction.** The two rows that are still accurate, U7 and C8, are accurate
because nobody wrote anything for them. The two that are wrong are wrong
because somebody did, updated the Tests table, and left the rule row alone. So
the "Enforced by" column is reliable precisely where it reports failure and
unreliable precisely where the crate improved — **a reader consulting it to find
the gaps is reading a record of the state at authoring time, presented as a live
claim.**

The Tests table is the one that gets updated because adding a test is what
prompts the edit. Any repair that only fixes C7 leaves the mechanism intact;
what the two tables need is a direction of derivation — the rule rows read from
the test citations rather than being maintained beside them.

### HD44 — The `Debug` Constraint Is Kept Four Crates Down by a Hand-Written Impl That Has Never Heard of This Row, and the Local Test Cannot See It

Two `Debug` constraints are stated in this crate's type instances: this file's
"must not drain to produce its output," and
[`type/001`](001_producer.md)'s "must not print the backend's contents." This
crate writes a derive:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the derive this crate writes, and the two below it --'
for c in ring_handle ring_core ring_spsc; do
  command grep -nB1 -E '^pub struct (Consumer|Ring)' ring/$c/src/lib.rs \
    | command grep -E 'derive|pub struct' | sed "s|^|    $c |" | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
done
echo '  -- where the constraint is actually kept --'
sed -n '/^impl< S > core::fmt::Debug for Ring< S >/,/^  }/p' ring_spsc/src/lib.rs \
  | command grep -E 'never slot|debug_struct|\.field|finish' | sed 's|^|    |'
echo '  -- and what this crate asserts about it --'
sed -n '/^fn every_handle_can_be_printed/,/^}/p' ring_handle/tests/handle_test.rs \
  | command grep -E 'assert' | sed 's|^|    |'
```

Live output:

```
  -- the derive this crate writes, and the two below it --
    ring_handle 178-#[ derive( Debug ) ]
    ring_handle 179:pub struct Consumer< 'a, T >
    ring_core 119-#[ derive( Debug ) ]
    ring_core 120:pub struct Ring< T >
    ring_core 528-#[ derive( Debug ) ]
    ring_core 529:pub struct Consumer< 'a, T >
    ring_spsc 239:pub struct Ring< S >
    ring_spsc 802-#[ derive( Debug ) ]
    ring_spsc 803:pub struct Consumer< 'a, S >
  -- where the constraint is actually kept --
      /// Cursor positions and capacity — never slot contents.
        f.debug_struct( "Ring" )
          .field( "capacity", &self.cursors.capacity().get() )
          .field( "produced", &self.cursors.producer().load( GATING ) )
          .field( "consumed", &self.cursors.consumer().load( GATING ) )
          .finish_non_exhaustive()
  -- and what this crate asserts about it --
      assert!( format!( "{split:?}" ).contains( "Split" ) );
      assert!( format!( "{ends:?}" ).contains( "Ends" ) );
      assert!( format!( "{producer:?}" ).contains( "Producer" ) );
      assert_eq!( loaded, 2 );
      assert!( format!( "{consumer:?}" ).contains( "Consumer" ) );
      assert_eq!( consumer.len(), loaded, "formatting the consumer drained it" );
      assert!( format!( "{:?}", consumer.drain() ).contains( "Drain" ) );
      assert_eq!( consumer.len(), loaded, "formatting a Drain drained it" );
```

`#[ derive( Debug ) ]` here delegates to `ring_core::Consumer`, which derives to
`ConsumerInner`, which derives to `ring_spsc::Consumer`, which holds
`&'a Ring< S >`. `ring_spsc::Ring` is the one link in the chain with no derive —
its `Debug` is written by hand, prints capacity and the two cursor positions,
and closes `finish_non_exhaustive()`.

**So the constraint holds, and it holds for a different reason than the one
stated here.** That impl's own comment gives soundness as the motive: formatting
the slots would read ones the producer may be writing this instant. This file
gives destructiveness, `type/001` gives noise in a panic message. Three
statements of one rule, in two crates, none citing another, and the only
executable one is the lowest.

**What runs here cannot notice if that changes.**
`every_handle_can_be_printed` makes two kinds of assertion — the output contains
the type's name, and `len()` is unchanged across the call. A `Debug` that
printed every pending record would satisfy both: the string still contains
`"Consumer"`, and printing is not draining. The test covers this file's
constraint and is blind to `type/001`'s, which is the one the derive chain
actually decides.

A local guard is available and cheap — bound the formatted length independently
of the item count, by formatting a ring holding one item and a ring holding many
and asserting the two strings are the same length. It is the same shape as the
`size_of` equality in `the_wrapper_costs_nothing`: assert the property, not the
figure (→ [HD41](001_producer.md#hd41--the-definition-table-says-one-pointer-the-validation-table-says-three-times-that-and-the-suite-pins-neither)).

The local guard is now written. `2>/dev/null` below drops cargo's own
build-progress chatter (compile timing and the hashed binary path both vary
run to run), leaving only the test's own deterministic stdout:

```sh
cd "$(git rev-parse --show-toplevel)"
cargo test -p ring_handle --test handle_test debug_output_length_does_not_grow_with_item_count 2>/dev/null
```

Live output:

```

running 1 test
test debug_output_length_does_not_grow_with_item_count ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 17 filtered out; finished in 0.00s
```

**Disposition:** applied — added
`debug_output_length_does_not_grow_with_item_count` to `tests/handle_test.rs`,
formatting a one-item `Consumer` against an eight-item `Consumer` and
asserting the two `Debug` strings are the same length, exactly the guard this
finding proposes. It cannot see *which* crate keeps the constraint the way
`every_handle_can_be_printed` already does not, but it now fails if a future
`Debug` impl anywhere in the derive chain starts printing slot contents. The
crate's 19 unit/integration tests in this suite (22 across all of
`ring_handle`'s test binaries) plus 2 doctests re-verified passing (`cargo
test -p ring_handle --all-features`, 2026-09-04). Now prints:
`test debug_output_length_does_not_grow_with_item_count ... ok`
