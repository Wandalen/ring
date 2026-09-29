# data_structure

The crate declares one data structure and zero fields. `Resolution` is three
fieldless variants occupying a single byte, which is the cheapest a named outcome
can be — and that cheapness is the whole argument for its existence, since it
replaces a `bool` at a `bool`'s price.

What the two instances here record is the gap between that byte and the shape it
is delivered in. `would_resolve` hands the byte back directly. `resolve` wraps it
in `Result< Resolution, RingError >`, which measures twenty-four bytes with
alignment eight, because the family's shared error enum carries two `usize`
payloads for cases this crate never returns.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_one_byte_and_two_hundred_fifty_three_spare_niches.md) | One Byte, and 253 Spare Niches | The declaration, its measured layout, and the free niche nothing uses |
| [002](002_a_one_byte_outcome_in_a_twenty_four_byte_result.md) | A One-Byte Outcome in a 24-Byte `Result` | What the error half costs, and where exhaustiveness stops |

## Two Enums That Look Interchangeable and Are Not

`OverflowPolicy` and `Resolution` are both three fieldless variants in one byte,
declared eight lines apart in two crates, ordered the same way. Neither carries a
`#[ repr ]` or an assigned discriminant, so the correspondence between position
zero and position zero is maintained by hand and by nothing else.

The one derive that differs is the right one: `OverflowPolicy` derives `Default`
and marks `DropNewest`, because a ring built without a stated policy needs one.
`Resolution` does not, because there is no default outcome — every resolution is
caused by an event.

## The Cheap Type and the Expensive Envelope

`Resolution` costs one byte and leaves 253 discriminants spare, one of which the
compiler spends to make `Option< Resolution >` free. `Result< Resolution, () >`
is likewise one byte. Neither shape appears anywhere in the crate or its consumer.

The shape that does appear is twenty-four bytes, and it is `RingError`'s doing —
nine variants, two of them carrying payloads, `#[ non_exhaustive ]`. The last
attribute is the one with teeth: a caller matching `resolve`'s error must write a
`_` arm forever, so the exhaustiveness this crate guarantees inside both its own
`match` blocks does not survive the return.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every type this crate declares --'
command grep -n 'pub enum \|pub struct ' ring_overflow/src/lib.rs
echo '  -- every field it declares --'
command grep -c '  [a-z_]* : ' ring_overflow/src/lib.rs || true
echo '  -- and the error enum it borrows for its Result --'
command grep -n 'non_exhaustive' ring_types/src/error.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV9 | `ring_overflow` | n/a — observation | `Resolution` is three fieldless variants in one byte with alignment one and the crate declares zero struct fields anywhere, leaving 253 spare discriminants of which the compiler spends one to make `Option< Resolution >` cost the same single byte — a free niche that no code in the crate or its consumer uses, and that no document records as available |
| OV10 | `ring_overflow` | **latent hazard** | `OverflowPolicy` and `Resolution` are both three fieldless one-byte variants declared in the same order eight lines apart in two crates, with no `#[ repr ]` and no assigned discriminants on either, so the positional correspondence a future author could read as licence to transmute or `as`-cast between them is maintained only by the two hand-written `match` blocks — and while a fourth *policy* breaks both at compile time, `Resolution` has no `ALL` constant or length assertion, so a fourth *resolution* compiled against everything except those matches; `Resolution::ALL` now exists and is length-asserted, while the discriminants stay deliberately unpinned |
| OV11 | `ring_overflow` | **measured cost** | `would_resolve` returns one byte and `resolve` returns twenty-four for the same three outcomes, a measured 24× — because `Result< Resolution, RingError >` inherits the family error enum's two `usize` payloads to express one fieldless outcome, `RingError::Full`, that carries nothing; `Result< Resolution, () >` measures one byte in the same run, and the alignment rise from one to eight is what actually forces the return out of a single byte |
| OV12 | `ring_overflow` | n/a — observation | `RingError` is `#[ non_exhaustive ]`, so every caller outside `ring_types` matching `resolve`'s error must carry a permanent `_` arm — leaving the return exhaustive on the `Ok` side, where two of three outcomes live, and structurally inexhaustible on the `Err` side, where the third does; `would_resolve`'s plain three-variant enum keeps full exhaustiveness, so the half the crate documents as the convenience form is the half with the stronger type |
