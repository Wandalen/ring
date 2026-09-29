# WaitKind::is_non_blocking

## Representation

True for exactly one of four variants: `WaitKind::None`. **The only one of the
crate's five predicates with a production caller anywhere in the family** — and
the caller's own callers are all tests.

That single edge is what makes this item worth its own instance rather than a row
in a table. `ring_config::RingConfig::is_tick_safe` reads:

```rust
pub const fn is_tick_safe( &self ) -> bool
{
  self.wait.is_non_blocking()
}
```

**One line, delegating rather than deciding.** A local `match self.wait {
WaitKind::None => true, _ => false }` would compile, read identically at the call
site, and answer wrongly the first time a fifth wait kind is added. The
delegation is deliberate, and it is the only predicate in this crate that
something outside the crate has written down a reason to keep
(→ [`../../pattern/001`](../../pattern/001_discriminants_here_handlers_elsewhere.md)).

## Kind

Associated Function/Method (§ Item Kind Taxonomy : Associated Item Kinds #1)

## Definition

`ring_types/src/policy.rs:81`

```rust
#[ must_use ]
pub const fn is_non_blocking( self ) -> bool
```

Body (`policy.rs:82`-`88`) is an exhaustive `match`:

```rust
match self
{
  Self::None => true,
  Self::Spin | Self::Yield | Self::Park => false,
}
```

**This predicate was the last of the crate's five classifiers to convert.**
[`is_configuration`](004_ring_error_is_configuration.md) and
[`is_transient`](005_ring_error_is_transient.md) converted under
`Fix(ring_error_classification_not_exhaustive)`;
[`reports_failure`](012_overflow_policy_reports_failure.md) and
[`drops_silently`](013_overflow_policy_drops_silently.md) converted under
`Fix(overflow_policy_classification_not_exhaustive)`. This one converted last,
under its own tag, `Fix(wait_kind_is_non_blocking_classification_not_exhaustive)`
(`policy.rs:67`-`79`) — the comment block recording the fix is now longer than
the function it sits above. `matches!` no longer appears anywhere in `src/`.

**Why this one mattered more than the other four.** `WaitKind` is not
`#[ non_exhaustive ]`; only `RingError` is (`error.rs:43` is the sole
occurrence in the crate). An ordinary hand-written `match self { ... }` over
`WaitKind` was therefore already forced to be exhaustive by the compiler —
adding a fifth variant would break every such site, family-wide, with or
without this fix. `matches!( self, pattern )`, though, macro-expands to
`match self { pattern => true, _ => false }`: the implicit wildcard it
introduces is a property of the macro, not of `#[ non_exhaustive ]`, so it
bypassed that protection regardless of the enum's own exhaustiveness posture.
This predicate's old `matches!( self, Self::None )` body was, for that reason,
uniquely blind among the crate's hand-written `WaitKind` code — a fifth
variant would have compiled and silently answered `false` ("not safe for the
tick path") instead of failing the build the way every other `WaitKind` match
already did.
`Fix(wait_kind_is_non_blocking_classification_not_exhaustive)` closed exactly
that gap by giving this predicate the same wildcard-free shape as its siblings.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/policy.rs` | 18-19, 60-61, 63-66, 67-79, 80-81, 82-88 | Doc example on the enum asserting `None` true and `Park` false (18-19); the doc summary, which states the membership as "true for exactly `WaitKind::None`" (60-61); doctest counting the matches across `ALL` (63-66); `Fix(wait_kind_is_non_blocking_classification_not_exhaustive)` comment (67-79); `#[ must_use ]` and **the definition (80-81)**; the body (82-88) |

**The doc example is a count over `ALL`, not a list of cases:**

```rust
assert_eq!( WaitKind::ALL.iter().filter( | w | w.is_non_blocking() ).count(), 1 );
```

That form survives a variant being added — the assertion becomes false if the
new variant is also non-blocking, which is the case worth catching — where three
individual `assert!` calls would not
(→ [`../associated_constant/002_wait_kind_all.md`](../associated_constant/002_wait_kind_all.md)).

Test-only references: `ring_types` — `tests/types_test.rs:160`,
`exactly_one_wait_kind_is_non_blocking`, whose name is the membership rule. Plus
two consumer suites: `ring_config/tests/config_test.rs:231` and
`ring_wait/tests/wait_test.rs:80`.

**`ring_wait` is the other half of the split and it asserts the same fact under
almost the same name.** Its test at `wait_test.rs:103` is
`exactly_one_discriminant_is_non_blocking`; `ring_types`' is
`exactly_one_wait_kind_is_non_blocking`. Two crates, one property, tested
independently on both sides of the discriminant/handler boundary — which is what
the discriminant/handler split is supposed to produce
(→ [`../../pattern/001`](../../pattern/001_discriminants_here_handlers_elsewhere.md)).

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/policy.rs` | Defining crate |
| `ring_config` | `src/lib.rs` | `RingConfig::is_tick_safe` delegates to it (`:240`) — **the family's only production call site for any of the five predicates** |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn '\.is_non_blocking(' ring_*/src | command grep -v '^ring_types/' \
  | command grep -v ':[0-9]*: *//' | command sed -E 's/^([^:]+):[0-9]+:/\1:/'
```

Live output:

```
ring_config/src/lib.rs:    self.wait.is_non_blocking()
```

returns one line: `ring_config/src/lib.rs`.

**Following the edge one step further is the honest reading.**
`is_tick_safe`'s own callers are its doctest (`ring_config/src/lib.rs:234-235`)
and three assertions in `ring_config/tests/config_test.rs` (`:230`, `:235`,
`:236`). No production code in the family asks whether a config is tick-safe.
So the production call site is real, and the chain it sits on terminates in
tests two steps up (→ [`../../api/002`](../../api/002_the_five_classifier_predicates.md)).

## Caller Tree

- *No caller within `ring_types`*
- *External: `ring_config::RingConfig::is_tick_safe`* (`ring_config/src/lib.rs:240`, fn at `:238`) — the only production caller
  - *External: `ring_config` doctest* (`:234`, `:235`)
  - *External: `ring_config/tests/config_test.rs`* (`:230`, `:235`, `:236`)
- *External test callers, direct: `ring_config/tests/config_test.rs:231`, `ring_wait/tests/wait_test.rs:80`*

## Callee Tree

- *(none)* — the `match` is direct, wildcard-free code, not a macro expansion

**This item has the crate's only written defence, and it lives in another
crate.** `ring_config/tests/manual/readme.md` § M4 is a manual check whose whole
subject is that `is_tick_safe` keeps delegating here:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -E -A 6 "pub (const )?fn is_tick_safe" ring_config/src/lib.rs
```

Live output:

```
  pub const fn is_tick_safe( &self ) -> bool
  {
    self.wait.is_non_blocking()
  }
}
```

**Expected:** the body delegates to `WaitKind::is_non_blocking` rather than
matching variants itself. Its Run Record entry for 2026-08-28 reads
"Body is `self.wait.is_non_blocking()` — delegation, not a local match."

**The check's own first run failed for the wrong reason**, and the readme records
it: the original pattern was `pub fn`, every method in `ring_config` is `pub const
fn`, and the grep reported the method missing. The `(const )?` above is the fix.
**A manual check that greps for a signature is checking a spelling, not a
property** — worth carrying to the other four predicates, none of which has any
defence at all.
