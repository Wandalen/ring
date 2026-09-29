# data_structure

`ring_batch` defines one structure and it is sixteen bytes: a `Seq` and a
`usize`, `Copy`, with no pointer, no allocation, and no destructor. It holds no
storage and takes no storage dependency, which is stated in the module comment
and confirmed by the manifest — one of the few boundaries in this family that a
reader can check without opening any source at all.

The second instance is about what happens one tier up. The same two fields, the
same eight method names, and one byte-identical body appear again in
`ring_claim`, in a crate that has no dependency edge to this one in either
direction and whose version is the one both ring implementations actually use.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_sixteen_bytes_that_are_not_a_buffer.md) | Sixteen Bytes That Are Not a Buffer | The layout, the missing niche, and the storage dependency the crate declines |
| [002](002_the_struct_ring_claim_wrote_again.md) | The Struct `ring_claim` Wrote Again | The same structure defined twice, and which of the two the family reaches |

## A Range Is Not a Container

The structure describes which sequences a caller owns and says nothing about
what goes in them. That is why it needs no capacity, no ring identity, and no
lifetime — a `BatchClaim` is meaningful before any ring exists, and ten of the
crate's 21 tests build one without touching a cursor.

The cost is symmetrical: because a claim carries no ring identity, nothing can
detect a claim being folded against the wrong ring's capacity. The boundary buys
independence and pays for it in the one check it makes impossible.

## Duplicated Upward, Not Downward

The usual reading of two identical types is that the lower one is the real one
and the upper one is redundant. Here it is the reverse. `ring_claim` sits at
Tier 5, has two dependents, and reaches `ring_mpsc` and `ring_publish`;
`ring_batch` sits at Tier 2, has one dependent, and reaches a thread-local
staging buffer.

The upper crate could not depend on the lower one and still gate against a
`GatingSet`, because that would invert the tier order — so it rebuilt the range
object rather than restructure the family. Nothing in either crate records that
the decision was made.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the one structure, and everything the crate depends on --'
command grep -m1 -A5 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]' ring_batch/src/lib.rs
command grep -E '^ring_' ring_batch/Cargo.toml
echo '  -- the same structure, one tier up --'
command grep -m1 -B2 -A2 -F '  start : Seq,' ring_claim/src/lib.rs
command grep -oE '^\s*pub (const )?fn [a-z_]+' ring_claim/src/lib.rs | head -8 | tr -s ' ' | tr '\n' ' '; echo
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA9 | `ring_batch` | n/a — observation | 16 bytes, 8-aligned, `Copy`, no niche — `Option< BatchClaim >` costs 24, because the empty claim the crate depends on uses the encoding a niche would need |
| BA10 | `ring_batch` | n/a — observation | The crate takes no storage dependency, says so in the module comment, and the manifest agrees; the cost is that a claim folded against a foreign capacity is undetectable |
| BA11 | `ring_claim` | n/a — duplication | `Claim` and `BatchClaim` differ in one field name; their `sequences` bodies are character-identical and all eight shared operations return the same results |
| BA12 | `ring_claim` | n/a — observation | The Tier 5 duplicate is the one with two dependents and both ring implementations behind it; the Tier 2 original reaches one leaf crate |
