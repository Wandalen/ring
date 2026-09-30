# Type: A `Seq`, a `usize`, and the One Cast

### Scope

- **Purpose**: Account for every type this crate names, and for the single `as` conversion in its source — why it is there, why it is lossless, and what the arithmetic underneath it commits to.
- **Responsibility**: List the type surface, justify the `Seq`/`usize` mix in one signature, prove the cast lossless, and record the overflow behaviour of the method `try_publish` actually calls — a behaviour this instance filed as undocumented and which has since been written.
- **In Scope**: Types appearing in `src/lib.rs`, and the conversion between them.
- **Out of Scope**: The traits `Publisher` derives and does not — see [`type/002`](002_two_derives_and_the_ones_that_are_absent.md).

### The Whole Type Surface

Six methods, and only five distinct types across all of them:

| Type | From | Appears as |
|------|------|-----------|
| `Publisher` | this crate | the receiver, and `new`'s return |
| `Seq` | `ring_types` | `start`, both return positions, `is_published`'s argument |
| `usize` | core | `len`, and nothing else |
| `PaddedCursor` | `ring_cursor` | the one field, and `cursor()`'s return |
| `bool` | core | `is_published`'s return |
| `Ordering` | core | the `PUBLISH` constant's type |

No generic parameter, no lifetime, no trait bound, no associated type, and no
type defined by this crate other than `Publisher` itself. `Result< Seq, Seq >` is
the only compound, and its two arms are the same type
([`api/002`](../api/002_a_result_whose_error_is_not_an_error.md)).

`Seq` is a public tuple struct — `pub struct Seq( pub u64 )` at
`ring_types/src/id.rs:25` — so it is a newtype for naming and comparison, not for
encapsulation. Anyone can write `Seq( 99 )`, which is what makes
[`pitfall/001`](../pitfall/001_publishing_a_range_you_never_claimed.md)'s mode 1
expressible in the first place.

### Why the Signature Mixes Two Numeric Types

```rust
pub fn try_publish( &self, start : Seq, len : usize ) -> Result< Seq, Seq >
```

`start` is a **position** and `len` is a **count**, and the family keeps those
apart deliberately:

| | Position | Count |
|---|----------|-------|
| Type | `Seq` (wraps `u64`) | `usize` |
| Ordered against | other positions | nothing |
| Sized by | the ring's history | the machine's addressable range |
| Wraps | never, by argument | n/a |

`ring_types/src/id.rs:1-7` states the same principle for the *other* pair —
`Seq` versus `SlotIndex( pub usize )`:

> They are separate types on purpose. […] keeping the sequence apart from the
> slot is what lets a gate compare two positions that are a full lap apart,
> which is impossible once both have been folded into `0..capacity`.

A `len` typed as `Seq` would be comparable against a frontier, which is
meaningless; a `start` typed as `usize` would be comparable against a capacity,
which is worse. The mix is the point.

`usize` specifically, rather than `u64`, because `len` counts slots, and slot
counts are bounded by what the machine can address — the same reason
`Capacity` and `SlotIndex` are `usize`-shaped.

### PB39 — The Family Casts 42 Times; This Crate Casts Once, and Only Ever Widens

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rhoE '\bas (u8|u16|u32|u64|usize|isize|i64)\b' ring_*/src/*.rs | sort | uniq -c
command grep -hE '\bas\b' ring_publish/src/lib.rs | sed 's/^ *//'
```

Live output:

```
     27 as u64
     15 as usize
let end = start.advanced_by(len as u64);
/// just as long, and that one is not the waiting caller's bug at all. The
```

Forty-two casts on forty-one lines across fourteen crates — 27 `as u64` and
15 `as usize`. This census counts every occurrence, doc examples included, which
is why `ring_barrier` appears at all: its single cast is inside a `///` example.
`ring_barrier`'s own BR21 counts only the lines the compiler sees and so reports
a smaller family — the same fact measured differently, not a disagreement, and
worth saying because the two numbers are otherwise easy to read as one:

| Crate | Casts | Crate | Casts |
|-------|------:|-------|------:|
| `ring_spsc` | 8 | `ring_testkit` | 2 |
| `ring_mpsc` | 7 | `ring_gating` | 2 |
| `ring_bench` | 5 | `ring_trace` | 1 |
| `ring_seqno` | 4 | **`ring_publish`** | **1** |
| `ring_claim` | 3 | `ring_debug` | 1 |
| `ring_batch` | 3 | `ring_cursor` | 1 |
| `ring_index` | 3 | `ring_barrier` | 1 |

`ring_publish` is tied for the minimum, and its one cast is the `advanced_by`
line inside `try_publish` — the second line the recipe above prints is a `///`
mention of the word, not a conversion:

```rust
let end = start.advanced_by( len as u64 );
```

It is there because `Seq::advanced_by` takes `u64` and `len` is `usize`, and it
is **lossless on every target Rust supports**: `usize` is at most 64 bits, so
`usize → u64` widens or is identity, never truncates.

The other direction is not uniform. Of the fifteen `as usize` casts, eleven
convert from a `u64` — `seq.0 as usize`, `distance_to( … ) as usize`,
`occupancy() as usize` — and on a 32-bit target those truncate. Two shapes
exist among them:

| Shape | Sites | Truncation |
|-------|-------|------------|
| masked immediately | `ring_index:51`, `ring_mpsc:543` — `( seq.0 as usize ) & capacity.mask()` | harmless; the mask discards the same bits |
| unmasked distance or difference | `ring_spsc:579,600,861,920,948`, `ring_mpsc:1056,1126`, `ring_bench:595`, `ring_seqno:98` | a distance above 2³² would truncate on a 32-bit target |

The remaining four are not `u64`-sourced at all, in three shapes a binary
"thirteen from `u64`, one pointer-to-integer" framing had no room for:

| Shape | Sites | Truncation |
|-------|-------|------------|
| pointer-to-integer | `ring_cursor:189` — `core::ptr::from_ref( self ) as usize` | n/a — not a numeric cast |
| `usize → usize`, no-op | `ring_batch:323` — `free_slots( … ) as usize`, and `free_slots` already returns `usize` | none; the cast converts nothing |
| `u32 → usize` | `ring_testkit:227,229` — both cast the `minted : u32` field | none; `u32` always fits in `usize` |

So the family's casts are not uniformly safe, and **`ring_publish` is on the side
that is**: its one cast widens, and it never performs the narrowing direction at
all. Nothing enforces this — `[workspace.lints.clippy]` denies only
`undocumented_unsafe_blocks`, not `cast_possible_truncation` — so it is a
property of the code as written, not of the build.

### PB40 — The Method This Crate Calls Documented No Overflow Behaviour, and Its Sibling Documented It Wrongly

`try_publish`'s arithmetic is `Seq::advanced_by`. When this was filed, that
method carried one summary line, two doctests, and **nothing about what
`self.0 + n` does when it overflows** — no `# Panics`, no note. Its sibling
`next`, four lines earlier and structurally identical (`Self( self.0 + 1 )`),
did address it, and said this:

```text
/// Panics on overflow in a debug build and saturates in a release build, which
/// is the standard `u64` addition behaviour — deliberately not wrapping, since
/// a wrapped `Seq` would silently violate the monotonicity every gate in the
/// family relies on.
```

Both halves have since been repaired, and the live text of both is quoted below
the analysis. The finding's reasoning is preserved because it is what the
correction was made from.

**Saturation is not what happens.** Rust's `+` on `u64` panics when
`overflow-checks` is on and wraps two's-complement when it is off; it never
saturates — that is `saturating_add`, which neither method calls. The workspace
sets no `[profile.release]` at all, so release builds take the default
`overflow-checks = false`:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'profile.release\|overflow-checks' Cargo.toml \
  || echo '(no matches — neither key appears in the manifest)'
# control: the identical pattern against input that does contain both keys
printf '[profile.release]\noverflow-checks = false\n' \
  | grep 'profile.release\|overflow-checks'
```

Live output:

```
(no matches — neither key appears in the manifest)
[profile.release]
overflow-checks = false
```

So the release behaviour was precisely the wrapping the comment said was
deliberately avoided, and the property the comment argued for — monotonicity —
was protected by nothing in the code.

What actually protects it is the crate-level argument at the top of the same
file: at 10⁹ publications per second a `u64` runs for roughly 584 years, so the
wrap point is unreachable and no mechanism is needed. That argument is sound and
is why this was a **documentation defect, not a bug** — the reachable behaviour
was correct, and the sentence describing the unreachable behaviour was wrong
twice over (it named saturation, and it claimed the wrapping it disclaimed).

For this crate the exposure is narrower still: `try_publish` adds `len as u64` to
a frontier, so overflowing needs a `start` already near `u64::MAX`, which needs
2⁶⁴ prior publications — the same unreachable premise, reached by the same
argument.

Here is what both methods say now:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- next, the sibling that was wrong --'
command grep -m1 -A4 -F '  /// Panics on overflow in a debug build and wraps to zero in a release' ring_types/src/id.rs
echo '  -- advanced_by, the one try_publish calls, which said nothing --'
command grep -m1 -A5 -F '  /// Overflow behaves exactly as [`Seq::next`] documents' ring_types/src/id.rs
echo '  -- and the word this finding was about --'
command grep -c 'saturates in a release build' ring_types/src/id.rs \
  | sed 's/^/    occurrences of the original claim: /'
```

Live output:

```
  -- next, the sibling that was wrong --
    /// Panics on overflow in a debug build and wraps to zero in a release
    /// build — the standard `u64` addition behaviour. A wrapped `Seq` would
    /// silently invert every gate comparison in the family, which is why the
    /// non-wrapping argument has to hold: at 10⁹ publications per second a
    /// `u64` runs for roughly 584 years, well past any reachable workload.
  -- advanced_by, the one try_publish calls, which said nothing --
    /// Overflow behaves exactly as [`Seq::next`] documents — debug panics,
    /// release wraps to zero. The reachability argument does not carry over
    /// unchanged: `next` needs 2⁶⁴ increments to reach the wrap, while this
    /// takes `n` from the caller and reaches it in a single call from any
    /// position. A caller deriving `n` from a batch length or a configured
    /// count owns that bound; nothing here checks it.
  -- and the word this finding was about --
    occurrences of the original claim: 0
```

Two things followed. The first was acted on and the second was wrong:

1. The correction belonged in `ring_types`, not here — one sentence, and
   `Seq::next`'s doctest would not change. That is exactly the shape the fix
   took: the sentence was replaced, the doctest is untouched.
2. **"`advanced_by` documenting nothing is the more defensible of the two
   states" does not survive.** It reads silence as a weaker claim than a wrong
   claim, but silence is not a claim at all — and `advanced_by` is the method
   this crate actually calls, so the undocumented one was the one carrying this
   crate's arithmetic. `advanced_by` now has an overflow paragraph of its own,
   and it says something `next`'s cannot: the reachability argument does *not*
   carry over, because `n` comes from the caller and reaches the wrap in a
   single call from any position. For `try_publish` that `n` is `len as u64`,
   bounded by a slice length that came from a real claim — so the bound is
   inherited from the caller rather than checked here, which is precisely the
   condition the new paragraph tells a reader they own.

**Disposition:** applied — elsewhere; both halves landed in `ring_types`: `next`'s
sentence now states debug panic / release wrap and keeps the 584-year figure as
the reason that is survivable, and `advanced_by` carries an overflow paragraph
naming the caller-supplied-`n` case this crate's `len as u64` falls under. The
recipe above prints both and asserts the original wording is gone. What remains
this instance's own is item 2's retraction, recorded above rather than deleted,
because the "silence is safer than a wrong note" reasoning is what let the
undocumented method sit unfixed while the documented one was argued about.
Now prints: `    occurrences of the original claim: 0`

### Where the Types Are Checked

| Check | What it establishes |
|-------|--------------------|
| `tests/publish_test.rs:45-58` | `cursor()` returns the same `PaddedCursor` every call, by `core::ptr::eq` |
| `tests/publish_test.rs:214-238` | 320 `is_published` assertions — the `Seq`/`bool` boundary across a frontier sweep |
| `#![ deny( missing_docs ) ]` at `src/lib.rs:55` | every public type and method carries documentation, so none can be added silently |
| the compiler | that `len as u64` is the only conversion, since removing it fails to build |

**None of the six manual checks covers the type surface.** P1–P6 check the loom
model's sensitivity, the cursor's single write path, the ordering constant, the
refusal shape, dependency usage, and harness exclusivity — a list that is about
*behaviour and structure*, not about which types appear. The surface is held by
the compiler and by `deny( missing_docs )` alone.

The last table row is the strongest and the cheapest: the cast is load-bearing,
so it cannot silently disappear or be duplicated without a build error.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_six_methods_and_no_caller.md](../api/001_six_methods_and_no_caller.md) | The six signatures these types appear in |
| [../api/002_a_result_whose_error_is_not_an_error.md](../api/002_a_result_whose_error_is_not_an_error.md) | The only compound type in the surface |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_padded_cursor_and_nothing_else.md](../data_structure/001_one_padded_cursor_and_nothing_else.md) | The one field, and the type it holds |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_is_published_is_exclusive_of_the_frontier.md](../invariant/002_is_published_is_exclusive_of_the_frontier.md) | The `Seq` comparison that makes the boundary strict |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_three_readings_of_the_cursor.md](../item/001_the_three_readings_of_the_cursor.md) | The three return types the cursor is read through |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_what_a_publication_costs.md](../non_functional_requirement/001_what_a_publication_costs.md) | The eight live bytes `Seq` occupies |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_publishing_a_range_you_never_claimed.md](../pitfall/001_publishing_a_range_you_never_claimed.md) | What `pub struct Seq( pub u64 )` makes constructible |

### Types

| File | Relationship |
|------|--------------|
| [002_two_derives_and_the_ones_that_are_absent.md](002_two_derives_and_the_ones_that_are_absent.md) | What `Publisher` implements, and what it deliberately does not |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:161-165` | The signature that mixes the two, and the one cast |
| `ring_types/src/id.rs:1-7` | Why a position and a count are different types |
| `ring_types/src/id.rs:24-25` | `Seq( pub u64 )` — a newtype with a public field |
| `ring_types/src/id.rs` — `Seq::next` | The overflow sentence PB40 found wrong twice; since replaced |
| `ring_types/src/id.rs` — `Seq::advanced_by` | The method `try_publish` calls; documented nothing about overflow when PB40 was filed, and now carries the caller-supplied-`n` paragraph |
| `Cargo.toml` | No `[profile.release]`, so release wrapping is the default |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:45-58` | The `PaddedCursor` identity, by pointer |
| `tests/publish_test.rs:214-238` | The `Seq` → `bool` boundary, 320 times |
| `tests/manual/readme.md § P1-P6` | Six checks, none of them about the type surface |
