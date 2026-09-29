# API: Five Functions and No Types

### Scope

- **Purpose**: Give the crate's entire public surface, and examine what a crate that exports only free functions can promise.
- **Responsibility**: Enumerate the five exports, establish that the surface has no types at all, and record the constness asymmetry against the crate it depends on.
- **In Scope**: `ring_seqno`'s public surface.
- **Out of Scope**: The argument-order inconsistency inside it — see [`002`](002_the_argument_order_split.md).

### The Whole Surface

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^pub ' ring_seqno/src/lib.rs
```

Live output:

```
pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
pub fn pending( producer : Seq, consumer : Seq ) -> u64
pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
```

| Line | Item | Kind |
|-----:|------|------|
| 50 | `laps_between( Seq, Seq, Capacity ) -> u64` | `pub fn` |
| 73 | `may_claim( Seq, Seq, Capacity ) -> bool` | `pub fn` |
| 95 | `free_slots( Seq, Seq, Capacity ) -> usize` | `pub fn` |
| 112 | `pending( Seq, Seq ) -> u64` | `pub fn` |
| 133 | `slowest( &[ Seq ] ) -> Option< Seq > ` | `pub fn` |

Five items, and the count of every other kind is zero:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -cE '^pub (struct|enum|trait|const|type|use|mod) ' ring_seqno/src/lib.rs || true
# control — the identical expression over a crate that does declare items
grep -cE '^pub (struct|enum|trait|const|type|use|mod) ' ring_cursor/src/lib.rs
```

Live output:

```
0
4
```

**No types. No constants. No re-exports.** A caller of this crate names
`ring_types::Seq` and `ring_types::Capacity` directly; nothing here stands
between them and it.

### What That Shape Buys

| Property | Consequence |
|----------|-------------|
| No type means no invariant to maintain | There is no state to get wrong, no constructor to validate, no `Drop` |
| No re-export means no second import path | Contrast `ring_cursor`, whose `SeqCell` re-export gives the family two ways to name one trait and no lint against mixing them |
| No constant means nothing to fork | Contrast `GATING`, restated four times across three crates because a `const` does not travel |
| Every function is `#[ must_use ]` | `grep -c must_use` → 5, one per function. A reading whose result is discarded is always a bug here, and all five say so |

The `must_use` count matching the function count exactly is worth stating: these
are pure functions with no side effect whatsoever, so calling one and dropping
the result is unambiguously dead code. There is no exception to argue about.

### What It Gives Up

**A free function does not travel across a manifest boundary on its own.** The
family has already demonstrated this twice — `ring_align::on_distinct_lines` was
reimplemented by `ring_mpsc`, and `ring_cursor::GATING` is restated wherever it
is needed. A crate exporting only free functions is maximally exposed to that
failure mode.

It has not happened here, and
[`algorithm/002`](../algorithm/002_the_slowest_fold.md) works out why: every tier
above this one changes the argument type, so every tier had to write a wrapper,
and each wrapper called down rather than reimplementing. That is a property of
the *consumers*, not of this crate's design — it could stop being true the moment
a consumer appears that already holds `&[ Seq ]`.

### Four of Five Could Be `const fn`, and None Is

```sh
cd "$(git rev-parse --show-toplevel)"
grep -cE 'pub const fn ' ring_seqno/src/lib.rs
grep -rcE 'pub const fn ' ring_types/src/*.rs
```

Live output:

```
0
ring_types/src/capacity.rs:3
ring_types/src/error.rs:2
ring_types/src/id.rs:4
ring_types/src/lib.rs:0
ring_types/src/policy.rs:3
```

Zero here; twelve across `ring_types`'s five files.

**Finding SQ5.** Every function this crate calls is `const` — `Seq::distance_to`,
`Seq::next`, `Seq::advanced_by`, `Capacity::get`, `Capacity::mask`,
`Capacity::new`, all twelve of `ring_types`'s `pub fn`. Nothing this crate
exports is. `ring_seqno` is the boundary at which the family loses `const`.

This was not assumed. It was compiled — the four bodies copied verbatim with only
`fn` changed to `const fn`, plus four `const` items to force actual
const-evaluation rather than mere acceptance of the annotation:

```rust
const CAP : Capacity = match Capacity::new( 8 ) { Ok( c ) => c, Err( _ ) => panic!() };
pub const LAPS  : u64   = laps_between( Seq( 0 ), Seq( 16 ), CAP );
pub const CLAIM : bool  = may_claim( Seq( 4 ), Seq( 0 ), CAP );
pub const FREE  : usize = free_slots( Seq( 3 ), Seq( 0 ), CAP );
pub const PEND  : u64   = pending( Seq( 5 ), Seq( 2 ) );
```

`cargo check` — clean, exit 0. So the four are const-legal as written; `/`,
`<`, `saturating_sub` and the two casts are all permitted in a `const fn` body.

`slowest` is the genuine exception: `Iterator::min` is not `const`, so it could
not take the annotation.

**Whether it matters is a separate question, and the honest answer is: barely.**
The three capacity readings' only in-family callers immediately combine them with
an atomic load, which is never a compile-time operation — `CursorPair::may_claim`
could not be `const` regardless of what this crate declares. What the annotation
would buy is compile-time capacity arithmetic for a caller that has no cursors
yet, and there is no such caller today.

It is recorded because the asymmetry is unexplained rather than because it is
costly. A reader who notices that `ring_types` is uniformly `const` and `ring_seqno`
is uniformly not will look for the reason, and there is no comment anywhere
giving one.

### The Surface Cannot Fail

```sh
cd "$(git rev-parse --show-toplevel)"
grep -cE 'Result|panic!|unwrap|expect|assert' ring_seqno/src/lib.rs
```

Live output:

```
16
```

Every one of those sixteen is inside a `///` doctest (`Capacity::new( 8 ).unwrap()`
and friends). No function here returns a `Result`, takes a fallible path, or can
panic on any input: `distance_to` saturates, `saturating_sub` saturates, and the
one division is by `Capacity::get()`, which the `Capacity` constructor already
guaranteed is non-zero. See
[`invariant/002`](../invariant/002_every_reading_is_total.md).

### Coverage

| Function | Unit tests | Doctest | Cross-crate callers |
|----------|-----------:|:-------:|--------------------:|
| `laps_between` | 4 | ✅ | **0** |
| `may_claim` | 3 | ✅ | 1 — `ring_cursor:419` |
| `free_slots` | 4 | ✅ | 3 — `ring_cursor:370`, `ring_gating:221`, `ring_batch:323` |
| `pending` | 1 | ✅ | 2 — `ring_cursor:373`, `ring_consume:342` |
| `slowest` | 2 | ✅ | **0** — was 1, until `b7e075ca` |

```sh
cd "$(git rev-parse --show-toplevel)"
for f in laps_between may_claim free_slots pending slowest; do
  printf '%-14s %s\n' "$f" "$( grep -rn "ring_seqno::$f" ring_*/src/*.rs \
    | grep -v '^ring_seqno/' | grep -vE ':\s*(///|//!|//)' | wc -l )"
done
```

Live output:

```
laps_between   0
may_claim      1
free_slots     3
pending        2
slowest        0
```

The census counts *qualified* mentions, so a crate that imports the name and
then calls it bare is counted at its `use` line rather than its call site —
`ring_batch` is the one that does this, and the coverage table above cites its
call at `:275` rather than the `use` at `:37` the census matched. The counts
agree either way; only the line attribution differs.

Five doctests, matching the five functions and matching M4's recorded run. There
are now **two** zeros in the last column. `laps_between`'s was always the finding
— see [`workaround/002`](../workaround/002_laps_between_has_no_caller.md).
`slowest`'s is newer and arrived from the opposite direction: it had exactly one
cross-crate caller, `ring_cursor::slowest`, and commit `b7e075ca` did not move
that caller elsewhere but stopped it delegating at all, folding the loads in
place. The function is unchanged and correct, and nothing outside this crate
calls it.

### SQ5 — Const-Legal and Not Declared

The crate one tier down declares every one of its functions `const`; this one declares none:

```
ring_types/src/capacity.rs   pub fn=3   const fn=3
ring_types/src/error.rs      pub fn=2   const fn=2
ring_types/src/id.rs         pub fn=4   const fn=4
ring_types/src/policy.rs     pub fn=3   const fn=3
ring_seqno/src/lib.rs          pub fn=5   const fn=0
```

**Finding.** Four of the five functions are const-legal and none is declared `const fn`, while all twelve `pub fn` in `ring_types` — the crate directly below — are.

---

### SQ6 — Full on One Attribute, Empty on the Other

Two attributes, opposite outcomes, one file:

```
crates at 100% must_use, family-wide:
  ring_cursor (13/13)
  ring_index  (3/3)
  ring_seqno    (5/5)
```

**Finding.** All five functions carry `#[ must_use ]`, one of exactly three crates in the family at full coverage, and the crate has no `const fn` at all — the most disciplined about return values and the least about compile-time evaluation.

---

### SQ7 — One Lint, and Not the One a Reader Assumes

The crate contains no `unsafe`. Nothing in it says it must not:

```
25:#![ deny( missing_docs ) ]

(no forbid( unsafe_code ), no deny( unsafe_code ), no workspace lint table entry)
```

**Finding.** `#![ deny( missing_docs ) ]` is the crate's only lint attribute, so nothing forbids `unsafe` and the crate's zero-`unsafe` property is a fact about the text rather than a guarantee about the future.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_four_readings_of_one_subtraction.md](../algorithm/001_four_readings_of_one_subtraction.md) | What the four binary functions compute |
| [../algorithm/002_the_slowest_fold.md](../algorithm/002_the_slowest_fold.md) | Why a free-function surface survived here |

### APIs

| File | Relationship |
|------|--------------|
| [002_the_argument_order_split.md](002_the_argument_order_split.md) | The inconsistency within these five |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_every_reading_is_total.md](../invariant/002_every_reading_is_total.md) | Why nothing here can fail |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_capacity_readings.md](../item/001_the_three_capacity_readings.md) | The three that take a `Capacity` |
| [../item/002_the_two_readings_without_a_capacity.md](../item/002_the_two_readings_without_a_capacity.md) | `pending` and `slowest` |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/002_laps_between_has_no_caller.md](../workaround/002_laps_between_has_no_caller.md) | The zero in the coverage table |

### Sources

| File | Relationship |
|------|--------------|
| `ring_seqno/src/lib.rs:49-136` | The five exports |
| `ring_types/src/id.rs:27-86` | Four `const fn` this crate calls |
| `ring_types/src/capacity.rs:25-79` | Three more |

### Tests

| File | Relationship |
|------|--------------|
| `tests/seq_test.rs:15` | The import line — the whole surface fits in one `use` |
| `tests/manual/readme.md` M4 | The doctests read as explanations, checked by hand |
