# Type: The Lifetime on `Consumer`

### Scope

**Purpose:** Record what the single `'a` on `Consumer< 'a >` binds, establish
that it costs the caller nothing, and record the ownership asymmetry it exposes
against the write half.

**Responsibility:** `Consumer`'s type parameters and derives as a type-level
statement about ownership and duplication.

**In Scope:** The one lifetime; its variance; the missing `Copy`; the comparison
with `Claimer< 'a >`.

**Out of Scope:** The runtime cost of the two borrows, which is
[`data_structure/002`](../data_structure/002_two_borrows_and_no_owned_state.md).
The `const` surface, which is [`001`](001_availables_const_surface.md).

---

## One Lifetime, Two Fields

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_consume/src/lib.rs | grep -A5 'pub struct Consumer'
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs   | grep -A5 'pub struct Claimer'
```

Live output:

```
pub struct Consumer< 'a >
{
  cursor : &'a PaddedCursor,
  barrier : Barrier< 'a >,
}

pub struct Claimer< 'a >
{
  cursor : PaddedCursor,
  consumers : &'a GatingSet,
}
```

```rust
pub struct Consumer< 'a >
{
  cursor : &'a PaddedCursor,
  barrier : Barrier< 'a >,
}

pub struct Claimer< 'a >
{
  cursor : PaddedCursor,          // owned
  consumers : &'a GatingSet,
}
```

### CN49 — The Single `'a` Unifies Two Independent Borrows and Costs Nothing

Both fields carry the same `'a`, which reads as a requirement that the cursor and
the barrier's dependency slice live equally long. They do not have to.

Both positions are covariant in `'a` — `&'a T` is, and `Barrier< 'a >` is a
slice reference and a length — so the compiler unifies `'a` to the *shorter* of
the two at every construction site. A cursor from an outer scope and a barrier
from an inner one compose without complaint:

```rust
let cursor = PaddedCursor::default();
{
  let barrier = Barrier::over( &published );        // inner scope
  let consumer = Consumer::new( &cursor, barrier ); // cursor from outer
  consumer.available()                              // -> len 350
}
// cursor still usable here
```

Measured: built, queried `available().len()` = 350, and the outer cursor remained
usable after the inner scope closed. So the single parameter is not a constraint
a caller can trip over.

The alternative was reachable and the family knows it — `ring_handle::Drain< 'c,
'a, T >` is the one struct in all 33 crates with two lifetime parameters, so
"two, when they are genuinely independent" is an established local convention.
`Consumer` did not need it because unification is free here, and `Drain` did
because its two borrows nest rather than unify.

**Cost:** none. Recorded because the signature looks like a constraint, is not
one, and the reason is variance — which the source does not mention and which a
reader would otherwise have to derive or test.

---

### CN50 — The Producer Owns Its Cursor and the Consumer Borrows Its Own; Withholding `Copy` Does Not Close What That Opens

The two halves of the handshake hold the same kind of state in opposite ways:

| | Own cursor | Other side's cursors |
|--|-----------|----------------------|
| `Claimer< 'a >` | `cursor : PaddedCursor` — **owned** | `consumers : &'a GatingSet` — borrowed |
| `Consumer< 'a >` | `cursor : &'a PaddedCursor` — **borrowed** | `barrier : Barrier< 'a >` — borrowed |

Same role, opposite ownership. It follows from how each is wired rather than from
a stated principle: a producer is naturally the sole writer of its own cursor and
can own it, whereas a consumer's cursor must be reachable by the producer's
gating set, so someone above both has to hold it and lend it out
([`data_structure/002`](../data_structure/002_two_borrows_and_no_owned_state.md)
CN24).

The consequence is that `Claimer` is structurally single-instance — it owns the
cursor, so a second `Claimer` means a second cursor, which is a different
producer — while `Consumer` is not. Nothing about `Consumer`'s type stops two
existing over one cursor, and the crate's single-consumer requirement
([`decisions/002`](../decisions/002_plain_stores_rather_than_compare_exchange.md))
is therefore a caller obligation with no type-level backing.

The one gesture toward it is a withheld derive:

```
195:#[ derive( Debug ) ]
196:pub struct Consumer< 'a >
```

`Debug` and nothing else. Every field is `Copy` — `&'a PaddedCursor` is, and
`Barrier< 'a >` derives `Debug, Clone, Copy` — so `Copy` and `Clone` were both
available and both were declined. That is the right instinct, and it is worth
being precise about what it buys, which is very little. Compiled and run:

```rust
let a = Consumer::new( &cursor, barrier );
let b = Consumer::new( &cursor, barrier );          // barrier is Copy
let c = Consumer::new( a.cursor(), a.barrier() );   // straight off an existing one
```

```
three consumers over one cursor: 10 10 10
after a commits:                  0  0  0
```

Three live `Consumer`s over one cursor, and the third is the sharp one: it is
built entirely out of `a`'s own accessors. `cursor()` returns the borrow at full
`'a`, `barrier()` returns a `Copy` value
([`item/002`](../item/002_the_eight_of_a_consumer.md) CN28, CN29), and together
they reconstruct the type that deliberately does not implement `Clone`. The
withheld derive is bypassed by the public surface of the same type.

The second line of output is the consequence: `a` commits, and `b` and `c`
immediately see an empty run for records they never read. All three will happily
call `commit`, racing plain stores against each other
([`decisions/002`](../decisions/002_plain_stores_rather_than_compare_exchange.md)
CN9).

So withholding `Copy` removes the *accidental* duplicate — a `Consumer` passed by
value where a reference was meant — and leaves the deliberate one entirely open.
That is a real and useful distinction, and the reason it is a finding is that
nothing says so. A reader who notices the missing derive can reasonably conclude
the type is protecting the single-consumer invariant. It is protecting one
narrow, common way of violating it by accident.

**Cost:** reachable as a documentation gap. The derive choice is correct, its
scope is much narrower than it looks, and the invariant it gestures at is
enforced nowhere.

---

## What the Type Does Say

| Signal | What a reader may conclude |
|--------|----------------------------|
| `'a` on both fields | the `Consumer` outlives neither the cursor nor the dependency set |
| no `Copy`, no `Clone` | duplication is not routine |
| `cursor : &'a` | the cursor is not this type's to own |
| `cursor() -> &'a PaddedCursor` | and the caller may have it back at full lifetime |

The last two together are the honest summary: the type is a *view*, and it says
so consistently. What it does not say is that only one such view may act at a
time.

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| type | [001](001_availables_const_surface.md) | the const surface and `Seq`'s representation |
| data_structure | [002](../data_structure/002_two_borrows_and_no_owned_state.md) | why the cursor must be borrowed |
| decisions | [002](../decisions/002_plain_stores_rather_than_compare_exchange.md) | the single-consumer requirement this does not enforce |
| item | [002](../item/002_the_eight_of_a_consumer.md) | `barrier()` returning `Copy` by value |
| lifecycle | [002](../lifecycle/002_the_consumer_over_a_rings_life.md) | disposability, which the same shape enables |

### Sources

| What | Where |
|------|-------|
| `Consumer`'s declaration | `ring_consume/src/lib.rs:209-214` |
| `Claimer`'s declaration | `ring_claim/src/lib.rs:256-261` |
| The only two-lifetime struct | `ring_handle/src/lib.rs:251` |
| The variance probe | two-scope construction, `available().len()` = 350 |

### Tests

| Claim | Verified by |
|-------|-------------|
| Cursor and barrier may come from different scopes | the two-scope probe above |
| `Consumer` derives only `Debug` | `lib.rs:195` |
| Both fields are `Copy` | `&T` always is; `Barrier` derives `Debug, Clone, Copy` at `ring_barrier/src/lib.rs:80` |
| Three `Consumer`s over one cursor compile and run | the duplication probe, including one built from `a.cursor()`/`a.barrier()` |
| `Drain` is the only two-lifetime struct | the family-wide `pub struct .*< ` grep |
