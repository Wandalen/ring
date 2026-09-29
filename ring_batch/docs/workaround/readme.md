# workaround

Two external constraints reach a 314-line crate that computes arithmetic on two
integers. The first is a language edition: Rust 2024 changed what a
return-position `impl Trait` captures, so both of the crate's iterator-returning
functions carry `+ use< >` — three such bounds exist across all thirty-three
crates and two of them are here. The second is the family's own type design:
`Seq` is a `u64` and `Capacity` and `count` are `usize`, so every comparison
between a position and a size crosses a width boundary, and the crate crosses it
three times.

Both absorptions are correct. Both are undocumented at the site, and one of the
three casts turns out to cross nothing at all.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_capture_bound_edition_2024_made_necessary.md) | The Capture Bound Edition 2024 Made Necessary | `+ use< >` proven load-bearing at both sites, edition-specific, and removable by a by-value receiver |
| [002](002_the_usize_u64_seam.md) | The `usize`/`u64` Seam | Three casts, two lossless widenings, and one that casts `usize` to `usize` |

## A Workaround Nobody Currently Needs

`+ use< >` exists so a caller can return `claim.sequences()` after the claim goes
out of scope. Stripped from the library, the library still compiles clean — the
error appears only at such a caller, and the identical source and caller pass
under edition 2021. Both sites were verified independently: keeping one bound and
dropping the other reproduces the failure at the other, so neither was copied
from its neighbour.

No such caller exists. `ring_tls` never calls either function, and the crate's own
suite consumes every iterator in the expression that produces it. The bound is
right, necessary for a reasonable shape, and protecting nobody yet — which is
exactly the state a `workaround/` entry is for.

## The Choice That Created It Sits Four Lines Away

`BatchClaim` is sixteen bytes and `Copy`, and both signatures take it by
reference. A reference is a lifetime, and a lifetime is what edition 2024
captures. Take the receiver and the parameter by value instead and both bounds
become unnecessary — verified, with the same caller passing.

`ring_claim::Claim` is the same two fields and the same eight methods three tiers
up, and already writes `pub fn sequences( self )`. Two crates built one range
type; one took `&self` and needed an edition workaround, the other took `self`
and did not, and neither records the difference.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every capture bound in the family --'
command grep -rn 'use<' --include=*.rs . | sed 's|ring/||'
echo '  -- the twin that needs none --'
command grep -m1 -F '  pub fn sequences( self ) -> impl Iterator< Item = Seq >' ring_claim/src/lib.rs
echo '  -- every cast in the crate body --'
command grep -n ' as u64\| as usize' ring_batch/src/lib.rs | command grep -v '///'
echo '  -- and what the third one is casting --'
command grep -m1 -F 'pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize' ring_seqno/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA50 | `ring_batch` | n/a — doc gap | `+ use< >` is edition-2024-specific and load-bearing at both sites — the same source and caller pass under 2021 — yet carries no comment at any of the family's three occurrences, and no caller needing it exists |
| BA51 | `ring_batch` | n/a — observation | Both bounds exist because a sixteen-byte `Copy` type is passed by reference; taking it by value removes both, which is what `ring_claim::Claim::sequences( self )` already does |
| BA52 | `ring_batch` | n/a — observation | The crate only ever widens `usize` → `u64`, which is why it has no fallible conversion and why `claim` can return a value rather than a `Result` |
| BA53 | `ring_batch` | n/a — observation | `free_slots( .. ) as usize` casts a `usize` to `usize`; it compiles away, clippy stays silent even with `unnecessary_cast` and `cast_lossless` forced on, and the parentheses make it read as a deliberate width fix |
