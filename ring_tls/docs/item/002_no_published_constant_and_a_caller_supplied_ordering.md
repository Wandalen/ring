# Item: No Published Constant, and a Caller-Supplied Ordering

### Scope

- **Purpose**: Record that this crate publishes no ordering constant where both composed cores publish several, and state what the caller therefore has to know.
- **Responsibility**: The absence of a constant, the `Ordering` parameter that stands in for it, and what a caller passing the wrong one gets.
- **In Scope**: `flush_into`'s `order` parameter and the family's published ordering vocabularies.
- **Out of Scope**: The orderings themselves, which are `ring_atomic`'s and `ring_batch`'s; the claim procedure that consumes the parameter (→ [`../algorithm/002`](../algorithm/002_the_fused_claim_and_drain.md)).

### The Comparison

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_tls ring_spsc ring_mpsc; do
  printf '%-11s published ordering constants: %s\n' "$c" \
    "$( grep -vE '^\s*(//|///|//!)' $c/src/lib.rs | grep -c '^pub const .*Ordering' )"
done
printf 'ring_tls takes one as a parameter instead:\n'
grep 'order : Ordering' ring_tls/src/lib.rs
```

Live output:

```
ring_tls    published ordering constants: 0
ring_spsc   published ordering constants: 2
ring_mpsc   published ordering constants: 4
ring_tls takes one as a parameter instead:
  pub fn flush_into< C >( &mut self, cursor : &C, order : Ordering ) -> Flush< '_, T >
```

Both composed cores name their orderings once, in a constant, and use the name
everywhere. This crate takes `order` from the caller on every `flush_into` and
names nothing.

**The doc examples pass three different values.** `Ordering::AcqRel` in the
crate-level example and in `flush_into`'s own; the tests pass what the counting
shim needs. Nothing in the signature says which is correct, and nothing rejects
`Relaxed` — a `fetch_add` with `Relaxed` still allocates the right sequences
and still publishes nothing, which is the failure that shows up as missing data
in a consumer rather than as an error here.

**This is the right shape and the wrong default.** Right, because the correct
ordering depends on what the caller does with the claim, which this crate
cannot see. Wrong to leave unnamed: `ring_spsc` faced the same question and
answered it by publishing `OWN` and `HANDOFF` so a caller has a vocabulary
rather than an enum.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | Declares `flush_into`'s `order` parameter and no constant |
| `../../../ring_spsc/src/lib.rs` | Publishes two constants for the same class of choice |
| `../../../ring_batch/src/lib.rs` | `claim`'s own contract for what `order` governs |

### TL35 — The Ordering Vocabulary Is a Parameter Here and a Constant in Both Siblings

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_tls ring_spsc ring_mpsc; do
  printf '%-11s published ordering constants: %s\n' "$c" \
    "$( grep -vE '^\\s*(//|///|//!)' $c/src/lib.rs | grep -c '^pub const .*Ordering' )"
done
```

Live output:

```
ring_tls    published ordering constants: 0
ring_spsc   published ordering constants: 2
ring_mpsc   published ordering constants: 4
```

The parameter is the right shape — the correct ordering depends on what the
caller does with the claim, which this crate cannot see. Leaving it unnamed is
the inconsistency: `ring_spsc` faced the same question and answered it by
publishing a vocabulary rather than an enum.

### TL36 — Nothing Rejects a `Relaxed` Flush

The `fetch_add` still returns a correct, non-overlapping range under
`Relaxed`. What is lost is the release edge that makes the staged items visible
to whoever reads the cursor.

The failure therefore appears as missing or torn data in `ring_flush` or
`ring_store`, with no error, no panic, and nothing pointing back to the
argument that caused it.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c 'must include Release semantics' ring_tls/src/lib.rs
```

Live output:

```
1
```

**Disposition:** applied — `flush_into` in `ring_tls/src/lib.rs` now
`debug_assert!`s that `order` is `Release`, `AcqRel`, or `SeqCst` before
claiming, naming the given ordering in the panic message when it is not, and
its doc comment states why a `Relaxed` or `Acquire` claim is well-formed but
silently loses the handoff. Debug-only by design: the check would cost a
branch on every flush in a release build for a mistake that, once made once
by a caller, is easy to keep from making twice.
Verified via `cargo test -p ring_tls --all-features`, 2026-09-04 — ring_tls's
22 unit tests plus 7 doctests all pass.
Now prints: `1`
