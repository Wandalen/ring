# Pattern: One Fold, Two Questions

### Scope

- **Purpose**: Identify the shape that lets one function serve two callers who mean opposite things by its answer, and name the single design move that makes it possible.
- **Responsibility**: Give the two questions, show where the shared fold lives and why, and state what enforces the arrangement.
- **In Scope**: `slowest` as shared machinery; `ring_gating` and `ring_barrier`'s opposing readings of it.
- **Out of Scope**: What the fold computes, and what it allocated until `b7e075ca`, which is [`algorithm/001`](../algorithm/001_the_slowest_fold.md).

### The Shape

Two callers ask structurally identical questions of the same kind of input and
draw opposite conclusions from the answer. The shape puts the *computation* in
one place and leaves the *interpretation* in each caller.

| Caller | Reads | To bound | An empty set means |
|--------|-------|----------|--------------------|
| `ring_gating::GatingSet` | a set of **consumers** | a **producer** | nobody is reading, so nothing can be lost → **full capacity** |
| `ring_barrier::Barrier` | a set of **dependencies** | a **consumer** | nothing is declared readable → **zero** |

Same slice type, same fold, same ordering. Opposite sign on the answer.

### Where the Fold Lives, and Why Not in Either Caller

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every mention in the family source --'
grep -rn 'ring_cursor::slowest' --include=*.rs */src/ | LC_ALL=C sort -t: -k1,1 -k2,2n | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and the test files that now exercise it --'
grep -rl 'ring_cursor::slowest' --include=*.rs */tests/ | LC_ALL=C sort
```

Live output:

```
  -- every mention in the family source --
ring_barrier/src/lib.rs://! [`ring_cursor::slowest`] and lives in neither of them.
ring_barrier/src/lib.rs:        ring_cursor::slowest(self.dependencies)
ring_cursor/src/lib.rs:/// assert_eq!(ring_cursor::slowest(&[]), None);
ring_cursor/src/lib.rs:/// assert_eq!(ring_cursor::slowest(&cursors), Some(Seq(4)));
ring_cursor/src/lib.rs:/// assert_eq!(ring_cursor::slowest(&cursors), Some(Seq(9)));
ring_gating/src/lib.rs://! `ring_cursor::slowest` returns `None` for an empty set rather than `Seq::ZERO`,
ring_gating/src/lib.rs:    /// The fold itself is [`ring_cursor::slowest`], shared with `ring_barrier`,
ring_gating/src/lib.rs:        ring_cursor::slowest(&self.cursors)
  -- and the test files that now exercise it --
ring_barrier/tests/allocation_test.rs
ring_claim/tests/allocation_test.rs
ring_cursor/tests/allocation_test.rs
```

**Eight source mentions; two of them calls.** The rest is documentation about the
fold or a doctest of it, and the test files listed after them measure what it
costs rather than call it in anger. `ring_gating`'s module doc now carries a
second mention — a `//!` line reattributing `slowest` from `ring_seqno` to
`ring_cursor` (a correction made elsewhere, not a new fold) — which is why this
census grew by one without a new caller appearing. The two callers are:

```
ring_gating/src/lib.rs:199:    ring_cursor::slowest( &self.cursors )
ring_barrier/src/lib.rs:193:    ring_cursor::slowest( self.dependencies )
```

Both consumer crates say so in their own documentation, in nearly the same words:

| Crate | Says |
|-------|------|
| `ring_cursor:96-99` | "The *questions* differ, and are argued in those crates; the fold does not, and lives here so that 'read every cursor at [`GATING`] and take the minimum' is written once. **A second copy is how one of them ends up reading `Relaxed`.**" |
| `ring_gating:175-176` | "The fold itself is [`ring_cursor::slowest`], shared with `ring_barrier`, which asks the opposite question of the same kind of slice." |
| `ring_barrier:28` | "…[`ring_cursor::slowest`] and lives in neither of them." |

**Three crates document one function's placement.** That is a lot of prose for a
`.min()`, and it is proportionate to the failure it prevents: two independently
written folds where one reads `Relaxed`, discovered as a torn read under
contention on some machines, some of the time.

### The Move That Makes It Work

The fold answers `None` for an empty slice rather than `Seq::ZERO`.

That single choice is what allows one function to serve both callers. `Seq::ZERO`
is a *valid position* — the position of a consumer that has read nothing — so
returning it would collapse "no cursors" into "a cursor at the start", and the
two callers need those to mean different things:

```rust
// ring_gating
self.slowest().map_or( self.capacity.get(), | slowest | … )   // empty → full capacity

// ring_barrier
self.frontier().map_or( 0, | frontier | … )                    // empty → zero
```

`map_or` on both sides, with opposite defaults. **The shared function declines to
pick an identity element, and each caller writes its own down where it can be
read and argued.**

Had the fold returned `Seq::ZERO`, `ring_gating` would report a gating set with
no consumers as blocking a producer one lap ahead — a ring nobody reads, blocked
forever — and nothing in `ring_cursor` would look wrong.

### The General Rule

> A fold shared by callers that disagree about the identity element must not pick
> one. `Option` is how it declines.

The rule generalises past this crate: any reduction over a possibly-empty
collection where the empty case is *semantically* rather than *arithmetically*
determined belongs in `Option`. Sum and product have identity elements the
mathematics supplies; "slowest" does not.

### What Enforces the Arrangement

**Nothing, and that is the honest answer.**

| # | Check | Status |
|---|-------|--------|
| V1 | The compiler | Would not object to either caller inlining its own fold |
| V2 | A test in `ring_cursor` | The crate has none for `slowest` at all — its coverage is one doctest plus the two consumers' suites |
| V3 | `ring_barrier/tests/barrier_test.rs:410` | `assert_eq!( barrier.frontier(), set.slowest() )` — the only assertion anywhere that compares the two callers' answers, and it lives in one of them |
| V4 | The three doc comments above | Prose. They tell a reader the arrangement exists; they do not detect its removal |

**V3 is weaker than it looks.** The test is named
`a_barrier_over_a_gating_set_reads_that_set_and_not_a_copy` and its comment says
what it is for: that the cursors a producer gates on are *the same objects* a
downstream barrier waits on. The `frontier() == slowest()` equality is how it
demonstrates aliasing, not a check that one fold serves both.

So it would pass unchanged if each caller grew its own private fold — as long as
both folds computed the same thing. It catches divergence in the *answer*, which
is the symptom; nothing catches divergence in the *ordering*, which is the failure
`ring_cursor:99` actually names ("a second copy is how one of them ends up
reading `Relaxed`"). A `Relaxed` fold returns the same value on any test that is
not racing.

**The defence against the documented failure mode is zero assertions.** The three
doc comments are the whole of it.

### Contrast With the Family-Scale Version

The same anti-duplication reasoning applied to `GATING` — one constant, ten
consumers — did **not** hold: the family states the same `Acquire` decision in
four places. The difference is instructive:

| | `slowest` | `GATING` |
|---|-----------|----------|
| Consumers | 2 | 10 |
| Duplicates | 0 | 3 |
| Why | Both callers need a `&[ PaddedCursor ]`, so both already depend on this crate | Three of the sites never handle a `PaddedCursor` — one is generic over `SeqCell`, and reaching the constant would mean a manifest edge for an `Ordering` |

**Sharing survives where the shared thing travels with a type both callers
already hold.** A constant has no type to travel with, which is why it leaks. The
measurement behind that claim is in
[`decisions/001`](../decisions/001_gating_is_fixed_not_a_parameter.md).

### CU39 — The Justification and the User Base Are the Same Two Lines

```
ring_barrier/src/lib.rs:193:    ring_cursor::slowest( self.dependencies )
ring_gating/src/lib.rs:199:    ring_cursor::slowest( &self.cursors )
```

The argument for placing the fold here is that two crates ask it opposite
questions. Those two crates are also its only two callers.

**Finding.** With n = 2, "shared" and "used twice" are the same observation, so
the pattern is asserted rather than demonstrated. That is not an argument against
the placement — the two questions genuinely are opposite — but it does mean the
third caller is the one that will test whether the fold generalises or whether it
was two special cases that happened to agree.

---

### CU40 — Three Crates Test the Empty Case, and Nothing Connects Them

| Where | What it asserts |
|-------|-----------------|
| this crate's `slowest` doctest | `slowest( &[] )` is `None` |
| `ring_gating/tests/gating_test.rs` | an empty gating set admits |
| `ring_barrier/tests/barrier_test.rs` | an empty barrier does not |

Plus `an_empty_barrier_and_an_empty_gating_set_answer_oppositely`, which asserts
the two readings differ.

**Finding.** All four assertions are independent. The `None` returned here is
what makes the two opposite readings possible, and no assertion links this
crate's return to either consumer's interpretation — so a change from `None` to
`Some( Seq::ZERO )` would fail three tests in two other crates and none here,
with nothing in any of the three naming the cause.

---

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_slowest_fold.md](../algorithm/001_the_slowest_fold.md) | The fold itself, its `None`, and the allocation it makes |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_surface_that_decides.md](../api/002_the_surface_that_decides.md) | `slowest` as a public item, and its degenerate case |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_gating_is_fixed_not_a_parameter.md](../decisions/001_gating_is_fixed_not_a_parameter.md) | The same reasoning at family scale, and why it did not hold there |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_who_reads_a_cursor.md](../integration/002_who_reads_a_cursor.md) | The two callers, among the ten consumers |

### Patterns

| File | Relationship |
|------|--------------|
| [001_the_forwarding_newtype.md](001_the_forwarding_newtype.md) | The crate's other anti-duplication shape — the one the compiler enforces |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:89-122` | The fold, and its own argument for living here |
| `ring_gating/src/lib.rs:202-228` | The producer-bounding question, and its `map_or` default |
| `ring_barrier/src/lib.rs:13-28, 190-219` | The consumer-bounding question, and its opposite default |

### Tests

| File | Relationship |
|------|--------------|
| `ring_barrier/tests/barrier_test.rs:399-412` | V3 — aliasing, which incidentally compares the two callers' answers |
| `ring_gating/tests/gating_test.rs:126-136` | The slowest consumer bounds regardless of its index |
| `ring_gating/tests/gating_test.rs:190` | The empty set answers `None` on the gating side |
| `src/lib.rs:106-118` | The fold's own doctest |
