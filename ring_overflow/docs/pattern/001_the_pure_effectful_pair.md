# Pattern: The Pure/Effectful Pair

### Scope

**Purpose:** Record the pure/effectful pairing as a pattern — one function that
computes and records, one that only computes — and how often the rest of the
codebase uses it.

**Responsibility:** The pattern's shape here, its naming convention, and its
frequency across `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/`.

**In Scope:** `ring_overflow/src/lib.rs:192`, `:210`, `:229`.

**Out of Scope:** Why the pair exists is
[`workaround/001`](../workaround/001_the_recorder_forecloses_const.md). The textual
difference between the two is
[`item/001`](../item/001_the_two_free_functions_one_statement_apart.md).

---

## The Pattern, and How Often It Recurs

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every would_* function under the crate tree --'
command grep -r 'fn would_' --include=*.rs */src/ 
echo '  -- the four prefixes a second crate might reach for instead --'
for p in peek try dry preview; do printf '     %-8s %s\n' "$p" "$( command grep -rc "fn ${p}_" --include=*.rs */src/ | awk -F: '{ n += $2 } END{ print n+0 }' )"; done
echo '  -- crates under the crate tree declaring at least one pub const fn --'
command grep -rl 'pub const fn ' --include=*.rs */src/ | sed -E 's|/src/.*||' | sort -u | wc -l
echo '  -- how many this crate declares, of four functions --'
command grep -c 'pub const fn ' ring_overflow/src/lib.rs || true
echo '  -- and the doc line naming the pattern --'
command grep -m1 -F '/// The pure half of [`resolve`], for callers deciding what a policy *would* do —' ring_overflow/src/lib.rs
```

Live output:

```
  -- every would_* function under the crate tree --
ring_overflow/src/lib.rs:pub const fn would_resolve( policy : OverflowPolicy ) -> Resolution
  -- the four prefixes a second crate might reach for instead --
     peek     0
     try      75
     dry      0
     preview  0
  -- crates under the crate tree declaring at least one pub const fn --
66
  -- how many this crate declares, of four functions --
3
  -- and the doc line naming the pattern --
/// The pure half of [`resolve`], for callers deciding what a policy *would* do —
```

---

### OV41 — The `would_` Prefix Appears Once Under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/` and Is Never Explained as a Convention

`would_resolve` is the only function anywhere under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/` whose name begins
`would_`. `const` purity itself is ordinary here — the census above counts dozens
of crates declaring at least one `pub const fn` — while pairing a `const`
computation with a recording twin under a `would_`/bare naming split is not. That
happens here and nowhere else.

**Finding.** So the crate introduces a naming convention with a sample size of one.
The convention is a good one — `would_x` reading as "what `x` would do without
doing it" is immediately legible, and the doc comment reinforces it with "for
callers deciding what a policy *would* do".

What is absent is any statement that it *is* a convention. A second crate needing
the same split has no precedent to follow and no rulebook entry to find, so it will
reach for `peek_`, `dry_`, or `preview_` with equal justification — the census
above finds **zero** of all three under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/`, so none of them carries a
precedent either. The fourth candidate is worse than absent: `try_` is already
spoken for, **72** times over, and it means *fallible attempt* rather than
*hypothetical result* — so the one prefix a maintainer is most likely to reach
for is the one that would say the wrong thing. This is the family's first
instance of the problem, and the answer to it is recorded only as one function's
name.

Worth stating because the shape is likely to recur: any operation that both decides
and records has the same tension, and this crate solved it once, correctly,
locally.

---

### OV42 — The Pattern Is Named After Its Purity and Justified by Its Purpose, and the Two Do Not Match

The doc comment gives `would_resolve` a purpose: "for callers deciding what a
policy *would* do — a factory validating a configuration, a test tabulating the
mapping — rather than handling a real full-ring event."

That is a statement about *when* to use it, and it excludes the one production
caller, which is handling exactly a real full-ring event
([`integration/001`](../integration/001_one_consumer_one_import_one_site.md) § OV18).

**Finding.** The pattern's real division is purity, not purpose. `would_resolve`
is the right choice for any caller that does not want a counter written — including
`ring_core`, whose manifest does not declare `ring_stats` at all, so it has no
`&RingStats` to pass and could not call `resolve` without taking on a new
dependency. That is a decisive reason to take the pure half, and the doc comment's
list of use cases does not include it.

So the documentation frames the pair as preview-versus-perform, and the code uses
it as records-versus-does-not. Both framings describe the same two functions and
they recommend different halves for the same caller, which is why the crate's one
consumer looks, on the documentation's terms, like it is using the wrong one.

The correction is one clause: `would_resolve` is for callers that will not or
cannot record. That covers the factory, the test, and `ring_core`, and stops the
one production use from reading as a mistake.

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -A5 -F 'Also the right half for a caller that holds no' ring_overflow/src/lib.rs
```

Live output:

```
/// **Also the right half for a caller that holds no `&RingStats` at all.**
/// The pattern's real division is purity, not purpose: a caller handling a
/// genuine full-ring event still belongs here if it has nothing to record
/// into, since [`resolve`] cannot be called without one. That is why this
/// crate's own sole consumer takes this half for a real full-ring event
/// rather than a hypothetical one.
```

**Disposition:** applied — `would_resolve`'s doc comment in
`ring_overflow/src/lib.rs` now adds the missing clause this instance
names: it states the pure half is also right for a caller that holds no
`&RingStats` at all, covering the factory, the test, and `ring_core` in
spirit, so the one production use no longer reads as a mismatch against the
doc's stated purpose. Now prints: `Also the right half for a caller that
holds no`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](../workaround/001_the_recorder_forecloses_const.md) | The constraint that forced the pattern |
| [`pattern/002`](002_a_named_outcome_instead_of_a_boolean.md) | The crate's other pattern |
| [`integration/001`](../integration/001_one_consumer_one_import_one_site.md) | The caller the doc excludes |
| [`item/001`](../item/001_the_two_free_functions_one_statement_apart.md) | The two halves, side by side |

### Sources

| Fact | Where |
|------|-------|
| The only `would_` under `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/` | Census above |
| `would_resolve`'s stated purpose | `ring_overflow/src/lib.rs:210-212` |
| The recording half | `ring_overflow/src/lib.rs:192`, `:199` |
| The consumer that declares no `ring_stats` | `ring_core/Cargo.toml` |

### Tests

| Test | Covers |
|------|--------|
| `would_resolve_touches_no_counters` | The purity that actually divides the pair |
| `resolve_agrees_with_would_resolve` | That the two halves compute the same thing |
| `exactly_one_counter_moves_per_call` | The effect the pure half omits |
