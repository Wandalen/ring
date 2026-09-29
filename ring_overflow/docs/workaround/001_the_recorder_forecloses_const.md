# Workaround: The Recorder Forecloses `const`

### Scope

**Purpose:** Record why the crate ships two functions computing one mapping — the
`const` one exists because the recording one cannot be `const`, and the reason is
three crates down.

**Responsibility:** The `const` split, the atomic that causes it, and the shape the
crate adopted in response.

**In Scope:** `ring_overflow/src/lib.rs:111`, `:138`, `:192`, `:199`, `:229`;
`ring_stats/src/lib.rs:293`.

**Out of Scope:** The duplication this creates is
[`item/001`](../item/001_the_two_free_functions_one_statement_apart.md) § OV26. The
two spellings of one outcome are
[`workaround/002`](002_one_outcome_expressed_in_two_type_systems.md).

---

## Three `const`, One Not, and Why

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- three const, one not --'
command grep 'pub const fn \|^pub fn ' ring_overflow/src/lib.rs
echo '  -- the statement that separates them --'
command grep -m1 -F '  stats.record_drop( policy, 1 );' ring_overflow/src/lib.rs
echo '  -- and the atomic underneath it --'
command grep -m1 -F '    counter.fetch_add( n, Ordering::Relaxed );' ring_stats/src/lib.rs
echo '  -- how many must_use marks, and on which --'
command grep -A1 'must_use' ring_overflow/src/lib.rs | command grep 'pub const fn'
```

Live output:

```
  -- three const, one not --
  pub const fn lost_an_item( self ) -> bool
  pub const fn accepted_incoming( self ) -> bool
pub fn resolve( policy : OverflowPolicy, stats : &RingStats ) -> Result< Resolution, RingError >
pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution
  -- the statement that separates them --
  stats.record_drop( policy, 1 );
  -- and the atomic underneath it --
    counter.fetch_add( n, Ordering::Relaxed );
  -- how many must_use marks, and on which --
  pub const fn lost_an_item( self ) -> bool
  pub const fn accepted_incoming( self ) -> bool
pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution
```

---

### OV37 — `would_resolve` Exists Because an Atomic Two Crates Away Cannot Run at Compile Time

`resolve` calls `record_drop`, which bottoms out in
`counter.fetch_add( n, Ordering::Relaxed )`. An atomic read-modify-write is not
callable in a const context, so `resolve` cannot be `const` no matter how the rest
of it is written.

Every other function in the crate is `const`. The three that are also carry
`#[ must_use ]`; `resolve` carries neither, because `Result` already supplies the
second mark.

**Finding.** So `would_resolve` is not primarily a convenience. It is the crate's
answer to a constraint imported from `ring_stats`: the mapping needs to be
available at compile time — for a factory validating a configuration, for a `const`
lookup, for anything in a `const fn` — and the function that records cannot
provide it. Splitting the mapping in two is the workaround, and the duplication it
causes is the price
([`item/001`](../item/001_the_two_free_functions_one_statement_apart.md) § OV26).

The reasoning is nowhere in the crate. `would_resolve`'s doc comment describes what
it is for — "the pure half of `resolve`, for callers deciding what a policy *would*
do" — and not why it must exist separately. A reader who assumed `resolve` could be
made `const` and the pair collapsed would find nothing here saying otherwise; the
obstacle is `fetch_add` at `ring_stats/src/lib.rs:293`, two crates and one
indirection away.

---

### OV38 — The Workaround Is Load-Bearing in the Direction Nobody Uses

The split buys `const`-evaluability for `would_resolve`, and nothing in the
workspace evaluates it at compile time. Its one production call site is
`ring_core:400`, an ordinary runtime `match` on a field, where `const` contributes
nothing.

The crate's own tests do not use it in a `const` context either: every call is a
runtime assertion or a `HashMap` key.

**Finding.** The property the workaround exists to preserve is therefore unexercised.
That is not an argument against it — `const` on a three-arm `match` over a fieldless
enum costs nothing and forecloses nothing — but it means the pair's justification
rests entirely on a capability no caller has taken up, while the pair's cost, a
hand-maintained duplicate of the mapping, is paid in full.

The observable consequence is that `resolve_agrees_with_would_resolve` exists at
all. It covers two of three arms and it is the only thing keeping the two copies
equal
([`item/001`](../item/001_the_two_free_functions_one_statement_apart.md) § OV26).
So the crate carries a test whose sole purpose is to defend a duplication that
exists to preserve a property nothing uses — a chain that is entirely sound at each
link and worth stating end to end, because no single doc comment spans it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/001`](../item/001_the_two_free_functions_one_statement_apart.md) | The duplication this workaround produces |
| [`api/001`](../api/001_four_declarations_three_of_them_const.md) | The attribute set that results |
| [`workaround/002`](002_one_outcome_expressed_in_two_type_systems.md) | The other consequence of the split |
| [`integration/002`](../integration/002_the_stats_edge_and_the_function_nobody_imports.md) | The dependency the constraint comes from |

### Sources

| Fact | Where |
|------|-------|
| Three `const`, one not | `ring_overflow/src/lib.rs:111`, `:138`, `:192`, `:229` |
| The recording statement | `ring_overflow/src/lib.rs:199` |
| The atomic beneath it | `ring_stats/src/lib.rs:293` |
| `would_resolve`'s stated purpose | `ring_overflow/src/lib.rs:208-212` |
| The one production call site | `ring_core/src/lib.rs:411` |

### Tests

| Test | Covers |
|------|--------|
| `would_resolve_touches_no_counters` | The property that makes `const` sound |
| `resolve_agrees_with_would_resolve` | The duplication the workaround requires defending |
| `exactly_one_counter_moves_per_call` | The effect that forecloses `const` |
| *(to create)* | Nothing evaluates `would_resolve` in a `const` context, so the property is unexercised |
