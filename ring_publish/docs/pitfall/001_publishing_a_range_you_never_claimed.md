# Pitfall: Publishing a Range You Never Claimed

### Scope

- **Purpose**: Record the crate's one unenforced precondition — publish only what you claimed — its four break modes, and why the type that would enforce it exists and is not used.
- **Responsibility**: State the precondition, enumerate what breaking it produces, show the type that could carry it and the two reasons it does not, and name what would catch each mode.
- **In Scope**: Caller-side misuse of `publish` and `try_publish`.
- **Out of Scope**: Merging the claimed and published cursors — see [`pitfall/002`](002_conflating_the_two_cursors.md).

### The Precondition

`publish( start, len )` is correct only when `start..start + len` is a range this
producer obtained from `Claimer::claim` and has finished writing. Nothing checks
it. `src/lib.rs:183-189` states it and states its own inability to enforce it:

> Never. The loop exits when the predecessor publishes, which it is committed to
> doing; a caller that publishes a range it never claimed deadlocks here instead,
> which is a caller bug this crate cannot detect — `try_publish` is the variant
> for a caller that wants to decide for itself.

Two things stand out. The failure is a **deadlock**, not a panic — a hang with no
message, no stack, no exit code. And the sentence appears under `# Panics`, which
is the only place in the crate's documentation it could have been put, because
Rust has no `# Deadlocks` convention
([`item/002`](../item/002_the_two_publications.md) § PB23).

### The Four Break Modes

| # | What the caller does | `try_publish` | `publish` | Detected by |
|--:|----------------------|---------------|-----------|-------------|
| 1 | Publishes a range never claimed, ahead of the frontier | `Err( frontier )` forever | **hangs forever** | nothing |
| 2 | Publishes the same claim twice | first `Ok`, second `Err` forever | first returns, **second hangs** | nothing |
| 3 | Publishes a `len` different from the claim's | `Ok` — the frontier lands wrong | same | nothing |
| 4 | Two threads publish copies of one `Claim` | one `Ok`, one `Err` forever | one returns, **one hangs** | nothing |

Mode 3 is the quiet one and the worst. It does not hang: `try_publish( start, len
)` succeeds for any `len` as long as `start` is the frontier, so publishing 3 when
4 were claimed leaves one sequence permanently claimed-but-unpublished — and the
*next* producer, whose `start` is 4, spins forever against a frontier stuck at 3.
The failure surfaces one publication later, in a thread that did nothing wrong.

Publishing a larger `len` than claimed is worse still and does not hang at all:
the frontier passes sequences no producer owns, and the consumer reads slots
nobody wrote. That is the same corruption as
[`pitfall/002`](002_conflating_the_two_cursors.md) produces, reached from the
caller's side rather than the crate's.

Mode 1's deadlock has a property worth naming: the spinning thread holds its core
and never yields, so a break in one producer degrades the whole machine rather
than just stalling one worker
([`non_functional_requirement/002`](../non_functional_requirement/002_what_the_spin_costs.md)).

### PB35 — The Type That Would Enforce This Exists, Says So In Its Own Warning, and Is Not On the Signature

`ring_claim::Claim` is exactly the two values `publish` takes:

```rust
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
pub struct Claim
{
  start : Seq,
  len : usize,
}
```

Of its eight public methods, three are the ones a publication would use:
`start()`, `len()`, and an `end()` whose body is `self.start.advanced_by(
self.len as u64 )` — byte for byte the arithmetic `try_publish` performs at
`src/lib.rs:163`. A signature of `publish( &self, claim : Claim ) -> Seq` would
need no arithmetic at all.

It would make **mode 3 unrepresentable**: `start` and `len` would arrive together
from the same value, so they could not disagree. Mode 1 it would only make
*deliberate* — and here the seam turns back on itself, because `Claim::new` is
public, so `publisher.publish( Claim::new( Seq( 99 ), 4 ) )` compiles and hangs
exactly as today's call does. The constructor is public for one stated reason:

> Public because `ring_publish` and the test suites of both crates need to
> construct one directly; a producer obtains real claims from [`Claimer::claim`],
> which is the only path that establishes exclusivity.

So the hole that would remain is the hole that was opened *for this crate* — and
`ring_publish` constructs no `Claim` at all
([`data_structure/002`](../data_structure/002_the_four_cursors_of_the_handshake.md)
§ PB16). The seam was designed for, documented, and never built, and the
concession made to build it is load-bearing for nothing.

The `must_use` message names *this crate's* failure mode from the other side:
*"stalls every consumer"*. So `ring_claim` already knows what happens when the
handshake is broken, and warns about it — for the one break mode a lint can see.

Two reasons the type is not on the signature, and only one of them holds:

| Reason | Verdict |
|--------|---------|
| It would create a dependency cycle | **false**, and this crate says so itself — `Cargo.toml:12-14` justifies the dev-dependency with *"None of them depends on this one, so the graph stays acyclic"* |
| The primitive would stop being independently testable | **holds** — `ring_claim` is a **dev**-dependency (`Cargo.toml:16`), and `tests/publish_test.rs`'s thirteen tests never name it at all; taking a `Claim` would promote it to a real one |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^ring_' ring_claim/Cargo.toml ring_publish/Cargo.toml
grep -c ring_claim ring_publish/tests/publish_test.rs   # 0
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
ring_claim/Cargo.toml:ring_types = { path = "../ring_types" }
ring_claim/Cargo.toml:ring_cursor = { path = "../ring_cursor" }
ring_claim/Cargo.toml:ring_gating = { path = "../ring_gating" }
ring_publish/Cargo.toml:ring_types = { path = "../ring_types" }
ring_publish/Cargo.toml:ring_cursor = { path = "../ring_cursor" }
ring_publish/Cargo.toml:ring_claim = { path = "../ring_claim" }
ring_publish/Cargo.toml:ring_consume = { path = "../ring_consume" }
ring_publish/Cargo.toml:ring_barrier = { path = "../ring_barrier" }
ring_publish/Cargo.toml:ring_gating = { path = "../ring_gating" }
0
```

The first row's argument is already written down for the reverse direction: the
dev-dependency block exists because the *reached-test* needs all four operations,
and the comment above it establishes acyclicity as a checked property rather than
a hope ([`workaround/002`](../workaround/002_four_dev_dependencies_that_look_like_a_cycle.md)).
Whatever keeps `Claim` off the signature, it is not the graph.

The second reason is the same argument
[`decisions/001`](../decisions/001_refused_rather_than_reordered.md) makes about
the publication mechanism — *keep the primitive exercisable on its own* — applied
to the type signature instead of the algorithm. It is consistent, and it is a
real trade rather than an oversight: the cost is that the crate's one precondition
lives in a doc comment.

### PB36 — And Even Taking It Would Not Close Mode 2 or Mode 4

`Claim` is `Copy`. So `publish( claim : Claim )` takes it by value without
consuming anything a caller cannot reproduce:

```rust
let claim = claimer.claim( 4 ).unwrap();
publisher.publish( claim );   // fine
publisher.publish( claim );   // compiles — `Copy` — and hangs forever
```

Closing modes 2 and 4 needs the opposite of `Copy`: a non-`Copy`, non-`Clone`
token consumed by publication, so the compiler can prove each claim is published
at most once. That is a linear-type discipline, and adopting it would change
`Claim`'s derive list — a Tier 5 change rippling into every caller of
`Claimer::claim`, including `ring_mpsc`, which uses the crate today.

`Copy` is not an accident either. Every one of `Claim`'s seven accessors takes
`self` **by value**, so this crate's own tests depend on the derive to compile at
all:

```rust
publisher.publish( claim.start(), claim.len() );
```

Two `self`-consuming calls on one value in one expression — `handshake_test.rs`
at `:108`, `:400`, `:424` and `:575`. Without `Copy` the second is a use after
move. Making `Claim` linear therefore means changing all seven signatures to take
`&self` as well as dropping the derive, which is why the change is Tier 5 in
practice and not just in principle.

### The Family Built That Token Twice, and Named the Shape It Rejected

The linear token modes 2 and 4 need is not hypothetical. Two crates have one:

| Crate | Type | Publication is | Type at | `Drop` at |
|-------|------|----------------|---------|-----------|
| `ring_mpsc` | `Reserved< 'a, S >` | a `Release` store to the slot's stamp | `857-874` | `922-929` |
| `ring_spsc` | `Reservation< 'a, S >` | a `Release` store to the producer cursor | `725-737` | `788-796` |

Both are non-`Copy`, non-`Clone`, hold `&'a Ring< S >` and one `Seq`, and publish
in `drop`. Both also carry the *mirror* hazard this crate does not have — a guard
dropped unwritten publishes an empty slot — and each documents it differently:
`ring_spsc:731` as a `must_use` message, `ring_mpsc:927-931` as three sentences
of prose arguing the outcome is defined rather than torn.

`ring_spsc:617-621` states the argument against `ring_publish`'s shape without
naming it:

> The guard shape rather than a bare `claim`/`publish` pair, because an early
> return between the two wedges the ring permanently and no runtime check can
> distinguish "claimed and about to publish" from "claimed and abandoned".
> Making the publish the drop makes the case unreachable, including on unwind.

*A bare `claim`/`publish` pair* is exactly `ring_claim` plus `ring_publish`. So
the shape this pitfall describes was considered, its failure mode was written
down, and the two crates with real callers chose the other one.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE '^impl.*Drop for' ring_*/src/*.rs   # 4 hits, all ring_mpsc / ring_spsc
grep -c Drop ring_publish/src/lib.rs          # 0
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
ring_mpsc/src/lib.rs:impl< S > Drop for Reserved< '_, S >
ring_mpsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Reservation< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
0
```

Four `Drop` impls exist family-wide and **none is in a Tier 5 primitive**. The
guard shape needs a ring to publish *into* — `Reserved` holds `&'a Ring< S >`,
`Reservation` holds `&'a Ring< S >` — and `Publisher` has no ring, only a
cursor. So the crate that cannot adopt the fix is the one whose independence from
a ring is its entire reason for existing
([`decisions/001`](../decisions/001_refused_rather_than_reordered.md)).

That is the honest summary, and it is a three-level result: the enforceable part
of the precondition (mode 3) has a type that is not used; the unenforceable part
(modes 1, 2 and 4) needs a guard this crate structurally cannot hold; and the
family built that guard twice, in the two crates that route around this one.

### What Would Catch Each Mode Today

| Mode | Caught by | Where |
|-----:|-----------|-------|
| 1 | a test that hangs — no assertion, no message | any suite with a timeout |
| 2 | the same | — |
| 3 | a downstream `assert_eq!( published(), expected )` | eight sites in `tests/publish_test.rs`, and only for the crate's own tests |
| 4 | the same as 1 | — |

Nothing catches any of them at the point of the mistake. The eight assertions
that would notice mode 3 (`publish_test.rs:31,69,81,98,111,154,180,225`) do so
because they pin the frontier's exact value after a run — a discipline this
crate's tests follow and a caller has no reason to.

`tests/publish_test.rs:72-85` is the closest thing to a test *of* the pitfall: it
publishes from a start past the frontier and asserts the refusal and that nothing
moved. It uses `try_publish` precisely so the test can finish — the same call
through `publish` would hang the suite, which is why no such test exists.

### The Rule, Stated Once

> A producer publishes exactly the range `Claimer::claim` returned to it, once,
> after writing every slot in it.

Four constraints — *exactly*, *the range returned*, *once*, *after writing* — and
the crate's signature expresses none of them. The last is the one the loom model
checks ([`lifecycle/001`](../lifecycle/001_a_slot_from_claim_to_visibility.md)
§ PB26); the first three are the caller's to keep.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_a_loop_with_no_budget.md](../algorithm/002_a_loop_with_no_budget.md) | The termination argument whose step 3 is this precondition |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_six_methods_and_no_caller.md](../api/001_six_methods_and_no_caller.md) | The signature that takes two scalars rather than a token |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_four_cursors_of_the_handshake.md](../data_structure/002_the_four_cursors_of_the_handshake.md) | `ring_claim`'s two documented seams, neither of them built |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_two_publications.md](../item/002_the_two_publications.md) | Where the precondition is written down, and why under `# Panics` |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_slot_from_claim_to_visibility.md](../lifecycle/001_a_slot_from_claim_to_visibility.md) | The 1 → 2 boundary this pitfall skips |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_the_spin_costs.md](../non_functional_requirement/002_what_the_spin_costs.md) | What a deadlocked producer costs the machine |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_try_and_loop_over_compare_exchange.md](../pattern/001_try_and_loop_over_compare_exchange.md) | Silent unboundedness, the pattern's second normal failure |

### Pitfalls

| File | Relationship |
|------|--------------|
| [002_conflating_the_two_cursors.md](002_conflating_the_two_cursors.md) | The same corruption, reached from inside the crate |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/002_four_dev_dependencies_that_look_like_a_cycle.md](../workaround/002_four_dev_dependencies_that_look_like_a_cycle.md) | Why `ring_claim` is reachable from the tests and not from the library |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:161-165,183-210` | The arithmetic a `Claim` would supply, and the precondition in prose |
| `ring_claim/src/lib.rs:94-160` | `Claim`, its `must_use` message, and the three accessors a publication would use |
| `ring_claim/src/lib.rs:104-108` | The constructor justified by a consumer that never arrived |
| `ring_claim/src/lib.rs:115-117` | Why the constructor carries no `must_use` of its own |
| `ring_claim/Cargo.toml:9-11` | Three dependencies, none of them `ring_publish` |
| `ring_publish/Cargo.toml:12-19` | The dev-dependency block, and the acyclicity it states |
| `ring_spsc/src/lib.rs:617-621` | The argument against a bare `claim`/`publish` pair, made by a crate that has one available |
| `ring_spsc/src/lib.rs:725-737, 788-796` | `Reservation` — non-`Copy`, publishes on drop, `must_use` on the mirror hazard |
| `ring_mpsc/src/lib.rs:920-937, 985-992` | `Reserved` — the same shape over a stamp instead of a cursor |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:72-85` | A start past the frontier, refused — through `try_publish`, so the test can end |
| `tests/publish_test.rs:87-99` | A start behind the frontier, refused twice, frontier unmoved |
| `tests/handshake_test.rs:399-400` | The discipline a caller is expected to follow, in two lines |
| `tests/handshake_test.rs:108,400,424,575` | Two `self`-consuming accessors per expression — the `Copy` derive load-bearing |
