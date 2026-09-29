# Invariant: Capability Follows the Handle

### Scope

- **Purpose**: State the property this crate exists to establish — that what a caller may do to a ring is determined by which value it holds, and by nothing else.
- **Responsibility**: The statement, where it is enforced, and what each way of breaking it actually costs.
- **In Scope**: The producer/consumer capability split; why it is total rather than advisory.
- **Out of Scope**: Which operations may park (→ [Nothing Reachable From a Handle Can Park](002_no_parking_operation_is_reachable.md)); the ring mechanics behind each capability, which are `ring_core`'s.

### Invariant Statement

> A `Producer` can publish and **cannot** drain. A `Consumer` can drain and
> **cannot** publish. Neither capability is available through any other route,
> and neither is checked at runtime — the operation the caller is not entitled
> to does not exist on the value it holds.

This crate's own design goal is what this discharges: it "makes 'who is
allowed to consume' a question answered by ownership rather than by
convention."

**Read that sentence twice, because it names two things and only one of them
is obvious.** The obvious half is that capability is *partitioned*. The
non-obvious half is that it is answered by **ownership** — so the answer moves
when the value moves, is checkable by the compiler, and cannot be gotten wrong
at a call site that never mentions handles at all.

### Enforcement Mechanism

**By absence, at compile time, with no runtime component whatsoever.** There is
no capability flag, no `debug_assert`, no `Result::Err( NotAConsumer )`. The
enforcement is that the method is not on the type.

| Mechanism | Enforces | Detected when |
|-----------|----------|---------------|
| `Producer` has no `drain`/`try_recv` method | Half the statement | Compile time, at the offending call site |
| `Consumer` has no `publish`/`try_push` method | The other half | Compile time, at the offending call site |
| Neither exposes the shared backend it holds | Both — a caller cannot route around the split | **Not a violation-detection claim** — true of today's source and checked by the compiler as it stands; a *new* accessor added later is caught by nothing (→ V4, HD49) |
| **Four** `trybuild` compile-fail cases | That the above stays true as the crate changes, plus that neither handle can be duplicated | Test run, in this crate — `tests/ui_test.rs` |

**The fourth row is doing work the first three cannot.** Rows one to three are
properties of today's source; nothing about them resists a well-meant edit. The
`trybuild` cases are the only mechanism that *fails* when someone adds a `drain`
to `Producer` — an ordinary test suite gets greener when a method is added,
never redder.

**That redder-not-greener property was measured, not assumed.** A `try_recv`
returning `None` was added to `Producer`, the suite was run and reported
`producer_drains.rs` failing, and the method was removed. A compile-fail suite
that has never been shown to go red is indistinguishable from one whose cases
fail to compile for an unrelated reason — the pinned `.stderr` catches the
second, and only an injected violation catches the first. Recorded at
`tests/manual/readme.md` H2.
[the acceptance table](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md)'s
row for this crate requires exactly this: "asserted by a `trybuild` compile-fail
case for each."

**Enforcement necessarily lives here rather than in the ring crates**, because
this is the only crate that can withhold anything. `ring_spsc` can state its
cardinality requirement and cannot enforce it; `ring_core` presents whatever
surface its backend has. The split is expressible exactly once in the family,
at the point where the ring stops being one value and becomes two
(→ [Splitting a Ring Into Two Ends](../algorithm/001_splitting_a_ring_into_two_ends.md)).

**What this crate does *not* enforce, and must not be read as enforcing:**

- **That drains happen only at a barrier.** The invariant makes the barrier
  *able* to hold the sole draining capability; it does not place the handle
  there. A `Consumer` moved into a mid-tick system
  satisfies this invariant completely and destroys determinism
  (→ [The Barrier Holds the Consumer](../lifecycle/002_the_barrier_holds_the_consumer.md)).
- **That exactly one producer exists.** That is a separate consequence of the
  same mechanism, and it depends on `Producer` also not being `Clone`
  (→ Violation Consequences, row 3).

### Violation Consequences

| # | Violation | Immediate effect | Real consequence |
|---|-----------|------------------|------------------|
| V1 | `Producer` gains a drain method | Compiles; `trybuild` case fails | **Caught.** The one violation this crate detects by itself |
| V2 | `Consumer` gains a publish method | Compiles; `trybuild` case fails | **Caught**, same mechanism |
| V3 | `Producer` derives `Clone` | Compiles; `tests/ui/producer_clones.rs` fails | **Caught** — by a case this crate added beyond the acceptance table's two. Uncaught it would be a silent data race: two producers against a ring built for one, and `ring_spsc`'s whole correctness argument void (→ [`ring_spsc` invariant/001](../../../ring_spsc/docs/invariant/001_exactly_one_producer_one_consumer.md)) |
| V4 | A handle exposes its backend (`pub fn inner( &self )`) | Compiles; nothing fails | **The split becomes advisory.** Both capabilities are reachable from either handle via one extra call; no `trybuild` case written against the *methods* catches it |
| V5 | The pair is handed out behind one `&mut Ring` instead | Compiles | The rejected alternative, verbatim: "the type that exists to avoid a lock ends up behind one" |

**V4 is now the one worth dwelling on. V3 was, and is not any more.** Both
compile and both pass every functional test, and neither is caught by the two
cases *the acceptance table specifies* — those assert that drain is absent from
`Producer` and publish is absent from `Consumer`, and a derived `Clone` is
orthogonal to both. This crate wrote a third and fourth case rather than
recording the gap and moving on, so V3 and its consumer-side twin are caught
here while remaining outside this crate's stated criterion.

V4 is not closed, and the reason is that it is genuinely harder: a case can name
one accessor that must not exist, but the violation is *any* public route to the
backend, and there is no way to write "no method returns `&Backend`" as a
compile-fail case. Whether to widen the
compile-fail cases to cover them is an open question this instance raises and
does not settle; what it settles is that the current specification does not
cover them.

**V3's severity is not this crate's to bear alone and is this crate's to
prevent alone.** The race corrupts data in `ring_spsc`; the missing `Clone` is
here. A future author who adds `#[derive(Clone)]` for convenience — handles
are small, copying one is cheap, several call sites would read better — has no
local signal that anything is wrong, which is precisely why the reason is
recorded in this table rather than only in the crate that suffers.

**V4's shape recurs whenever a type enforces by withholding**, which is why it
is generalized in
[Enforce by Withholding, Not by Checking](../pattern/001_enforce_by_withholding.md)
rather than treated as a local hazard.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The half of the split that publishes, and the methods deliberately absent from it |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | The half that drains, same |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_splitting_a_ring_into_two_ends.md](../algorithm/001_splitting_a_ring_into_two_ends.md) | The single point at which this invariant is established |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) | What each handle holds, and why V4 is a structural risk rather than a stylistic one |

### Invariants

| File | Relationship |
|------|--------------|
| [002_no_parking_operation_is_reachable.md](002_no_parking_operation_is_reachable.md) | The second restriction on the same surface, enforced by the same mechanism against a different failure |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_barrier_holds_the_consumer.md](../lifecycle/002_the_barrier_holds_the_consumer.md) | What this invariant makes possible and does not itself achieve |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_proven_by_code_that_must_not_compile.md](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md) | V1 and V2's detector, stated as this crate's acceptance criterion |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_enforce_by_withholding.md](../pattern/001_enforce_by_withholding.md) | The general practice, and V4 as its characteristic failure |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_convenience_method_undoes_the_crate.md](../pitfall/001_a_convenience_method_undoes_the_crate.md) | V3 and V4 as the edits that actually get made, and why they look reasonable |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_producer.md](../type/001_producer.md) | The value V3 must not become `Clone` |
| [../type/002_consumer.md](../type/002_consumer.md) | The value the barrier holds |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | This crate's row — the `trybuild` requirement, and the scope V3/V4 fall outside |
| [../lifecycle/002_the_barrier_holds_the_consumer.md](../lifecycle/002_the_barrier_holds_the_consumer.md) | What the split enables: "the barrier can hold the only thing capable of draining" |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/handle_test.rs`](../../tests/handle_test.rs) | This crate's claiming test — cites this crate's declared feature identifier so gate `g3_features.sh` records the claim |
| [`tests/ui/producer_drains.rs`](../../tests/ui/producer_drains.rs) | A `trybuild` compile-fail case: calling a drain method on `Producer` does not compile — V1 |
| [`tests/ui/consumer_publishes.rs`](../../tests/ui/consumer_publishes.rs) | A `trybuild` compile-fail case: calling a publish method on `Consumer` does not compile — V2 |

### HD49 — The Enforcement Table Claims Compile-Time Detection for the Exact Property the Violation Table Says Nothing Detects

Two tables in this file answer the same question about the same property and
give opposite answers:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -nE '^\| (Neither exposes|\*\*Four\*\*)' \
  ring_handle/docs/invariant/001_capability_follows_the_handle.md \
  | sed -E 's/^(.{0,140}).*/\1/' | sed 's|^|    |' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
command grep -nE '^\| V4 \|' \
  ring_handle/docs/invariant/001_capability_follows_the_handle.md \
  | sed -E 's/^(.{0,140}).*/\1/' | sed 's|^|    |' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
    | Neither exposes the shared backend it holds | Both — a caller cannot route around the split | **Not a violation-detection claim** — tru
    | **Four** `trybuild` compile-fail cases | That the above stays true as the crate changes, plus that neither handle can be duplicated | T
    | Neither exposes the shared backend it holds | Both — a caller cannot route around the split | **Not a violation-detection claim** — tr
    | V4 | A handle exposes its backend (`pub fn inner( &self )`) | Compiles; nothing fails | **The split becomes advisory.** Both capabiliti
```

The Enforcement table's third row puts "Neither exposes the shared backend it
holds" under **Detected when: Compile time**. V4 — a handle exposing its
backend — reads **Compiles; nothing fails**, and the prose beneath calls it the
one violation still open.

**The column means two different things in adjacent rows.** Rows one and two
use "Detected when" for *when a violation is caught*: adding a `drain` to
`Producer` breaks a call site, and a `trybuild` case turns red. Row three uses
it for *when the property is currently true*: no caller can reach the backend
today, and the compiler is what stops them. Under that second reading the cell
is defensible. Under the header it shares with its siblings it is a coverage
claim, and V4 is the same file saying there is no coverage.

A reader scanning the Enforcement table to find the unguarded edge finds four
rows that all say Compile time or Test run and concludes there is none. The
gap is sixty lines further down, in a table headed Violation Consequences,
which is where one goes to learn what happens *after* a failure rather than
whether one would be noticed.

This is the third table in the crate whose detection column is unreliable —
[`type/002`](../type/002_consumer.md)'s HD43 measures two more, by a different
mechanism: those rows went stale when a test was added, this one was never
about detection at all.

```sh
# -mN and no -n: this file is its own subject, so an unbounded
# match also finds this command line and every copy of its own
# output below, and -n re-prefixes a fresh line number onto each
# earlier pass's output -- the stacked addresses this block carried
cd "$(git rev-parse --show-toplevel)"
command grep -m1 '^| Neither exposes the shared backend it holds' ring_handle/docs/invariant/001_capability_follows_the_handle.md
```

Live output:

```
| Neither exposes the shared backend it holds | Both — a caller cannot route around the split | **Not a violation-detection claim** — true of today's source and checked by the compiler as it stands; a *new* accessor added later is caught by nothing (→ V4, HD49) |
```

**Disposition:** applied — the Enforcement table's third row no longer
shares the plain "Compile time" phrasing its violation-catching siblings
use; it now states explicitly that the row describes a currently-true
property rather than a caught violation, and points at V4 as the case
where the same property, if broken, is caught by nothing.
Now prints: `Not a violation-detection claim`

### HD50 — The Family Answers This Question Twice, Once by Absence and Once at Runtime, and Only the Absence Is Written Down Here

The Invariant Statement says neither capability "is checked at runtime — the
operation the caller is not entitled to does not exist on the value it holds."
That is true of this crate and false one crate down:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the same question, answered at runtime by the backend --'
command grep -A8 'pub fn try_clone' ring_core/src/lib.rs \
  | sed -E 's/^(.{0,120}).*/\1/' | sed 's|^|    |'
echo '  -- and this crate withholds it, with a case to keep it withheld --'
ls ring_handle/tests/ui/*.rs | sed 's|.*/|    |'
command grep -vE '^\s*(//|$)' ring_handle/tests/ui/producer_try_clones.rs | sed 's|^|    |'
```

Live output:

```
  -- the same question, answered at runtime by the backend --
      pub fn try_clone( &self ) -> Option< Producer< 'a, T > >
      {
        match &self.inner
        {
          ProducerInner::Spsc( _ ) => None,
          ProducerInner::Mpsc( producer ) =>
          {
            Some( Producer { inner : ProducerInner::Mpsc( *producer ), overflow : self.overflow } )
          }
  -- and this crate withholds it, with a case to keep it withheld --
    consumer_clones.rs
    consumer_publishes.rs
    producer_clones.rs
    producer_drains.rs
    producer_shared_across_threads.rs
    producer_try_clones.rs
    ring_used_after_split.rs
    use ring_config::RingConfig;
    use ring_core::Ring;
    use ring_handle::Split;
    fn main()
    {
      let ring : Ring< u32 > = Ring::new( &RingConfig::new( 8 ).unwrap() ).unwrap();
      let mut split = Split::new( ring );
      let mut ends = split.ends();
      let ( producer, _consumer ) = ends.split();
      let _second = producer.try_clone();
    }
```

`ring_core::Producer::try_clone` returns `Option< Producer >`: `None` on an
SPSC ring, `Some` on MPSC. That is "who may be a second producer" answered at
runtime, by the backend, with a refusal that is representable — and the
`ring_core` doc comment beside it states the principle explicitly: *cardinality
is the backend's to decide*.

**This crate decides it instead, for every backend.** `try_clone` is not on the
forwarded surface, and `producer_try_clones.rs` is a compile-fail case that
keeps it off. So on an MPSC ring reached through a `Split`, the second producer
`ring_core` would grant cannot be requested — the invariant is strictly
stronger than the property `ring_spsc` needs, applied uniformly because the
handle surface is uniform
(→ [`decisions/001`](../decisions/001_what_this_crate_is_for.md)'s HD14 on the
withheld list, which counts this one).

That is a defensible choice and it is not recorded as one. The Violation
Consequences table treats duplication as a single failure caught by a single
case; what actually happens is that this crate overrides a capability the layer
beneath grants conditionally. An MPSC user who needs a second producer has no
route through `ring_handle` and no sentence here telling them why, or where to
go instead — the answer is `ring_core` directly, which
[`integration/002`](../integration/002_on_the_export_surface.md)'s HD24 records
is not on the Contract.
