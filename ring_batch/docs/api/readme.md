# api

Twelve public items: one struct, its eight inherent methods, and three free
functions. Seven of the twelve carry `#[ must_use ]`, and the distribution is
the first instance below — the attribute lands on every item where forgetting
the result is free and misses the one where forgetting it cannot be undone.

The second instance asks who calls any of it. The answer is one crate, four
items, and one call site, which puts both remaining free functions — half of the
feature the crate exists to deliver — outside anything the family executes.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_twelve_items_seven_must_use.md) | Twelve Items, Seven `must_use` | The surface with its attributes, and the one call whose dropped result burns sequences |
| [002](002_two_claim_functions_one_caller.md) | Two Claim Functions, One Caller | One manifest edge, one import line, and a design decision that travelled without one |

## A Surface Split Three Ways by Consequence

The eight `BatchClaim` methods are pure reads of two fields. The three free
functions are not alike: `claim` mutates a shared cursor, `claim_gated` mutates
it conditionally, and `drain_order` mutates nothing and does not even iterate
until polled. Reading the surface as "a struct and three functions" hides that
split; reading it by what each item does to shared state produces exactly one
item — `claim` — whose misuse is silent and unrecoverable.

That is the item the attribute is missing from, and no test can express its
absence: a missing lint is not observable from inside the language.

## Reach Is Not Proportional to Deliverable

This crate's own contract is "Batch Claim And Batch Drain." The claim half has one caller. The
drain half has none, and neither does the gated form that the module comment
describes as the multi-producer path. Both are implemented, both are tested, and
both are unreachable from any code that runs outside this crate's own suite.

The consequence runs both ways. It is why the window recorded in `pitfall/001`
has never cost the family anything, and it is why nothing would have noticed if
it had.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the surface, with attributes --'
command grep -E '^\s*(#\[ must_use \]|pub (const )?fn |pub struct )' ring_batch/src/lib.rs
echo '  -- everything outside the crate that reaches it --'
command grep -rl '^ring_batch = ' --include=Cargo.toml .
# `-E`: without it the alternation is literal, so this matched the string
# `use ring_batch|claim_gated|drain_order` and reported an empty reach for a
# crate that does have external mentions
command grep -rE 'use ring_batch|claim_gated|drain_order' --include=*.rs . | command grep -v '^ring_batch/'
```

Live output:

```
  -- the surface, with attributes --
pub struct BatchClaim
  #[ must_use ]
  pub const fn new( start : Seq, count : usize ) -> Self
  #[ must_use ]
  pub const fn start( &self ) -> Seq
  #[ must_use ]
  pub const fn len( &self ) -> usize
  #[ must_use ]
  pub const fn is_empty( &self ) -> bool
  #[ must_use ]
  pub const fn end( &self ) -> Seq
  #[ must_use ]
  pub const fn contains( &self, seq : Seq ) -> bool
  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >
  #[ must_use ]
  pub const fn overlaps( &self, other : &Self ) -> bool
#[ must_use ]
pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim
pub fn claim_gated< P : SeqCell, C : SeqCell >
pub fn drain_order( claim : &BatchClaim, capacity : Capacity )
  -- everything outside the crate that reaches it --
ring_tls/Cargo.toml
ring_tls/tests/tls_test.rs:use ring_batch::BatchClaim;
ring_tls/src/lib.rs:use ring_batch::{ claim, BatchClaim };
ring_cursor/src/lib.rs://! rather than take a parameter — the same choice `ring_batch::claim_gated`
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA5 | `ring_batch` | n/a — observation | `#[ must_use ]` is on all seven pure `const` accessors, where dropping the result does nothing, and on none of the three free functions |
| BA6 | `ring_batch` | **latent hazard** | `claim( &cursor, 8, order );` as a bare statement compiles clean under `-D warnings`, advances the cursor, and orphans eight sequences no consumer will ever be released from |
| BA7 | `ring_batch` | n/a — coverage | One manifest dependent reaching four of twelve items; `claim_gated` and `drain_order` — half of this crate's own stated deliverable — have no caller in the 33 crates |
| BA8 | `ring_cursor` | n/a — observation | The only cross-crate mention of `claim_gated` restates its ordering justification almost verbatim, between two crates with no dependency edge in either direction |
