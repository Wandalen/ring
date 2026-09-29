# decisions

Two rulings, and the second exists only because of the first. Refusing to publish
out of order is what creates a wait; having created it, the crate then declines
every mechanism the family has for managing waits.

Both are recorded with their alternatives priced and their reversal conditions
named, because both look like omissions on a first reading — a missing feature and
a missing parameter — and both are the opposite.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Refused Rather Than Reordered](001_refused_rather_than_reordered.md) | PB17 — the three designs priced side by side, the testability argument that decided it, the grep that keeps the alternatives out, and what happened when the crate that needed them arrived |
| 002 | [A Plain Spin Rather Than a `WaitKind`](002_a_plain_spin_rather_than_a_wait_kind.md) | PB18 — the family's three waiting primitives, the Tier 5 sibling that waits on the same event and does take a strategy, and the four things a budget's exhaustion could mean |

### The Two Rulings

| | 001 — refuse | 002 — spin bare |
|--|--------------|-----------------|
| Ruling | advance only from the exact frontier | no `WaitKind`, no budget |
| Rejected | highest-contiguous scan; per-slot bitmap | `Yield`; `Park`; a `spins` parameter |
| Decided by | testability — the alternatives need two producers to exercise | meaning — every non-trivial value of both parameters is wrong |
| Cost | a producer waits on a peer | an unbounded loop in a Tier 5 primitive |
| Guarded by | `§ P4`, a grep for an absence | nothing — the absence is the signature |
| Escape hatch | — | `try_publish`, which returns instead |
| Live reversal condition | none | large payloads, where `Yield` would stop being strictly worse |
| Who overturned it | `ring_mpsc`, by building its own | — |

The link between them is the "cost" row of 001: refusing creates the wait that
002 then argues should be bare. Reverse 001 and 002 becomes moot; reverse 002 and
001 is untouched.

### The Argument Neither Makes

Neither ruling claims its choice is faster. 001's case is that the alternatives
cannot be validated without contention, and *"a primitive whose correctness
argument requires two threads is a primitive whose correctness argument is a
scheduling accident"*. 002's case is that a strategy parameter with three arms of
which two are strictly worse is not a choice, and a budget whose only correct
exhaustion handling is *keep waiting* is a knob that does nothing.

Both are arguments about what can be checked and what can be meant — not about
cycles. The one place cycles would decide it is named in 002's reversal table and
is unmeasured.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the rejection, checked rather than trusted — expect no output
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -niE "bitmap|contiguous|while|for |max\(|highest"

# every waiting primitive in the family
for f in ring_*/src/*.rs; do
  h=$( grep -vE '^[[:space:]]*//' "$f" \
       | grep -cE 'spin_loop|yield_now|thread::sleep|::park' )
  [ "$h" -gt 0 ] && printf "%-38s %s\n" "$f" "$h"
done

# every public function taking a wait strategy
grep -rnE '^\s*pub (const )?fn .*WaitKind' ring_*/src/*.rs

# the sibling that waits on the same event and takes both parameters
command grep -m1 -A23 -F '  /// # Errors' ring_barrier/src/lib.rs

# what ring_mpsc built instead, and what it removed to do it
command grep -m1 -A15 -F '//! # Publication is a per-slot stamp, and that is this crate'"'"'s whole addition' ring_mpsc/src/lib.rs
command grep -m1 -A3 -F '# Eight, not the seven this manifest was scaffolded with. `ring_publish` and' ring_mpsc/Cargo.toml
```

| | Value |
|--|------:|
| Designs considered in 001 | 3 |
| …taken by this crate | 1 |
| …taken by `ring_mpsc` | 1 — per-slot stamps |
| …taken by nobody | 1 — the contiguous scan |
| Output of the rejected-alternatives grep | empty |
| Crates containing a waiting primitive | 3 of 33 |
| …that take a `WaitKind` | 1 — `ring_wait` |
| …that open-code a budget | 1 — `ring_poll` |
| …that take neither | **1** — this crate |
| Public functions taking a `WaitKind`, family-wide | 10, across 4 crates |
| `WaitKind` arms | 4 — `Spin`, `Yield`, `Park`, `None` |
| …that would beat `Spin` here | **0** |
| `Park`'s actual implementation | a 50 µs sleep (`ring_wait:134-141`) |
| Options after a hypothetical budget expires | 4 |
| …that are correct | **1** — retry, i.e. what no budget does |
| Reversal conditions listed | 3 per decision |
| …currently holding | 0 |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB17 | `ring_publish` | n/a — observation | The rejection is enforced by a grep rather than trusted to prose, and the comment-stripping filter is load-bearing: this crate's own module documentation argues about bitmaps and contiguity at length, so an unfiltered grep would report the rejected designs as implementations |
| PB18 | family | n/a — observation | Three of 33 crates contain a waiting primitive; the other two take a strategy or a budget and this one takes neither, and `ring_poll` declines `ring_wait` on exactly opposite grounds — it must never block, this crate must always eventually succeed |
| PB49 | `ring_mpsc` | n/a — drift | The rejection defers per-slot availability to `ring_mpsc` by naming a bitmap; `ring_mpsc` says `bitmap` zero times and `stamp` fifty-two, and puts the mechanism in a heading — the second forecast in this crate overtaken by the crate it named |
