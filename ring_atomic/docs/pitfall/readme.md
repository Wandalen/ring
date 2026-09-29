# pitfall

Neither pitfall here is a crash, a panic, or a value out of range. Both are the same
shape: a construct chosen defensively, doing exactly what it was designed to do, and
in doing so converting an anomaly into a number that looks entirely reasonable.
`distance_to` saturates rather than going negative, so a producer that has lapped its
consumer reports zero in flight and the ring reads empty. `OpCounts::total` is summed
from the four values just read rather than re-derived, so a snapshot describing a
state that never existed passes the only consistency check the type exposes. In both
cases the well-formed answer is the problem.

What makes them pitfalls rather than defects is that every path a reader could take
to discover them is clear. The crate has no error type — its three `Result`
signatures are all `compare_exchange`, whose `Err` carries a `Seq` rather than a
failure — so neither condition can be reported. It says nothing about overflow,
wrap, or saturation anywhere in its source or tests. And `counts()` carries one line
of contract, which reads as an instant. Each pitfall's own instance records what one
added clause would cost.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_the_snapshot_that_never_happened.md) | The Snapshot That Never Happened | Why a torn `OpCounts` is undetectable from the returned value, and the four artifacts that say it is safe |
| [002](002_the_wrap_that_reads_as_an_empty_ring.md) | The Wrap That Reads as an Empty Ring | `fetch_add` at the top of `u64`, the guard that sits on another path, and why every gate then grants |

## Three Guards, and the Order They Run In

The wrap chain is the sharper of the two because the family built defences against
it and they are individually sound. `Seq::next` refuses to wrap and says why. Then
`Seq::distance_to` clamps, and then `ring_seqno::free_slots` clamps again. The wrap
passes all three, and the reason is ordering rather than weakness: the panic is on
the arithmetic path while production advances through the atomic one; and of the two
clamps, the one that fails open (`distance_to`, collapsing an overrun to zero) runs
before the one that fails safe (`free_slots`, which would have reported zero free and
jammed the ring visibly). Reverse those two and the same bug is loud.

## The Discipline Nobody Wrote Down

Both instances end at the same place. Sixteen `counts()` call sites across
`ring_batch` and `ring_tls` read quiescent cells, unanimously and correctly.
Thirteen construction sites keep the counting cell out of production, unanimously and
correctly. No cursor in the family is seeded near `u64::MAX`. The conventions hold
perfectly and none of them is stated, so what protects the crate is that everyone who
has touched it so far happened to know.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- both pitfalls are a defensive construct returning a plausible number --'
printf '     the overrun collapsed by saturating_sub : %s\n' \
  "$( command grep -m1 -F '    later.0.saturating_sub( self.0 )' ring_types/src/id.rs | sed 's/^ *//' )"
printf '     the tear certified by a derived total   : %s\n' \
  "$( command grep -m1 -F '      total : loads + stores + fetch_adds + compare_exchanges,' ring_atomic/src/lib.rs | sed 's/^ *//' )"
echo '  -- and what the owning crate offers a caller who wants to know --'
printf '    overflow/wrap/saturat mentions in src+tests : %s\n' \
  "$( command grep -ciE 'overflow|wrap|saturat' ring_atomic/src/lib.rs \
      ring_atomic/tests/atomic_test.rs | cut -d: -f2 | paste -sd+ | bc )"
printf '    doc lines of contract on counts()           : %s\n' \
  "$( command grep -m1 -F '  /// What this cell has been asked to do so far.' ring_atomic/src/lib.rs | command grep -c '///' || true )"
printf '    Result signatures, all compare_exchange     : %s\n' \
  "$( command grep -c -- '-> Result<' ring_atomic/src/lib.rs || true )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT41 | `ring_atomic` | **latent hazard** | `Seq::next` refuses to wrap and states the stakes correctly, but production advances through `AtomicSeq::fetch_add`, which wraps identically in debug and release — so the guard is absent from the only path that carries traffic, and the owning crate mentions overflow, wrap and saturation zero times in source or tests; release also disproves `next`'s own claim to saturate, yielding `0` |
| AT42 | `ring_atomic` | **latent hazard** | Past the wrap every gate in the family grants — `free_slots` reports 64 of 64, `may_claim` true, `headroom` 64, `check` `Ok(())` — because `distance_to`'s saturation fails open and runs before `free_slots`' saturation, which would have failed safe; backpressure does not degrade, it inverts, and no `RingError` is ever constructed |
| AT43 | `ring_atomic` | **latent hazard** | About 1 snapshot in 100 reports a state the cell was never in, off by up to 1,198 operations, and all of them pass `total == loads + stores + fetch_adds + compare_exchanges` — the only cross-field relation `OpCounts` exposes is derived from the torn reads, so the check a suspicious caller would reach for is the one that cannot detect the tear |
| AT44 | `ring_atomic` | **misleading doc** | `counts()`' one-line contract reads as an instant, its doctest is single-threaded, `reset_counts` describes the quiescent pattern without requiring it, and `counts_are_exact_under_contention` — the only place "counts" and "contention" meet — reads after every thread has joined; sixteen downstream call sites obey a discipline nothing states |
