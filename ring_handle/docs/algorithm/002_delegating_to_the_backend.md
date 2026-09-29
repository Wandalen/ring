# Algorithm: Delegating an Operation to the Backend

### Scope

- **Purpose**: Specify what a handle method does between being called and returning — which is almost nothing — and state why "almost nothing" is a requirement rather than an observation.
- **Responsibility**: The delegation steps, the transformations permitted and forbidden, and the cost this crate is allowed to add.
- **In Scope**: The per-operation path through a handle.
- **Out of Scope**: The ring operation itself, which is `ring_core`'s and its backends'; the split that created the handle (→ [Splitting a Ring Into Two Ends](001_splitting_a_ring_into_two_ends.md)).

### Abstract

**A handle method forwards to the backend and returns what it gets.** It adds
no synchronization, no buffering, no retry, and no state of its own.

That sounds like a description and it is a constraint. This crate sits directly
on the hot path — every publish in the system passes through a `Producer`
method — and it is the one crate on that path whose purpose is *not*
performance. The temptation to put something useful here is therefore
continuous, and every useful thing costs the whole family.

**The measurable form of the constraint:** a handle method must compile to the
same work as calling the backend directly, plus at most one pointer
indirection.

### Algorithm

For a publish through `Producer` (the drain through `Consumer` is the same shape):

| Step | Operation | Permitted cost |
|------|-----------|----------------|
| 1 | Reach the backend through the handle's own field | One pointer indirection |
| 2 | Call the corresponding backend operation, passing arguments through unchanged | The backend's own cost |
| 3 | Return the backend's `Result`/`Option` to the caller | Zero, or a newtype wrap |
| 4 | — | — |

**There is no step 4.** The gap is the specification: no counter increment, no
`if self.closed`, no lock, no allocation, no logging call.

**Step 3's newtype wrap is the only transformation permitted, and only in one
direction.** Widening a backend error into a handle-level error type is
acceptable — the handle is the exported surface and the backend is not, so an
error type that names internal crates would leak the family's internals across
the export boundary. Narrowing is not: collapsing two distinct backend refusals
into one `None` destroys information the caller needs to choose between
retrying and dropping.

**What must not be added here, with the reason each is tempting:**

| Addition | Why it is tempting | What it costs |
|----------|--------------------|---------------|
| A statistics counter | This is the natural chokepoint — every operation passes through | An RMW on the hot path, violating [`ring_spsc`'s no-RMW invariant](../../../ring_spsc/docs/invariant/002_no_lock_in_the_path.md) from a crate that crate does not depend on. Stats belong in [`ring_stats`](../../../ring_stats/readme.md) |
| A retry loop on a full ring | `try_publish` returning `Err` is inconvenient for callers | **Parking by another name** — [Nothing Reachable From a Handle Can Park](../invariant/002_no_parking_operation_is_reachable.md)'s W3, which returns a `Result` and spends the frame budget anyway |
| A closed check | Publishing to a closed ring should fail | Duplicated state. `close()` is [`ring_shutdown`](../../../ring_shutdown/readme.md)'s, and a second copy of the flag here can disagree with it |
| A local batch buffer | Amortizing the indirection across several items | The handle acquires state, which makes drop lossy and makes the pair's behaviour depend on when it is dropped rather than only on what was called |
| A `Deref` to the backend | Removes the forwarding boilerplate entirely | Exposes the full backend surface through both handles — [Capability Follows the Handle](../invariant/001_capability_follows_the_handle.md)'s V4, and the split becomes advisory |

**The last row deserves emphasis because it is the labour-saving one.** Writing
forwarding methods by hand is tedious and a `Deref` impl removes all of it in
three lines. It also removes the crate's entire reason to exist: with `Deref`,
`Producer` reaches every method the backend has, including drain.

**The batch row is subtler than it looks.** A handle that buffers is a handle
whose `Drop` must flush, which means a dropped-during-unwind handle silently
loses records, and a moved handle changes when its contents become visible.
Batching is a real requirement — it is [`ring_batch`](../../../ring_batch/readme.md)'s,
where the buffer's lifetime is the explicit subject rather
than a side effect of handle ownership.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The methods this procedure implements, and their error shapes |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | Same, draining |

### Algorithms

| File | Relationship |
|------|--------------|
| [001_splitting_a_ring_into_two_ends.md](001_splitting_a_ring_into_two_ends.md) | Creates the handle whose field step 1 reads |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) | The field step 1 indirects through, and why the structure carries no other state |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_one_dependency_and_the_backends_beneath.md](../integration/001_one_dependency_and_the_backends_beneath.md) | The `ring_core` operation step 2 calls, and the three backends it may resolve to |
| [../integration/002_on_the_export_surface.md](../integration/002_on_the_export_surface.md) | Why step 3's error type may not name an internal crate |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_no_parking_operation_is_reachable.md](../invariant/002_no_parking_operation_is_reachable.md) | The retry row's violation, stated as an invariant |
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | The `Deref` row's violation |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_convenience_method_undoes_the_crate.md](../pitfall/001_a_convenience_method_undoes_the_crate.md) | The five forbidden additions as edits that actually get proposed |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_poll/readme.md`](../../../ring_poll/readme.md) | Why step 3 returns a `Result` rather than resolving the refusal here |
| [`ring_stats/readme.md`](../../../ring_stats/readme.md) | Where the statistics row's counter belongs instead |
| [`ring_batch/readme.md`](../../../ring_batch/readme.md) | Where the batching row's buffer belongs instead |

### Tests

| File | Relationship |
|------|--------------|
| **not written** | A publish through `Producer` performs the same atomic operations as the backend call it forwards to. The counting shim this needs has nowhere to attach — `ring_core::Backend` is private and chosen inside `Ring::new`. `the_wrapper_costs_nothing` measures the weaker property (no added state) and is what currently stands in for this row; the gap is that a same-sized wrapper can still add a fence |
| [`tests/ui/producer_try_clones.rs`](../../tests/ui/producer_try_clones.rs) | A compile-fail case: backend methods are not reachable through a handle by `Deref`. It is named for the method it calls rather than the mechanism it detects, because `try_clone` is a real `ring_core` capability — the case fails today by name resolution and would start compiling the moment a `Deref` impl appeared. It does **not** cover "or any other coercion": an explicit `pub fn inner()` accessor leaves it rejected exactly as it is now |

### HD3 — Step 6 Is an Absence and Nothing Mechanically Checks It

"Never expose the shared owner from either handle" is listed as load-bearing:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- accessors that would violate step 6 --'
printf '  fn inner/as_ref/get_ref:  %s\n' \
  "$( command grep -cE '^  pub (const )?fn (inner|as_ref|get_ref|ring)\(' ring_handle/src/lib.rs )"
printf '  impl Deref:               %s\n' \
  "$( command grep -cE '^impl.*Deref.* for ' ring_handle/src/lib.rs )"
printf '  pub fields:               %s\n' \
  "$( command grep -cE '^  pub [a-z_]+ :' ring_handle/src/lib.rs )"
echo '  -- and the compile-fail case that would notice --'
ls ring_handle/tests/ui/*.rs | sed 's|.*/||' | command grep -iE 'deref|inner|accessor' || echo '  (none)'
```

Live output:

```
  -- accessors that would violate step 6 --
  fn inner/as_ref/get_ref:  0
  impl Deref:               0
  pub fields:               0
  -- and the compile-fail case that would notice --
  (none)
```

All three counts are zero, and no ui case targets the accessor shape.

**Step 6 holds today by nobody having written the accessor, not by anything
objecting if they did.** The suite's seven cases all fail by name resolution or
by the borrow checker on code a *caller* writes; none of them fails on code a
*maintainer* writes. `producer_try_clones` comes closest and
[`algorithm/002`](002_delegating_to_the_backend.md)'s own Tests row already
concedes the limit: "an explicit `pub fn inner()` accessor leaves it rejected
exactly as it is now."

That concession is correct and it is filed under the wrong procedure. It is
step 6 of *this* one that has no detector, and this instance lists step 6 as
load-bearing without saying so.

### HD4 — Neither Procedure Can Fail, and the Family Has a Config That Would Make One

[`001`](001_splitting_a_ring_into_two_ends.md) closes with "It does not fail.
There is no `Result` here — a constructed ring can always be split." This
procedure inherits that: a forwarded method returns whatever the backend
returns, so no delegation ever reports a producer-count mismatch either. The
claim is true of both signatures; the mismatch it waves at is real:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what the config can ask for --'
command grep -E 'producers|with_producers' ring_config/src/lib.rs \
  | command grep -vE '^\s*(///|//!)' | head -6
echo '  -- and what the split returns, regardless --'
command grep -E '^  pub fn split\(' ring_handle/src/lib.rs
echo '  -- the rejection site this instance defers to --'
command grep -A6 '^pub enum BuildError' ring_factory/src/lib.rs \
  | command grep -E '^\s*([A-Z][A-Za-z]+,|pub enum)'
```

Live output:

```
  -- what the config can ask for --
  producers : usize,
        producers : 1,
  pub const fn with_producers( mut self, producers : usize ) -> Self
    self.producers = if producers == 0 { 1 } else { producers };
  pub const fn producers( &self ) -> usize
    self.producers
  -- and what the split returns, regardless --
  pub fn split( &'a mut self ) -> ( Producer< 'a, T >, Consumer< 'a, T > )
  -- the rejection site this instance defers to --
pub enum BuildError
```

A ring configured for several producers splits into exactly one `Producer`, with
no signal at any point in this crate.

**The instance names the right owner for that rejection and the ownership never
landed.** It defers to `ring_factory`. `ring_factory::build`
does return a `Result< Split< S >, BuildError >`, so the seam exists — and
`BuildError` has exactly one variant, `NameTaken`, which is about the registry
and not about the config. A config asking for four producers produces a ring,
not an error, because a multi-producer ring is a legitimate thing to build. It
only becomes wrong when it is split by *this* crate.

So the mismatch has a documented owner, a live seam with a `Result` already on
it, and no variant on either side of it. Recorded here rather than in
`ring_factory` because this is the procedure that silently narrows four
producers to one.
