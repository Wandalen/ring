# invariant

The reached-test carries four clauses, and this crate answers all four.
Two of them are the invariants recorded here: capacity equals length, and two
distinct slot indices never alias. Both hold. What the instances find is that
both hold for reasons weaker or further away than the tests naming them suggest.

The first traces the capacity/length relation to the one line that establishes it
and finds nothing that could subsequently break it — no `resize`, no second
constructor, private fields — which makes it an unusually strong invariant with
almost no state space to test. Along the way it finds the reached-test naming an
indexed `set` the type does not have, because writing a payload is the slot's
operation. The second finds that the non-aliasing clause, as storage can state
it, is discharged by the language: `get` is a slice index, `len` is the slice's
own length, and the mismatch the stronger test's comment describes cannot arise.
The version that can fail — sequences within a lap folding to one index — belongs
to `ring_index`, is tested there, and is nowhere connected to this clause.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Capacity Equals Length, Always](001_capacity_equals_length_always.md) | BF22, BF23 — an invariant nothing can break after `new`, and a contract clause naming an absent API |
| 002 | [Two Distinct Indices Never Alias](002_two_distinct_indices_never_alias.md) | BF24, BF25 — a test that cannot fail, and the tier that actually earns the assurance |

### The Four Clauses and What Discharges Each

| Clause | Discharged by | Recorded in |
|--------|---------------|-------------|
| allocates exactly `N` slots once | `new`'s three steps, measured at one allocation | [`algorithm/002`](../algorithm/002_one_allocation_n_defaults.md) |
| exposes indexed get/set | `get`/`get_mut`, with the write as an exclusive borrow — there is no `set` | BF23 |
| holds no cursor and no ordering state | A `size_of` pin, a scrambled-order sweep, and a human reading | [`data_structure/001`](../data_structure/001_three_words_whatever_the_slot_costs.md) |
| two distinct slot indices never alias | Slice semantics, trivially — the substantive form lives in `ring_index` | BF24, BF25 |

### Why the Capacity/Length Relation Cannot Drift

`new` sets the slice length from `capacity.get()` and stores the `Capacity`
beside it. After that: no `resize`, no `push`, no `truncate`, no second
constructor, and the fields are private. The only code that could violate the
relation is the three lines that establish it, so a buffer surviving `new` keeps
it for life by construction rather than by discipline.

### Why the Non-Aliasing Test Cannot Fail

The stronger test's comment names its target: *a `len` that over-reported while
the slots were shared*. But `len` is `self.slots.len()` — there is no separate
count to drift — and `get` is `&self.slots[ index.get() ]`, so distinct in-range
indices give distinct elements by the language's guarantee. Both tests are drift
guards: they would catch a `get` rewritten to fold or mask before indexing, which
is the one change that could introduce aliasing here. They are not correctness
guards, and the comment invites reading them as such.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the line that establishes the capacity/length relation
command grep -m1 -A5 -F '  pub fn new( capacity : Capacity ) -> Self' ring_store/src/lib.rs

# the reached-test this crate answers, in the feature's own words
command grep -m1 -A3 -F '//! Claims `docs/feature/168_ring_buffer_storage.md`, whose reached-test reads:' ring_store/tests/buffer_test.rs

# the clause's "get/set" against the surface that answers it
grep -n 'pub fn set\|pub fn get' ring_store/src/lib.rs

# why the non-aliasing test cannot fail: a slice length and a slice index
awk '/^  \/\/\/ assert_eq!\( buffer\.len\(\), buffer\.capacity\(\)\.get\(\) \);$/{ n1 = NR } n1 && NR >= n1 + 2 && NR <= n1 + 6 { print } /^  \/\/\/ condition to handle\.$/{ n2 = NR } n2 && NR >= n2 + 1 && NR <= n2 + 5 { print }' ring_store/src/lib.rs

# the stronger assertion, and the scenario its comment names
command grep -m1 -A14 -F 'fn distinct_indices_have_distinct_addresses()' ring_store/tests/buffer_test.rs

# the predicate for the substantive form of the property
command grep -m1 -A15 -F '/// can be tested against it; it does not prevent anything itself.' ring_index/src/lib.rs | tail -n 14
```

The capacity-1024 equality comes from a release probe; it is quoted in
[`invariant/001`](001_capacity_equals_length_always.md).

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF22 | `ring_store` | n/a — observation | The capacity/length relation is established in one line and unbreakable afterwards, since nothing writes either value and no second constructor exists |
| BF23 | `ring_store` | **wrong doc** | This crate's reached-test — the contract it is verified against — names an indexed `set` that does not exist; writing a payload is the slot's operation, expressed here as an exclusive borrow |
| BF24 | `ring_store` | n/a — coverage | `distinct_indices_have_distinct_addresses` asserts a property of slices, not of `Buffer`; the `len`/storage mismatch its comment names is unreachable given a two-line implementation |
| BF25 | `ring_store` | n/a — doc gap | The load-bearing form of non-aliasing belongs to `ring_index` and is tested there; nothing connects that clause to the tier that actually discharges it |
