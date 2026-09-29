# Non-Functional Requirement: The One Allocation and the Zero Callers

### Scope

**Purpose:** Record that two of the three functions allocate nothing and the
third allocates once per call at exactly `8 × count` bytes, that this rules it
out of every path the family cares about, and that the crate's one plausible
consumer already avoided it.

**Responsibility:** Allocation behaviour of the public surface, measured, and
what it implies about where each function can be called.

**In Scope:** `of`, `aliases`, `run` under a counting global allocator, release
build.

**Out of Scope:** cycle costs are
[`non_functional_requirement/001`](001_what_the_fold_costs.md). The return-type
decision itself is
[`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md).

---

## Measured Under a Counting Allocator

A global allocator that increments a counter on every `alloc`, wrapping the
system allocator, release build:

```
--- (2) what each of the three functions allocates ---
  of                          0 allocs over 10,000 calls (sink true)
  aliases                     0 allocs over 10,000 calls (sink false)
  run (count = 8)         10000 allocs over 10,000 calls, 640000 bytes (len 80000)

--- (3) what one `run` costs, by count ---
  count 0      0 alloc       0 bytes  len 0  capacity 0
  count 1      1 alloc       8 bytes  len 1  capacity 1
  count 8      1 alloc      64 bytes  len 8  capacity 8
  count 1024   1 alloc    8192 bytes  len 1024  capacity 1024
  count 4096   1 alloc   32768 bytes  len 4096  capacity 4096
```

---

### IX23 — The Allocation Is Exactly Sized, Once, and Still Disqualifying

**Finding.** The good news first, because it is real: `run` allocates *once*, not
once per element, and the resulting `Vec` has `capacity == len` at every count
measured. There is no growth-doubling and no slack — `collect` reads an exact
`size_hint` from the `Map` over a `Range` and asks for the final size directly.
`count = 0` allocates nothing at all. As a `Vec`-returning function, this is the
best version of itself.

It is still the wrong shape for this family. Every path `ring_index` exists to
serve is a lock-free write or read path: a producer claiming a sequence, a
consumer draining a batch. Allocation on those paths reintroduces exactly what
the ring was built to avoid — an unbounded operation that can block, fail, or
call into a global lock, in the middle of a sequence of atomic operations that
were carefully chosen not to.

So the measurement is not "`run` is slow." It is that `run` cannot be called
from the code this crate is for, at any speed, and no amount of tuning the
allocation changes that. `of` and `aliases` measured zero allocations over
10,000 calls each, which is what makes them usable there.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_index
command grep -F 'disqualifies it from the family'"'"'s' src/lib.rs
```

Live output:

```
/// but an allocation regardless. That disqualifies it from the family's
```

**Disposition:** applied — `run`'s own doc comment now discloses the
allocation profile this finding measured: one exactly-sized allocation per
call, still disqualifying for the family's lock-free claim and consume
paths regardless of speed, naming `of`/`aliases` as the zero-allocation
functions those paths use instead.
Now prints: `disqualifies it from the family's`

---

### IX24 — The Cost Is Never Paid, Because Nothing Calls It

[`api/001`](../api/001_three_functions_three_must_use_one_reached.md) IX6
records that no source or test file in any of the 33 crates names `run`. So the
640,000 bytes above is a number this repository never allocates.

**Finding.** That makes IX23 a latent property rather than an active cost, and
it also explains why nobody noticed. A function with one caller and a bad
allocation profile gets found the first time someone profiles that caller. A
function with zero callers never appears in a profile at all.

The one crate that wanted the functionality reached past it:
`ring_batch::drain_order` yields `( Seq, SlotIndex )` pairs from an
`impl Iterator`, composing `of` directly, and allocates nothing
([`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md)).
That is the whole empirical record on `run` — one crate had the use case, looked
at the surface, and built the non-allocating version itself.

`run` is therefore paying no runtime cost and carrying a maintenance one: it is
tested by four of the crate's ten tests, it is `#[ must_use ]`, it is documented
with a justification that cites a feature implemented by the function that
replaced it, and it has never run outside its own test file.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md) | The return type as a decision, with the alternative priced |
| [`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md) | The non-allocating version that exists, in another crate |
| [`api/001`](../api/001_three_functions_three_must_use_one_reached.md) | The reach census this depends on |
| [`item/002`](../item/002_the_two_that_nothing_calls.md) | `run` and `aliases` read on their own |

### Sources

| Fact | Where |
|------|-------|
| Allocation counts per function | Counting-allocator probe, release, quoted above |
| Exact sizing at five counts | Same probe, section 3 |
| Zero callers of `run` | `api/001` IX6 census |
| The non-allocating alternative | `ring_batch/src/lib.rs:358-362` |

### Tests

| Test | Covers |
|------|--------|
| `an_empty_run_is_empty` | `count = 0`, which is also the only allocation-free case |
| `a_run_wraps_at_most_once_within_one_capacity` and two siblings | `run`'s output, never its allocation |
| *(to create)* | Nothing here is a unit-test assertion; an allocation count needs a harness the family does not have |
