# Lifecycle: Split, Move and Drop

### Scope

- **Purpose**: Trace a handle pair from the split that creates it to the drop that ends it, and identify the phase at which each of the crate's guarantees becomes true or stops being true.
- **Responsibility**: The phases, their transitions, their ordering dependencies, and the cleanup obligations at each end.
- **In Scope**: The pair's arc within one program run; what a drop of one end means for the other.
- **Out of Scope**: Ring construction, which is `ring_factory`'s; where the consumer is placed (→ [The Barrier Holds the Consumer](002_the_barrier_holds_the_consumer.md)); `close`/`reset`/`drain_all`, which are `ring_shutdown`'s.

### Lifecycle Phases

| # | Phase | Begins when | The crate's guarantees here |
|---|-------|-------------|-----------------------------|
| L1 | **Ring exists, unsplit** | `ring_factory` returns a ring | **None.** The ring is one value with a full surface; capability is not yet partitioned |
| L2 | **Split** | The split consumes the ring | All of them, at once. The ring becomes unreachable except through two values with disjoint capabilities |
| L3 | **Paired and co-located** | The split returns | In force. Both handles are owned by one scope |
| L4 | **Separated** | One handle is moved to another thread | In force, and now doing work — this is the phase the crate exists for |
| L5 | **One end dropped** | Either handle's `Drop` runs | The surviving end still works; the ring is alive |
| L6 | **Both ends dropped** | The second `Drop` runs | **Not the ring's own drop** — the ring is owned by `Split`, which outlives both handles by construction; it drops, taking any undrained items with it, only when `Split` itself drops, a seventh value this table does not name (→ HD29) |

**L1 is the phase where the guarantees do not hold, and its duration should be
as close to zero as the API allows.** Anything that can happen to an unsplit
ring can happen through its full surface. A factory that returns a ring for the
caller to split later leaves a window; one that returns the pair directly does
not — which is what `ring_factory`'s
reached-test describes when it says `Factory::build( cfg )` "returns a handle
pair."

**L2 is instantaneous and total.** There is no partial split, no state where
one handle exists and the other does not, and no way back. This is what makes
the guarantees expressible as type properties rather than as lifecycle rules
(→ [Splitting a Ring Into Two Ends](../algorithm/001_splitting_a_ring_into_two_ends.md)).

**L5 is the phase with a real open question**, and it is not settled here: what
does it mean to publish into a ring whose consumer has been dropped?

| Behaviour on `Producer::try_push` after the `Consumer` drops | Consequence |
|---|---|
| Succeed until full, then refuse | The producer fills the ring, then spends the rest of the run getting `Err( Full )` with no way to know why |
| Refuse immediately, as `Closed` | Requires the drop to set a flag — a `Drop` with an observable side effect, which is fine but must be deliberate |
| Undefined / not specified | The current state, and the worst option: callers will discover it empirically and depend on whatever they find |

The second row is the defensible one and it costs a `Drop` impl on `Consumer`
that touches shared state. Whether the flag is the same one `close()` sets,
or a distinct one, is a question for `ring_shutdown` as much as this one.

### Phase Transitions

| # | Transition | Trigger | Reversible | Notes |
|---|-----------|---------|------------|-------|
| T1 | L1 → L2 | The split is called | **No** | The ring is moved; there is no unsplit |
| T2 | L2 → L3 | The split returns the tuple | — | Not a real transition; L2 and L3 differ only in whether the call has returned |
| T3 | L3 → L4 | A handle is moved across a thread boundary | No, and it need not be | A move is a move; nothing tracks where the handle went |
| T4 | L4 → L4 | The handle is moved again | — | Unbounded. Nothing limits how many times a handle relocates, which is why [C8](../type/002_consumer.md) is unenforceable |
| T5 | L3/L4 → L5 | One handle drops | No | Which one dropped is observable only if the drop records it |
| T6 | L5 → L6 | The second handle drops | No | Drops nothing by itself — the ring stays alive until `Split` drops; this crate holds no `Arc` (→ HD29) |

**T3 is the transition the whole crate is built to permit**, and it is
noteworthy that it requires no support at all: a `Send` owned value crosses a
thread boundary by being moved. The crate's work is in making that move *safe*,
not in making it possible.

**T4 is where the determinism guarantee leaks.** Nothing prevents a `Consumer`
being moved from the barrier into a system, and nothing records that it happened
(→ [The Barrier Holds the Consumer](002_the_barrier_holds_the_consumer.md)).

**T5's asymmetry is worth naming.** A dropped `Producer` means nothing more will
be published, which the consumer can act on — it may stop polling. A dropped
`Consumer` means nothing more will be drained, which the producer *cannot* act
on unless L5's open question is resolved. The two drops are not symmetric even
though the types are.

### Dependencies

| # | Depends on | For | Ordering constraint |
|---|-----------|-----|---------------------|
| D1 | [`ring_factory`](../../../ring_factory/readme.md) | The ring entering L1, with a validated config | Must precede L2. A `producer_count > 1` config must be rejected *there*, not detected here — this crate cannot see it |
| D2 | [`ring_core`](../../../ring_core/readme.md) | The backend both handles reference | Must outlive both handles — enforced as a lifetime obligation on `Split`, not an `Arc` refcount; this crate holds no `Arc` (→ HD29) |
| D3 | [`ring_shutdown`](../../../ring_shutdown/readme.md) | `close()`, which is a separate axis from drop | Independent of this lifecycle: a ring may be closed in L4 and remain in L4 |
| D4 | The thread that will hold each handle | T3's destination | None imposed. This is the gap C8 names |

**D3's independence is easy to get wrong.** Closed and dropped are two
different things: a closed ring still has both handles, still refuses publishes,
and still yields its outstanding items. Conflating them produces a `close()`
that consumes the handles, which is a different and more restrictive design than
`ring_shutdown` specifies (→ [Ring Liveness Through a Handle](../lifecycle/004_ring_liveness_through_a_handle.md)).

**D1 states an obligation this crate can only document.** By L2 the config is
already spent — the ring exists, and its producer count is baked into which
backend `ring_core` selected. A mismatch that reaches here is invisible.

### Cleanup Requirements

| # | Requirement | Rationale |
|---|-------------|-----------|
| R1 | **`Drop` on either handle must not flush, drain, or publish** | A `Drop` with data-moving side effects makes visibility depend on *when* a value goes out of scope, including during unwinding |
| R2 | **`Drop` must not block** | A dropped handle during a panic unwind must not park; a blocking drop turns a panic into a hang |
| R3 | **Dropping one handle leaves the other fully usable** | L5's whole premise. A drop that invalidates the sibling is a use-after-free the type system was supposed to prevent |
| R4 | **Undrained items at L6 are dropped, not leaked** | `T`'s own `Drop` must run for each item still in the ring. A ring of `Box`es dropped without draining leaks every one |
| R5 | **L5's publish semantics are specified, whichever way** | The open question above. Unspecified is the one unacceptable answer |
| R6 | **Nothing is registered that must be deregistered** | The handles hold no global state, so `Drop` has no bookkeeping to do beyond the backend's refcount — and this is a property to preserve, since it is what makes R2 achievable |

**R4 is the requirement most easily missed and it is not this crate's to
implement** — the backend owns the storage — but it is this crate's to *state*,
because L6 is the phase where it comes due, and L6 is a handle-lifecycle event.
`ring_shutdown`'s `drain_all()` is the explicit alternative for callers who need the
items rather than their destructors.

**R1 and R5 pull in opposite directions and both are right.** R1 forbids a
`Drop` that moves data; R5 asks for a `Drop` that sets a flag. A flag is not
data movement, and the distinction is the line: `Drop` may record that this end
is gone, and may not act on the ring's contents.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_splitting_a_ring_into_two_ends.md](../algorithm/001_splitting_a_ring_into_two_ends.md) | T1 in full — the step-by-step of the only irreversible transition |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The surface available in L3 and L4, and L5's unresolved refusal shape |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | Same, draining |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) | Why R2 and R6 are achievable — the handles carry no state to clean up |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_one_dependency_and_the_backends_beneath.md](../integration/001_one_dependency_and_the_backends_beneath.md) | D1, D2 and D3 as seams rather than as ordering constraints |

### Lifecycles

| File | Relationship |
|------|--------------|
| [002_the_barrier_holds_the_consumer.md](002_the_barrier_holds_the_consumer.md) | T4's leak, and what would have to exist above this family to close it |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_send_without_sync.md](../non_functional_requirement/002_send_without_sync.md) | T3, stated as an acceptance criterion |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_handle_ownership.md](../lifecycle/003_handle_ownership.md) | These phases as states, with the transitions the type system permits |
| [../lifecycle/004_ring_liveness_through_a_handle.md](../lifecycle/004_ring_liveness_through_a_handle.md) | D3's independent axis |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_factory/readme.md`](../../../ring_factory/readme.md) | "Returns a handle pair" — why L1 should be vanishingly short |
| [`ring_shutdown/readme.md`](../../../ring_shutdown/readme.md) | D3's separate axis, and R4's explicit alternative |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `dropping_one_handle_leaves_the_other_usable` | Dropping the `Consumer` leaves the `Producer` usable — R3, asserted rather than assumed |
| [`tests/handle_test.rs`](../../tests/handle_test.rs) · `undrained_records_are_dropped_exactly_once` | Dropping both handles runs `T`'s destructor once per undrained item — R4, asserted with a drop counter |
| [`tests/ui/ring_used_after_split.rs`](../../tests/ui/ring_used_after_split.rs) | The ring is unusable after the split — T1's irreversibility, as a compile-fail case. It lands in `tests/ui/`, not `handle_test.rs`, because the program has to be *rejected*: a passing test cannot contain code that does not compile |

### HD29 — L6 Says the Ring Drops With the Handles, and This Crate's Own Test Drops Something Else

The phase table ends at L6, "Both ends dropped — the ring is dropped. Any
undrained items go with it." The signatures say otherwise, and so does R4's test:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- who owns the ring, and who borrows it --'
command grep -E '^  pub (const )?fn (new|ends|split)\(' ring_handle/src/lib.rs
echo '  -- what R4 test drops, and where its assertion fires --'
sed -n '/^fn undrained_records_are_dropped_exactly_once/,/^}/p' \
  ring_handle/tests/handle_test.rs \
  | command grep -E 'let mut split|ends\.split|drop\(|assert_eq'
echo '  -- and whether the shape those rows name exists in the source --'
printf '  Arc/Rc in ring_handle src: %s\n' \
  "$( command grep -cE 'Arc<|Arc< |Rc<|Rc< ' ring_handle/src/lib.rs )"
```

Live output:

```
  -- who owns the ring, and who borrows it --
  pub const fn new( ring : Ring< T > ) -> Self
  pub fn ends( &mut self ) -> Ends< '_, T >
  pub fn split( &'a mut self ) -> ( Producer< 'a, T >, Consumer< 'a, T > )
  -- what R4 test drops, and where its assertion fires --
    fn drop( &mut self )
  let mut split = Split::new( Ring::< Counted >::new( &config ).unwrap() );
    let ( mut producer, mut consumer ) = ends.split();
    drop( consumer.try_recv().unwrap() );
    assert_eq!( DROPPED.load( Ordering::Relaxed ), 1, "the drained one, and only it" );
  drop( split );
  assert_eq!( DROPPED.load( Ordering::Relaxed ), 5, "the four still in the ring, once each" );
  -- and whether the shape those rows name exists in the source --
  Arc/Rc in ring_handle src: 0
```

`Split::new` takes the ring **by value**; `split` returns `Producer< 'a, T >`
and `Consumer< 'a, T >`, which *borrow* from it. The `Split` outlives both
handles by construction — it has to, or the borrow does not typecheck.

`undrained_records_are_dropped_exactly_once` shows the consequence directly: the
handles fall out of scope at the end of an inner block and the drop counter does
not move. It reaches 5 on the next line, `drop( split )`.

**So L6's trigger is the wrong event and R4 is filed against a phase that does
not do the thing R4 is about.** Dropping both ends drops nothing; the undrained
items go when the owner goes, and the owner is a seventh value the six-phase
table never names. T6's "the ring drops with it under the `Arc` shape" and D2's
"trivially satisfied under the `Arc` shape" are the tell — this crate contains
no `Arc`
(→ [`algorithm/001`](../algorithm/001_splitting_a_ring_into_two_ends.md)'s HD2),
and the lifecycle was traced against the shape that was considered rather than
the one that shipped.

The correction is not cosmetic. Under `Arc` the pair keeps the ring alive and
L5's open question — what a publish means after the consumer drops — is a
question about a live ring with one end gone. Under the real shape the consumer
cannot drop while the producer lives without the `Split` still being there, so
the interesting phase is `Split` outliving both, which no row covers.

**Disposition:** applied — L6's guarantee, T6's Notes, and D2's Notes no
longer describe an `Arc`-refcounted ring dropping with its last handle; all
three now name `Split` as the actual owner whose own drop is what takes the
ring (and any undrained items) with it, and state plainly that this crate
holds no `Arc`, matching this section's own Live output.
Now prints: `Arc/Rc in ring_handle src: 0`
