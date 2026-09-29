# Non-Functional Requirement: Seven Times the Cost, on the Half Nobody Ships

### Scope

**Purpose:** Record what the recording half costs over the pure half, measured, and
which half the shipping configuration actually executes.

**Responsibility:** The per-call cost of `resolve` against `would_resolve`, the
single instruction that accounts for the difference, and the coverage gap the
measurement cannot close.

**In Scope:** `ring_overflow/src/lib.rs:192`, `:199`, `:229`;
`ring_stats/src/lib.rs:293`; `ring_core/src/lib.rs:80`, `:411`.

**Out of Scope:** Why the two halves exist at all is
[`workaround/001`](../workaround/001_the_recorder_forecloses_const.md). The
`const`-evaluability the split preserves, and that nothing uses it, is § OV38
there. The return-shape cost — 1 byte against 24 — is
[`data_structure/002`](../data_structure/002_a_one_byte_outcome_in_a_twenty_four_byte_result.md)
§ OV11.

---

## What Separates Them, and Who Takes Which

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the one statement that separates the two halves --'
command grep -m1 -F '  stats.record_drop( policy, 1 );' ring_overflow/src/lib.rs
echo '  -- the instruction it bottoms out in --'
command grep -m1 -F '    counter.fetch_add( n, Ordering::Relaxed );' ring_stats/src/lib.rs
echo '  -- which half the one production caller takes --'
command grep 'ring_overflow::\|would_resolve( self' ring_core/src/lib.rs
echo '  -- benches/ directories under , across 33 crates --'
ls -d */benches 2>/dev/null | wc -l
```

Live output:

```
  -- the one statement that separates the two halves --
  stats.record_drop( policy, 1 );
  -- the instruction it bottoms out in --
    counter.fetch_add( n, Ordering::Relaxed );
  -- which half the one production caller takes --
use ring_overflow::{ would_resolve, Resolution };
      Err( record ) => match would_resolve( self.overflow )
  -- benches/ directories under , across 33 crates --
0
```

---

## The Measurement

No crate under `any crate root` carries a `benches/` directory, so this number was taken
outside the tree, with a throwaway binary linking `ring_overflow`, `ring_stats`
and `ring_types` by path at `--release`. Its loop, so the number can be
reconstructed without it:

```rust
const ITERS : u64 = 20_000_000;
const REPS : usize = 9;

// pure half
for _ in 0..ITERS { acc += would_resolve( black_box( policy ) ) as u64; }
// recording half
for _ in 0..ITERS { acc += resolve( black_box( policy ), stats ).is_ok() as u64; }
```

Paired, per the corpus convention: both variants run back-to-back *inside* each
repetition, so a frequency change or a scheduling hiccup moves both and cancels in
the ratio. Nine repetitions, median reported, min/max printed beside it,
reproduced across two independent runs. `black_box` on the policy is what stops
the pure half from folding away entirely — it is a `const fn` over a
`Copy` fieldless enum, and without the barrier there is no loop left to time.

Run 1:

```
  -- cost per call, 9 paired repetitions of 20000000 iterations --

    would_resolve   median 0.677 ns   min 0.674   max 0.821
    resolve         median 4.822 ns   min 4.717   max 4.996

    ratio           median 7.00x      min 5.87x    max 7.40x
```

Run 2:

```
  -- cost per call, 9 paired repetitions of 20000000 iterations --

    would_resolve   median 0.684 ns   min 0.674   max 0.692
    resolve         median 4.859 ns   min 4.738   max 4.955

    ratio           median 7.02x      min 6.95x    max 7.35x
```

Two independent runs agree to 7.00x and 7.02x. Host is aarch64, 16 cores,
single-threaded, hot cacheline, no other producer touching the counter.

---

### OV49 — The Recording Half Costs 7x the Pure Half, and Production Takes the Pure One

The two functions compute the same mapping and differ by one statement:
`stats.record_drop( policy, 1 )`, which bottoms out in
`counter.fetch_add( n, Ordering::Relaxed )`. That one relaxed read-modify-write
is the entire measured difference — 0.68 ns against 4.86 ns, a median ratio of
7.00x reproduced at 7.02x, or about 4.2 ns absolute.

**Finding.** The interesting part is not the ratio, it is who pays it.
`ring_core` imports `would_resolve` and calls `would_resolve`, and names `resolve`
nowhere ([`integration/001`](../integration/001_one_consumer_one_import_one_site.md)
§ OV17). So every execution of the 4.86 ns half in this workspace happens under
`cargo test`, and the shipping binary executes only the 0.68 ns one.

Which makes the cost real and the exposure nil. Worth recording because the number
reads as a reason to prefer the pure half, and it is not one — `ring_core` takes
the pure half because it has no `&RingStats` to pass, not because 4 ns was
too much ([`pattern/001`](../pattern/001_the_pure_effectful_pair.md) § OV42). A
future caller that *does* hold stats has no cost argument against `resolve`
either: 4 ns sits against a full-ring event that has already cost a failed push
and a return through two crates.

The measurement's use is bounding, then. It says the recording half is not
expensive enough to design around, and it says the crate is not currently paying
even that.

**Disposition:** declined — this instance's own text records the ratio as
bounding evidence with no design action implied ("not expensive enough to
design around... the crate is not currently paying even that"); the actual
reason `ring_core` takes the pure half is fixed as the applied disposition on
`pattern/001_the_pure_effectful_pair.md` § OV42, not this cost, so no source
or doc change in `ring_overflow/src/lib.rs` is implied by this
measurement itself.

---

### OV50 — The One Cost That Would Matter Cannot Be Measured Here, and Is Measured Nowhere

The number above is uncontended: one thread, one counter, a cacheline nobody else
wants. A full ring in production is the opposite case by construction — the
condition that triggers `resolve` is *every producer arriving at once*, and they
all target the same `AtomicU64` for their policy. A relaxed `fetch_add` under that
contention is not 4 ns.

**Finding.** So the measurement that would inform a design decision is the one this
probe cannot take, and no crate under `any crate root` has a `benches/` directory — the
census returns zero across all 33, including `ring_bench`, whose name promises
otherwise and which never mentions `ring_overflow`.

That gap is currently harmless for the same reason § OV49 is: production takes the
pure half, which touches no shared state and therefore has no contended case.
It stops being harmless the moment a second consumer appears holding a
`&RingStats`, because that is the caller for whom the uncontended number is
actively misleading — it is the best case, taken on the path that only runs when
the system is already in its worst one.

Recording it here rather than as a request for a benchmark: the benchmark cannot
be written usefully until there is a real multi-producer call site to model, and
there is exactly one call site and it is not that.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`workaround/001`](../workaround/001_the_recorder_forecloses_const.md) | Why the two halves exist, and the atomic that forced them |
| [`integration/001`](../integration/001_one_consumer_one_import_one_site.md) | The one consumer, and the half it takes |
| [`pattern/001`](../pattern/001_the_pure_effectful_pair.md) | The real reason the pure half is chosen |
| [`nfr/002`](002_a_core_only_crate_that_never_says_so.md) | The other properties inherited from the same atomic |

### Sources

| Fact | Where |
|------|-------|
| The one differing statement | `ring_overflow/src/lib.rs:199` |
| The relaxed read-modify-write it reaches | `ring_stats/src/lib.rs:293` |
| The consumer's import and call, both of the pure half | `ring_core/src/lib.rs:80`, `:411` |
| Zero `benches/` directories under `any crate root` | Census above |
| 7.00x / 7.02x across two paired runs | Measurement above |

### Tests

| Test | Covers |
|------|--------|
| `would_resolve_touches_no_counters` | That the pure half omits the measured statement |
| `exactly_one_counter_moves_per_call` | The write whose cost is measured |
| `resolve_agrees_with_would_resolve` | That the two halves compute the same thing at different prices |
