# Type: The Lifetime on the Claimer

### Scope

- **Purpose**: Record what `Claimer< 'a >`'s single lifetime parameter guarantees, what it does not, and the two accessors that treat it in opposite ways without saying so.
- **Responsibility**: Show that one accessor's reference escapes the `Claimer` and the other's cannot, connect that to the purpose `cursor()` is documented for, and name the auto-trait the whole design rests on.
- **In Scope**: `'a` on `Claimer`, the lifetimes of its accessors' return types, and `Send`/`Sync`.
- **Out of Scope**: Why the field is a borrow rather than an owned value — see [`data_structure/002`](../data_structure/002_the_borrow_that_is_half_the_type.md), which also places `Claimer` among the family's 23 lifetime-carrying types. The integers are [`type/001`](001_a_seq_a_usize_and_three_casts.md).

### What `'a` Says

```rust
pub struct Claimer< 'a >          // :250
{
  cursor : PaddedCursor,          // owned
  consumers : &'a GatingSet,      // borrowed        :253
}
```

One parameter, one obligation: **the gating set outlives the claimer.** That is
the whole of it, and it is enough — a `Claimer` whose gate had been dropped
would read freed memory on every `headroom` call, and the borrow checker makes
that unrepresentable rather than merely unlikely.

What it does *not* say is longer than what it does:

| `'a` does not guarantee | Which means |
|-------------------------|-------------|
| that any claim is ever published | the strand in [`pitfall/001`](../pitfall/001_dropping_a_claim.md) is fully type-correct |
| that the gating set has consumers | `GatingSet::new( cap, 0 )` is legal and 4 of 5 concurrency tests use it ([`invariant/001`](../invariant/001_no_two_producers_hold_one_sequence.md)) |
| that this claimer is the only one over that gate | two `Claimer`s over one `GatingSet` compile, and each has its own cursor |
| anything about ordering | [`lifecycle/002`](../lifecycle/002_the_claimer_over_a_rings_life.md) |

The third row is worth a second look, because it looks like a type error and is
not. `Claimer::new` takes `&'a GatingSet` — a *shared* reference — so nothing
stops a second `Claimer` being built over the same gate. Each holds its own
`PaddedCursor`, both gate against the same consumers, and neither sees the
other's grants. Reproduced against the shipping crate, in six safe lines with no
threads:

```rust
let a = Claimer::new( &set );
let b = Claimer::new( &set );   // second claimer over the same gate
let ca = a.claim( 4 ).unwrap();
let cb = b.claim( 4 ).unwrap();
```
```
claimer a granted Seq(0)..Seq(4)
claimer b granted Seq(0)..Seq(4)
overlaps: true
```

That is the crate's central invariant — *no two producers hold one sequence*
([`invariant/001`](../invariant/001_no_two_producers_hold_one_sequence.md)) —
broken without `unsafe`, without concurrency, and without a single diagnostic.
The invariant is guaranteed *per `Claimer`*, and nothing in the type, the
documentation, or the test suite says that qualifier out loud.

Correct usage is one `Claimer` per ring. The type system does not express it —
expressing it would need `new` to consume the `GatingSet` or take `&mut`, which
would then forbid the gate being shared with the consumer side that must also
read it. `ring_mpsc` resolves it structurally instead, constructing exactly one
`Claimer` inside `Ends` and never exposing the constructor
([`integration/001`](../integration/001_two_dependents_that_split_one_feature.md)),
which works precisely because it owns both halves. A caller assembling the
primitives directly has no such protection and no warning that it is needed.

### CL49 — One Accessor's Reference Outlives the Claimer, the Other's Cannot, and the Docs Are Symmetric

The two accessors sit fifteen lines apart and differ in exactly one respect:

```rust
pub const fn cursor( &self ) -> &PaddedCursor      // :289 — elided to `&self`
pub const fn consumers( &self ) -> &'a GatingSet   // :305 — explicitly `'a`
```

`consumers()` returns the borrow the `Claimer` was built from, so it outlives
the `Claimer` entirely. `cursor()` returns a reference into the `Claimer`'s own
field, so it dies with it. Verified against the shipping crate:

```rust
// compiles, prints "capacity 4"
let escaped : &GatingSet = { let c = Claimer::new( &set ); c.consumers() };

// error[E0597]: `c` does not live long enough
let escaped : &PaddedCursor = { let c = Claimer::new( &set ); c.cursor() };
```

Both are correct. Neither documentation mentions it — the two doc comments are
one sentence each, in the same shape, with no hint that the returned references
have different reach.

The asymmetry matters because of what `cursor()` is documented *for* (`:274-275`):

> The producer cursor, for `ring_publish` to read and for a gating set on the
> other side of the ring to be built against.

Both stated purposes require the reference to travel — to be handed to another
crate, or stored in a structure built alongside the ring. Both are exactly what
the elided lifetime forbids: a `&PaddedCursor` from `cursor()` cannot outlive
the `Claimer`, so it cannot be stored anywhere the `Claimer` is not.

[`item/002`](../item/002_the_seven_of_the_claimer.md) § CL30 records that both
stated purposes are false for other reasons — `ring_publish` owns its own
cursor, and the `ring_mpsc` dependency runs the other way. This is the third
reason, and it is the one visible in the signature alone: **the accessor
documented for cross-crate use is the only one of the two whose lifetime cannot
leave the local scope.** Had the doc been accurate, the signature would have had
to be `-> &'a PaddedCursor`, and it cannot be — the cursor is owned, not
borrowed.

So the honest reading of `cursor()` is the one its single real caller already
uses: a short-lived borrow for an immediate question, which is precisely
`ring_mpsc:862` calling `.addr()` for a cache-line assertion and discarding it.

### CL50 — The Property the Whole Design Rests On Is Auto-Derived and Never Named

`Claimer`'s reason to exist is that several producer threads share one. That
requires `Claimer : Sync`, which requires `PaddedCursor : Sync` and
`GatingSet : Sync`. Verified:

```rust
fn assert_sync< T : Sync >() {}
assert_sync::< Claimer< '_ > >();   // compiles
assert_sync::< GatingSet >();       // compiles
assert_sync::< ring_claim::Claim >();
```

All three hold, all three by auto-derivation, and the crate never says so:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c 'Send\|Sync\|thread' ring_claim/src/lib.rs   # 0
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
0
```

**Zero occurrences** of `Send`, `Sync`, or `thread` in the source — in the crate
whose module doc opens by explaining how several producers contend for one
cursor, and whose founding decision was taken to keep `&self` methods usable
from many threads at once
([`decisions/001`](../decisions/001_compare_exchange_rather_than_fetch_add.md)).

The family does reason about auto-traits elsewhere: `ring_shutdown`,
`ring_factory`, `ring_flush`, and `ring_handle` all write explicit `T : Send`
bounds on their generic parameters. This crate has no generic parameter to bound,
so there is nothing to *write* — but there is also nothing to *state*, and the
result is that the property is invisible.

It is not, however, unenforced. Five tests in `claim_test.rs` share a
`&Claimer` across a `std::thread::scope`, which requires `Claimer : Sync` to
compile at all:

| Enforcement | Strength |
|-------------|----------|
| 5 concurrency tests sharing `&claimer` across threads | **compile-time** — losing `Sync` breaks the build, not a run |
| a documented claim | none |
| an explicit `assert_sync` or trait bound | none |

So the guard is real and it is accidental. A future field that removed `Sync` —
a `Cell` for a retry counter, a `RefCell` for diagnostics — would fail to
compile, which is the right outcome, but it would fail in `tests/claim_test.rs`
with an error about `thread::scope`, not at the field that caused it. The
diagnostic points at the five tests rather than the one line, and nothing in the
crate explains why those tests are load-bearing for a property they do not
mention.

`Claim : Send + Sync` matters for the same reason and is even less visible: the
concurrency tests collect `Vec< Vec< Claim > >` across a scope boundary and
join, which requires `Claim : Send`. It holds because `Claim` is two integers
([`pattern/002`](../pattern/002_the_half_open_range_as_a_value.md)), and that is
the deepest reason the value shape was the right call — not ergonomics, but that
a bare value is unconditionally `Send` where a guard holding `&'a Ring< S >` is
`Send` only if `S` is.

### Types

| File | Relationship |
|------|--------------|
| [001_a_seq_a_usize_and_three_casts.md](001_a_seq_a_usize_and_three_casts.md) | The other half of the type — the integers, not the borrow |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_borrow_that_is_half_the_type.md](../data_structure/002_the_borrow_that_is_half_the_type.md) | Why the field is a borrow, and the 23 lifetime-carrying types |
| [../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md) | The owned cursor `cursor()` hands a short borrow into |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_seven_of_the_claimer.md](../item/002_the_seven_of_the_claimer.md) | The other two reasons `cursor()`'s documented purpose does not hold |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_compare_exchange_rather_than_fetch_add.md](../decisions/001_compare_exchange_rather_than_fetch_add.md) | The decision that requires `&self` methods and therefore `Sync` |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_claimer_over_a_rings_life.md](../lifecycle/002_the_claimer_over_a_rings_life.md) | What the lifetime does not constrain about ordering |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependents_that_split_one_feature.md](../integration/001_two_dependents_that_split_one_feature.md) | Where the one-claimer-per-ring rule is enforced structurally instead |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:256-261` | The struct — one owned field, one borrowed |
| `ring_claim/src/lib.rs:281-282,315` | `cursor()`'s documented purpose, against its elided lifetime |
| `ring_claim/src/lib.rs:331` | `consumers()`, the one accessor returning `'a` |
| `ring_mpsc/src/lib.rs:862` | The single real caller, taking a short borrow and discarding it |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:320,343,377,427,463` | The five `thread::scope` tests that enforce `Sync` by compiling |
| `tests/claim_test.rs:288` — `the_claimer_exposes_the_gate_it_was_built_over` | `consumers()`, though not that its result outlives the claimer |
| `tests/claim_test.rs:309` — `the_producer_cursor_occupies_its_own_cache_line` | `cursor()` used the way its real caller uses it |
| — | **No test asserts `Send` or `Sync` directly**, and none names the property the five above depend on |
