# Lifecycle: A Slot From Claim to Visibility

### Scope

- **Purpose**: Trace one sequence through every state it occupies between being unclaimed and being readable, and record which transitions are observable and which are not.
- **Responsibility**: Enumerate the states, name the cursor that moves at each boundary, identify the one boundary that moves nothing, and show what the test suite had to build to observe it.
- **In Scope**: The life of a single sequence, from the producer's side, up to the moment `available()` will return it.
- **Out of Scope**: The four-operation protocol as a whole — see [`lifecycle/002`](002_the_four_operation_handshake.md).

### The Five States

| # | State | Predicate | Who may touch the slot |
|--:|-------|-----------|------------------------|
| 1 | Unclaimed | `seq >= claimed` | nobody |
| 2 | Claimed, unwritten | `published <= seq < claimed` | its producer, exclusively |
| 3 | Claimed, written | `published <= seq < claimed` | its producer, exclusively |
| 4 | Published | `consumer_position <= seq < published` | consumers, read-only |
| 5 | Committed | `seq < consumer_position` | its producer again, after a lap |

States 2 and 3 have the **same predicate**. That is not a presentation choice —
no cursor in the family distinguishes them, which is the whole content of PB25
below.

### The Four Transitions

```
      1 ─── claim ──▶ 2 ─── write ──▶ 3 ─── publish ──▶ 4 ─── commit ──▶ 5
          claimed++       (nothing)       published++      position++
         ring_claim                      ring_publish     ring_consume
```

| Boundary | Operation | Cursor moved | Crate | Ordering |
|----------|-----------|--------------|-------|----------|
| 1 → 2 | `Claimer::claim` | claimed | `ring_claim` | `compare_exchange` |
| 2 → 3 | the producer's own store into the slot | **none** | the caller's | the caller's |
| 3 → 4 | `Publisher::publish` | published | `ring_publish` | `Release` / `Acquire` |
| 4 → 5 | `Consumer::commit` | consumer position | `ring_consume` | `Release` |

### PB25 — Four Transitions, Three Cursor Moves, and the One That Matters Moves Nothing

The 2 → 3 boundary is the entire reason this crate exists, and it is the only one
of the four that changes no shared state at all. `src/lib.rs:18-27` names the
window without naming its invisibility:

> `ring_claim` advances a cursor when a producer *takes* a range; this crate
> advances a different one when the producer has *finished writing* it. Between
> the two, the slot is claimed and unwritten — and the feature's central
> requirement is that a consumer never sees it.

Checked against the two crates that could have recorded it:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*pub (fn|struct|enum|const fn)' ring_slot/src/lib.rs
grep -E '^\s*pub (fn|struct|enum|const fn)' ring_store/src/lib.rs
grep -rlE 'stamps|slot_state|per-slot' ring_*/src/*.rs
```

Live output:

```
pub struct TypedSlot< T >( Option< T > );
  pub const fn empty() -> Self
  pub fn set( &mut self, value : T ) -> Option< T >
  pub const fn get( &self ) -> Option< &T >
  pub fn take( &mut self ) -> Option< T >
pub struct BytesSlot< const N : usize >
  pub const fn empty() -> Self
  pub const fn capacity( &self ) -> usize
  pub const fn len( &self ) -> usize
  pub const fn is_empty( &self ) -> bool
  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >
  pub fn read( &self ) -> &[ u8 ]
pub struct Buffer< S >
  pub fn new( capacity : Capacity ) -> Self
  pub fn clear( &mut self )
  pub fn all_empty( &self ) -> bool
  pub const fn capacity( &self ) -> Capacity
  pub const fn len( &self ) -> usize
  pub const fn is_empty( &self ) -> bool
  pub fn get( &self, index : SlotIndex ) -> &S
  pub fn get_mut( &mut self, index : SlotIndex ) -> &mut S
  pub fn at( &self, seq : Seq ) -> &S
  pub fn at_mut( &mut self, seq : Seq ) -> &mut S
  pub fn iter( &self ) -> core::slice::Iter< '_, S >
  pub fn iter_mut( &mut self ) -> core::slice::IterMut< '_, S >
ring_store/src/lib.rs
ring_mpsc/src/lib.rs
ring_publish/src/lib.rs
ring_spsc/src/lib.rs
```

| Crate | Per-slot state | Concurrently readable |
|-------|----------------|:---------------------:|
| `ring_store` | none — `get`/`get_mut`/`at`/`at_mut`, no flag of any kind | — |
| `ring_slot` | `TypedSlot` is `Option< T >`, `BytesSlot` has a `len` | **no** — every mutator takes `&mut self` |
| `ring_mpsc` | stamps, one `AtomicSeq` per slot | yes |
| `ring_spsc` | stamps | yes |

So within the tiered stack this crate sits in, states 2 and 3 are genuinely
indistinguishable: `ring_store` stores no flag, and `ring_slot`'s `Option` is
`&mut`-gated, so no concurrent reader can consult it. The two crates that *can*
tell written from unwritten per slot are the two that built stamps — and both did
so instead of using this crate
([`decisions/001`](../decisions/001_refused_rather_than_reordered.md)).

That is the design, not an oversight. The published cursor is a **single**
boundary between "definitely written" and "possibly not", so the 2 → 3 transition
does not need to be observable — it only needs to *happen before* the 3 → 4
transition, which is what `PUBLISH`'s `Release` and `GATING`'s `Acquire`
guarantee. `src/lib.rs:60-67`:

> `Release` on the store so every slot write a producer performed before it is
> visible to a consumer that acquires the cursor. On x86 and aarch64 this is free
> in the store itself; the annotation is what makes it correct everywhere else.

The cost of the design is that nothing can *check* the ordering by looking at
cursors, because the thing being ordered leaves no cursor trace.

### PB26 — So the Loom Model Builds Its Own Observation Instrument

`tests/handshake_test.rs:73-75` and `:89-92` state the problem and the fix:

```rust
/// What the producer writes into the slot. Any value the slot cannot hold by
/// accident; zero would be indistinguishable from "never written".
const WRITTEN : usize = 0xABC;
```

```rust
// The slot itself, so that "did the consumer read something the producer
// had not written" is an observable question rather than a claim about
// sequence numbers.
let slot = Arc::new( AtomicUsize::new( 0 ) );
```

The model imports no slot type. It cannot use `ring_store` (no flag) or
`ring_slot` (not concurrently readable), so it constructs a one-slot ring out of a
bare `AtomicUsize` — the smallest thing that can hold "written" and "not written"
and be read from another thread under loom's instrumented atomics.

`0xABC` is chosen against a specific failure: with `WRITTEN = 1` a bug that
published before writing would be caught only if the slot's initial value were
not 1, and with `WRITTEN = 0` it would never be caught at all, because the
uninitialised state *is* zero. The constant is a sentinel, and the comment says
so before the value is used.

The producer's window is then two adjacent statements (`:105-108`):

```rust
slot.store( WRITTEN, Ordering::Release );
publisher.publish( claim.start(), claim.len() );
```

and the drain's assertion (`:137-141`) is the 2 → 3 → 4 ordering, made into a
value comparison:

```rust
assert_eq!(
  slot.load( Ordering::Acquire ),
  WRITTEN,
  "available offered a slot the producer had claimed but not written"
);
```

Reaching that line at all means `available()` offered the sequence, so state 4
was entered; the assertion then checks state 3 was entered first. On x86 the
store and the publish are ordered by the hardware whatever the code asks for, so
a threaded test cannot fail here — which is exactly why the model exists
([`workaround/001`](../workaround/001_the_loom_seam_and_its_only_user.md)).

`tests/manual/readme.md § P1` confirms the instrument works by breaking it: two
mutations, one reordering the store after the publish, both failing at this
assertion with `left: 0` — the slot's initial value, read by a consumer that was
offered it too early.

### The Windows, Sized

| Window | Duration | Bounded by |
|--------|----------|-----------|
| 1 → 2 | one `compare_exchange`, retried under contention | `ring_claim`'s own loop |
| 2 → 3 | **the payload write** — caller code, arbitrary | nothing |
| 3 → 4 | one `compare_exchange`, plus a spin if a predecessor is in its own 2 → 3 | the *predecessor's* payload write |
| 4 → 5 | consumer's processing of the whole available run | nothing |

The third row is the coupling this crate's design accepts: a producer's publish
latency is bounded below by a *peer's* 2 → 3 window, not its own.
[`decisions/001`](../decisions/001_refused_rather_than_reordered.md)'s "The Cost
Nobody Pays Yet" records what that costs at large payload sizes, and
[`non_functional_requirement/002`](../non_functional_requirement/002_what_the_spin_costs.md)
records that nothing has measured it above a one-word payload.

### Where a Sequence Can Be Observed From Outside

| Question | Answerable by | State it distinguishes |
|----------|---------------|------------------------|
| Has it been claimed? | nothing public — the claimed cursor is read by nobody | — |
| Is it readable? | `Publisher::is_published( seq )` | 4 and 5 from 1, 2, 3 |
| How far has publication reached? | `Publisher::published()` | the 3 / 4 boundary exactly |
| Has the consumer passed it? | `GatingSet::cursor( i )` | 5 from 4 |

Three of the five states are externally indistinguishable from each other:
**unclaimed, claimed-unwritten, and claimed-written all answer `false` to
`is_published`**, which is precisely the guarantee the feature is graded on. A
consumer cannot tell a slot nobody has touched from a slot being written right
now — and does not need to, because both mean *not yours*.

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_frontier_moves_only_by_compare_exchange.md](../invariant/001_the_frontier_moves_only_by_compare_exchange.md) | Why the 3 → 4 boundary is the only way into state 4 |
| [../invariant/002_is_published_is_exclusive_of_the_frontier.md](../invariant/002_is_published_is_exclusive_of_the_frontier.md) | The comparison that puts a sequence on one side or the other |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_two_publications.md](../item/002_the_two_publications.md) | The two ways to cross 3 → 4 |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_the_four_operation_handshake.md](002_the_four_operation_handshake.md) | The same story told by operation rather than by state |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_the_named_ordering_constant.md](../pattern/002_the_named_ordering_constant.md) | The `Release`/`Acquire` pair that orders 2 → 3 before 3 → 4 |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_conflating_the_two_cursors.md](../pitfall/002_conflating_the_two_cursors.md) | What collapsing states 2 and 4 into one produces |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_loom_seam_and_its_only_user.md](../workaround/001_the_loom_seam_and_its_only_user.md) | The harness in which the instrument above runs |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:18-27,60-67` | The window, and the ordering pair that makes it safe without being observable |
| `ring_store/src/lib.rs:49-279` | Slot storage with no per-slot state |
| `ring_slot/src/lib.rs:62-396` | `Option`-shaped slots, every mutator `&mut` |
| `ring_claim/src/lib.rs:336-341` | The 1 → 2 boundary, and the same requirement from its side |
| `ring_consume/src/lib.rs:12-19` | The 4 → 5 boundary, and why it is a separate call |

### Tests

| File | Relationship |
|------|--------------|
| `tests/handshake_test.rs:73-75,89-92` | The sentinel and the instrument, with their reasons |
| `tests/handshake_test.rs:105-108` | The 2 → 3 → 4 sequence, two statements wide |
| `tests/handshake_test.rs:137-141` | The assertion that state 3 preceded state 4 |
| `tests/publish_test.rs:203-212` | States 1, 2 and 3 all answering `false` |
| `tests/manual/readme.md § P1` | Both mutations failing at the instrument with `left: 0` |
