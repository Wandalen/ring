# api

Twelve public functions, and the census is the interesting part. Two splits run
through it — `const` against ordinary, `#[ must_use ]` against bare — and one of
them turns out to be an exact rule applied twelve times while the other turns out
to be four different reasons wearing one appearance.

The first instance reads both columns. `must_use` marks every reader and no
mutator, and the single apparent exception is `iter`, whose return type carries
the attribute already — a regularity a reader can rely on and that nothing states.
`const` marks three functions, and of the nine it does not mark, two compile as
`const` unchanged, two are blocked by a missing keyword in a four-line Tier 1
function, and five are blocked by allocation or by `&mut`. The second instance
takes the six accessors, finds them to be three mutability pairs asking one
question each, and finds the one direction the type does not offer: a slot can
enter a buffer and never leave it.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Twelve Functions, Three Const, Seven must_use](001_twelve_functions_three_const_seven_must_use.md) | BF14, BF15 — an exact `must_use` rule nobody states, and a `const` boundary drawn one crate away |
| 002 | [Six Ways to Reach a Slot](002_six_ways_to_reach_a_slot.md) | BF16, BF17 — three pairs of which consumers use one, and an exit that does not exist for a reason nothing records |

### The Whole Surface

| Function | `const` | `must_use` | Receiver |
|----------|:-------:|:----------:|----------|
| `new` | — | ✔ | — |
| `clear` | — | — | `&mut self` |
| `all_empty` | — | ✔ | `&self` |
| `capacity` | ✔ | ✔ | `&self` |
| `len` | ✔ | ✔ | `&self` |
| `is_empty` | ✔ | ✔ | `&self` |
| `get` | — | ✔ | `&self` |
| `get_mut` | — | — | `&mut self` |
| `at` | — | ✔ | `&self` |
| `at_mut` | — | — | `&mut self` |
| `iter` | — | — † | `&self` |
| `iter_mut` | — | — | `&mut self` |

† `core::slice::Iter` is itself `#[ must_use ]`, so the rule holds here through
the return type rather than the function.

### Why Only Three Are `const`

| Function | Blocked by |
|----------|------------|
| `new` | Allocation — `const` is impossible regardless |
| `get`, `get_mut` | Nothing; both compile as `const` unchanged |
| `at`, `at_mut` | `ring_index::of` is a plain `fn`, though it compiles as `const` |
| `clear`, `all_empty`, `iter`, `iter_mut` | `&mut` receivers and trait calls |

`ring_types::Capacity` is 3 of 3 `const`. `ring_index` is 0 of 3. The constness
propagates from Tier 0 and stops at Tier 1, and that stop is what makes this
crate's sequence-addressed accessors ordinary functions.

### Three Pairs and One Closed Direction

`get`/`get_mut` address by `SlotIndex`, `at`/`at_mut` by `Seq`, `iter`/`iter_mut`
over all. The two real consumers call `at` and `at_mut` and nothing else, so four
of the six have never met a caller outside this crate's own suite.

`IntoIterator` is implemented for `&Buffer` and `&mut Buffer`, never for
`Buffer`. With `new` the only constructor, private fields and a `Debug`-only
derive, slots enter through `S::default` and leave only when the buffer drops.
That closure is load-bearing — both consumers hold raw pointers into the
allocation — and nothing in the crate records it as deliberate.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the whole surface, with both splits
awk '/#\[ must_use \]/{mu=1; next} /pub (const )?fn /{
  name=$0; sub(/^ *pub /,"",name); sub(/\(.*/,"",name);
  isc = ($0 ~ /pub const fn/) ? "const" : "-";
  printf "  %-28s %-6s must_use=%s\n", name, isc, (mu?"yes":"no"); mu=0 }' ring_store/src/lib.rs

# const counts upstream, where the boundary is actually drawn
printf '  ring_types::Capacity const fns: %s of %s\n' "$( grep -c 'pub const fn ' ring_types/src/capacity.rs )" "$( grep -c 'pub const fn \|pub fn ' ring_types/src/capacity.rs )"
printf '  ring_index const fns:           %s of %s\n' "$( grep -c 'pub const fn ' ring_index/src/lib.rs )" "$( grep -c 'pub const fn \|pub fn ' ring_index/src/lib.rs )"

# the four impl blocks — two inherent, two IntoIterator, both for references
grep -n '^impl' ring_store/src/lib.rs

# one producer, no decomposer
grep -n 'pub fn .*-> Self\|pub fn .*-> Vec<\|pub fn .*-> Box<\|impl.*From.*for ' ring_store/src/lib.rs
```

The `const`-qualification probes for `get`, `get_mut` and `of`, and the
`slice::Iter` warning, come from `rustc` runs quoted in
[`api/001`](001_twelve_functions_three_const_seven_must_use.md).

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF14 | `ring_store` | n/a — observation | `#[ must_use ]` marks every reader and no mutator across all twelve functions, and `iter`'s apparent exception is the rule holding through `slice::Iter`'s own attribute — a regularity a reader can rely on and that nothing states |
| BF15 | `ring_index` | n/a — doc gap | `get`/`get_mut` compile as `const` unchanged and `at`/`at_mut` cannot only because `ring_index::of` is a plain `fn` though it too compiles as `const`; four different reasons for non-constness are indistinguishable from outside |
| BF16 | `ring_store` | n/a — coverage | Six accessors are three mutability pairs; consumers call one pair, so four have never met a caller and their ergonomics are untested by use |
| BF17 | `ring_store` | n/a — doc gap | No by-value `IntoIterator` and no decomposing function, so slots leave only when the buffer drops — a closure that protects the raw pointers two consumers hold, recorded nowhere |
