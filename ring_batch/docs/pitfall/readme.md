# pitfall

Both of this crate's hazards are in the two places it does something other than
hand back a `fetch_add`. The gated entry point reads the consumer's position and
then advances the cursor in a second, unlinked operation, so several producers can
be granted the same free slots — measured at roughly one round in two hundred at
sixteen contending producers, with the excess a whole batch each time. And
`end()`'s `+` is the crate's only arithmetic, and when these were filed it
carried no `# Panics` section: it panics in debug and wraps in release, leaving a
claim that reports a length of one and denies containing its own start. The
section now exists and says exactly that; the arithmetic is unchanged.

Neither is exposed today. `claim_gated` has no caller outside this crate's own
tests, and no cursor will reach 2⁶⁴. Both are reachable through the public API in
one line.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_window_between_the_gate_and_the_advance.md) | The Window Between the Gate and the Advance | The overrun rates under contention, and the CAS loop three tiers up that names the failure mode |
| [002](002_the_addition_with_no_panics_section.md) | The Addition With No Panics Section | Debug panic, release wrap, and the `ring_types` doc that claimed saturation until it was corrected |

## A Check In Front of an Unconditional Operation Is Advice

`fetch_add` cannot refuse. That is why it costs one operation regardless of
count, and it is why a gate placed before it can only ever be a hint: by the time
the addition runs, the values the gate read may be several producers stale. The
crate's own doc points the other way — `claim`'s "no gating … `claim_gated` is
the form that checks first" reads as though checking first were the property that
prevents overrun.

`ring_claim::Claimer::claim` solves it by making the advance refusable — a
`compare_exchange` whose failure re-enters a loop that re-reads the gate — and
carries a four-line comment naming this exact hazard. Neither crate references
the other.

## The Same Operator, Documented Three Ways

`Seq( self.start.0 + self.count as u64 )` here now carries a `# Panics` section
(BA44, applied). `Seq::next` and `Seq::advanced_by` are the same `+` one crate
down, and when BA45 was filed one paragraph covered both, stating the addition
"saturates in a release build … deliberately not wrapping, since a wrapped `Seq`
would silently violate the monotonicity every gate in the family relies on."
Measured through this crate, it wraps: `Seq( u64::MAX )` plus one is `Seq( 0 )`.
Further down the same `impl` block, `distance_to` uses a genuine
`saturating_sub`, so the vocabulary was available and the choice was not made.

Both halves have since been corrected in `ring_types`, and the recipe below is
re-anchored on the replacement text — its `ring_types` stage was anchored on the
deleted sentence and had gone silent, which is invisible to a byte-equality gate
because an empty stage reproduces perfectly.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the gate, and the advance it does not hold across --'
command grep -m1 -A7 -F '  let at = producer.load( Ordering::Acquire );' ring_batch/src/lib.rs
echo '  -- the loop that closes the same window, three tiers up --'
command grep -m1 -A3 -F '    // The gate is the loop condition, and is therefore re-read on every' ring_claim/src/lib.rs
echo '  -- the crate one addition, and what it promises --'
command grep -m1 -F '    Seq( self.start.0 + self.count as u64 )' ring_batch/src/lib.rs
command grep '# Panics\|wrapping_\|saturating_\|checked_' ring_batch/src/lib.rs || echo '    (no # Panics, no checked arithmetic, anywhere)'
echo '  -- and what ring_types claims about the same operator, since corrected --'
command grep -m1 -A1 -F '  /// Panics on overflow in a debug build and wraps to zero in a release' ring_types/src/id.rs
command grep -m1 -A1 -F '  /// Overflow behaves exactly as [`Seq::next`] documents' ring_types/src/id.rs
command grep -m1 -F '    Self( self.0 + 1 )' ring_types/src/id.rs
echo '  -- the deleted claim this section was filed about --'
command grep -c 'saturates in a release build' ring_types/src/id.rs \
  | sed 's/^/    occurrences: /'
```

Live output:

```
  -- the gate, and the advance it does not hold across --
  let at = producer.load( Ordering::Acquire );
  let behind = consumer.load( Ordering::Acquire );
  if ( free_slots( at, behind, capacity ) as usize ) < count
  {
    return Err( RingError::Full );
  }

  Ok( claim( producer, count, order ) )
  -- the loop that closes the same window, three tiers up --
    // The gate is the loop condition, and is therefore re-read on every
    // iteration: on a failed exchange another producer moved the cursor, so
    // the headroom computed against the old value is stale and granting on it
    // would overlap that producer's range.
  -- the crate one addition, and what it promises --
    Seq( self.start.0 + self.count as u64 )
  /// # Panics
  -- and what ring_types claims about the same operator, since corrected --
  /// Panics on overflow in a debug build and wraps to zero in a release
  /// build — the standard `u64` addition behaviour. A wrapped `Seq` would
  /// Overflow behaves exactly as [`Seq::next`] documents — debug panics,
  /// release wraps to zero. The reachability argument does not carry over
    Self( self.0 + 1 )
  -- the deleted claim this section was filed about --
    occurrences: 0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BA42 | `ring_batch` | **latent hazard** | The gate and the advance are separate operations, so the cursor passes `consumer + capacity` in roughly one round in two hundred at 16 producers, by a whole batch each time; `claim`'s doc offers the gated form as the one that does not outrun the ring |
| BA43 | `ring_claim` | n/a — duplication | `Claimer::claim` closes the identical window with a `compare_exchange` retry loop and a four-line comment naming the failure mode; neither crate references the other |
| BA44 | `ring_batch` | **latent hazard** | `end()` panics in debug at `lib.rs:131:10` and wraps in release, after which `len()` says 1, `is_empty()` says false, and every method routing through `end()` behaves as though the claim were empty; when filed, no `# Panics` section existed anywhere in the crate. **Doc half applied** — `end()` now carries one, printed by the recipe above; the behaviour is unchanged and no test constructs a claim at the boundary |
| BA45 | `ring_types` | **wrong doc** | `Seq::next`'s doc said the addition "saturates in a release build … deliberately not wrapping"; `next` and `advanced_by` are plain `+`, which wraps, and `distance_to` further down the same `impl` block uses a real `saturating_sub`. **Since corrected** — `next` now says "wraps to zero", and `advanced_by`, which had no overflow note at all, now says the reachability argument does not carry over to it |
