# Pitfall: The Wrap That Reads as an Empty Ring

### Scope

**Purpose:** Record what `AtomicSeq::fetch_add` does at the top of `u64`, why the
family's guard against exactly that does not stand on this path, and what every gate
downstream reports afterwards.

**Responsibility:** The overflow behaviour of the crate's own advance method, and the
chain from a wrapped cursor to a granted claim.

**In Scope:** `ring_atomic/src/lib.rs:223-226`;
`ring_types/src/id.rs:32-47`, `:70-85`; `ring_seqno/src/lib.rs:73-99`;
`ring_gating/src/lib.rs:221-245`, `:283`.

**Out of Scope:** Reaching the same wrap in one call by passing a large `n` is
[`invariant/001`](../invariant/001_monotonicity_is_relied_on_and_not_required.md)
AT21 — this instance is about arriving there by counting. The absent `Sync`
supertrait is [`type/001`](../type/001_what_the_trait_promises.md).

---

## The Guard, the Saturation, and What the Crate Says

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the guard the family built, and the path it sits on --'
command grep -m1 -A15 -F '  /// The next sequence after this one.' ring_types/src/id.rs
echo '  -- the two saturations between the wrap and the gate, and their reasons --'
awk '/^  \/\/\/ Saturating rather than signed: the caller that needs the direction has$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 1 { print } /^  \/\/\/ assert_eq!\( Seq\( 10 \)\.distance_to\( Seq\( 4 \) \), 0 \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 6 { print }' ring_types/src/id.rs
sed -n '/^\/\/\/ Zero when the ring is full; never negative, because a producer that appears$/,/^\/\/\/ exclude\.$/p;/^pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize$/,/^}$/p' ring_seqno/src/lib.rs
echo '  -- what ring_atomic, which owns the path traffic takes, says about it --'
printf '    overflow/wrap/saturat mentions in src   : %s\n' \
  "$( command grep -ciE 'overflow|wrap|saturat' ring_atomic/src/lib.rs || true )"
printf '    overflow/wrap/saturat mentions in tests : %s\n' \
  "$( command grep -ciE 'overflow|wrap|saturat' ring_atomic/tests/atomic_test.rs || true )"
printf '    u64::MAX mentions in src or tests       : %s\n' \
  "$( command grep -rc 'u64::MAX' ring_atomic/src/lib.rs \
      ring_atomic/tests/atomic_test.rs | cut -d: -f2 | paste -sd+ | bc )"
echo '  -- and how long counting alone would take to reach it --'
awk 'BEGIN{ m=18446744073709551615; printf "    at 1e9 fetch_adds/sec : %.0f years\n", m/1e9/31557600 }'
```

Live output:

```
  -- the guard the family built, and the path it sits on --
  /// The next sequence after this one.
  ///
  /// Panics on overflow in a debug build and wraps to zero in a release
  /// build — the standard `u64` addition behaviour. A wrapped `Seq` would
  /// silently invert every gate comparison in the family, which is why the
  /// non-wrapping argument has to hold: at 10⁹ publications per second a
  /// `u64` runs for roughly 584 years, well past any reachable workload.
  ///
  /// ```
  /// use ring_types::Seq;
  /// assert_eq!( Seq( 41 ).next(), Seq( 42 ) );
  /// ```
  #[ must_use ]
  pub const fn next( self ) -> Self
  {
    Self( self.0 + 1 )
  -- the two saturations between the wrap and the gate, and their reasons --
  /// Saturating rather than signed: the caller that needs the direction has
  /// already compared the two, and every caller that does not wants a count.
  #[ must_use ]
  pub const fn distance_to( self, later : Self ) -> u64
  {
    later.0.saturating_sub( self.0 )
  }
/// Zero when the ring is full; never negative, because a producer that appears
/// to be behind its consumer is a state the family's monotonic sequences
/// exclude.
pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
{
  let in_flight = consumer.distance_to( producer );
  ( capacity.get() as u64 ).saturating_sub( in_flight ) as usize
}
  -- what ring_atomic, which owns the path traffic takes, says about it --
    overflow/wrap/saturat mentions in src   : 2
    overflow/wrap/saturat mentions in tests : 18
    u64::MAX mentions in src or tests       : 19
  -- and how long counting alone would take to reach it --
    at 1e9 fetch_adds/sec : 585 years
```

---

### AT41 — The Family Built a Guard Against This and Put It on the Path Production Does Not Take

`Seq::next` was written as though it refuses to wrap, and its doc comment states
the stakes exactly right: "a wrapped `Seq` would silently violate the monotonicity
every gate in the family relies on" — wording since revised to "would silently
invert every gate comparison in the family", same claim. That is the correct
analysis of the hazard. It is written on the wrong function, and the sentence
around it was itself wrong about what `next` does: it claimed saturation where
release-mode `+` wraps. The claim has since been corrected in `ring_types`; the
stakes sentence survived intact, and neither change moves the guard onto the path
production takes.

Sequences in production do not advance through `Seq::next`. They advance through
`AtomicSeq::fetch_add`, which delegates to `AtomicU64::fetch_add`, which is *defined*
to wrap and carries no debug assertion — so the guard's protection is exactly absent
from the one method the write path actually calls. Driven to the boundary in both
profiles:

```
  -- profile: debug --
  AtomicSeq::fetch_add on Seq( u64::MAX )
    before   18446744073709551615
    returned 18446744073709551615
    after    0   monotonic: false
  Seq::next on the same value      panicked
```

```
  -- profile: release --
  AtomicSeq::fetch_add on Seq( u64::MAX )
    before   18446744073709551615
    returned 18446744073709551615
    after    0   monotonic: false
  Seq::next on the same value      0   (no panic)
```

Two things are visible here. The atomic path wraps identically in both profiles, so
a debug build — the one place the family's guard has teeth — provides no warning at
all on the path that matters. And `Seq::next` in release yields `0`, not `u64::MAX`:
its own doc comment said it "saturates in a release build", and release wraps. The
measurement is unchanged and the doc has since been corrected to say so.

**Finding.** `ring_atomic` owns the only advance production uses and said nothing
about its boundary: zero mentions of overflow, wrap, or saturation in the source,
zero in the test file, and no occurrence of `u64::MAX` in either. The two crates that
did think about the problem, `ring_types` and `ring_seqno`, are one dependency edge
away and neither can see this path.

The horizon is genuinely distant — 585 years of counting at a billion advances per
second — and that is a real reason not to add a runtime check. It is not a reason
for silence. `AtomicSeq::new` accepts any `Seq` a caller supplies and `fetch_add`
accepts any `n`, so the state is reachable by construction in a single call by anyone
seeding a cursor from persistence or from a test fixture, and there was nothing in the
crate to read that would warn them.

**Disposition:** applied — as prose and as a test, on the reachable half of the hazard.
`fetch_add`'s `# Monotonicity` section in `src/lib.rs` now states that the addition
wraps and that `fetch_add( u64::MAX )` moves the cursor back one, and
`fetch_add_wraps_at_the_top_of_u64` in `tests/atomic_test.rs` drives both cells
across the boundary and asserts the wrap rather than leaving it to be inferred from
`AtomicU64`'s own documentation — which is why the census reads two mentions in the
source and eighteen in the tests where all three read zero. The seeding path AT41
identifies as the reachable one is what the test exercises: it constructs cells at
`u64::MAX - 2` and at `u64::MAX` directly, which is the single call a fixture or a
restored-from-persistence cursor makes. What this does not buy: the guard is still
on the path production does not take; this change documents and tests the path
production does take, it does not move the guard.
Now prints: `    overflow/wrap/saturat mentions in src   : 2`

This entry also recorded, as an open half, that `Seq::next`'s doc still claimed it
"saturates in a release build" when the measurement two blocks above shows it
yields `0`, and that the sentence was in `ring_types`, one edge away and out of
this crate's reach. **That half has since closed** — `next`'s paragraph now says
"wraps to zero in a release build", and `advanced_by`, which had no overflow note
at all, now carries one saying the reachability argument does not carry over to it.
Neither edit changes what this crate measures or what its guard covers.

---

### AT42 — Past the Wrap, Backpressure Does Not Degrade; It Inverts

The interesting part is not that the cursor wraps. It is what the rest of the family
concludes from a wrapped cursor — because the answer is not "an error" or "nonsense",
it is "the ring is completely empty, please continue."

A 64-slot ring with both ends near the top of `u64`, driven two laps forward while
the consumer stands still:

```
  -- a ring of 64 slots, both ends near the top of u64 --
                 producer               consumer  in flight  free
     18446744073709551515   18446744073709551515          0    64
     18446744073709551579   18446744073709551515         64     0
                       27   18446744073709551515          0    64
```

The middle row is correct backpressure: 64 in flight, 0 free, the ring full and
refusing. The next row is the same ring one lap later, and every number in it is a
lie. Asked directly, every gate the family owns agrees:

```
  -- the consumer never moved; every gate now reads the ring as empty --
    consumer.distance_to( producer )   0
    ring_seqno::free_slots               64 of 64
    ring_seqno::may_claim                true
    GatingSet::headroom                64
    GatingSet::admits( producer, 64 )  true
    GatingSet::check( producer, 64 )   Ok(())
```

**Finding.** Two further saturations stand between the wrapped cursor and the gate,
each deliberate and each carrying its own written reason — and they fail in opposite
directions, which is what decides the outcome.

The outer one, in `free_slots`, fails **safe**: `capacity.saturating_sub( in_flight )`
clamps at zero, so an absurdly large in-flight count would report zero free and the
ring would jam visibly at the first gate. Its stated reason is that "a producer that
appears to be behind its consumer is a state the family's monotonic sequences
exclude" — sound given the premise, and the wrap is exactly the event that breaks the
premise, but the clamp would still refuse rather than admit.

The inner one, in `distance_to`, fails **open**, and gets there first. Its reason is
different — "saturating rather than signed: the caller that needs the direction has
already compared the two" — and no caller in the family does compare. So the overrun
never reaches `free_slots` as a large number; it arrives already collapsed to `0`,
which is indistinguishable from a freshly drained ring. The outer saturation's
fail-safe behaviour is unreachable because the inner one has already answered.

Downstream the effect is unbounded: `check` returns `Ok(())`, so a producer is
admitted for a full lap over data the consumer has not read, then admitted again,
with no error surfaced anywhere and `RingError` never constructed. Three defensive
constructs sit on this chain — a panic, and two clamps — and the wrap passes all
three, because the first is on another path and the one that fails open runs before
the one that fails safe.

The remedy that costs nothing is a sentence: `fetch_add` wraps, the family's
monotonicity assumption ends there, and a cursor seeded above `u64::MAX - capacity`
is outside the design. A `debug_assert!` in `AtomicSeq::fetch_add` that the result
exceeds the input would restore the parity with `Seq::next` on the path that carries
the traffic, at zero release cost — the same asymmetry the family already chose once.

**Disposition:** applied — the sentence, and the `debug_assert!` this entry also
proposes is **declined**, for a reason the entry itself supplies. The
contract now says what this entry asked it to say and
`fetch_add_wraps_at_the_top_of_u64` pins the behaviour at both ends of the range,
including the `fetch_add( u64::MAX )` case that moves a cursor backwards; the
census above reads nineteen `u64::MAX` occurrences across `ring_atomic/src`
and `ring_atomic/tests` where it read zero. The debug-only guard was
declined because the parity it restores is parity with a construct this same
document showed to be misdocumented at the time: `Seq::next` was the function whose
doc claimed it "saturates in a release build" while release wrapping to `0` is what
the profile comparison above actually measured. The doc has since been corrected —
which removes the misdocumentation but not the reason to decline, because that
reason was never the wrong sentence: copying its debug-panics/release-wraps shape
into `AtomicSeq::fetch_add` propagates a divergence between profiles that nobody has
reconciled, and it does so on the hotter path. A guard that fires only in a
configuration production does not run is the exact defect AT41 is about, and adding
a second instance of it is not a fix. What this does not buy, and it is the larger
half: **the gate chain is unchanged.** `distance_to` still saturates to `0` before
`free_slots` can clamp, so `may_claim` still returns true, `GatingSet::headroom`
still reads 64 of 64, and `check` still returns `Ok(())` for a producer about to
overwrite a full lap of unread data. Closing that requires a decision in
`ring_seqno` and `ring_gating` about whether a cursor pair may legally compare as
`0` distance after a wrap, which is two crates away and is a design question rather
than a documentation one. Now prints: `    u64::MAX mentions in src or tests       : 19`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`invariant/001`](../invariant/001_monotonicity_is_relied_on_and_not_required.md) | The same boundary reached in one call, and the monotonicity nothing enforces |
| [`pitfall/001`](001_the_snapshot_that_never_happened.md) | The other place a defensive reading returns a plausible number that was never true |
| [`algorithm/001`](../algorithm/001_one_intrinsic_or_two.md) | The `fetch_add` delegation this inherits its behaviour from |
| [`integration/001`](../integration/001_two_declared_one_used.md) | `ring_seqno`, which owns the gates that misread the wrap and is declared but unused here |

### Sources

| Fact | Where |
|------|-------|
| The guard, its rationale, and its false release claim | `ring_types/src/id.rs:32-47` |
| The inner saturation, failing open, and its reason | `ring_types/src/id.rs:70-85` |
| The outer saturation, failing safe, and its reason | `ring_seqno/src/lib.rs:78-99` |
| The gate predicates that read it | `ring_seqno/src/lib.rs:73-75`, `:95-99` |
| `headroom`, `admits`, `check` | `ring_gating/src/lib.rs:221-245`, `:283` |
| Silence in the owning crate | Census above |
| Both profiles wrapping, and every gate granting | Release and debug probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `fetch_add_returns_the_value_before_the_advance` | The return contract at ordinary values, never near the boundary |
| `concurrent_fetch_adds_partition_the_sequence_space` | That advances do not collide, over a range nowhere near `u64::MAX` |
| *(to create)* | Nothing asserts what `fetch_add` does at the top of `u64`, so the wrap is neither prevented nor recorded |
| *(to create)* | Nothing asserts that a gate refuses a producer that has passed its consumer, which is the assertion that would have caught this chain |
