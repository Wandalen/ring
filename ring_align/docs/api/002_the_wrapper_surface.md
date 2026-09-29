# API: The Wrapper Surface

### Scope

- **Purpose**: Document the producing half of the crate — four associated functions over a type that carries no data of its own — and the one obligation it cannot discharge for its caller.
- **Responsibility**: State the surface, the `const` asymmetry and its cause, what the type refuses to do for ergonomics, and what a caller still has to get right.
- **In Scope**: `new`, `get`, `get_mut`, `into_inner`; the absent `Deref`.
- **Out of Scope**: The layout the wrapper produces, which is [`algorithm/002`](../algorithm/002_rounding_a_payload_up_to_whole_lines.md); the derives, which are [`type/002`](../type/002_cache_aligned.md).

### Surface

```rust
impl< T > CacheAligned< T >
{
  pub const fn new( value : T ) -> Self;
  pub const fn get( &self ) -> &T;
  pub const fn get_mut( &mut self ) -> &mut T;
  pub       fn into_inner( self ) -> T;
}
```

Four functions, no trait implementations beyond the derives, no bounds on `T`.
**Importing this half acquires a capability** — the ability to give a value a
cache line — which is what distinguishes it from
[`api/001`](001_the_reading_surface.md), where a caller acquires only a way to
check.

### The `const` Asymmetry

Three are `const fn`; `into_inner` is not, and cannot be. Taking `self` by
value drops the wrapper at the end of the body, and for an unbounded generic
`T` the compiler must assume a destructor exists — `E0493`. The full
explanation and a one-line reproduction are in
[`type/002`](../type/002_cache_aligned.md) § Accessor Surface.

**What the asymmetry costs a caller:** a `CacheAligned` can be *built* in a
`const` or `static` initialiser and *read* there, but not unwrapped there. In
practice this constrains nothing the family does — `ring_cursor` builds
`PaddedCursor` through `new` and never unwraps — but it is the kind of gap a
caller discovers at the point of writing a `const`, which is late.

### Why `get` Returns a Reference

`get( &self ) -> &T`, not `get( &self ) -> T`. A by-value accessor would need
`T : Copy` and would be useless for the crate's actual consumer:
`ring_cursor`'s payload is an `AtomicSeq`, which must be *borrowed* to be
loaded or stored. A `Copy` of an atomic is a different atomic.

So the reference-returning form is not conservatism — it is the only form that
serves the one real use.

### What the Type Refuses

**No `Deref`.** It would let `padded.load( Ordering::Acquire )` work directly,
and that is exactly the reason it is absent: the explicit `.get()` keeps the
padding visible at the call site. A reader of

```rust
self.0.get().load( Ordering::Acquire )
```

can see there is a wrapper. A reader of `self.0.load( … )` cannot, and would
have no cue that the field costs 64 bytes or that its position in the struct
matters. The ergonomic cost is one method call; the information preserved is
the whole point of the crate.

**No size check on the payload.** `CacheAligned< [ u8; 4096 ] >` compiles and
occupies 64 lines. That is correct behaviour — the guarantee is whole lines,
not one line — and it is a real memory cost the type will not warn about
(→ [`algorithm/002`](../algorithm/002_rounding_a_payload_up_to_whole_lines.md) § The Cost, Stated).

### The Obligation the Caller Keeps

**Wrapping a value does not separate it from anything.** The type guarantees
its own size and alignment; the property the family needs — two things not
contending — is a property of *where a caller puts two of them*
(→ [`invariant/001`](../invariant/001_two_wrapped_fields_never_share_a_line.md)).

Three ways a caller can hold this API correctly and still get false sharing:

| # | What the caller does | Why the wrapper does not help |
|---|----------------------|-------------------------------|
| W1 | Wraps one cursor and not the other | The unwrapped one can sit in the padded one's trailing bytes |
| W2 | Wraps both, then puts them behind one `Box` alongside a third hot field | The third field lands in a padded value's spare space |
| W3 | Wraps both and the constant is wrong for the host | Both are separated by 64 on a 128-byte-line machine (→ [`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md)) |

None is checkable from inside this crate. `ring_cursor` discharges W1 and W2 by
holding both cursors in one type it owns, and asserts the result:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_cursor
out="$( cargo test --test cursor_test 2>&1 )"
printf '%s\n' "$out" | command grep -E '^test .+ \.\.\.' | LC_ALL=C sort
printf '%s\n' "$out" | command grep -E '^test result:' | sed -E 's/; finished in .*/; finished/'
```

Live output:

```
test a_capacity_of_one_still_has_room_for_one ... ok
test a_consumer_moving_on_reopens_the_ring ... ok
test a_cursor_holds_the_sequence_it_was_built_with ... ok
test a_cursor_is_shared_by_reference_not_by_copy ... ok
test a_fresh_pair_has_the_whole_ring_free ... ok
test a_padded_cursor_is_its_atomic_and_nothing_else ... ok
test a_padded_cursor_occupies_exactly_one_cache_line ... ok
test a_pair_is_two_lines_plus_its_capacity ... ok
test an_array_of_cursors_gives_each_its_own_line ... ok
test every_cursor_starts_on_a_line_boundary ... ok
test exactly_one_lap_ahead_is_full_and_one_less_is_not ... ok
test free_slots_falls_as_the_producer_advances ... ok
test many_producers_on_one_cursor_lose_nothing ... ok
test may_claim_and_free_slots_never_disagree ... ok
test padding_does_not_change_what_the_cell_does ... ok
test pending_ignores_capacity_and_free_slots_does_not ... ok
test pending_is_the_distance_the_consumer_still_has_to_travel ... ok
test the_gap_survives_the_pair_being_moved ... ok
test the_pair_reads_both_cursors_for_every_reading ... ok
test two_cursors_in_one_struct_are_at_least_a_line_apart ... ok
test two_threads_advancing_two_cursors_do_not_lose_writes ... ok
test writing_one_cursor_leaves_the_other_alone ... ok
test result: ok. 22 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out; finished
```

W3 is nobody's, and that is the finding
[`pitfall/001`](../pitfall/001_a_constant_too_small_buys_nothing.md) records.

### AL5 — The One Function That Can Destroy a Value Is the One Without `#[ must_use ]`

Five entry points, one annotation, and it is on the predicate:

```
  -- every callable entry point, and its const-ness --
63:  pub const fn new( value : T ) -> Self
75:  pub const fn get( &self ) -> &T
88:  pub const fn get_mut( &mut self ) -> &mut T
99:  pub fn into_inner( self ) -> T
122:pub const fn on_distinct_lines( a : usize, b : usize ) -> bool
  -- how many of them carry #[ must_use ] --
1
  -- control: the family annotates freely elsewhere --
ring_types/src/capacity.rs:2
ring_seqno/src/lib.rs:5
```

Ignoring `on_distinct_lines`'s result is useless. Ignoring `into_inner`'s
result **consumes the wrapper and drops the payload** — the only operation in
the crate that can lose data, and it compiles without a warning.

**Finding.** The annotation went to the case a reader thinks of first rather
than to the case that costs something. `ring_types` and `ring_seqno` annotate
freely, so this is not the workspace declining to use `must_use`; it is this
crate's five entry points never having been compared against each other.

---

### AL6 — Three of Four Are `const`, and the Fourth Is Not a Choice

`new`, `get` and `get_mut` are `const fn`; `into_inner` is not. The asymmetry
looks like a judgement about which operations belong in a `const` initialiser,
and it is not one — `E0493` refuses the fourth, because taking `self` by value
drops an unbounded generic whose destructor the compiler must assume exists.

**Finding.** The measured cost of the refusal is zero: no crate in the family
unwraps a `CacheAligned`
(→ [`workaround/002`](../workaround/002_into_inner_cannot_be_const.md)). What
the asymmetry costs is a reader's time — it reads as a decision, so a reader
looks for the reasoning, and the reasoning is a compiler error rather than a
design.

---

### AL8 — The Type Cannot Require the Thing Its Guarantee Is About

Every function on this surface operates on one wrapper. The property the crate
exists for — two things not contending — is a property of two wrappers *and the
struct that holds them*, and no signature here can mention that struct. The
three ways a caller holds this API correctly and still gets false sharing, in
the W1/W2/W3 table above, are all of this shape.

**Finding.** The API's boundary and the guarantee's boundary do not coincide,
and the gap is discharged one crate away by `ring_cursor` holding both cursors
in a type it owns. That is the right place for it, and it means reading this
surface alone gives a caller no way to tell whether they have the property —
which is what [`invariant/001`](../invariant/001_two_wrapped_fields_never_share_a_line.md)
exists to state and what this API deliberately does not try to enforce.

### APIs

| File | Relationship |
|------|--------------|
| [001_the_reading_surface.md](001_the_reading_surface.md) | The checking half — grants no capability, and is how a caller confirms it discharged W1/W2 |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_rounding_a_payload_up_to_whole_lines.md](../algorithm/002_rounding_a_payload_up_to_whole_lines.md) | What `new` actually produces, and the cost it does not warn about |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_a_padded_pair_in_one_struct.md](../data_structure/002_a_padded_pair_in_one_struct.md) | The arrangement W1 and W2 fail to build |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_two_wrapped_fields_never_share_a_line.md](../invariant/001_two_wrapped_fields_never_share_a_line.md) | The property this surface makes achievable but does not itself guarantee |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_cache_aligned_and_its_associated_functions.md](../item/001_cache_aligned_and_its_associated_functions.md) | Declaration sites and grep-verified call sites for all four functions |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_the_wrapped_values_arc.md](../lifecycle/001_the_wrapped_values_arc.md) | The order these four are called in, and the one that ends the guarantee |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_newtype_as_layout_carrier.md](../pattern/001_the_newtype_as_layout_carrier.md) | Why the field is private and `Deref` is absent |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_cache_aligned.md](../type/002_cache_aligned.md) | The type these functions belong to, its derives, and the `E0493` reproduction |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_two_wrapped_fields_never_share_a_line.md`](../invariant/001_two_wrapped_fields_never_share_a_line.md) | The invariant this surface delivers the mechanism for |

### Tests

| File | Relationship |
|------|--------------|
| `tests/align_test.rs` | `the_wrapper_round_trips_its_payload` exercises all four; `the_wrapper_is_copy` fails to compile rather than fails to pass if `Copy` is dropped |
