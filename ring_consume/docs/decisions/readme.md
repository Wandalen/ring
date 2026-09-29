# decisions

Two rulings, both of which chose the cheaper mechanism and paid for it in
guarantees the caller now has to supply. The first split one operation into two
calls, opening a window nothing guards. The second used plain stores where the
write half uses compare-exchange, which is correct for exactly one consumer and
silently wrong for two.

They are the same ruling seen twice: *this crate does less, and the correctness
argument moves to the caller.* Both are argued in the module documentation, and
both arguments are good. What neither says is that the resulting obligation has
no enforcement anywhere.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Two Calls, Not One](001_two_calls_not_one.md) | CN6, CN7 — the alternative named and priced, and the window between `available` and `commit` that nothing can guard |
| 002 | [Plain Stores Rather Than Compare-Exchange](002_plain_stores_rather_than_compare_exchange.md) | CN8, CN9 — an exact atomic mirror of `ring_claim`, and a single-consumer premise enforced by prose |

### The Two Rulings

| | 001 — two calls | 002 — plain stores |
|--|----------------|--------------------|
| Rejected | one call returning a guard | compare-exchange on the consumer cursor |
| Because | the guard would have to own the read | one consumer means no contention to resolve |
| Costs | an unguarded window | correctness depends on a premise nothing checks |
| Argued in | module doc, §1 | module doc, §3 |
| Enforced by | nothing | nothing |

### Why Both Are Right and Both Are Exposed

The module documentation's argument for two calls is the strongest prose in the
crate: a single call returning a guard would have to know when the read
finished, and only the caller knows that. Correct — and it means the window
between the calls cannot be closed by any signature this crate could offer.

The argument for plain stores is equally sound and rests on a premise instead of
a mechanism. One consumer means the store has no competitor, so `Release` is
sufficient and a compare-exchange would be pure cost. The premise is stated. It
is not checkable: `Consumer::new` is public, its arguments are a shared reference
and a `Copy` value, and three consumers over one cursor compile and run
([`type/002`](../type/002_the_lifetime_on_consumer.md) CN50).

So both decisions are correct, both are documented, and both convert a
guarantee into an obligation without saying that the obligation is unbacked.

### The Exact Mirror

```
                 CAS   store   load
ring_claim        2      0       1
ring_consume      0      2       1
```

The write half resolves contention and the read half assumes there is none —
visible in the instruction counts alone, and stated nowhere. It is the family's
cleanest structural fact about the two halves and neither crate mentions the
other's shape.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# the atomic mirror: CAS, store and load counts for both halves.
# the comment filter is load-bearing — without it doctest lines treble the counts
for c in ring_claim ring_consume ring_publish ring_barrier ring_gating ring_cursor; do
  body=$( grep -vE "^[[:space:]]*//" ring/$c/src/lib.rs )
  printf '%-14s CAS %d  store %d  load %d\n' "$c" \
    "$( printf '%s' "$body" | grep -c 'compare_exchange' )" \
    "$( printf '%s' "$body" | grep -c '\.store(' )" \
    "$( printf '%s' "$body" | grep -c '\.load(' )"
done

# the two stores, and the ordering they use
command grep 'self.cursor.store' ring_consume/src/lib.rs
command grep 'const COMMIT' ring_consume/src/lib.rs

# the single-consumer premise, and everything that enforces it
command grep -i 'single.consumer\|one consumer' ring_consume/src/lib.rs

# `Consumer::new` construction sites, counted per file. The count is the
# evidence and the lines are not: they are near-identical
# `let consumer = Consumer::new( .. )`, and what the premise turns on is that
# there are this many of them and no signature anywhere says how many a cursor
# may have.
command grep -rc 'Consumer::new' ring_consume/src/ ring_consume/tests/ \
  | command grep -v ':0$' | sed 's|ring_consume/||' | sort

# the window: is there any guard type in this crate?
command grep 'impl Drop\|struct .*Guard' ring_consume/src/lib.rs || echo "  none"
```

Live output:

```
ring_claim     CAS 2  store 0  load 1
ring_consume   CAS 0  store 2  load 1
ring_publish   CAS 1  store 0  load 1
ring_barrier   CAS 0  store 0  load 0
ring_gating    CAS 0  store 0  load 0
ring_cursor    CAS 2  store 1  load 5
    self.cursor.store( through, COMMIT );
      self.cursor.store( end, COMMIT );
const COMMIT : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
//! Single-consumer available-range computation and commit.
//! ## Why this is single-consumer
/// One consumer's position, and the barrier bounding how far it may read.
src/lib.rs:12
tests/allocation_test.rs:2
tests/consume_test.rs:18
tests/manual/readme.md:2
  none
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CN6 | `ring_consume` | n/a — observation | The rejected single-call-with-guard design is named and priced in the module doc, which is the crate's primary asset |
| CN7 | `ring_consume` | **latent hazard** | The window between `available` and `commit` is explicit, unguarded, and unguardable by any signature this crate could offer |
| CN8 | family | n/a — observation | The two halves are exact atomic opposites — 2 CAS / 0 store against 0 CAS / 2 store — and neither crate records it |
| CN9 | `ring_consume` | **latent hazard** | Single-consumer is a premise enforced by prose; the plain stores are correct only while it holds and nothing checks that it does |
