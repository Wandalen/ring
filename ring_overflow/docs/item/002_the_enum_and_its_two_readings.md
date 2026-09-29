# Item: The Enum and Its Two Readings

### Scope

**Purpose:** Record `Resolution` as an item — one enum, one `impl` block, two
methods — and what the two methods' patterns cover between them.

**Responsibility:** The type's whole inherent surface, both `match` bodies, and
the variants each arm names.

**In Scope:** `pub enum Resolution`, `impl Resolution`, and the bodies of
`Resolution::lost_an_item` and `Resolution::accepted_incoming`, all in
`ring_overflow/src/lib.rs`.

**Out of Scope:** The two free functions are
[`item/001`](001_the_two_free_functions_one_statement_apart.md). Who calls the
predicates is [`api/002`](../api/002_two_predicates_and_no_caller.md).

---

## Everything the Type Offers

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the type, and every method on it --'
command grep -o 'pub enum Resolution\|^impl Resolution\|  pub const fn [a-z_]*' ring_overflow/src/lib.rs
echo '  -- both bodies --'
sed -n '/^  pub const fn lost_an_item( self ) -> bool$/,/^  }$/p;/^  pub const fn accepted_incoming( self ) -> bool$/,/^  }$/p' ring_overflow/src/lib.rs
echo '  -- how each body is spelled --'
sed -n '/^  pub const fn lost_an_item( self ) -> bool$/,/^  }$/p;/^  pub const fn accepted_incoming( self ) -> bool$/,/^  }$/p' ring_overflow/src/lib.rs | command grep -cE '^    match self$|matches!\(' | sed 's/^/    match-or-matches! arms: /'
echo '  -- and the variants each one names --'
sed -n '/^  pub const fn lost_an_item( self ) -> bool$/,/^  }$/p' ring_overflow/src/lib.rs | command grep -oE 'Self::[A-Za-z]+ [|] Self::[A-Za-z]+ => true|Self::[A-Za-z]+ => true' | sed 's/^/    lost_an_item true on:      /'
sed -n '/^  pub const fn accepted_incoming( self ) -> bool$/,/^  }$/p' ring_overflow/src/lib.rs | command grep -oE 'Self::[A-Za-z]+ [|] Self::[A-Za-z]+ => true|Self::[A-Za-z]+ => true' | sed 's/^/    accepted_incoming true on: /'
```

Live output:

```
  -- the type, and every method on it --
pub enum Resolution
impl Resolution
  pub const fn lost_an_item
  pub const fn accepted_incoming
  -- both bodies --
  pub const fn lost_an_item( self ) -> bool
  {
    match self
    {
      Self::DroppedIncoming | Self::EvictedOldest => true,
      Self::Refused => false,
    }
  }
  pub const fn accepted_incoming( self ) -> bool
  {
    match self
    {
      Self::EvictedOldest => true,
      Self::DroppedIncoming | Self::Refused => false,
    }
  }
  -- how each body is spelled --
    match-or-matches! arms: 2
  -- and the variants each one names --
    lost_an_item true on:      Self::DroppedIncoming | Self::EvictedOldest => true
    accepted_incoming true on: Self::EvictedOldest => true
```

---

### OV27 — One `impl` Block, Two Methods, Both Taking `self` by Value

The type's entire inherent surface is two `pub const fn` taking `self` — not
`&self` — which is right for a one-byte `Copy` enum and unusual enough in a
codebase to be worth stating: a caller can read a resolution twice without
borrowing it, and `Resolution::lost_an_item` works as a function reference, which
is exactly how `policy_self_description_agrees_with_the_handler` in
`tests/overflow_test.rs` uses it inside `is_ok_and`.

Both bodies are an exhaustive `match` over all three variants. There is no
`Display`, no `as_str`, no conversion, and no constructor.

**Finding.** The absent `Display` is the one worth noting, because a test needed
it and wrote it locally: `resolution_has_exactly_three_variants_and_no_overwrite`
defines a private `name()` returning `"dropped_incoming"`, `"evicted_oldest"`,
`"refused"`. So a string form of the type exists, is exercised, and lives in the
test file.

That placement is deliberate in effect if not in intent — it is what makes the
test a structural guard, since an exhaustive `match` in the test file is what stops
compiling when a fourth variant appears
([`invariant/001`](../invariant/001_the_mapping_is_total_and_injective.md) § OV22).
Moving `name()` into the crate as a `Display` impl or an `as_str` would give
callers the strings and keep the guard, since the match would still be exhaustive.
Nothing records that the test is currently doing double duty.

---

### OV28 — Three Variants, and Every Variant Now Answered Explicitly

Between them the two predicates answer for all three variants twice over.
`lost_an_item` is `true` for `DroppedIncoming | EvictedOldest` and `false` for
`Refused`; `accepted_incoming` is `true` for `EvictedOldest` and `false` for
`DroppedIncoming | Refused`. `Refused` is named in both.

That correctness is now stated rather than inherited. `a_refusal_loses_nothing`
asserts the behaviour, and the shape of the code asserts it a second way: a
variant that is not listed does not compile.

**Finding, and what closed it.** Both readings used to be positive `matches!`
patterns, so the answer for any variant not named was `false` by construction
rather than by decision — right for the one unnamed variant at the time, and the
mechanism by which a fourth variant would have inherited `Refused`'s profile
silently ([`invariant/001`](../invariant/001_the_mapping_is_total_and_injective.md)
§ OV22). The alternative this file weighed — an exhaustive `match` per predicate,
three arms each, returning `true` or `false` explicitly — is what the crate now
does, at both readings rather than only in the test helper.

The asymmetry the finding recorded is gone with it: the two free functions and
the two predicates all use exhaustive `match`, for the same reason, and
`lost_an_item`'s own doc comment now states that reason at the site — a fourth
variant would read `false` here and `false` in `accepted_incoming`, which is
`Refused`'s profile exactly, *"by a predicate that never mentioned it. Spelled
this way it stops compiling instead, which is the whole point of a closed enum."*

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/001`](001_the_two_free_functions_one_statement_apart.md) | The two functions producing this type |
| [`invariant/001`](../invariant/001_the_mapping_is_total_and_injective.md) | What the positive patterns leave unguarded |
| [`type/001`](../type/001_six_derives_on_a_fieldless_enum.md) | What the derive list adds to this surface |
| [`api/002`](../api/002_two_predicates_and_no_caller.md) | Every call of both methods |

### Sources

| Fact | Where |
|------|-------|
| The enum and its `impl` block | `pub enum Resolution`, `impl Resolution` in `ring_overflow/src/lib.rs` |
| `lost_an_item`, whole | `Resolution::lost_an_item` in `ring_overflow/src/lib.rs` |
| `accepted_incoming`, whole | `Resolution::accepted_incoming` in `ring_overflow/src/lib.rs` |
| The local `name()` a test defines | inside `resolution_has_exactly_three_variants_and_no_overwrite` in `ring_overflow/tests/overflow_test.rs` |
| The function-reference use | inside `policy_self_description_agrees_with_the_handler` in `ring_overflow/tests/overflow_test.rs` |

### Tests

| Test | Covers |
|------|--------|
| `the_two_readings_partition_the_outcomes` | All six variant/predicate pairs |
| `a_refusal_loses_nothing` | The variant neither pattern names |
| `resolution_has_exactly_three_variants_and_no_overwrite` | The local `name()`, and the structural guard |
| `a_resolution_is_a_plain_comparable_value` | That `self`-by-value is sound here |
