# Pattern: A Named Outcome Instead of a Boolean

### Scope

**Purpose:** Record the crate's stated design pattern — an enum in place of a
boolean — against how the type is actually declared and consumed.

**Responsibility:** The module comment's argument, the two boolean readings the
crate ships alongside the enum, and what the one consumer does with the named
outcome.

**In Scope:** `ring_overflow/src/lib.rs:15-16`, `:111`, `:138`;
`ring_core/src/lib.rs:411-414`.

**Out of Scope:** The truth table the two readings form is
[`type/002`](../type/002_three_variants_and_two_questions.md). The pure/effectful
split is [`pattern/001`](001_the_pure_effectful_pair.md).

---

## The Argument, and What Sits Beside It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the module comment says the type is for --'
command grep -m1 -A1 -F '//! kept the item. [`Resolution`] below is the type that makes the alternative' ring_overflow/src/lib.rs
echo '  -- the boolean it replaces, and the two it ships instead --'
sed -n '/^  pub const fn lost_an_item( self ) -> bool$/p;/^  pub const fn accepted_incoming( self ) -> bool$/p' ring_overflow/src/lib.rs
echo '  -- how the one consumer reads the named outcome --'
command grep -m1 -A3 -F '      Err( record ) => match would_resolve( self.overflow )' ring_core/src/lib.rs
```

Live output:

```
  -- what the module comment says the type is for --
//! kept the item. [`Resolution`] below is the type that makes the alternative
//! outcomes explicit rather than leaving them to a boolean.
  -- the boolean it replaces, and the two it ships instead --
  pub const fn lost_an_item( self ) -> bool
  pub const fn accepted_incoming( self ) -> bool
  -- how the one consumer reads the named outcome --
      Err( record ) => match would_resolve( self.overflow )
      {
        Resolution::DroppedIncoming => Ok( () ),
        Resolution::EvictedOldest | Resolution::Refused => Err( record ),
```

---

### OV43 — The Type That Replaces a Boolean Ships Two Booleans

The module comment is explicit: `Resolution` "makes the alternative outcomes
explicit rather than leaving them to a boolean." The crate then declares two
`pub const fn ... -> bool` on that type.

Both are correct and neither contradicts the argument, but the shape is worth
stating plainly: the crate replaced one boolean with an enum, and then added two
booleans as the way to read it.

**Finding.** The two are not the boolean that was rejected. That one would have
answered "did the publish succeed" — a single bit conflating `DroppedIncoming` and
`Refused`, which are the two outcomes a caller most needs to tell apart
([`type/002`](../type/002_three_variants_and_two_questions.md) § OV36). The two
shipped booleans are a different, finer pair: jointly injective, so they lose
nothing.

That is the actual defensible claim, and it is stronger than the one the module
comment makes. "Named outcomes beat a boolean" invites the obvious reply that two
booleans were added anyway; "three variants is the smallest encoding of two
questions with one answer forbidden" does not. The crate has the better argument
available in its own source and states the weaker one.

---

### OV44 — The One Consumer Spends the Named Outcome as the Boolean It Was Meant to Replace

`ring_core` matches `would_resolve( self.overflow )`, names
`Resolution::DroppedIncoming`, groups the other two in a second arm, and returns
`Result< (), T >`.

Three named outcomes go in; one bit comes out. The value the crate built to avoid
a boolean is consumed by immediately reducing it to one.

**Finding.** The reduction is correct for this caller. `ring_core` needs to know
whether to swallow the failure or hand the record back, which is genuinely one
bit, and grouping the second arm is safe because both variants in it mean "give
the record back"
([`algorithm/002`](../algorithm/002_where_the_third_outcome_goes.md)).

What it means for the pattern is that the type's value here is entirely in the
writing, not the reading. The enum makes the *implementation* of the mapping
explicit and checkable — a fourth policy breaks both `match` sites — and gives the
consumer's own `match` a name to bind rather than a `true` to interpret. It buys
nothing at the consumer's output, which is a bit either way.

That is a real benefit and a smaller one than "makes the alternative outcomes
explicit" suggests. Recording it because the crate has exactly one consumer, so
the pattern's payoff is measurable rather than hypothetical, and the measurement
is: one named variant bound, two discarded, one bit produced.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/002`](../type/002_three_variants_and_two_questions.md) | The encoding argument the comment could have made |
| [`algorithm/002`](../algorithm/002_where_the_third_outcome_goes.md) | The consumer's match, and why grouping two variants is safe |
| [`pattern/001`](001_the_pure_effectful_pair.md) | The crate's other pattern |
| [`api/002`](../api/002_two_predicates_and_no_caller.md) | Who calls the two booleans |

### Sources

| Fact | Where |
|------|-------|
| The stated design argument | `ring_overflow/src/lib.rs:15-16` |
| Both boolean readings | `ring_overflow/src/lib.rs:111`, `:138` |
| The consumer's reduction | `ring_core/src/lib.rs:411-414` |
| The injectivity that makes the pair lossless | `ring_overflow/src/lib.rs:113-117`, `:140-144` |

### Tests

| Test | Covers |
|------|--------|
| `the_two_readings_partition_the_outcomes` | That the two booleans are jointly complete |
| `resolution_has_exactly_three_variants_and_no_overwrite` | The naming the pattern buys |
| `no_resolution_overwrites_unread_data_silently` | The constraint a single boolean could not carry |
