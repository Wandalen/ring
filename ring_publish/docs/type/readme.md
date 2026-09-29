# type

Five distinct types across six methods, one `as` conversion in the whole crate,
and a derive list of two. The type surface is the smallest thing this crate has,
and almost all of it is inherited: `Seq` and `PaddedCursor` come from elsewhere,
the derives come from the field, and the traits that are absent are absent
because of an `AtomicU64` three levels down.

What is recorded here is which of those facts are chosen and which are
consequences — and, in both instances, a documentation claim that does not match
the code it describes.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A `Seq`, a `usize`, and the One Cast](001_a_seq_a_usize_and_the_one_cast.md) | PB39, PB40 — the whole type surface, why a position and a count are different types, the family's 42 casts, and the overflow sentence that is wrong twice |
| 002 | [Two Derives, and the Ones That Are Absent](002_two_derives_and_the_ones_that_are_absent.md) | PB41, PB42 — `Debug` on 88 of 89, the three types deriving exactly `Debug, Default`, and `Sync` obtained by composition where two crates pay for it in `unsafe` |

### Chosen Versus Inherited

| Property | Chosen here | Inherited from |
|----------|:-----------:|----------------|
| `start : Seq`, `len : usize` | ✔ | — |
| `len as u64` | ✔ | forced by `advanced_by`'s signature |
| `Debug` | — | `missing_debug_implementations = "warn"` |
| `Default` | — | `PaddedCursor`'s own derive |
| No `Clone` / `Copy` / `PartialEq` | — | `AtomicU64` implements none |
| `Send` + `Sync` | — | composition; never declared |
| `new()` not being `const` | — | loom's atomics have no `const` constructor |

Six of the seven rows are consequences. The crate's only genuine type decisions
are the two in the signature — and those two are the family's, not this crate's:
`ring_types` splits position from count and `ring_publish` follows.

### Two Claims That Did Not Match Their Code

Both have since been repaired in `ring_types`; the table records what PB40 found
and what replaced it.

| Claim as filed | Where | Reality then | State now |
|----------------|-------|--------------|-----------|
| `Seq::next` *"saturates in a release build"* | `ring_types` — `Seq::next` | `+` on `u64` panics with `overflow-checks` on and **wraps** with it off; the workspace sets no `[profile.release]`, so release wraps — the behaviour the same sentence called *"deliberately not wrapping"* | corrected: the sentence now states debug panic / release wrap, and keeps the 584-year reachability figure as the reason that is survivable |
| `advanced_by` — the method this crate actually calls — documents overflow at all | `ring_types` — `Seq::advanced_by` | one summary line and two doctests; no `# Panics`, no note | written: it now says the reachability argument does *not* carry over, because `n` comes from the caller and reaches the wrap in one call |

Neither was reachable: 2⁶⁴ publications at 10⁹/s is roughly 584 years, which is
`ring_types`' own crate-level argument and is sound. Both were recorded because
the correction was one sentence in `ring_types`, and — this half did not survive
— because the *silent* method looked like the more defensible of the two, on the
reasoning that a reader applying Rust's ordinary rules gets the right answer
while a reader trusting a wrong note does not.

**That reasoning was wrong, and it is the more interesting half of PB40.**
Silence is not a weaker claim than a false claim; it is no claim, and it leaves
the reader to guess which of two plausible readings applies. `advanced_by` is
also the method `try_publish` actually calls, so the undocumented one was the
one carrying this crate's arithmetic while the documented one was being argued
about. Both are now written, and `advanced_by`'s says the thing `next`'s cannot:
`n` comes from the caller, so the 584-year argument does not transfer.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

echo '-- every widening cast in the family, and this crate'"'"'s one --'
command grep -rhoE '\bas (u8|u16|u32|u64|usize|isize|i64)\b' ring_*/src/*.rs | sort | uniq -c
command grep -hE '\bas\b' ring_publish/src/lib.rs | sed 's/^ *//'

echo '-- the derive census --'
command grep -rhcE '^#\[ derive\(' ring_*/src/*.rs | paste -sd+ | bc
command grep -rhE  '^#\[ derive\(' ring_*/src/*.rs | grep -c Debug
command grep -rhE  '^#\[ derive\( Debug, Default \) \]' ring_*/src/*.rs | sort | uniq -c

echo '-- who pays for Sync, and who gets it free --'
command grep -rlE 'unsafe impl.*(Send|Sync)' ring_*/src/*.rs
command grep -rl 'allow( unsafe_code )' ring_*/src/*.rs

echo '-- the release profile that decides the overflow behaviour --'
# absence is the point: there is no [profile.release] to set overflow-checks in
command grep -c 'profile.release\|overflow-checks' Cargo.toml \
  | sed 's/^/   keys present in the workspace manifest: /'

echo '-- and both halves of PB40, as they read now --'
command grep -m1 -A4 -F '  /// Panics on overflow in a debug build and wraps to zero in a release' ring_types/src/id.rs
command grep -m1 -A5 -F '  /// Overflow behaves exactly as [`Seq::next`] documents' ring_types/src/id.rs
command grep -c 'saturates in a release build' ring_types/src/id.rs \
  | sed 's/^/   occurrences of the original claim: /'
```

Live output:

```
-- every widening cast in the family, and this crate's one --
     27 as u64
     15 as usize
let end = start.advanced_by( len as u64 );
/// just as long, and that one is not the waiting caller's bug at all. The
-- the derive census --
89
88
      3 #[ derive( Debug, Default ) ]
-- who pays for Sync, and who gets it free --
ring_mpsc/src/lib.rs
ring_spsc/src/lib.rs
ring_mpsc/src/lib.rs
ring_spsc/src/lib.rs
-- the release profile that decides the overflow behaviour --
   keys present in the workspace manifest: 0
-- and both halves of PB40, as they read now --
  /// Panics on overflow in a debug build and wraps to zero in a release
  /// build — the standard `u64` addition behaviour. A wrapped `Seq` would
  /// silently invert every gate comparison in the family, which is why the
  /// non-wrapping argument has to hold: at 10⁹ publications per second a
  /// `u64` runs for roughly 584 years, well past any reachable workload.
  /// Overflow behaves exactly as [`Seq::next`] documents — debug panics,
  /// release wraps to zero. The reachability argument does not carry over
  /// unchanged: `next` needs 2⁶⁴ increments to reach the wrap, while this
  /// takes `n` from the caller and reaches it in a single call from any
  /// position. A caller deriving `n` from a batch length or a configured
  /// count owns that bound; nothing here checks it.
   occurrences of the original claim: 0
```

| | Value |
|--|------:|
| Distinct types in the public surface | 5 |
| Types this crate defines | 1 — `Publisher` |
| Generic parameters, lifetimes, trait bounds | 0 |
| `as` conversions in `src/lib.rs` | **1** |
| Casts family-wide | 42, on 41 lines, across 14 crates |
| …widening (`usize → u64`) | 27 |
| …potentially narrowing (`u64 → usize`) | 11 |
| …pointer-to-integer | 1 |
| …usize → usize, no-op | 1 |
| …u32 → usize | 2 |
| Derive attributes family-wide | 89 |
| …including `Debug` | **88 (99%)** |
| …that are exactly `Debug, Default` | 3 |
| …that are exactly `Debug` alone | 39 |
| `unsafe impl … Sync` in the family | 2 |
| Crates opting out of `unsafe-code = "deny"` | 2 |
| Manual checks covering the type surface | **0 of 6** |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB39 | family | n/a — observation | The family casts 42 times across 14 crates; `ring_publish` casts once, and only in the widening direction, while eleven `u64 as usize` casts elsewhere would truncate on a 32-bit target |
| PB40 | `ring_types` | **wrong doc** | `advanced_by`, the method `try_publish` calls, documents no overflow behaviour, while its structurally identical sibling `next` documents it and gets it wrong twice: it names saturation, which Rust never does, and disclaims the wrapping that release builds actually perform |
| PB41 | family | n/a — observation | 88 of the family's 89 derive attributes include `Debug`, driven by a lint rather than a convention; the one exception hand-writes `Debug` instead so the lint is satisfied without it; only three derives are exactly `Debug, Default`, and two of the three are this crate and the type it wraps |
| PB42 | `ring_publish` | n/a — unenforced | Every absent trait is absent because `AtomicU64` implements none of them, not because of a decision here; a *manual* `Clone` would compile and would reach [`pitfall/002`](../pitfall/002_conflating_the_two_cursors.md)'s corruption by a fresh route |
