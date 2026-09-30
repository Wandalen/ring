# Lifecycle: The Empty Claim as a First-Class State

### Scope

**Purpose:** Record that zero is a legal count at every entry point, what the
type does with it across all eight methods, and what the one caller in the family
pays for taking that path unconditionally.

**Responsibility:** `count == 0` through `claim`, `claim_gated`, `sequences`,
`drain_order`, `contains`, and `overlaps`; and `TlsBuffer::flush_into`'s
documented decision not to check.

**In Scope:** `ring_batch/src/lib.rs:103-106`, `:185-189`;
`ring_tls/src/lib.rs:242-247`, `:280`.

**Out of Scope:** The absence of a destructor is
[`lifecycle/001`](001_no_lifecycle_and_no_rollback.md). The niche the empty state
costs the type is
[`data_structure/001`](../data_structure/001_sixteen_bytes_that_are_not_a_buffer.md)
BA9.

---

## Zero Is Not an Error Anywhere

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the state, and the two methods that test for it --'
command grep -m1 -A6 -F '  /// assert!( BatchClaim::new( Seq( 4 ), 0 ).is_empty() );' ring_batch/src/lib.rs | tail -n 5
command grep -m1 -A1 -F '    !self.is_empty() && !other.is_empty()' ring_batch/src/lib.rs
echo '  -- the gated entry point on a full ring --'
command grep -m1 -A12 -F 'fn a_gated_claim_of_zero_always_succeeds_even_on_a_full_ring()' ring_batch/tests/batch_test.rs
echo '  -- and the one caller, which does not check --'
command grep -m1 -F '    let claim = claim( cursor, self.items.len(), order );' ring_tls/src/lib.rs
```

Live output:

```
  -- the state, and the two methods that test for it --
  #[ must_use ]
  pub const fn is_empty( &self ) -> bool
  {
    self.count == 0
  }
    !self.is_empty() && !other.is_empty()
      && self.start.0 < other.end().0 && other.start.0 < self.end().0
  -- the gated entry point on a full ring --
fn a_gated_claim_of_zero_always_succeeds_even_on_a_full_ring()
{
  let capacity = cap( 2 );
  let producer = AtomicSeq::default();
  let consumer = AtomicSeq::default();

  claim_gated( &producer, &consumer, 2, capacity, Ordering::AcqRel ).unwrap();
  let batch = claim_gated( &producer, &consumer, 0, capacity, Ordering::AcqRel )
    .expect( "asking for nothing needs no room" );

  assert!( batch.is_empty() );
  assert_eq!( batch.start(), Seq( 2 ) );
}
  -- and the one caller, which does not check --
    let claim = claim( cursor, self.items.len(), order );
```

---

### BA32 — Six Behaviours That All Agree, and the One That Had to Be Written By Hand

An empty claim is legal at both entry points and meaningful in every operation:
`is_empty` is true, `len` is zero, `end` equals `start`, `sequences` yields
nothing, `drain_order` yields nothing, `contains` is false for every sequence
including its own start, and `overlaps` is false against everything including
itself. `claim_gated` grants it on a completely full ring, because asking for
nothing needs no room.

Five of those seven fall out of the arithmetic with no code written for them —
`start..start` is an empty range, and a point compared against it is outside it.
The sixth had to be written:

**Finding.** `overlaps` carries an explicit `!self.is_empty() && !other.is_empty()`
guard, and it is load-bearing. Without it, an empty claim at sequence 5 compared
against `[0, 16)` satisfies both `5 < 16` and `0 < 5` and reports an overlap it
does not have. `contains`, three lines above, needs no guard for the same edge
case because it compares a single point rather than two ranges.

The distinction is not documented in either method. `an_empty_claim_overlaps_nothing_even_inside_another`
records why the guard matters — "Treating it as one would make every flush of an
empty buffer look like a protocol violation" — which is the clearest statement in
the crate of why zero had to be a first-class state rather than an error, and it
lives in a test rather than in the type.

---

### BA33 — The Empty Flush Is Free as a Value and Not Free as an Operation

`TlsBuffer::flush_into` calls `claim( cursor, self.items.len(), order )` with no
zero check, and says exactly why:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  /// Flushing an empty buffer is legal and claims nothing — a zero-length' ring_tls/src/lib.rs
```

Live output:

```
    /// Flushing an empty buffer is legal and claims nothing — a zero-length
    /// `fetch_add` still costs one atomic, which is why a caller in a hot loop
    /// should check [`TlsBuffer::is_empty`] first. This function does not check
    /// on the caller's behalf: a silent skip would make the operation count
    /// depend on the data, and the whole point of the counting shim is that it
    /// does not.
```

The reasoning is sound and the warning is explicit. What neither says is what the
atomic costs the threads that do have work. Measured — one producer claiming
eight slots, alongside N threads flushing nothing:

```
--- one producer claiming 8 slots, alongside N threads with nothing to publish ---
   idlers       fetch_add(0)          load only       cost
        1           22.24 ns           11.36 ns       2.0x
        3           33.82 ns           24.45 ns       1.4x
        7           66.61 ns           52.50 ns       1.3x
       15          110.12 ns           84.14 ns       1.3x
```

**Finding.** A single thread flushing an empty buffer in a loop doubles the
working producer's per-claim cost, 22.24 ns against 11.36 ns. The `fetch_add(0)`
takes the cursor's cache line exclusively exactly as a real claim does — zero is
the argument, not the memory traffic — so a thread with nothing to say competes
for the line on equal terms with one that has sixty-four items staged.

At higher idler counts the ratio falls to 1.3× only because the line is already
contended; the absolute penalty keeps growing, from 10.9 ns to 26 ns per claim.

This is a cost, not a defect: `ring_tls`'s reason for not checking — keeping the
operation count independent of the data, so the counting shim measures what it
claims to — is a real property worth paying for. The number is what the doc does
not give. A caller reading "still costs one atomic" would reasonably estimate a
few nanoseconds of its own time; the measurement says the bill is paid by
somebody else, and is a multiple rather than an increment.

**Disposition:** declined — the fix belongs in `ring_tls/src/lib.rs`'s
own doc comment on `flush_into`, a different crate from `ring_batch` (the crate
this disposition pass is currently scoped to). Editing another crate's source
is out of this instance's own scope absent explicit user authorization; the
measured numbers are recorded above and stand as the finding's own record.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/001`](001_no_lifecycle_and_no_rollback.md) | The state transitions the type does not have |
| [`data_structure/001`](../data_structure/001_sixteen_bytes_that_are_not_a_buffer.md) | The niche this state costs `Option< BatchClaim >` |
| [`item/001`](../item/001_the_method_whose_reason_was_declined.md) | The `overlaps` guard and the tests that cover it |
| [`non_functional_requirement/001`](../non_functional_requirement/001_what_one_batch_actually_buys.md) | The uncontended cost of the same `fetch_add` |

### Sources

| Fact | Where |
|------|-------|
| `is_empty` and the `overlaps` guard | `ring_batch/src/lib.rs:103-106`, `:187-188` |
| Zero granted on a full ring | `ring_batch/tests/batch_test.rs:299-311` |
| The unconditional call | `ring_tls/src/lib.rs:280` |
| The documented decision not to check | `ring_tls/src/lib.rs:242-247` |
| The contended cost | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `an_empty_claim_is_a_success_not_a_failure` | That zero is a value, not an error |
| `a_gated_claim_of_zero_always_succeeds_even_on_a_full_ring` | The gate's zero path |
| `an_empty_claim_drains_to_nothing` | `sequences` and `drain_order` on zero |
| `an_empty_claim_contains_nothing_at_all` | The five behaviours that need no guard |
| `an_empty_claim_overlaps_nothing_even_inside_another` | The one that does |
| *(to create)* | The contended cost of `fetch_add(0)` — measurable, warned about in prose, measured nowhere |
