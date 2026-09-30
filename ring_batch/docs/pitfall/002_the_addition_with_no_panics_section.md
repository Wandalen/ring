# Pitfall: The Addition With No Panics Section

### Scope

**Purpose:** Record what the crate's single arithmetic operator does when it
overflows, what the type reports about itself afterwards, and the family-wide doc
comment that describes the opposite behaviour.

**Responsibility:** `BatchClaim::end`'s `+`, the four methods that route through
it, and `ring_types::Seq`'s account of the same operator.

**In Scope:** `BatchClaim::end`'s body and its `# Panics` section in
`ring_batch/src/lib.rs`; `Seq::next`, `Seq::advanced_by` and
`Seq::distance_to` in `ring_types/src/id.rs`, with their overflow
paragraphs — named rather than numbered, because correcting those paragraphs is
what moved every line number this section used to carry.

**Out of Scope:** `end()`'s doc claim about the cursor is
[`item/002`](../item/002_one_past_the_end.md) BA28. The gate race is
[`pitfall/001`](001_the_window_between_the_gate_and_the_advance.md).

---

## One Operator, No Documentation

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the crate one addition --'
command grep -m1 -A3 -F '  pub const fn end( &self ) -> Seq' ring_batch/src/lib.rs
echo '  -- what the crate says about overflowing it --'
command grep '# Panics\|overflow\|wrapping_\|saturating_\|checked_' ring_batch/src/lib.rs || echo '    (nothing)'
```

Live output:

```
  -- the crate one addition --
  pub const fn end( &self ) -> Seq
  {
    Seq( self.start.0 + self.count as u64 )
  }
  -- what the crate says about overflowing it --
  /// # Panics
  /// In a debug build, if `start.0 + count` overflows `u64` — unreachable via
///   its overflow policy.
```

The single hit is `claim_gated`'s `# Errors` section telling a caller to apply
its ring-overflow policy on `Full` — a different sense of the word entirely. The
crate has no `# Panics` section anywhere, and never mentions `wrapping_add`,
`saturating_add`, or `checked_add`.

---

### BA44 — In Release the Type Contradicts Itself

Measured. `BatchClaim::new( Seq( u64::MAX ), 1 )` — a claim of one sequence,
positioned at the last sequence there is:

```
=== debug ===
--- a claim of one sequence, at the last sequence there is ---
  the two fields, read directly
    start()      Seq(18446744073709551615)
    len()        1
    is_empty()   false
  everything that routes through end()

thread 'main' (2126323) panicked at .../ring_batch/src/lib.rs:131:10:
attempt to add with overflow
=== release ===
--- a claim of one sequence, at the last sequence there is ---
  the two fields, read directly
    start()      Seq(18446744073709551615)
    len()        1
    is_empty()   false
  everything that routes through end()
    end()                     Seq(0)
    contains( its own start ) false
    sequences().count()       0
    overlaps( itself )        false
```

**Finding.** Debug panics at `lib.rs:131:10` with "attempt to add with overflow",
from a `const fn` with no `# Panics` section. Release is worse than a panic: the
addition wraps to `Seq( 0 )` and the type splits into two halves that disagree.

`len()` reads `count` directly and says 1. `is_empty()` reads `count` directly
and says false. Every method that routes through `end()` then behaves as though
the claim were empty — it contains nothing, not even its own start; it yields no
sequences; it does not overlap itself. A caller holding this value is told, by
the same object, that it owns one sequence and that the sequence it owns is not
in it.

Reachability is the interesting part. The cursor will not get here — at one claim
per nanosecond, 2⁶⁴ sequences is 584 years. But `BatchClaim::new` is public and
`const`, takes a `Seq` and a `usize` with no validation, and
[`api/001`](../api/001_twelve_items_seven_must_use.md) records it as one of
the crate's eleven callable items. A test fixture, a fuzz harness, or a caller
reconstructing a claim from a serialized cursor reaches this in one line, and the
release build's answer is silence.

`end()` now carries the `# Panics` section its own title says is missing:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A3 -F '  /// In a debug build, if `start.0 + count` overflows' ring_batch/src/lib.rs
```

Live output:

```
    /// In a debug build, if `start.0 + count` overflows `u64` — unreachable via
    /// the cursor in practice (2⁶⁴ sequences at one claim per nanosecond is 584
    /// years) but reachable in one line through [`BatchClaim::new`], which is
    /// public, `const`, and takes both fields unvalidated. In a release build
```

**Disposition:** applied — added a `# Panics` section to `end()`'s doc comment
in `src/lib.rs`, stating both halves measured above: the debug panic and its
site, and that release wraps rather than panicking and leaves `contains`,
`sequences`, and `overlaps` all reporting the claim as empty. This is a
documentation fix, not a behavior change — switching the arithmetic to
`checked_add`/`saturating_add` would be a breaking API change to a `const fn`
four other methods route through, and is a separate design decision from
naming the behavior the type already has. The crate's 21 unit tests plus 10
doctests re-verified passing (`cargo test --all-features`, 2026-09-04). Now
prints:
`and takes both fields unvalidated. In a release build`

---

### BA45 — `ring_types` Documented the Behaviour It Does Not Have

The same `+` on the same field appears twice in `ring_types`. When this finding
was filed, one doc comment covered both, and it read:

```text
  /// Panics on overflow in a debug build and saturates in a release build,
  /// deliberately not wrapping: a wrapped `Seq` would silently violate the
  /// monotonicity every gate in the family relies on.
```

That sentence is gone from the source. The recipe below is anchored on the text
that replaced it rather than on the text this finding was about, because a
recipe anchored on a deleted string reproduces *emptiness* perfectly and the
recipe/quote gate cannot tell that apart from a stage that correctly has nothing
to say — which is exactly what this stage did until it was re-anchored:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what ring_types says about the operator now --'
command grep -m1 -A4 -F '  /// Panics on overflow in a debug build and wraps to zero in a release' ring_types/src/id.rs
command grep -m1 -A5 -F '  /// Overflow behaves exactly as [`Seq::next`] documents' ring_types/src/id.rs
echo '  -- and the two bodies it says it about --'
command grep -m1 -A3 -F '  pub const fn next( self ) -> Self' ring_types/src/id.rs
command grep -m1 -A3 -F '  pub const fn advanced_by( self, n : u64 ) -> Self' ring_types/src/id.rs
echo '  -- next to one that really does saturate --'
command grep -m1 -A3 -F '  pub const fn distance_to( self, later : Self ) -> u64' ring_types/src/id.rs
echo '  -- and the word this finding was about, nowhere near the two additions --'
command grep -c 'saturates in a release build' ring_types/src/id.rs \
  | sed 's/^/    occurrences of the original claim: /'
```

Live output:

```
  -- what ring_types says about the operator now --
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
  -- and the two bodies it says it about --
  pub const fn next( self ) -> Self
  {
    Self( self.0 + 1 )
  }
  pub const fn advanced_by( self, n : u64 ) -> Self
  {
    Self( self.0 + n )
  }
  -- next to one that really does saturate --
  pub const fn distance_to( self, later : Self ) -> u64
  {
    later.0.saturating_sub( self.0 )
  }
  -- and the word this finding was about, nowhere near the two additions --
    occurrences of the original claim: 0
```

**Finding.** Three claims in four lines, and the middle one was false. Rust's
release behaviour for `+` on an overflowing `u64` is two's-complement wrapping,
not saturation — `Seq( u64::MAX ).next()` yields `Seq( 0 )`, exactly as
`ring_batch::end()` does above, and exactly what the doc's own next clause said
must never happen. Saturating would have given `Seq( u64::MAX )`.

The debug half of the sentence was right, and the *reason* was right and
important: "a wrapped `Seq` would silently violate the monotonicity every gate in
the family relies on" is a correct statement of the stakes. What was wrong was
the claim that the code already prevents it. It does not; `Self( self.0 + 1 )` is
the wrap.

Two details made this worth correcting rather than shrugging at. `distance_to`,
further down the same `impl` block, uses a real `saturating_sub` — so the
vocabulary was in hand and the choice on `next` was not a deliberate one. And
`ring_batch::end()` is the same construct written a third time, one crate up,
which at the time carried no doc comment at all; a reader who checked what
`Seq`'s own arithmetic promised would find that paragraph and reasonably conclude
the addition was safe.

The measurement above is the family's `Seq` overflowing through a `ring_batch`
method, so this was never a hypothetical about `ring_types` in isolation.

**Disposition:** applied — elsewhere, and the reason this instance recorded for
declining has since expired. The original disposition was *declined* on scope
grounds — the wrong claim lived in `ring_types/src/id.rs`, a different
crate from `ring_batch`, and editing another crate's source was out of that
pass's scope. That reasoning was sound and the finding was correct; what it
could not do was fix it. Both halves have since been repaired in `ring_types`,
and the false claim's own text is gone from it. Now prints: `    occurrences of the original claim: 0`

- `next`'s paragraph now says "wraps to zero in a release build", and keeps the
  monotonicity argument as the *reason the wrap is survivable* (2⁶⁴ increments,
  584 years at 10⁹/s) rather than as a claim about what the arithmetic does.
- `advanced_by`, which this finding named as the second body the sentence covered,
  had no overflow note at all until one was written. It now states that the
  reachability argument does **not** carry over from `next`: `n` comes from the
  caller and reaches the wrap in a single call from any position. That is the
  same shape as BA44's own finding about `BatchClaim::new` — a public entry point
  that reaches the boundary in one line — one crate down.

What this instance still owns is unchanged and still open: `ring_batch::end()`'s
own `# Panics` section (BA44, applied) documents the behaviour, and no test in
this crate constructs a claim at the boundary. The two `*(to create)*` rows in
§ Tests below are the residue.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](../item/002_one_past_the_end.md) | The other thing `end()`'s four lines get wrong |
| [`api/001`](../api/001_twelve_items_seven_must_use.md) | `BatchClaim::new` as a public, unvalidated entry point |
| [`pitfall/001`](001_the_window_between_the_gate_and_the_advance.md) | The crate's other latent hazard, in the other half of the API |
| [`invariant/002`](../invariant/002_ascending_not_contiguous.md) | The monotonicity a wrapped `Seq` would break |

### Sources

| Fact | Where |
|------|-------|
| The addition, and the crate's silence about it | `BatchClaim::end` in `ring_batch/src/lib.rs`; census above |
| Debug panic site and release values | Probe, quoted above |
| The doc that claimed saturation | `Seq::next`'s overflow paragraph in `ring_types/src/id.rs` — since rewritten; the original text is quoted as history under BA45 |
| The two plain-`+` bodies | `Seq::next` and `Seq::advanced_by`, same file, both printed by BA45's recipe |
| The method that does saturate | `Seq::distance_to`, same file, printed by the same recipe |

### Tests

| Test | Covers |
|------|--------|
| `a_claim_reports_its_own_extent` | `end()` in the range where it works |
| *(to create)* | `BatchClaim::new( Seq( u64::MAX ), 1 )` — nothing in the family constructs a claim near the boundary |
| *(to create)* | `Seq( u64::MAX ).next()` and `Seq( 1 ).advanced_by( u64::MAX )` — the two additions BA45 is about, neither of which any test in the family reaches; `ring_types`' own `seq_does_not_wrap_within_any_reachable_workload` calls neither |
