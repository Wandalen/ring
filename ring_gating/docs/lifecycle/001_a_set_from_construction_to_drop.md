# Lifecycle: A Set from Construction to Drop

### Scope

- **Purpose**: Walk a `GatingSet` from `new` to deallocation and show that it has exactly one state, so that what looks like a lifecycle is four events with nothing between them.
- **Responsibility**: Give the four points, the allocation ledger at each, the one decision made at construction, and the reason there is no destructor.
- **In Scope**: The set's own life.
- **Out of Scope**: The producer's walk against a stalled consumer — see [`002`](002_the_producer_walking_a_lap_against_a_stall.md).

### The Four Points

| Point | What happens | Who does it |
|-------|--------------|-------------|
| Construct | `GatingSet::new( capacity, consumers )` — one allocation, every cursor at zero | The ring |
| Hand out | `&GatingSet` to the gate reader, `&PaddedCursor` to each consumer | The ring |
| Read | Any of the eleven methods, any number of times, from any number of threads | Producers and consumers |
| Drop | The `Vec`'s deallocation, and nothing else | The ring |

**Only the first and last are transitions.** Between them the type has one state,
because there is no `&mut self` method to leave it by
([`api/001`](../api/001_eleven_methods_over_one_owned_vec.md) § G1). Membership,
capacity and every cursor's address are fixed the moment `new` returns.

That is not the same as *nothing changes*. The cursors' contents change
constantly — that is the whole point — but they change through interior
mutability, at addresses the set fixed at construction, without the set
observing it. The set is a fixed frame around moving parts.

### Construction

```rust
// ring_gating/src/lib.rs:93-98
pub fn new( capacity : Capacity, consumers : usize ) -> Self
{
  let mut cursors = Vec::with_capacity( consumers );
  cursors.resize_with( consumers, PaddedCursor::default );
  Self { cursors, capacity }
}
```

Three lines, one allocation, no fallibility — the signature returns `Self` rather
than a `Result` because there is nothing to reject. `consumers` of zero is
legal, `consumers` greater than `capacity` is legal
([`item/002`](../item/002_the_four_accessors_and_the_limit.md) § G11 records
why), and `capacity` was already validated when the `Capacity` was made.

**The allocation ledger:**

| `consumers` | Allocations | Bytes |
|------------:|------------:|------:|
| 0 | **0** | 0 |
| 1 | 1 | 64 |
| 4 | 1 | 256 |

The zero row is a real property, not an assumption — `Vec::with_capacity( 0 )`
does not allocate, so an ungated set is a `Capacity` and a dangling aligned
pointer:

```sh
# PaddedCursor's layout, standing in for it — the subject is Vec's own
# allocation behaviour, which GatingSet does nothing but forward.
cat > /tmp/-padded_probe.rs <<'EOF'
#[ repr( align( 64 ) ) ]
#[ allow( dead_code ) ]
struct Padded( u64 );

fn main()
{
  for n in [ 0usize, 1, 4 ]
  {
    let mut v : Vec< Padded > = Vec::with_capacity( n );
    v.resize_with( n, || Padded( 0 ) );
    let ptr = v.as_ptr() as usize;
    println!( "consumers {n}: ptr==align {} aligned {} allocated {}",
      ptr == 64, ptr % 64 == 0, v.capacity() > 0 );
  }
}
EOF
rustc -O --crate-name padded_probe -o /tmp/-padded_probe /tmp/-padded_probe.rs
/tmp/-padded_probe
rm -f /tmp/-padded_probe /tmp/-padded_probe.rs
```

Live output:

```
consumers 0: ptr==align true aligned true allocated false
consumers 1: ptr==align false aligned true allocated true
consumers 4: ptr==align false aligned true allocated true
```

`ptr==align true` on the zero row is the whole property: an unallocated `Vec`
reports its alignment — 64 here — as its pointer, and never reaches the
allocator. Addresses themselves are deliberately not printed; under ASLR they
would differ every run, and the three facts that do not are the ones the ledger
rests on. So the ungated ring, the one shape this crate argues hardest about
([`decisions/001`](../decisions/001_capacity_for_an_empty_set.md)), costs nothing
at all to represent.

### The Decision Made at Construction

Every cursor starts at `Seq::ZERO`, which is not a neutral value:

| Built with | `slowest()` | `headroom( ZERO )` | Meaning |
|------------|-------------|-------------------:|---------|
| `new( cap( 8 ), 0 )` | `None` | 8 | Ungated — nothing to wait for, ever |
| `new( cap( 8 ), 1 )` | `Some( ZERO )` | 8 | One consumer that has read nothing |

The two agree at construction and diverge one lap later — the second gates the
producer at 8, the first never gates it at all. So the `consumers` argument is
not a sizing hint; it is the choice between two different rings, made once, with
no way to revise it afterwards.

`tests/gating_test.rs:207-217` asserts exactly that divergence, and
`tests/gating_test.rs:329-339` asserts the starting positions it depends on.

### Handing Out

Two routes, both established at construction time and neither revisable:

| Route | Written | Lifetime |
|-------|---------|----------|
| Borrow | `Claimer::new( &shared.consumers )` | The reader is lexically bounded by the owner |
| `Arc` | `Arc::new( GatingSet::new( … ) )` | The set outlives every scope that reads it |

`ring_mpsc` takes the borrow, and pays for it in its own API shape:

> Two steps rather than one because `Claimer` borrows the `GatingSet` it gates on
>
> — `ring_mpsc/src/lib.rs:559`

The `Arc` route appears only in `ring_publish`'s tests. See
[`pattern/001`](../pattern/001_the_owned_set_with_shared_readers.md) for the
three roles this split creates.

### Reading — the Steady State

The whole middle of the lifecycle is reads, and no gating read allocates:

```rust
// ring_cursor::slowest in ring_cursor/src/lib.rs
cursors.iter().map( | c | c.load( GATING ) ).min()
```

So the ledger for a set's life is **one allocation to build it, and none per
gating read**. It used to be one per read forever — the fold `collect()`ed the
cursor positions into a `Vec< Seq >` before folding them, and the steady state
dwarfed the construction by any measure that counts. Commit `b7e075ca` inverted
that: construction is now the whole heap cost of a set's life, and the steady
state is free. That is
[`non_functional_requirement/001`](../non_functional_requirement/001_every_gating_read_allocates_nothing.md),
and it remains a lifecycle fact as much as a performance one — the shape of the
ledger changed, not just a number in it.

### Drop

`GatingSet` has no `Drop` impl. Neither does `PaddedCursor`, `CacheAligned`, or
`AtomicSeq` — the whole chain from the set down to the atomic is destructor-free,
so dropping a set is a `Vec` deallocation and nothing more:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'impl.*Drop for' ring_*/src/*.rs
# ring_mpsc:985   impl< S > Drop for Reserved< '_, S >
# ring_mpsc:1257  impl< S > Drop for Batch< '_, S >
# ring_spsc:788   impl< S > Drop for Reservation< '_, S >
# ring_spsc:1130  impl< S > Drop for Batch< '_, S >
```

Live output:

```
ring_mpsc/src/lib.rs:impl< S > Drop for Reserved< '_, S >
ring_mpsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Reservation< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
```

**Four `Drop` impls in all 33 crates, and all four are publish-on-drop guards:**

```rust
// ring_mpsc/src/lib.rs:985-992
impl< S > Drop for Reserved< '_, S >
{
  /// Publish, with the one `Release` store the whole protocol turns on.
  fn drop( &mut self )
  { self.ring.stamp( self.seq ).store( self.seq, PUBLISH ); }
}
```

That is a different kind of lifecycle entirely — a guard whose *destruction* is
the operation. The gating chain has none, which means:

| Consequence | Why |
|-------------|-----|
| No unwinding hazard | A panic mid-read runs no gating code on the way out |
| No ordering obligation at teardown | Nothing is published, signalled or flushed by a drop |
| Nothing to get wrong twice | There is no drop order between the set and its cursors — they are one allocation |

### What Prevents a Use-After-Free

Nothing in this crate does. The set holds no reference count, no generation, no
epoch, and no flag saying whether readers are still out. The borrow checker is
the entire mechanism — and it is sufficient precisely *because* the borrow route
is the production one and the `Arc` route stays in tests.

The cost is the one `ring_mpsc` records: `Ring::ends` cannot return a `Claimer`
and the ring together in one value, so it returns an `Ends` holding both. A
runtime mechanism would buy back that ergonomics at the price of a counter on
the hot path. The family chose the compile-time one, once, at the only site that
owns a set.

### GT31 — The Ungated Set Costs Nothing to Represent

```
GatingSet::new( capacity, 0 )
  -> Vec::with_capacity( 0 )   -> no allocation, dangling-aligned pointer
  -> size_of::< GatingSet >()  -> three words plus the capacity
```

The empty set is the shape this crate argues hardest about — it is the subject
of a decision, a pitfall and two tests. It is also free.

**Finding.** It allocates nothing at all — `Vec::with_capacity( 0 )` reports the alignment as its pointer, so the shape this crate argues hardest about is free to represent

---

### GT32 — No Destructor at Any Level of the Chain

```
impl Drop in all 33 crates : 4
  ring_mpsc/src/lib.rs:985   Reserved
  ring_mpsc/src/lib.rs:1257  Batch
  ring_spsc/src/lib.rs:788   Reservation
  ring_spsc/src/lib.rs:1130  Batch
```

All four are publish-on-drop borrow guards in the two rings. From `GatingSet`
down through `PaddedCursor` to `AtomicSeq` there is nothing that runs on the way
out.

**Finding.** Four `Drop` impls exist in all 33 crates and all four are publish-on-drop guards; the gating chain from `GatingSet` down to `AtomicSeq` has no destructor at any level, so a set's end of life is a deallocation with no consumer notified

---

### GT33 — Two Events and Nothing Between Them

```
construct : 93   pub fn new
read      : 108, 121, 142, 155, 168, 197, 222, 242, 283, 321
drop      : ( compiler-generated )
transition: none — no &mut self, no Drop
```

A lifecycle document usually traces states. This structure has one state, and
the motion that matters happens inside objects it owns and does not drive.

**Finding.** Construct, read, drop. With no `&mut self` method and no `Drop` impl there is no state transition between the first and the last, so the structure has no lifecycle in the usual sense — all the motion is inside cursors it owns and does not drive

---


### APIs

| File | Relationship |
|------|--------------|
| [../api/001_eleven_methods_over_one_owned_vec.md](../api/001_eleven_methods_over_one_owned_vec.md) | G1 — the absence of `&mut self` that makes the state singular |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_set_that_cannot_grow.md](../data_structure/001_the_set_that_cannot_grow.md) | Why the middle of the lifecycle has no transitions |
| [../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md](../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md) | The `Vec` that is allocated and freed |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_capacity_for_an_empty_set.md](../decisions/001_capacity_for_an_empty_set.md) | The two rings the `consumers` argument chooses between |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_four_accessors_and_the_limit.md](../item/002_the_four_accessors_and_the_limit.md) | G11 — the constructor argument nothing checks |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_the_producer_walking_a_lap_against_a_stall.md](002_the_producer_walking_a_lap_against_a_stall.md) | The lifecycle that does have a sequence — the caller's, not the set's |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_every_gating_read_allocates_nothing.md](../non_functional_requirement/001_every_gating_read_allocates_nothing.md) | The steady state's cost |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_owned_set_with_shared_readers.md](../pattern/001_the_owned_set_with_shared_readers.md) | The three roles the handing-out step creates |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:93-98` | `new` |
| `ring_mpsc/src/lib.rs:380, 534` | The one production construction, and the lifetime it costs |
| `slowest` in `ring_cursor/src/lib.rs` | Where the per-read allocation was, until `b7e075ca` |
| `ring_mpsc/src/lib.rs:985-992` | A `Drop` that is an operation, for contrast |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:315-327` | One cursor per consumer, across five counts including zero |
| `tests/gating_test.rs:329-339` | Every cursor at zero after construction |
| `tests/gating_test.rs:341-353` | The 64-byte stride the allocation buys |
| `tests/gating_test.rs:355-359` | The capacity, remembered |
| `tests/gating_test.rs:207-217` | The two rings the `consumers` argument chooses between, disagreeing |
