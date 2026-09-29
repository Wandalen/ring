# Invariant: Monotonicity Is Relied On Here and Required Nowhere

### Scope

**Purpose:** Record the property the family's gates depend on, what this crate —
the one that owns every cursor mutation — requires of a caller regarding it, and
where the gap is caught instead.

**Responsibility:** `SeqCell`'s three mutating methods and their contracts, the
family's monotonicity statements, and `ring_debug`'s detector.

**In Scope:** `ring_atomic/src/lib.rs:113-114`, `:116-141`, `:143-150`;
`Seq::next`'s overflow paragraph in `ring_types/src/id.rs` — named rather
than numbered, because that paragraph was rewritten and the numbers it used to
carry moved; `ring_consume/src/lib.rs:31-35`; `ring_debug/src/lib.rs:161-174`,
`:328-340`.

**Out of Scope:** What happens at `u64::MAX` specifically is
[`pitfall/002`](../pitfall/002_the_wrap_that_reads_as_an_empty_ring.md). The
counting property the suite does establish is
[`invariant/002`](002_every_increment_survives.md).

---

## What Is Relied On, and What Is Required

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what the family relies on --'
# re-anchored: this stage used to grep "wrapping, since a wrapped `Seq` would
# silently violate the monotonicity", which was rewritten out of ring_types and
# left the stage printing nothing — a hole a byte-equality gate cannot see,
# because an empty stage reproduces itself perfectly
command grep -m1 -A1 -F '  /// silently invert every gate comparison in the family, which is why the' ring_types/src/id.rs
command grep -m1 -A2 -F '//! ## Why commit is monotonic and clamped' ring_consume/src/lib.rs
echo '  -- what the primitive requires --'
command grep -m1 -A1 -F '  /// Overwrite the sequence.' ring_atomic/src/lib.rs
command grep -m1 -A2 -F '  /// Advance by `n` and return the sequence as it was *before* the advance —' ring_atomic/src/lib.rs
echo '  -- and the crate that exists because neither enforces it --'
command grep 'CursorWentBackwards' ring_debug/src/lib.rs | head -3
echo '  -- and the word that used to be absent from the crate that owns the advance --'
printf '    monotonic mentions in ring_atomic/src : %s\n' "$( command grep -ci 'monotonic' ring_atomic/src/lib.rs )"
command grep -m1 -A3 -F '  /// # Monotonicity' ring_atomic/src/lib.rs
```

Live output:

```
  -- what the family relies on --
  /// silently invert every gate comparison in the family, which is why the
  /// non-wrapping argument has to hold: at 10⁹ publications per second a
//! ## Why commit is monotonic and clamped
//!
//! [`Consumer::commit`] refuses to move backwards and refuses to move past what
  -- what the primitive requires --
  /// Overwrite the sequence.
  fn store( &self, value : Seq, order : Ordering );
  /// Advance by `n` and return the sequence as it was *before* the advance —
  /// which is the first sequence the caller now owns.
  ///
  -- and the crate that exists because neither enforces it --
  CursorWentBackwards
      Self::CursorWentBackwards { cursor, was, now } => write!
///   Err( Violation::CursorWentBackwards { cursor : Cursor::Producer, was : Seq( 7 ), now : Seq( 2 ) } )
  -- and the word that used to be absent from the crate that owns the advance --
    monotonic mentions in ring_atomic/src : 3
  /// # Monotonicity
  ///
  /// **Not guaranteed by this method.** `n` is a `u64` because the cell is a
  /// `u64`, and the addition wraps: `fetch_add( u64::MAX )` moves the cursor
```

---

### AT21 — All Three Mutating Methods Move a Cell Backwards, Including the One Called `fetch_add`

`store` is documented in four words — "Overwrite the sequence." — with no
constraint, and it is the primitive by which every cursor commit in the family
moves: `ring_consume::commit`, `ring_mpsc`'s publish and commit, `ring_spsc`'s
handoff. `compare_exchange` will swap any value for any other. And `fetch_add`
takes an unsigned `u64` that wraps:

```
  store( Seq( 7 ) ) on a cell at 100      : cell now Seq(7)
  fetch_add( u64::MAX ) on a cell at 100  : returned Seq(100), cell now Seq(99)
  compare_exchange( 100 -> 3 )            : Ok(Seq(100)), cell now Seq(3)
  three ways backwards, zero refusals
```

**Finding.** The `fetch_add` line is the one that matters. Its name says advance,
its doc says "Advance by `n`", and its return value is documented as "the first
sequence the caller now owns" — and `fetch_add( u64::MAX )` moves the cursor back
by one while returning a value indistinguishable from a legitimate claim of a huge
range. Nothing in the signature, the type, or the contract rules it out; `n` is a
`u64` because the cell is a `u64`, and the reverse direction comes free with that
choice.

The family states the dependency plainly in at least two other crates.
`ring_types` says a wrapped `Seq` "would silently invert every gate comparison in
the family" — it said "would silently violate the monotonicity every gate in the
family relies on" when this instance was written, and the surrounding sentence
was corrected since, from a false claim that the addition saturates to the true
one that it wraps; the dependency it names survived the correction unchanged.
`ring_consume` devotes a module section to why its commit is monotonic and
clamped. The crate that owns every operation those statements are about said
nothing about monotonicity at all — the word did not appear in it.

**Disposition:** applied — to the contract only, and the asymmetry is the point.
`fetch_add`'s declaration in `src/lib.rs` now carries a `# Monotonicity` section
that opens **"Not guaranteed by this method"** and names the exact mechanism —
`n` is a `u64` because the cell is a `u64`, the addition wraps, and
`fetch_add( u64::MAX )` moves the cursor back one while returning a value shaped
exactly like a legitimate claim — carrying a `Fix(AT21, AT41, AT42)` line, and the
census above now reads three mentions where it read zero. `store` and
`compare_exchange` were deliberately left as they are: both are *supposed* to move
a cell anywhere, that is what a cursor commit needs, and documenting them as
monotonicity hazards would misstate their job. What this does not buy: a contract
saying "not guaranteed" is still not a guarantee. Nothing in the signature, the
type, or the compiler rejects `fetch_add( u64::MAX )`; `ring_debug`'s
`CursorWentBackwards` watcher remains the family's only actual detection, it runs
after the fact, and it is opt-in — so this entry's title is still accurate and the
change moves the gap from undocumented to documented, not from open to closed. Now
prints: `    monotonic mentions in ring_atomic/src : 3`

---

### AT22 — The Family's Answer Is a Watcher, Not a Type

`ring_debug` exists in part for this. It defines `Violation::CursorWentBackwards`,
a `Watch` that samples a cursor pair and reports the violation after it has
happened, and a doctest that produces one on purpose:

> ```
> pair.producer().store( Seq( 5 ), Ordering::Release );
> let mut watch = Watch::new( &pair ).expect( "a valid pair" );
> pair.producer().store( Seq( 7 ), Ordering::Release );
> assert!( watch.observe( &pair ).is_ok(), "forward is fine" );
> pair.producer().store( Seq( 2 ), Ordering::Release );
> ```

Five, then seven, then two, through this crate's `store`, with no complaint from
any layer below the watcher.

**Finding.** The design is coherent and worth naming as a design rather than a
gap: a primitive that refused non-monotonic stores could not implement
`ring_consume`'s clamping (which needs to compute the clamped value and then write
it) and could not implement a reset, and the cost of a comparison on every cursor
write is exactly the cost this crate exists to avoid
([`decisions/002`](../decisions/002_a_trait_because_the_criteria_needed_two.md)).
Detection one tier up, opt-in, is a reasonable place to put it.

What is missing is any statement of that reasoning where a caller of this crate
would find it. The trait's four-word `store` contract implies no obligation; the
`fetch_add` contract implies the opposite of what `u64::MAX` does; and the fact
that a whole crate exists to catch violations is discoverable only from that
crate. A reader who wants to know what must be true of a `SeqCell` finds no
invariant section, no `# Panics`, and no `# Safety`-style contract anywhere in the
crate — in the one crate whose stated purpose is to make the family's memory model
"one thing that can be read in one place".

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/002`](002_every_increment_survives.md) | The property the crate does hold and the suite does test |
| [`pitfall/002`](../pitfall/002_the_wrap_that_reads_as_an_empty_ring.md) | The same wrap reached by counting up rather than by passing `u64::MAX` |
| [`api/001`](../api/001_the_return_value_that_is_a_claim.md) | `fetch_add`'s return value, and the guard it does not carry either |
| [`decisions/001`](../decisions/001_orderings_named_never_defaulted.md) | The crate's stated position on what it does and does not decide for a caller |

### Sources

| Fact | Where |
|------|-------|
| `store`'s four-word contract | `ring_atomic/src/lib.rs:113-114` |
| `fetch_add`'s advance contract | `ring_atomic/src/lib.rs:116-117` |
| The family's monotonicity statements | `ring_types/src/id.rs:36-37`; `ring_consume/src/lib.rs:31-35` |
| The violation variant and its doctest | `ring_debug/src/lib.rs:161-174`, `:328-340` |
| Three backwards moves, no refusal | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `store_overwrites_and_load_observes` | That `store` overwrites — the mechanism, not the constraint |
| `compare_exchange_succeeds_on_the_expected_value_and_reports_the_actual_otherwise` | Both exchange paths, with no ordering constraint on either value |
| *(to create)* | Nothing in this crate asserts what a cell may not be moved to, because nothing forbids anything |
