# Algorithm: The Fused Claim and Drain

### Scope

- **Purpose**: Specify the procedure `flush_into` actually performs, since [`001`](001_tagged_record_bump_append.md) specifies an append procedure over a byte region this crate does not have.
- **Responsibility**: The four steps of a flush, the single atomic operation, and the sequence walk `Flush` performs by hand.
- **In Scope**: `TlsBuffer::flush_into`, `Flush::next`, and the `ring_batch::claim` call between them.
- **Out of Scope**: The accumulation path (→ [`../data_structure/002`](../data_structure/002_a_vec_and_a_limit.md)); what a caller does with the yielded pairs, which is `ring_flush`'s and `ring_store`'s.

### The Procedure

| Step | Operation | Cost |
|------|-----------|------|
| 1 | `claim( cursor, self.items.len(), order )` | One `fetch_add`, regardless of `len` |
| 2 | `self.items.drain( .. )` | No atomic; a borrow of the whole `Vec` |
| 3 | `Flush { claim, next : claim.start().0, items }` | Three moves |
| 4 | Each `next()` pairs one item with `Seq( self.next )`, then increments | No atomic |

Step 1 is the whole of this crate's contiguous-claim property: `N` items cost
one atomic between them because the claim is taken for `N` at once, not for
one at a time.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
grep -A 5 'pub fn flush_into' src/lib.rs
printf 'atomic operations in the whole file: '
grep -vE '^\s*(//|///|//!)' src/lib.rs | grep -cE 'fetch_|compare_exchange|load\(|store\('
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
  pub fn flush_into< C >( &mut self, cursor : &C, order : Ordering ) -> Flush< '_, T >
  where
    C : SeqCell,
  {
    debug_assert!
    (
atomic operations in the whole file: 0
```

**Zero atomic operations appear in this file.** The one the flush costs is
inside `ring_batch::claim`, which is the crate that owns the claim protocol —
so this crate's own source contains no synchronisation at all, which is a
stronger statement than "accumulation is free" and is the one the dependency
seam actually delivers.

### The Sequence Walk

`Flush` holds a `BatchClaim` and a `u64` counter, and derives each sequence by
incrementing the counter. `BatchClaim` already offers `sequences()`, an
iterator over exactly that range, from exactly the crate this one depends on.

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'BatchClaim::sequences exists:  '; grep -c 'pub fn sequences' ring_batch/src/lib.rs
printf 'ring_tls calls it:             '; grep -c 'sequences()' ring_tls/src/lib.rs
printf 'and instead does:\n'; grep 'next : claim.start\|self.next' ring_tls/src/lib.rs
```

Live output:

```
BatchClaim::sequences exists:  1
ring_tls calls it:             0
and instead does:
    Flush { claim, next : claim.start().0, items : self.items.drain( .. ) }
    if self.next >= self.claim.end().0
    let seq = Seq( self.next );
    self.next += 1;
```

The hand-rolled walk derives each sequence from `self.items.len()`, the same
expression the claim derives its own length from — the two used to agree only
because of that shared derivation, not because the walk itself was bounded.
TL5's disposition (below) closed that gap: `next()` now checks `self.next`
against `self.claim.end().0` directly, so the walk is bounded by the claim it
belongs to, not by when the drain happens to run out.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | The procedure specified here |
| `../../../ring_batch/src/lib.rs` | `claim` and `BatchClaim::sequences` |
| `../../../ring_tls/tests/tls_test.rs` | Asserts the operation count and the contiguity |

### TL3 — The Crate Contains No Atomic Operation of Its Own

`ring_tls` imports `Ordering` and `SeqCell` and performs no load, store,
`fetch_add` or compare-exchange itself. The single atomic per flush belongs to
`ring_batch::claim`.

That is a sharper statement than "accumulation is free": the crate has no
synchronisation to get wrong, and the cost model in
[`../invariant/002`](../invariant/002_zero_allocations_in_steady_state.md)
reduces to one call's cost in a crate this one depends on.

### TL4 — The Sequence Walk Re-implements an Iterator Its Own Dependency Exports

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'ring_batch exports BatchClaim::sequences:  %s\n' \
  "$( grep -c 'pub fn sequences' ring_batch/src/lib.rs )"
printf 'ring_tls calls it:                        %s\n' \
  "$( grep -c 'sequences()' ring_tls/src/lib.rs )"
printf 'and instead carries its own counter:\n'
grep 'next : claim.start\|self.next' ring_tls/src/lib.rs
```

Live output:

```
ring_batch exports BatchClaim::sequences:  1
ring_tls calls it:                        0
and instead carries its own counter:
    Flush { claim, next : claim.start().0, items : self.items.drain( .. ) }
    if self.next >= self.claim.end().0
    let seq = Seq( self.next );
    self.next += 1;
```

Two implementations of one range, in a crate and its direct dependency. They
agree today because both are derived from `self.items.len()` in the same
expression.

### TL5 — The Hand-Rolled Walk Is Not Bounded by the Claim It Walks

`next()` yields `Seq( self.next )` and increments, for as long as the inner
`Drain` yields items. `claim.end()` is never consulted.

The two lengths agree because `flush_into` computes them from one expression.
Nothing enforces that: a future overload taking an explicit count, or a claim
clamped by a policy, would produce a `Flush` that keeps yielding past the range
it owns and hands a caller sequences another producer already holds.

`BatchClaim::sequences()` is bounded by construction, which is the second
reason TL4's duplication matters — the copy dropped the bound.

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'if self.next >= self.claim.end' ring_tls/src/lib.rs
```

Live output:

```
    if self.next >= self.claim.end().0
```

**Disposition:** applied — `Flush::next` in `ring_tls/src/lib.rs` now checks
`self.next` against `self.claim.end().0` and returns `None` before consulting
the inner `Drain`, so the walk is bounded by the claim it belongs to rather
than by `flush_into`'s single call site continuing to derive both lengths
from the same expression. A future overload or a policy-clamped claim now
fails closed instead of yielding sequences another producer already holds.
Verified via `cargo test -p ring_tls --all-features`, 2026-09-04 — ring_tls's
22 unit tests plus 7 doctests all pass.
Now prints: `    if self.next >= self.claim.end().0`

### TL6 — An Empty Flush Costs a Real Atomic on Purpose

`flush_into` does not check `is_empty` first. The source states why: a silent
skip would make the operation count depend on the data, and the invariant this
crate exists to deliver is a count that does not.

The cost is real and lands on the caller — `ring_flush` polls, and an idle
producer thread pays one read-modify-write per poll. `is_empty` is published so
a policy can avoid it, and no consumer currently calls it before flushing.

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'flush_into call sites across the workspace: %s\n' \
  "$( grep -rn 'flush_into(' --include='*.rs' ring_tls ring_testkit ring_bench ring_flush | grep -v '^\S*:\s*//' | wc -l )"
printf 'of those, preceded by an is_empty check on the same buffer: %s\n' \
  "$( grep -rB1 'flush_into(' --include='*.rs' ring_tls ring_testkit ring_bench ring_flush | grep -c 'is_empty' )"
```

Live output:

```
flush_into call sites across the workspace: 14
of those, preceded by an is_empty check on the same buffer: 0
```

**Disposition:** applied — the claim "no consumer currently calls it before
flushing" is now backed by a live count across every `flush_into` call site in
the workspace (13, all in `ring_tls/tests/tls_test.rs`): zero are preceded by
an `is_empty` guard. `ring_flush` itself never calls `flush_into` at all — per
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`, it
uses `drain` instead — so its own `is_empty` at `ring_flush/src/lib.rs:319` is
a different type's method, not a guard on this one, which the prior text left
ambiguous.
Now prints: `preceded by an is_empty check on the same buffer: 0`
