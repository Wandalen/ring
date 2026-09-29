# invariant

The crate has no invariant section, no `# Panics`, and one contract clause in the
whole file — a `# Errors` on `compare_exchange` describing what the failure path
returns, not what a caller owes. It sits underneath a family that names
monotonicity as the property "every gate relies on". Everything the family assumes
about a cursor is assumed of a value this crate hands out and never constrains.

What the crate does hold, it holds because `AtomicU64` holds it. Both concurrency
tests here assert properties the language already guarantees, through a
one-line delegation, sixty thousand operations deep. The properties that are
genuinely the crate's own — the relation between the shim's counter and its cell,
and what a caller may do to a cell — are the two with no test at all.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_monotonicity_is_relied_on_and_not_required.md) | Monotonicity Is Relied On Here and Required Nowhere | The three ways backwards, what the family says about it, and the crate built to detect it |
| [002](002_every_increment_survives.md) | Every Increment Survives, and Nothing Relates the Two Numbers | What the two concurrency tests actually rest on, and the ordering nothing checks |

## What the Trait Promises

Four methods, four sentences. `load` reads, `store` overwrites, `fetch_add`
advances and returns the pre-advance value, `compare_exchange` swaps conditionally
and reports what it found — the last with a `# Errors` clause naming the failure
value as "the multi-producer claim's retry input". None of the four names a
precondition, a forbidden argument, or a postcondition beyond the value returned.

That is a defensible position for a primitive — the alternative is a comparison on
every cursor write, which is precisely the cost this crate exists to avoid — but it
is a position, and the crate states it nowhere. `fetch_add`'s contract says
"advance"; `fetch_add( u64::MAX )` retreats.

## Where the Family Puts the Check Instead

`ring_debug` defines `Violation::CursorWentBackwards` and a `Watch` that samples a
cursor pair and reports the violation after the fact. Detection one tier up,
opt-in, rather than prevention at the primitive. Coherent, and discoverable only
from `ring_debug`.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every contract clause in the crate, and what it is about --'
command grep -nE '# Panics|# Safety|# Errors' ring_atomic/src/lib.rs
command grep -m1 -A1 -F '  /// The sequence actually found, when it was not `current` — the multi-producer' ring_atomic/src/lib.rs
echo '  -- and the word the family builds its gates on --'
command grep -rc 'monoton' ring_atomic/src/lib.rs
command grep -rl 'monoton' --include=lib.rs --include=id.rs */src/ | sed -E 's|/src/.*||' | sort -u | tr '\n' ' '; echo
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT21 | `ring_atomic` | **latent hazard** | All three mutating methods move a cell backwards with no refusal, `fetch_add( u64::MAX )` included despite being named and documented as an advance — while `ring_types` and `ring_consume` both name monotonicity as what the family's gates rely on, and the word appears nowhere in this crate |
| AT22 | `ring_atomic` | n/a — doc gap | The family's answer is `ring_debug`'s `Violation::CursorWentBackwards` — runtime detection one tier up, demonstrated by a doctest that walks a producer 5 → 7 → 2 — and nothing in this crate records that the trade was made or that the detector exists |
| AT23 | `ring_atomic` | n/a — coverage | Both concurrency tests, 60,000 operations between them, assert properties of `AtomicU64` reached through a one-line delegation — they cannot fail unless the standard library is wrong |
| AT24 | `ring_atomic` | **latent hazard** | The shim increments its counter before delegating, so the cell can never lead at any instant — yet 7,176 of 1,000,000 paired reads observed exactly that, because a caller's `counts()` and `load()` are two moments, and the ordering decision behind it is written only in the body |
