# data_structure

`ring_index` defines no data structure. That fact is the definition's subject
rather than a reason to leave it empty, because the crate is what makes a data
structure elsewhere circular — and it does so without any structure of its own
existing at any point.

Two instances follow from that. The first records where the ring's topology
actually lives, given that no struct in the family holds a head, a tail, or a
wrap marker. The second describes the one owned collection the crate can produce,
which is `run`'s `Vec` — measured as a layout rather than argued about as an API.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_ring_is_a_computation_not_a_layout.md) | The Ring Is a Computation, Not a Layout | `Buffer`'s two flat fields, and the absence of `SlotIndex` from every struct in the family |
| [002](002_the_one_collection_the_crate_builds.md) | The One Collection the Crate Builds | `run`'s `Vec`: exactly sized by accident of body shape, and mostly header at the sizes anyone would ask for |

## Sequences Are Stored; Slots Are Not

The family stores sequence numbers everywhere — cursors, stamps, `AtomicSeq`
fields — because a sequence is state that outlives the expression producing it. It
stores slot indices nowhere. Zero fields of type `SlotIndex`,
`Option< SlotIndex >`, or `Vec< SlotIndex >` exist in any of the 33 crates.

A slot is a *view* of a sequence through a capacity, both of which are already
held, and reconstructing that view costs one `and` instruction. So every
`SlotIndex` that has ever existed at runtime was a temporary between `of` and a
subscript — which is also why the type's `pub` field has never caused a problem
despite enforcing nothing.

## The Topology Cannot Go Stale and Can Be Duplicated

Because the circularity is a function rather than a field, it has the properties
of a function. It cannot disagree with the data, the way a classical head/tail
pair can. And it can be written twice, the way a field cannot — a second
`& mask` anywhere is a second ring over the same array, and no compiler notices.

The family has exactly one instance of each: no staleness bug anywhere, and one
duplicate fold in `ring_mpsc`.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the storage, whole --'
command grep -m1 -A4 -F 'pub struct Buffer< S >' ring_store/src/lib.rs
echo '  -- SlotIndex as a struct field, family-wide --'
command grep -rnE '^\s*[a-z_]+ *: *(SlotIndex|Option< SlotIndex|Vec< SlotIndex)' --include=*.rs */ || echo '  (none)'
echo '  -- everything ring_index owns --'
command grep -nE 'Vec|Box|String|to_vec|collect|to_owned' ring_index/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| IX49 | `ring_store` | n/a — observation | `Buffer` holds a flat `Box< [ S ] >` and a `Capacity` — no head, tail, or wrap marker; the ring topology is recomputed at every access and stored nowhere |
| IX50 | `ring_types` | n/a — observation | `SlotIndex` is never a struct field in any of the 33 crates; every one that exists is a temporary between `of` and a subscript |
| IX51 | `ring_index` | n/a — doc gap | `run`'s `Vec` is exactly sized because the body collects an `ExactSizeIterator`; the property is undocumented and no test would notice a rewrite that lost it |
| IX52 | `ring_index` | **measured cost** | A `Vec< SlotIndex >` is 24 bytes of header per 8 bytes of element; at the batch sizes the family actually uses, over a quarter of the structure is bookkeeping |
