# Lifecycle: The Claimer Over a Ring's Life

### Scope

- **Purpose**: Trace the `Claimer` itself from construction to drop, and show that it has almost no lifecycle — one borrow, one monotonic counter, and no destructor.
- **Responsibility**: State what the type guarantees over its whole life, record that its one accessor makes that guarantee publicly breakable, and check the arithmetic the guarantee rests on.
- **In Scope**: `Claimer` construction, the cursor's monotonicity, the borrow's bound, and `Seq` exhaustion.
- **Out of Scope**: The life of a range — see [`lifecycle/001`](001_a_range_from_grant_to_publication.md).

### The Whole Life

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -E 'pub (const )?fn|impl'
```

Live output:

```
impl Claim
  pub const fn new( start : Seq, len : usize ) -> Self
  pub const fn start( self ) -> Seq
  pub const fn end( self ) -> Seq
  pub const fn len( self ) -> usize
  pub const fn is_empty( self ) -> bool
  pub const fn contains( self, seq : Seq ) -> bool
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  pub const fn overlaps( self, other : Self ) -> bool
impl< 'a > Claimer< 'a >
  pub fn new( consumers : &'a GatingSet ) -> Self
  pub const fn cursor( &self ) -> &PaddedCursor
  pub const fn consumers( &self ) -> &'a GatingSet
  pub fn claimed( &self ) -> Seq
  pub fn headroom( &self ) -> usize
  pub fn claim( &self, count : usize ) -> Result< Claim, RingError >
  pub fn claim_up_to( &self, max : usize ) -> Result< Claim, RingError >
```

| Event | Mechanism | Reversible |
|-------|-----------|:----------:|
| Construction | `Claimer::new( &consumers )` → `PaddedCursor::default()`, so `Seq::ZERO` | — |
| Every grant | `compare_exchange`, strictly forward | **no** |
| Every refusal | nothing happens at all | — |
| Destruction | the struct goes out of scope | — |

There is no `reset`, no `rewind`, no `set_claimed`, and no `Drop`. A `Claimer`'s
entire observable history is a `u64` that only ever increases, from zero, until
the value is dropped.

That is the shortest lifecycle of any type in the family, and it is deliberate:
the type has no state to unwind because unwinding is the operation
this crate's module documentation forbids (`src/lib.rs:44-49`):

> Releasing is not possible: another producer may already have claimed the range
> beyond it, so rewinding the cursor would hand out sequences twice

The `'a` borrow is the only thing the type system enforces about the type's
life: a `Claimer< 'a >` cannot outlive the `GatingSet` it was built over. That
is a real guarantee and it is the reason `ring_mpsc` needed an `Ends` struct to
own both together ([`data_structure/002`](../data_structure/002_the_borrow_that_is_half_the_type.md)).

### CL33 — The Accessor Hands Out `store` and `fetch_add`

The monotonicity above is a property of the *methods*, not of the cursor. And
`cursor()` gives the cursor away:

```rust
pub const fn cursor( &self ) -> &PaddedCursor
```

`PaddedCursor` implements `SeqCell` (`ring_cursor/src/lib.rs:199`), which
is a **public** trait — deliberately re-exported by `ring_cursor` (`:60-68`)
because "a crate holding a `PaddedCursor` and not this trait holds a value it
cannot load". Its full surface (`ring_atomic/src/lib.rs:108-151`):

| Method | Takes | Effect on a `Claimer`'s cursor |
|--------|-------|--------------------------------|
| `load` | `&self` | harmless — this is what `claimed()` wraps |
| **`store`** | `&self` | **rewinds it to any value at all** |
| **`fetch_add`** | `&self` | **advances it past the gate — the rejected design** |
| `compare_exchange` | `&self` | the sanctioned operation, now callable with any ordering |

All four take `&self`, so a `&PaddedCursor` is sufficient for every one of them.
Two lines that compile today against the public API:

```rust
claimer.cursor().store( Seq::ZERO, Ordering::Release );      // hands out every sequence twice
claimer.cursor().fetch_add( 4, Ordering::AcqRel );           // grants 4 slots without consulting the gate
```

The second is the exact design the module documentation spends thirteen lines
rejecting and the crate's founding decision exists to exclude
([`decisions/001`](../decisions/001_compare_exchange_rather_than_fetch_add.md)) —
reachable from outside in one line, with no `unsafe`.

Run against the crate as it ships, this breaks the multi-producer exclusivity
invariant and `Claim::overlaps` reports it. The sequence lives in
`writing_through_the_cursor_accessor_defeats_the_gate`, pulled here by pattern
rather than by line address so an edit above it cannot silently re-target this
recipe at a different function:

```sh
cd "$(git rev-parse --show-toplevel)"
command sed -n '/fn writing_through_the_cursor_accessor_defeats_the_gate/,/^}$/p' \
  ring_claim/tests/claim_test.rs \
  | command grep -E 'claim\( |cursor\(\)\.|overlaps|stolen'
cargo test -p ring_claim --test claim_test writing_through_the_cursor_accessor_defeats_the_gate 2>&1 \
  | command grep -E '^test .* \.\.\. ok$'
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
  let first = claimer.claim( 4 ).expect( "an empty ring admits a full-capacity claim" );
  assert_eq!( claimer.claim( 1 ), Err( RingError::Full ), "and then the gate refuses" );
  let stolen = claimer.cursor().fetch_add( 4, Ordering::AcqRel );
  assert_eq!( stolen, Seq( 4 ), "granted the sequence the gate had just withheld" );
  claimer.cursor().store( Seq::ZERO, Ordering::Release );
  let again = claimer.claim( 4 ).expect( "a rewound cursor makes the gate admit again" );
    first.overlaps( again ),
test writing_through_the_cursor_accessor_defeats_the_gate ... ok
```

Both failure modes the crate exists to prevent, in the seven lines printed
above and no `unsafe` among them: a grant past a gate that had just refused, and
two live claims covering the same four sequences — confirmed by the very method
(`Claim::overlaps`) the crate makes public so a test can assert exclusivity
([`invariant/001`](../invariant/001_no_two_producers_hold_one_sequence.md)).

And the manual check written to guard against it does not reach:

> `tests/manual/readme.md § C2`: the comment-filtered source must contain
> exactly two atomic mutations, both `compare_exchange`, with no `fetch_add` and
> no `store`.

`§ C2` greps `ring_claim/src/lib.rs`. It is a check on this crate's own
implementation, and it passes — correctly — while the accessor exports the
capability to every caller. The check is not wrong; its scope is simply the
inside of the crate, and the hole is on the outside.

Three things keep this from being a live defect:

1. `cursor()` has exactly one external caller, and it uses `addr()` — the
   cursor's address, not its value ([`item/002`](../item/002_the_seven_of_the_claimer.md)).
2. Doing either of these is transparently wrong at the call site in a way that
   `let _ = claimer.claim( 4 )` is not — it takes deliberate effort, not an
   omission.
3. The cursor is genuinely needed as a value, for the cache-line check and for
   tests that assert layout.

What it does mean is that the crate's monotonicity guarantee is a **convention
above the public API**, not an encapsulated invariant. `claimed()` is the
sanctioned reader and returns a plain `Seq` with nothing writable attached; a
narrower accessor — `cursor_addr() -> usize` for the alignment check, and
nothing else — would close the gap for the one real caller. Whether that is
worth the churn is a judgement, but the current shape should not be mistaken for
encapsulation.

**Disposition:** applied — the accessor was left as it is and the gap was made
visible instead, in the two places a reader can reach it. `cursor()`'s own doc
comment now carries a `# This is a convention, not encapsulation` section stating
that `PaddedCursor` implements the public `SeqCell` trait, that every one of its
methods takes `&self`, and therefore that a `&PaddedCursor` is enough to `store`
the cursor backwards or `fetch_add` it past the gate; it says outright that
monotonicity is a property of `claim`, `claim_up_to` and `claimed` rather than of
the cursor, gives the three sanctioned uses — read it, hand it to `ring_publish`,
take its address — and records that the manual `§ C2` check greps this crate's
own source and so says nothing about callers. The second place is executable:
`writing_through_the_cursor_accessor_defeats_the_gate` in `tests/claim_test.rs`
performs the eight-line sequence above against the shipping crate and asserts
both failure modes, so the hazard is pinned rather than described. It is written
to keep passing — narrowing `cursor()` to an address or a read-only view would
stop it compiling, which is the signal the gap closed. Now prints:
`test writing_through_the_cursor_accessor_defeats_the_gate ... ok`

### CL34 — The Non-Wrapping Argument, and the One Sentence That Gets Its Failure Backwards

A `Claimer` never wraps, and the family depends on that everywhere:
`Seq` comparison is plain `<`, with no lap-aware logic anywhere
(`ring_types/src/id.rs:11-14`):

> Monotonic and, for every reachable workload, non-wrapping: at 10⁹
> publications per second a `u64` runs for roughly 584 years. The family
> depends on that — `Seq` comparison is plain `<`, with no lap-aware
> wrap-around logic anywhere, because the wrap point is unreachable.

The arithmetic checks out: `2^64 / 10^9 ≈ 1.844 × 10^10` seconds ≈ **584.9
years**. And 10⁹ claims per second is already generous by two or three orders of
magnitude for a compare-exchange loop under contention, so the real figure is
larger still. The argument is sound and the design that rests on it is correct.

What is not correct is the sentence describing what would happen if it *were*
reached. `Seq::next`'s doc, as this instance found it — the lines have since
been rewritten, so the quote below is history and the recipe further down is
the current text:

> Panics on overflow in a debug build and **saturates in a release build**,
> which is the standard `u64` addition behaviour — deliberately not wrapping,
> since a wrapped `Seq` would silently violate the monotonicity every gate in
> the family relies on.

Standard `u64` addition in a release build **wraps**. It does not saturate.
Measured:

```rust
#[ inline( never ) ]
fn advanced_by( v : u64, n : u64 ) -> u64 { v + n }
// release, default overflow-checks:  u64::MAX + 1 = 0
// debug:                             panicked: attempt to add with overflow
```

| Claimed | Actual |
|---------|--------|
| debug: panics | ✔ correct |
| release: saturates at `u64::MAX` | ✘ **wraps to `0`** |
| "deliberately not wrapping" | ✘ — plain `+` is exactly wrapping |

The error runs in the unsafe direction. Saturation would *preserve*
monotonicity — the cursor would jam at `u64::MAX`, every claim would be refused,
and the ring would stop loudly. Wrapping destroys it: the cursor returns to zero
and every gate comparison in the family silently inverts, which is precisely the
outcome the same sentence's final clause says must not happen. So the comment
promises the safe failure mode and the code delivers the dangerous one.

`advanced_by` (`:57-60`) is the method this crate actually calls — `Claim::end()`
and every cursor advance go through it — and it is the same `self.0 + n` with no
overflow note at all.

Two things to be clear about. This is a **documentation** defect, not a
correctness one: at 584 years the branch is unreachable, and no reachable
workload reaches it. And the fix is not to add checked arithmetic — that would
buy nothing and cost an instruction on the hot path. The fix is to say what
happens: *wraps in release, which would silently invert every gate comparison —
which is why the 584-year argument above has to hold rather than merely being
likely.* Stated that way the comment reinforces the design instead of quietly
contradicting it.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A1 -F 'wraps to zero in a release' ring_types/src/id.rs
```

Live output:

```
    /// Panics on overflow in a debug build and wraps to zero in a release
    /// build — the standard `u64` addition behaviour. A wrapped `Seq` would
```

**Disposition:** applied — `Seq::next`'s doc comment in `ring_types` no
longer claims release-mode saturation; it now says the true failure mode
(wraps to zero) and restates why the 584-year argument has to hold rather
than merely being likely. Now prints: `Panics on overflow in a debug build and wraps to zero in a release`

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_a_range_from_grant_to_publication.md](001_a_range_from_grant_to_publication.md) | The range's life, which has the window this type does not |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_borrow_that_is_half_the_type.md](../data_structure/002_the_borrow_that_is_half_the_type.md) | The `'a` that bounds this life |
| [../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md](../data_structure/001_sixteen_bytes_and_one_hundred_twenty_eight.md) | The padded cursor `cursor()` hands out |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_the_seven_of_the_claimer.md](../item/002_the_seven_of_the_claimer.md) | `cursor()`'s documented purposes against its one real caller |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_compare_exchange_rather_than_fetch_add.md](../decisions/001_compare_exchange_rather_than_fetch_add.md) | The design `fetch_add` on the handed-out cursor reproduces |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_no_two_producers_hold_one_sequence.md](../invariant/001_no_two_producers_hold_one_sequence.md) | The property a `store` on the cursor would break without a race |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/src/lib.rs:44-49` | Why there is no rewind |
| `ring_claim/src/lib.rs:273-315` | Construction at zero, and the accessor |
| `ring_atomic/src/lib.rs:108-151` | `SeqCell`'s four methods, all on `&self` |
| `ring_cursor/src/lib.rs:60-68,199` | Why the trait is public, and the impl that makes the cursor writable |
| `ring_types/src/id.rs:11-14` | The 584-year argument |
| `ring_types/src/id.rs`, `Seq::next`'s overflow paragraph | The sentence that got release-mode overflow backwards, since corrected — de-addressed because the correction moved it |

### Tests

| File | Relationship |
|------|--------------|
| `tests/claim_test.rs:298` — `the_producer_cursor_is_readable_and_starts_at_zero` | Construction at `Seq::ZERO` |
| `tests/claim_test.rs:137` — `successive_claims_are_contiguous…` | Monotonicity across many grants |
| `tests/manual/readme.md § C2` | The `fetch_add`/`store` check, and the boundary of what it scans |
