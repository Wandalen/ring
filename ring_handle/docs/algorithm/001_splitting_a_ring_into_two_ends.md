# Algorithm: Splitting a Ring Into Two Ends

### Scope

- **Purpose**: Specify the one procedure that establishes everything else this crate guarantees — turning a single ring value into a producer/consumer pair — and identify the step where each guarantee is actually created.
- **Responsibility**: The steps, what each one costs, and which of them are load-bearing rather than bookkeeping.
- **In Scope**: The split procedure and its ownership consequences.
- **Out of Scope**: What the handles then do (→ [Delegating an Operation to the Backend](002_delegating_to_the_backend.md)); ring construction, which is `ring_factory`'s.

### Abstract

**The split takes a ring by value and returns two handles.** Taking it *by
value* is the entire mechanism: the original ring is consumed, so no third
reference to it can exist, and the only ways to reach the ring afterwards are
the two returned values — one of which can publish and one of which can drain.

Everything this crate guarantees is created here and merely preserved
afterwards. There is no ongoing enforcement, no per-call check, and no state to
maintain: after the split returns, the type system holds the invariants without
further help.

**The procedure runs once per ring, does no synchronization, and allocates at
most once.** Its cost is not interesting; its ownership effects are.

### Algorithm

Given a constructed ring `r`:

| Step | Operation | Cost | Load-bearing? |
|------|-----------|------|---------------|
| 1 | Accept `r` **by value**, not by reference | Move | **Yes** — this is what makes the original unreachable |
| 2 | Place the ring behind a shared owner both handles can hold | One allocation, or none if the backend is already shared | **Yes** — determines whether the pair is `Send` (→ [Send Without Sync](../non_functional_requirement/002_send_without_sync.md)) |
| 3 | Construct `Producer` holding a clone of that owner | Refcount increment, or a pointer copy | No — bookkeeping |
| 4 | Construct `Consumer` holding another | Same | No |
| 5 | Return `( Producer, Consumer )` as a tuple, both by value | Move | **Yes** — a caller that stores only one still moves the other |
| 6 | Never expose the shared owner from either handle | — | **Yes**, and it is an absence rather than a step |

**Step 1 is where the "no shared `&mut`" property is created**, and it is
created by a signature rather than by logic. A `fn split( &mut self )` would
leave the caller holding the ring *and* both handles — three routes to one
value, which is this crate's own If Missing arriving through the front door.

**Step 2 read as the one real design decision in the procedure. It is not one.**
The table below was written as an open trade; implementation found that two of
its three rows do not typecheck or do not pass a gate, leaving exactly one
shape. The reasoning is in
[Two Handles Over One Backend](../data_structure/001_two_handles_over_one_backend.md)'s
settled section; the table is kept because its cost columns are still the right
way to read what the surviving shape costs:

| Shape | Pair is `Send` | Cost per handle | Cost per operation |
|-------|---------------|-----------------|--------------------|
| `Arc<Ring>` | Yes, if `Ring: Send + Sync` | 8 bytes | An atomic refcount touch only at clone/drop, not per publish |
| Raw pointer + a lifetime on the ring | Yes, but the ring must outlive both handles by a borrow | 8 bytes | None |
| Each handle owns its half outright, sharing only the cursors | Yes | Larger | None |

**The `Arc` shape is the obvious one and it is not obviously right** — and, as
it turned out, not available either. `ring_core`'s publish and drain take
`&mut self`, which an `Arc` cannot hand out. The RMW argument that would have
decided against it never had to be made.

**That is the useful part of this record.** The instance spent its analysis on
whether an atomic refcount touch at split and drop violates
[`ring_spsc`'s no-RMW invariant](../../../ring_spsc/docs/invariant/002_no_lock_in_the_path.md)
(it does not — the invariant governs the publish path), while the actual
constraint was a receiver type one crate down. **Cost analysis of an option that
does not compile is analysis spent before the cheapest question was asked**,
and the cheapest question here was "can this shape call the methods at all".

**Step 6 is not a step and is listed as one deliberately.** The procedure's
last obligation is to *not* do something: no `fn inner( &self ) -> &Ring`, no
`Deref`, no public field. Steps 1 through 5 are undone by any of the three,
which is [Capability Follows the Handle](../invariant/001_capability_follows_the_handle.md)'s
V4.

**What the procedure does not do, and must not be extended to do:**

- **It does not validate the config.** A ring built for four producers, split
  into one `Producer`, is a mismatch this procedure cannot see — the handles
  look identical either way. Rejection belongs at `ring_factory`
  (→ [`ring_spsc` integration/001](../../../ring_spsc/docs/integration/001_family_dependency_seam.md)).
- **It does not decide where the handles go.** Moving the `Consumer` to a
  barrier is what buys determinism, and the procedure returns a value that could
  go anywhere (→ [The Barrier Holds the Consumer](../lifecycle/002_the_barrier_holds_the_consumer.md)).
- **It does not fail.** There is no `Result` here — a constructed ring can
  always be split. An error type would imply a runtime condition, and there is
  none.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | What step 3's value can do |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | What step 4's value can do |

### Algorithms

| File | Relationship |
|------|--------------|
| [002_delegating_to_the_backend.md](002_delegating_to_the_backend.md) | Every operation after this procedure returns, and why step 6's absence must hold for all of them |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) | Step 2's three candidate shapes, worked out as a structure rather than a step |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_one_dependency_and_the_backends_beneath.md](../integration/001_one_dependency_and_the_backends_beneath.md) | Where the ring in step 1 comes from, and who validated it |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | Established at steps 3, 4 and 6; nowhere else |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_split_move_and_drop.md](../lifecycle/001_split_move_and_drop.md) | This procedure as a lifecycle phase, with what precedes and follows it |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_send_without_sync.md](../non_functional_requirement/002_send_without_sync.md) | Step 2's determination of N1, N2 and N3 |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_handle_ownership.md](../lifecycle/003_handle_ownership.md) | The transition this procedure is |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_capability_follows_the_handle.md`](../invariant/001_capability_follows_the_handle.md) | "Without sharing a mutable reference" — step 1's requirement |
| [`ring_factory/readme.md`](../../../ring_factory/readme.md) | Where the ring is built and its config validated, which this procedure assumes done |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/ui/ring_used_after_split.rs`](../../tests/ui/ring_used_after_split.rs) | The ring is not usable after the split — a compile-fail case, since step 1 makes it a move. `E0382`, the only case in the suite rejected by the borrow checker rather than by name resolution |
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `the_two_ends_travel_to_separate_threads` | Splitting yields handles that reach the same ring: an item published through one is drained through the other, from a different thread |

### HD1 — The One Procedure Is Three Calls, and the Six Steps Do Not Line Up With Them

The abstract says the split "takes a ring by value and returns two handles". No
method does both:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the signatures a caller actually walks --'
command grep -E '^  pub (const )?fn (new|ends|split)\(' ring_handle/src/lib.rs
echo '  -- and how the crate doc writes the sequence --'
command grep 'Split::new( ring )\|\.ends()\|ends\.split()' ring_handle/src/lib.rs \
  | command grep '///' | head -3
```

Live output:

```
  -- the signatures a caller actually walks --
  pub const fn new( ring : Ring< T > ) -> Self
  pub fn ends( &mut self ) -> Ends< '_, T >
  pub fn split( &'a mut self ) -> ( Producer< 'a, T >, Consumer< 'a, T > )
  -- and how the crate doc writes the sequence --
/// let mut split = Split::new( ring );
/// let mut ends = split.ends();
/// let ( mut producer, mut consumer ) = ends.split();
```

`Split::new` takes the ring by value and returns one value. `ends` borrows.
`split` consumes that borrow and returns the pair. Three calls, three lines in
every doctest.

**The six-step table describes a procedure with one entry point, and the code
has three.** Step 1 (take by value) is `new`; step 5 (return the tuple) is
`split`; steps 2–4 happen somewhere across `ring_core`'s `ends()` and `split()`
and cannot be pointed at from here at all. The table is a correct account of
*what happens* and a misleading account of *where*, and a reader tracing a
guarantee to its enforcement site will look for a `split` that owns the ring and
not find one.

The shape is not incidental — it is forced, and
[`ring_factory/docs/decisions/001`](../../../ring_factory/docs/decisions/001_the_owner_is_the_return_value.md)
records why: collapsing the three calls needs a self-referential struct. The
gap is that this instance's own table predates that ruling and was never
re-anchored to it.

### HD2 — Step 2's Allocation Is in a Crate This One Cannot Name

The procedure budgets "one allocation, or none". This crate allocates nothing:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- sharing and allocation primitives in this crate --'
for p in 'Arc<' 'Rc<' 'Box<' 'Vec<' 'NonNull' 'unsafe'; do
  printf '  %-10s %s\n' "$p" "$( command grep -cF "$p" ring_handle/src/lib.rs )"
done
echo '  -- what Split actually holds --'
sed -n '/^pub struct Split/,/^}/p' ring_handle/src/lib.rs
```

Live output:

```
  -- sharing and allocation primitives in this crate --
  Arc<       0
  Rc<        0
  Box<       0
  Vec<       2
  NonNull    0
  unsafe     0
  -- what Split actually holds --
pub struct Split< T >
{
  ring : Ring< T >,
}
```

Zero of each. `Split< T >` holds a `Ring< T >` by value and nothing else; the
storage was allocated before this crate saw it.

**So the cost column that decides whether the pair is `Send` describes work
performed one crate down, in code this crate does not call and cannot inspect.**
The `Arc` / raw-pointer / owned-halves table below step 2 reads as a live trade
available at this seam. It is not — the shape was fixed by `ring_core::Ring`'s
construction long before `Split::new` is reachable.

The instance already says the trade collapsed, and says so well. What it does
not say is that the collapse also moved the *cost* out of this crate, which is
why `the_wrapper_costs_nothing` can assert byte-for-byte identity: there is
nothing here to cost anything.
