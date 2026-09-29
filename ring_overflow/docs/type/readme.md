# type

One public type: a fieldless three-variant enum with six derives and two inherent
readings. The instances here take it as a type — what the derives commit to, what
the variants claim, and what the two predicates jointly encode.

The result is sharper than the crate's own framing. `Resolution` is described as
replacing a boolean with named outcomes; what it actually encodes is two booleans
with one combination forbidden, and three variants is the smallest closed form of
that.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_six_derives_on_a_fieldless_enum.md) | Six Derives on a Fieldless Enum | Each derive, where it is exercised, and the absent seventh |
| [002](002_three_variants_and_two_questions.md) | Three Variants and Two Questions | The truth table, and the pair with no variant |

## Six Derives, All Exercised, None by the Consumer

`Debug`, `Clone`, `Copy`, `PartialEq`, `Eq`, `Hash` — every one is used, and one
test uses five of them in eleven lines, including `Eq + Hash` as a `HashMap` key
bound. Outside the crate the type appears twice, in an import and a `match` arm,
neither of which needs a derive at all.

The absent seventh is the interesting one. `OverflowPolicy` derives `Default` with
`#[ default ]` on `DropNewest`; `Resolution` does not, because no outcome is
uncaused and a default would be a value nothing produced. That is the crate's
sharpest type-level decision, guarded by nothing — adding `Default` compiles and
passes the whole suite.

## Two Questions, Four Answers, Three Variants

`lost_an_item` and `accepted_incoming` are jointly injective: their answer pair
recovers the variant exactly. Three of the four pairs are occupied, and the fourth
— lost nothing, accepted the incoming item — describes a state a full ring cannot
be in.

That impossibility is what `no_resolution_overwrites_unread_data_silently` asserts.
The named type is what makes the constraint statable; a single boolean could not
carry it, and a fourth variant would have to violate it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the type, its derives, and its variants --'
command grep -m1 -A10 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]' ring_overflow/src/lib.rs | command grep -v '^  ///'
echo '  -- and the two matches that read them --'
sed -n '/^  pub const fn lost_an_item( self ) -> bool$/,/^  }$/p;/^  pub const fn accepted_incoming( self ) -> bool$/,/^  }$/p' ring_overflow/src/lib.rs | command grep -E '=> true,|=> false,'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| OV33 | `ring_overflow` | n/a — observation | All six derives are exercised and all six only by this crate's own suite — `a_resolution_is_a_plain_comparable_value` uses five in eleven lines, including `Eq + Hash` as a `HashMap` key bound — while the consumer's two mentions are an import and a `match` arm, neither of which needs any derive, so the capabilities are inferred from what such a value should support rather than from any caller's request |
| OV34 | `ring_overflow` | **latent hazard** | `OverflowPolicy` derives `Default` with `#[ default ]` on `DropNewest` and `Resolution` deliberately does not, since every outcome is caused and a default would be a value nothing produced — but the two declarations sit eight lines apart in two crates differing by one token, no comment or test guarded the omission, and adding `Default` in a good-faith normalisation compiled and passed the entire suite; a comment now sits directly above the derive list, where that edit would be typed |
| OV35 | `ring_overflow` | n/a — doc gap | The two predicates are jointly injective, so `Resolution` encodes two booleans with one of the four answer pairs — lost nothing, accepted the incoming item — forbidden as physically impossible for a full ring, which is exactly what `no_resolution_overwrites_unread_data_silently` asserts; three variants is therefore the smallest closed encoding of two questions minus one answer, and that reasoning appears in no doc comment |
| OV36 | `ring_overflow` | n/a — doc gap | `DroppedIncoming` and `Refused` leave the ring in the same state and differ only in who holds the item afterwards, making `lost_an_item` the crate's most load-bearing reading — not `accepted_incoming`, which selects the variant no production build can reach — and neither variant's doc comment says the two are indistinguishable from the ring's side and opposite from the caller's |
