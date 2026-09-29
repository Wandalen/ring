# type

Two type-level decisions, one on each half of the API. `claim_gated` takes its
producer and consumer as two unrelated references with two unrelated type
parameters, so the same cell can be passed for both — and when it is, the gate
reports the ring completely empty at every position and grants every request
forever. `sequences` and `drain_order` return `impl Iterator`, which hides a
concrete type no caller should be naming and costs exactly one capability plus
one thing nobody expected.

Both are recorded here rather than as defects because both are correct at this
tier and wrong only in what they leave unsaid. The crate has no storage
dependency by design, so two loose cursors is what remains; the fold is
`const`-adjacent arithmetic that should not be frozen into a public type name.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_ring_that_can_gate_against_itself.md) | The Ring That Can Gate Against Itself | Two unrelated cursor parameters, the aliased call, and how the rest of the family avoided it |
| [002](002_the_iterator_nobody_can_name.md) | The Iterator Nobody Can Name | What `impl Iterator` erases, what survives it, and what the one dependant paid |

## The Type System Cannot Say "Two Ends of One Ring"

`free_slots( at, at, capacity )` is `capacity` — always, because
`distance_to( x, x )` is zero and an empty ring has all its slots free. That is
correct arithmetic on an input the arithmetic was never meant to receive, and
neither `claim_gated` nor `free_slots` can tell the difference between two
distinct cursors that happen to be equal and one cursor handed over twice.

Every other crate in the family sidesteps it structurally: nine
`ring : &'a Ring< S >` fields across the two assemblies, `Claimer`'s owned cursor
beside a borrowed `GatingSet`, and `ring_debug`'s two distinct concrete types.
`claim_gated` is the only function in thirty-three crates that can be handed one
ring end twice, and the remedy available at this tier is a sentence, not a
signature.

## Erasure Costs Land Where the Value Is Stored

`rev()` is the one trait lost — `Range< u64 >` is double-ended and the bound
erases it. `size_hint` survives, exact, because it lives on `Iterator` itself;
`ExactSizeIterator::len` was never available, since `Range< u64 >` does not
implement it either.

The cost that actually landed is elsewhere. `ring_tls::Flush` — the family's only
consumer of this crate — holds a `BatchClaim` and a parallel `next : u64` and
hand-writes `Iterator`, because an opaque return type cannot be a struct field
without naming a type nobody wants to name or boxing into an allocation the
family avoids. It also carries the one `ExactSizeIterator` impl in all
thirty-three crates.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- two unrelated parameters --'
command grep -m1 -A4 -F 'pub fn claim_gated< P : SeqCell, C : SeqCell >' ring_batch/src/lib.rs
echo '  -- and the arithmetic that cannot notice --'
command grep -m1 -A1 -F '  let in_flight = consumer.distance_to( producer );' ring_seqno/src/lib.rs
command grep -m1 -F '/// assert_eq!( free_slots( Seq( 0 ), Seq( 0 ), cap ), 4 );' ring_seqno/src/lib.rs
echo '  -- the only other function taking both ends separately --'
command grep -rn -A1 'producer : &' --include=lib.rs ring_*/src/ | command grep -B1 'consumer : &' | sed 's|ring/||'
echo '  -- every impl Iterator return, and the one stronger trait --'
command grep -rn -e '-> impl Iterator' --include=lib.rs ring_*/src/ | sed 's|ring/||'
command grep -rn -e 'DoubleEndedIterator' -e 'ExactSizeIterator' --include=lib.rs ring_*/src/ | sed 's|ring/||'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA46 | `ring_batch` | **latent hazard** | `claim_gated( &cell, &cell, .. )` type-checks and grants every request — six of six on a ring of four — because `free_slots( at, at, cap )` is always the full capacity |
| BA47 | `ring_batch` | n/a — observation | The only function in 33 crates that can be handed one ring end twice; every other holds both ends through a single `Ring< S >` or `GatingSet`, and `ring_debug`'s two concrete types make the aliased call impossible |
| BA48 | `ring_batch` | n/a — observation | `impl Iterator` costs exactly `DoubleEndedIterator`; `size_hint` survives exact, and `ExactSizeIterator::len` was never available because `Range< u64 >` does not implement it |
| BA49 | `ring_tls` | n/a — duplication | The one dependant stores a `BatchClaim` beside a redundant `next : u64` and hand-writes `Iterator`, because the opaque return cannot be a struct field; it carries the family's only `ExactSizeIterator` impl |
