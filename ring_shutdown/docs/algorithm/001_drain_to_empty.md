# Algorithm: Drain to Empty

### Scope

- **Purpose**: Record the one procedure this crate owns — emptying a closed ring — and why a single `try_recv_batch` is not it.
- **Responsibility**: The loop, its termination condition, and a measurement artifact that dictated its shape.
- **In Scope**: `Stopped::drain_all`, `Stopped::discard_all`.
- **Out of Scope**: The bound the loop relies on (→ [`invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)); the per-backend drain shapes underneath (→ [`ring_core/docs/algorithm/002`](../../../ring_core/docs/algorithm/002_uniform_drain_over_three_shapes.md)).

### Abstract

Emptying a closed ring is a loop, not a call. The underlying
`try_recv_batch` returns one batch — what happened to be available at the
instant it looked — so a single call can return a plausible count and leave the
ring non-empty. The procedure below repeats until a batch comes back empty, and
it is safe to write without an internal bound only because it is unreachable
without a preceding close.

### Algorithm

```
1. taken ← consumer.try_recv_batch( out )
2. while taken > 0:
3.     total ← total + taken
4.     taken ← consumer.try_recv_batch( out )
5. return total
```

`discard_all` follows the same drain-until-empty idea over `try_recv` instead
of `try_recv_batch`, but is not the same spelling: it is a
`while let Some( record ) = …` rather than `drain_all`'s `while taken > 0`
with a duplicated first read, chosen for no recorded reason and carrying none
of `drain_all`'s six-line coverage comment. Dropping each record as it is
taken still lets a `T` with a `Drop` impl run it.

#### Why one batch is not a drain

`ring_core::Consumer::try_recv_batch` returns *one* batch — whatever was
available at the instant it looked. It is not a "drain everything" call and
does not claim to be.

Two conditions leave records behind after one call:

1. **The occupied region wraps.** A ring filled, partly drained, and refilled
   holds records on both sides of the wrap point, and the in-house backends'
   `drain()` returns one contiguous run.
2. **A publish landed during the call.** In-flight producers can add records
   after the batch's extent was computed.

Both are ordinary, and neither reports itself: a single-batch "drain" returns a
plausible count and leaves the ring non-empty. The loop is what makes `empty`
mean empty, and `drain_all_loops_until_the_ring_is_actually_empty` builds
condition 1 explicitly — fill 4, take 2, push 2 — rather than hoping for it.

#### Termination

The loop has no internal bound. It stops because publication has stopped, which
is enforced by `drain_all` being reachable only through
[`Stopped`](../type/001_stopped_proof_token.md) — the full argument, and the
one case where it fails, is
[`invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md).

#### Why `while` and not `loop`

The natural spelling is a `loop` with an inner `return` on a zero batch. It is
written as a `while` with a duplicated first read instead, and the reason is
measurement rather than style:

**`llvm-cov` opens a coverage region on a bare `loop` line and never attributes
a hit to it.** The line reads as uncovered however hard the tests drain, so the
crate reports 72/73 rather than 73/73 — and G1 requires 100%, so the artifact
is indistinguishable from a real gap.

| Spelling | Coverage, identical suite |
|---|---|
| `loop` with inner `return` | 72/73 |
| `while` with a duplicated first read | **73/73** |

Recorded rather than silently worked around, because the next author will
reach for the `loop` — it is the better spelling — and should know what it
costs before deciding. This is the same class of finding as
[`ring_core/docs/pitfall/002`](../../../ring_core/docs/pitfall/002_feature_gated_code_reads_as_uncovered.md):
a coverage number that is wrong about the code rather than about the tests.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'loop spellings in this crate:  %s\n' "$( command grep -vE '^ *//' ring_shutdown/src/lib.rs | command grep -ohE '^ *(while let|while|loop)' | sed 's/^ *//' | sort | uniq -c | tr '\n' ' ' )"
printf 'comment lines on drain_all:    %s\n' "$( awk '/pub fn drain_all/{f=1} f&&/^  \}$/{exit} f&&/^ *\/\//' ring_shutdown/src/lib.rs | wc -l )"
printf 'comment lines on discard_all:  %s\n' "$( awk '/pub fn discard_all/{f=1} f&&/^  \}$/{exit} f&&/^ *\/\//' ring_shutdown/src/lib.rs | wc -l )"
printf 'what drain_all recvs with:     %s\n' "$( awk '/pub fn drain_all/{f=1} f&&/^  \}$/{exit} f' ring_shutdown/src/lib.rs | command grep -ohE 'try_recv[_a-z]*' | sort -u | tr '\n' ' ' )"
printf 'what discard_all recvs with:   %s\n' "$( awk '/pub fn discard_all/{f=1} f&&/^  \}$/{exit} f' ring_shutdown/src/lib.rs | command grep -ohE 'try_recv[_a-z]*' | sort -u | tr '\n' ' ' )"
printf 'crates still using a bare loop: %s\n' "$( command grep -rl '^ *loop$' ring_*/src 2>/dev/null | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'bare loop lines in those:      %s\n' "$( command grep -rc '^ *loop$' ring_*/src/lib.rs 2>/dev/null | command grep -v ':0$' | tr '\n' ' ' )"
printf 'crates recording it in src:    %s\n' "$( command grep -rl 'llvm-cov' ring_*/src 2>/dev/null | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'crates recording it in docs:   %s\n' "$( command grep -rl 'llvm-cov' ring_*/docs 2>/dev/null | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'either of those two doing so:  %s\n' "$( command grep -rl 'llvm-cov' ring_bench ring_publish 2>/dev/null | wc -l )"
```

Live output:

```
loop spellings in this crate:        1 while       1 while let 
comment lines on drain_all:    6
comment lines on discard_all:  0
what drain_all recvs with:     try_recv_batch 
what discard_all recvs with:   try_recv 
crates still using a bare loop: ring_bench ring_publish 
bare loop lines in those:      ring_bench/src/lib.rs:5 ring_publish/src/lib.rs:1 
crates recording it in src:    ring_core ring_poll ring_shutdown 
crates recording it in docs:   ring_core ring_factory ring_poll ring_shutdown 
either of those two doing so:  0
```

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_drain_terminates_because_close_preceded_it.md](../invariant/002_drain_terminates_because_close_preceded_it.md) | The bound this loop does not carry itself |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_stopped_proof_token.md](../type/001_stopped_proof_token.md) | Why the procedure is a method rather than a free function |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_core/docs/algorithm/002`](../../../ring_core/docs/algorithm/002_uniform_drain_over_three_shapes.md) | The three backend drain shapes `try_recv_batch` unifies, and why its extent is computed up front |
| [`ring_core/docs/pitfall/002`](../../../ring_core/docs/pitfall/002_feature_gated_code_reads_as_uncovered.md) | The same class of coverage artifact, at a different cause |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `drain_all_loops_until_the_ring_is_actually_empty` — condition 1, built deliberately |
| `tests/manual/readme.md` | D2 — the coverage measurement above, which no test can assert about itself |


### SD1 — The Pseudocode Covers One of the Two Procedures It Claims To

The Algorithm section above states the loop once and then adds a sentence:
*"`discard_all` is the same shape over `try_recv` instead of `try_recv_batch`."*

The shapes are not the same, and the difference is the one this document spends
a whole section on. `drain_all` is a `while taken > 0` with the first read
duplicated above the loop — the spelling chosen deliberately, over the more
natural `loop` with an inner `return`, to keep `llvm-cov` from reporting a line
as uncovered. `discard_all` is a `while let Some( record ) = …`, a third
spelling, chosen for no recorded reason, carrying no comment at all where
`drain_all` carries six lines of it.

Both spellings are fine for coverage — that is not the finding. The finding is
that the crate's one recorded procedural decision is attached to one of the two
procedures the section says are the same, so a maintainer rewriting
`discard_all` finds no reason not to reach for the `loop`, and the document
that would have told them says the two are interchangeable.

It compounds with where the two are used:
[`non_functional_requirement/002`](../non_functional_requirement/002_the_teardown_path_takes_the_slow_one.md)
shows that `reset` — the crate's headline teardown — routes through
`discard_all`, so the loop with no explanation is the one on the path a caller
actually reaches for.

**Disposition:** applied — the Abstract/Algorithm section's "same shape" claim
now states the actual difference: `discard_all` is a `while let Some( record )`
loop, not `drain_all`'s `while taken > 0` with a duplicated first read, and
carries none of its six-line coverage comment. Now prints:
`while let Some( record ) = consumer.try_recv()`

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/pub fn discard_all/{f=1} f&&/^  \}$/{exit} f&&/while/{print; exit}' ring_shutdown/src/lib.rs
```

Live output:

```
    while let Some( record ) = consumer.try_recv()
```

### SD2 — The Workaround Is Recorded in the Three Crates That Adopted It and in Neither of the Two That Need It

The `llvm-cov`-opens-a-region-on-`loop` artifact is recorded in two source
files — `ring_core/src/lib.rs` and this one — and in the doc corpora of three
crates: `ring_core`, `ring_poll` and `ring_shutdown`, each with a matching
manual-probe entry. It is a well-recorded piece of family knowledge, and it is
recorded most thoroughly where it has already been acted on.

Two crates in the family still contain a bare `loop`: `ring_bench` with five
and `ring_publish` with one. Neither mentions `llvm-cov` anywhere in its
source.

So the knowledge has propagated exactly through the crates that already
changed their spelling, and stopped at the boundary of the ones that did not.
Either those six lines cost a coverage point that nobody has attributed, or the
artifact is narrower than the three write-ups state and the workaround is being
carried on out-of-date grounds — and the two possibilities are distinguished by
one measurement that nobody has run, because the artifact is described in prose
in three places and asserted in none.
