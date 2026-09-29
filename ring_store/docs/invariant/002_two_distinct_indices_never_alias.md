# Invariant: Two Distinct Indices Never Alias

### Scope

**Purpose:** Record that the non-aliasing clause is asserted twice — by value at
64 slots and by address at 16 — that the scenario the stronger test names as its
target cannot arise given the implementation, and that the version of the
property which can actually fail lives one crate down.

**Responsibility:** The reached-test's fourth clause: what it claims, what discharges
it, and where the failure it guards against would really occur.

**In Scope:** `ring_store/tests/buffer_test.rs:84-124`;
`ring_store/src/lib.rs:167-171, 203-207`.

**Out of Scope:** The fold that maps sequences onto indices is
[`algorithm/001`](../algorithm/001_one_mask_no_modulo.md). The capacity/length
relation is [`invariant/001`](001_capacity_equals_length_always.md).

---

## Two Assertions of One Clause

The suite states the clause in the reached-test's own words and asserts it twice. The
value form writes a distinguishable value into all 64 slots and reads them back;
the address form compares storage locations directly:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A14 -F 'fn distinct_indices_have_distinct_addresses()' ring_store/tests/buffer_test.rs
```

Live output:

```
fn distinct_indices_have_distinct_addresses()
{
  // The stronger form of the same claim: not merely "the values differ" but
  // "the storage differs". A `len` that over-reported while the slots were
  // shared would pass the value test and fail this one.
  let buffer : Buffer< TypedSlot< u8 > > = Buffer::new( cap( 16 ) );
  let mut seen : Vec< usize > = Vec::new();

  for i in 0..16
  {
    let address = std::ptr::from_ref( buffer.get( SlotIndex( i ) ) ) as usize;
    assert!( !seen.contains( &address ), "slot {i} shares an address with an earlier slot" );
    seen.push( address );
  }
}
```

---

### BF24 — The Scenario the Stronger Test Names Cannot Arise

The comment states the failure it exists to catch: *a `len` that over-reported
while the slots were shared*. Both halves of that are foreclosed by the
implementation.

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^  \/\/\/ assert_eq!\( buffer\.len\(\), buffer\.capacity\(\)\.get\(\) \);$/{ n1 = NR } n1 && NR >= n1 + 2 && NR <= n1 + 6 { print } /^  \/\/\/ condition to handle\.$/{ n2 = NR } n2 && NR >= n2 + 1 && NR <= n2 + 5 { print }' ring_store/src/lib.rs
```

Live output:

```
  #[ must_use ]
  pub const fn len( &self ) -> usize
  {
    self.slots.len()
  }
  #[ must_use ]
  pub fn get( &self, index : SlotIndex ) -> &S
  {
    &self.slots[ index.get() ]
  }
```

`len` is the slice's own length, so it cannot over-report — there is no separate
count to drift. And `get` is a slice index, so distinct in-range indices yield
distinct elements by the language's guarantee about slices, not by anything this
crate arranges. For the test to fail, `Box< [ S ] >` would have to be unsound.

**Finding.** `distinct_indices_have_distinct_addresses` asserts a property of
Rust rather than a property of `Buffer`. It cannot fail while the implementation
is a slice index over a boxed slice, which is what both functions above are.

That does not make it worthless — it makes it a *drift* test rather than a
correctness test, and the same is true of its value-form sibling. If `get` were
ever rewritten to fold, mask, or otherwise transform the index before indexing —
the one change that could introduce aliasing here — both tests would catch it
immediately. What is worth recording is that the comment describes the guard as
protecting against a `len`/storage mismatch that the two-line implementation
makes unreachable, so a reader assessing coverage will credit this clause with
more assurance than it carries.

---

### BF25 — The Version That Can Fail Is `ring_index`'s, and This Crate Depends On It Silently

The clause that matters operationally is not about indices. A ring addresses
slots by *sequence*, and the property it needs is that two sequences less than a
lap apart never fold to one index — which is `ring_index`'s business, not this
crate's:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A15 -F '/// can be tested against it; it does not prevent anything itself.' ring_index/src/lib.rs | tail -n 14
```

Live output:

```
/// ```
/// use ring_types::{ Capacity, Seq };
/// use ring_index::aliases;
///
/// let cap = Capacity::new( 4 ).unwrap();
/// assert!( aliases( Seq( 1 ), Seq( 5 ), cap ) );
/// assert!( aliases( Seq( 1 ), Seq( 9 ), cap ) );
/// assert!( !aliases( Seq( 1 ), Seq( 2 ), cap ) );
/// ```
#[ must_use ]
pub fn aliases( a : Seq, b : Seq, capacity : Capacity ) -> bool
{
  of( a, capacity ) == of( b, capacity )
}
```

`ring_index` publishes the predicate for exactly this question and, as
[`algorithm/001`](../algorithm/001_one_mask_no_modulo.md) BF11 records, nothing
in the family calls it.

**Finding.** The reached-test assigns the non-aliasing clause to storage, and storage
discharges it trivially. The substantive form — sequences a lap apart alias,
sequences within a lap do not — is a property of the mask, lives in `ring_index`,
and is tested there. This crate's own suite exercises it incidentally, in
`a_full_lap_overwrites_and_a_partial_one_does_not` and in the forty-sequence
sweep of `a_sequence_addresses_the_slot_ring_index_says_it_does`, both of which
would fail if the fold were wrong.

So the assurance is real and it comes from the tier below. What is missing is any
statement of that: a reader auditing the reached-test finds the clause and finds two
tests named for it, both of which check the trivial reading, and nothing telling
them that the load-bearing version is one crate down and passing. The suite's
module comment does this well for the *third* clause — it names the two stand-ins
and their holes — and does not do it here.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](001_capacity_equals_length_always.md) | The other two clauses of the same reached-test |
| [`algorithm/001`](../algorithm/001_one_mask_no_modulo.md) | The fold that owns the substantive property, and `aliases`, which nothing calls |
| [`decisions/001`](../decisions/001_panic_rather_than_option.md) | What happens to an index outside the range this invariant assumes |
| [`data_structure/001`](../data_structure/001_three_words_whatever_the_slot_costs.md) | How the suite handles the one clause it cannot assert directly |

### Sources

| Fact | Where |
|------|-------|
| Both assertions | `ring_store/tests/buffer_test.rs:84-124` |
| `len` as the slice's own length | `ring_store/src/lib.rs:167-171` |
| `get` as a slice index | `ring_store/src/lib.rs:203-207` |
| `aliases`, the predicate for the substantive form | `ring_index/src/lib.rs:64-77` |
| That nothing calls it | Census in `algorithm/001` |

### Tests

| Test | Covers |
|------|--------|
| `two_distinct_slot_indices_never_alias` | The value form, 64 slots |
| `distinct_indices_have_distinct_addresses` | The address form, 16 slots — a drift guard rather than a correctness one |
| `a_full_lap_overwrites_and_a_partial_one_does_not` | The substantive property, incidentally |
| `ring_index` — `index_test.rs` | The substantive property, deliberately |
| *(to create)* | A comment or test note pointing from this clause to the tier that discharges it |
