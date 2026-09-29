# Data Structure: The Struct `ring_claim` Wrote Again

### Scope

**Purpose:** Record that the family defines this structure twice — same two
fields, same eight method names, one byte-identical body — in two crates with no
dependency edge between them in either direction.

**Responsibility:** `ring_batch::BatchClaim` against `ring_claim::Claim`: their
fields, their surfaces, their measured layouts, and their reach.

**In Scope:** `ring_batch/src/lib.rs:56-189`;
`ring_claim/src/lib.rs:96-235`.

**Out of Scope:** The layout on its own is
[`data_structure/001`](001_sixteen_bytes_that_are_not_a_buffer.md). The shared
shape read as a family pattern is
[`pattern/001`](../pattern/001_the_range_object.md).

---

## The Same Structure, Twice

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two structs --'
command grep -m1 -A4 -F 'pub struct BatchClaim' ring_batch/src/lib.rs
command grep -m1 -B2 -A2 -F '  start : Seq,' ring_claim/src/lib.rs
echo '  -- the two method sets, in file order --'
command grep -oE '^\s*pub (const )?fn [a-z_]+' ring_batch/src/lib.rs | head -8 | tr -s ' ' | tr '\n' ' '; echo
command grep -oE '^\s*pub (const )?fn [a-z_]+' ring_claim/src/lib.rs | head -8 | tr -s ' ' | tr '\n' ' '; echo
echo '  -- the two sequences() bodies --'
command grep -m1 -A3 -F '  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >' ring_batch/src/lib.rs
command grep -m1 -A3 -F '  pub fn sequences( self ) -> impl Iterator< Item = Seq >' ring_claim/src/lib.rs
echo "  -- ring_claim in ring_batch's manifest: $( command grep -c 'ring_claim' ring_batch/Cargo.toml || true ) --"
echo "  -- ring_batch in ring_claim's manifest: $( command grep -c 'ring_batch' ring_claim/Cargo.toml || true ) --"
```

Live output:

```
  -- the two structs --
pub struct BatchClaim
{
  start : Seq,
  count : usize,
}
pub struct Claim
{
  start : Seq,
  len : usize,
}
  -- the two method sets, in file order --
 pub const fn new  pub const fn start  pub const fn len  pub const fn is_empty  pub const fn end  pub const fn contains  pub fn sequences  pub const fn overlaps 
 pub const fn new  pub const fn start  pub const fn end  pub const fn len  pub const fn is_empty  pub const fn contains  pub fn sequences  pub const fn overlaps 
  -- the two sequences() bodies --
  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >
  {
    ( self.start.0..self.end().0 ).map( Seq )
  }
  pub fn sequences( self ) -> impl Iterator< Item = Seq >
  {
    ( self.start.0..self.end().0 ).map( Seq )
  }
  -- ring_claim in ring_batch's manifest: 0 --
  -- ring_batch in ring_claim's manifest: 0 --
```

---

### BA11 — Two Fields, Eight Method Names, One Identical Body

The structs differ in exactly one identifier: `count` against `len`. The method
sets are the same eight names in a different declaration order. `sequences`'
body is character-for-character the same expression in both files.

Behaviourally they are interchangeable, measured:

```
--- the same range, built by each, yields the same sequences ---
  BatchClaim::new( Seq( 5 ), 4 ).sequences() : [5, 6, 7, 8]
  Claim::new(      Seq( 5 ), 4 ).sequences() : [5, 6, 7, 8]
  identical                                  : true
  contains / end / is_empty / overlaps agree : true
```

**Finding.** Two 16-byte, 8-aligned, `Copy` range objects over `Seq`, with the
same eight operations and agreeing results on every one of them. Neither
manifest names the other, so this is not a wrapper, a re-export, or a
specialisation — it is the same type written twice by two crates that cannot see
each other.

The differences that do exist are all receiver-level rather than semantic:
`BatchClaim` takes `&self` and is `const` on seven of eight; `Claim` takes
`self` and is `const` on five of eight. That one choice is what forced `use< >`
onto `BatchClaim::sequences` and left `Claim::sequences` without it — see
[`workaround/001`](../workaround/001_the_capture_bound_edition_2024_made_necessary.md).

---

### BA12 — The Duplicate Is Not the Unused One

The reflex reading is that one of the two is dead weight. It is not the one this
crate defines that is less reachable — it is more:

| | `ring_batch::BatchClaim` | `ring_claim::Claim` |
|---|---|---|
| manifest dependents | `ring_tls` | `ring_mpsc`, `ring_publish` |
| tier | 2 | 5 |
| produced by | `claim`, `claim_gated` | `Claimer::claim`, `Claimer::claim_up_to` |
| gating | a free function taking two cursors | a `Claimer` holding a `GatingSet` |

**Finding.** `ring_claim` is Tier 5, has two dependents, and reaches the family's
two ring implementations. `ring_batch` is Tier 2, has one dependent, and reaches
a thread-local staging buffer. The higher tier did not build on the lower one; it
rebuilt it, and its version is the one the rings use.

That inverts what the tier ordering implies. A Tier 2 crate exists to be a
substrate for the tiers above it, and this one is a substrate for a leaf. The
sixteen bytes are duplicated because the crate that needed them at Tier 5 needed
a *gate* that reads a `GatingSet` — which `ring_batch` cannot depend on without
inverting the tier order — and rewriting the range object was cheaper than
restructuring the family to share it.

Nothing records that this happened. Neither crate's documentation mentions the
other's type; the only reason the pair is visible at all is that both spell
`sequences` the same way.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/001`](001_sixteen_bytes_that_are_not_a_buffer.md) | The layout both types share |
| [`pattern/001`](../pattern/001_the_range_object.md) | The shape read as a family pattern rather than a duplication |
| [`workaround/001`](../workaround/001_the_capture_bound_edition_2024_made_necessary.md) | The one difference the receiver choice actually forced |
| [`api/002`](../api/002_two_claim_functions_one_caller.md) | The reach numbers in the table above |

### Sources

| Fact | Where |
|------|-------|
| `BatchClaim` and its methods | `ring_batch/src/lib.rs:56-189` |
| `Claim` and its methods | `ring_claim/src/lib.rs:96-235` |
| Neither manifest names the other | `ring_batch/Cargo.toml`, `ring_claim/Cargo.toml` |
| Interchangeable results | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `overlap_is_exactly_range_intersection` | `BatchClaim::overlaps`, in this crate |
| `an_empty_claim_overlaps_nothing_even_inside_another` | The empty case, in this crate |
| *(to create)* | Nothing asserts the two types agree; the probe above is the only comparison that exists |
