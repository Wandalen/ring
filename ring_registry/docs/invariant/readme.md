# invariant

Three properties, one enforcement table, and a set of tripwires — R1 one name to
one ring, R2 exactly one owner, R3 the count that makes the other two checkable.
The set was discovered by implementing this crate, not read off an external
specification — worth recording, because an earlier version of this document
claimed the opposite, attributing the properties to a criterion that was never
actually written (→ RG22).

The enforcement is where the four findings converge. Three methods can write to
the map and the table watches one of them; the door it leaves open reaches the
exact outcome the crate's central pitfall exists to prevent, in one statement,
from the public surface. Against that, two properties the crate genuinely holds
go unclaimed: a refusal is total on the map, not merely length-preserving, and
the count accounting R3 is exact about sits beside a storage accounting nothing
mentions.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_one_name_one_ring.md) | One Name, One Ring | R1–R3, the enforcement table's third writer, and where the properties came from |
| [002](002_what_survives_a_refusal_and_what_does_not.md) | What Survives a Refusal and What Does Not | The refusal measured against the whole map, and the quantity R3 does not count |

## Three Writers, One Guarded

`register`, `get_mut` and `remove` all take `&mut self`; the four reads take
`&self` and cannot touch the map. E2 answers E1's concession with the `Entry`
match and a Gap of **None known**, `remove` is a deliberate transfer, and
`get_mut` is unexamined — yet a `&mut Split< T >` is an assignable place.
`*registry.get_mut( "events" ).unwrap() = fresh;` destroys six unread records
with `len` still 1 and `contains` still true: `insert`'s semantics precisely,
arriving through the one door the table does not watch. `core::mem::replace`
through the same borrow moves a registered ring out without `remove`, which is
the move E3's note says cannot happen except by `remove`.

All ten `Registry::get_mut` call sites in the workspace either test
`is_some`/`is_none` or bind and call `.ends()`. Not one assigns through the
borrow, so the hazard is as unexercised as it is undocumented.

## Two Properties Held and Not Claimed

A refusal is stronger than V2 asserts. Length is a coarse witness — a replace, a
rehash, a reservation would all leave it untouched — and measured across a
refusal on a map holding eight names, length stays 8 and capacity stays 14, with
the only allocations being the two six-byte `String`s the name costs. That makes
the refusal retryable, which is the property worth claiming and the one nothing
states.

The count accounting is exact and the storage accounting is unstated. After
eight registrations and eight removals `len()` is 0, `is_empty()` is `true`, and
the table still holds fourteen slots; no method on the surface names capacity,
reserve or shrink. It is not a leak — the memory is reachable and freed on drop
— but it is a fourth number in a crate whose central document is about
accounting.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the three writers and the four readers --'
command grep -n '^  pub fn' ring_registry/src/lib.rs | cut -c1-76 | sed 's/^/    /'
echo '  -- and the quantity no method reports --'
printf '    capacity / reserve / shrink on the surface: %s\n' \
  "$( command grep -c 'pub fn capacity\|pub fn reserve\|pub fn shrink' ring_registry/src/lib.rs || true )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RG21 | `ring_registry` | **latent hazard** | E1's Gap concedes the map alone is not enough and E2 answers with the `Entry` match and a Gap of "None known", which is exact for the write path it has in mind and covers one of three: `register`, `get_mut` and `remove` all take `&mut self`, `remove` is a deliberate transfer, and `get_mut` is unexamined even though a `&mut Split< T >` is an assignable place — `*registry.get_mut( "events" ).unwrap() = fresh;` destroys **six unread records with `len` still 1 and `contains` still true**, no refusal and no return value to ignore, which is `insert`'s semantics arriving through the one door the enforcement table does not watch; the second spelling is worse for E3, since `core::mem::replace` through the same borrow moves ownership of a *registered* ring out without `remove` — the move E3's note says cannot happen "except by `remove`, which transfers rather than duplicates" — while R2 itself survives because exactly one owner exists at every instant; all ten workspace `Registry::get_mut` sites test `is_some`/`is_none` or bind and call `.ends()`, so the hazard is entirely unexercised, and the repairs are an E5 row and a test that assigns through the borrow and asserts the drop count V1 already uses |
| RG22 | `ring_registry` | **wrong doc** | The V1 note grounds the drop counter in an upstream requirement — "which is why the acceptance criterion asks for one" — and the Sources table cites the same feature for "R1's first two clauses and R2's third, as the acceptance criterion states them", and there is no acceptance criterion: `docs/feature/181_named_ring_registry.md` is **30 lines carrying six headings** with **zero** occurrences of *criteri*, *drop*, *invariant*, *MUST* or *shall*, and no numbered clauses for "first two clauses" to number against; its one property-shaped phrase, rings "none reachable except through the registry that owns it", is R2 in other words, and R1 and R3 have no source outside this file; the attribution runs backwards — the properties were discovered by implementing and this document is where they are first stated, which is the more valuable thing to be, and presenting them as ratified upstream hides it while making the citation unfollowable; 402 of the 426 feature files are `Status: planned`, so citing one as a settled criterion is a mistake that scales |
| RG23 | `ring_registry` | n/a — doc gap | V2 detects "a refused registration mutates the map" through an assertion on `len() == 1`, and length is a coarse witness — a refusal that replaced one ring with another, rehashed the table, or reserved a slot it did not use would all leave it untouched; measured across a refusal on a map holding eight names, **length stays 8 and capacity stays 14**, the table neither grown nor rehashed nor touched, and the only allocations are the two six-byte `String`s of RG1, neither belonging to the map — so combined with the drop-counter test that already establishes no ring is dropped, the refusal is total on the registry's own state; that is a better property than V2 claims and worth claiming, because it is what makes the refusal *retryable* — a caller can register under a different name immediately with no reasoning about what the failed attempt left behind — and R1–R3 are all about what a registry contains while none is about what an operation preserves, so one row stating it, checkable with `capacity()` rather than only `len()`, is what is missing |
| RG24 | `ring_registry` | n/a — observation | R3 is the accounting invariant — "`len()` equals the number of live names, which equals the number of owned rings" — and E4 explains it cannot drift because `len()` forwards to the map with no second counter, both true, and there is a second *quantity* that is not a counter: after eight registrations and eight removals `len()` is 0, `is_empty()` is `true`, and the map still holds a **fourteen-slot table**, because `remove` releases the ring (measured: zero allocations, ownership transferred) and never releases the storage, while all eight methods report counts, names or rings and **zero** name capacity, reserve or shrink; the scale is small and it is not a leak, since the memory is reachable, accounted and freed on drop — what it is, is a fourth number in a crate whose central document is about accounting, present in no invariant and reportable by no method, so a long-lived registry cycling short-lived names grows a table it never shrinks while `is_empty()` keeps saying `true`; R3 should say the count accounting is exact and the storage accounting is unstated, and whether to expose `shrink_to_fit` is a separate question with a real answer either way |
