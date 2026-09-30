# Pitfall: The Obvious Implementation Forks the Constant

### Scope

- **Purpose**: Describe the change that replaces an inherited constant with a local literal, show why it passes every check, and give the evidence that the family has already made it three times.
- **Responsibility**: State the trap, name what makes it attractive, list the checks it survives, and give the counter-measure that actually works.
- **In Scope**: `#[ repr( align( 64 ) ) ]` in place of `CacheAligned`; the three existing restatements of the gating ordering.
- **Out of Scope**: The restriction stated positively, which is [`invariant/002`](../invariant/002_the_number_64_never_appears_here.md).

### The Trap

A reviewer looks at `PaddedCursor` and sees a dependency that supplies one attribute:

```rust
// today
use ring_align::CacheAligned;
pub struct PaddedCursor( CacheAligned< AtomicSeq > );

// the "simplification"
#[ repr( align( 64 ) ) ]
pub struct PaddedCursor( AtomicSeq );
```

**The second version is shorter, has one fewer dependency, one fewer indirection,
and passes every automatic check this crate has.** It is not a careless change.
It is the change a competent reviewer suggests.

### Why It Passes

| Check | Result after the change | Why |
|-------|-------------------------|-----|
| `align_of::< PaddedCursor >() == CACHE_LINE` | ✅ passes | `CACHE_LINE` is 64 and so is the literal |
| `size_of::< PaddedCursor >() == CACHE_LINE` | ✅ passes | `repr( align )` rounds the size up too |
| Two cursors ≥ 64 bytes apart | ✅ passes | Same layout, arrived at differently |
| Every cursor on a line boundary | ✅ passes | Same |
| Array stride == `CACHE_LINE` | ✅ passes | Same |
| `cargo clippy -- -D warnings` | ✅ passes | Nothing about it is a lint |
| The doc examples | ✅ passes | The public surface is unchanged |

**All five layout tests still pass, because they were all still true.** The type
still occupies exactly one cache line. What changed is *where the number came
from*, and no test can see that — a test observes values, and both versions
produce the same value.

### What It Actually Costs

The family acquires a second, independent statement of the cache-line size.
Today both say 64, so nothing is wrong. The cost is realised only on the day
someone ports to a 128-byte line:

| | `ring_align::CACHE_LINE` | `PaddedCursor` | Result |
|---|---|---|---|
| Before the port | 64 | inherits 64 | Correct |
| After editing `ring_align` only | **128** | still 64 | **Cursors share lines. Every test passes.** |

The last row is the failure. `align_of == CACHE_LINE` compares 64 against 128 and
fails — *if the type still inherits*. With a literal, the test compares 64
against 128 and **also** fails, which sounds like it saves us. It does not,
because the natural fix at that point is to change the literal to 128 in one
place, and the person doing it has no reason to look for a third.

The real cost is that the number stops having one owner. Everything after that is
a search problem.

### The Family Has Already Done This — Three Times

Not with the cache line. With the gating ordering, which is the same shape:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'Ordering::Acquire' --include=*.rs */src/ \
  | grep -vE ':[[:space:]]*(///|//!)'
```

Live output:

```
ring_batch/src/lib.rs:    let at = producer.load(Ordering::Acquire);
ring_batch/src/lib.rs:    let behind = consumer.load(Ordering::Acquire);
ring_cursor/src/lib.rs:pub const GATING: Ordering = Ordering::Acquire;
ring_debug/src/lib.rs:const OBSERVE: Ordering = Ordering::Acquire;
ring_mpsc/src/lib.rs:pub const OBSERVE: Ordering = Ordering::Acquire;
ring_shutdown/src/lib.rs:        self.closed.load(Ordering::Acquire)
```

| Site | Form | Cites `ring_cursor` in prose? | Could reach `GATING`? |
|------|------|:-----------------------------:|-----------------------|
| `ring_cursor:89` | `pub const GATING` | — | the original |
| `ring_debug:73` | `const OBSERVE : Ordering = Ordering::Acquire;` | **yes** — "matching the gating reads in `ring_cursor`" | **yes**, it declares `ring_cursor` |
| `ring_batch:321-322` | inline, twice | **no** — `grep -c ring_cursor` over its source is `0` | **no**, no manifest edge; generic over `SeqCell` |
| `ring_mpsc:250` | `pub const OBSERVE : Ordering = Ordering::Acquire;` | **no** — the doc comment now says only "the other half of `PUBLISH`"; the crate's one remaining prose mention of `GATING` sits on `OWN` instead (`:273-286`, a correct contrast, not a claimed match) | **yes**, and it *already imports `GATING`* at `:205` |

**The citation column no longer tracks the reachability column as cleanly as it
once did.** `ring_debug` still writes the paragraph explaining it is matching
`GATING`, then still doesn't use it. `ring_mpsc` used to do the same on
`OBSERVE`; that paragraph has since been edited out, and the crate's only
current prose mention of `GATING` sits on the unrelated `OWN` constant, which
correctly contrasts rather than claims equivalence. Losing the citation did not
close the fork — `OBSERVE` still hardcodes `Acquire` independently of the
`GATING` already in scope two constants away. The one that could not reach it
doesn't mention `ring_cursor` at all.

So the failure has two distinct causes, and only one of them is about awareness:

| Site | Cause | Fixable by |
|------|-------|------------|
| `ring_debug:73`, `ring_mpsc:250` | Chose a local name over an available import | An edit — `GATING` is one `use` away |
| `ring_batch:321-322` | Generic over `SeqCell`, never names a `PaddedCursor`, so reaching the constant means a manifest edge for one `Ordering` | Not an edit — a design question |

**`ring_mpsc` is still a sharp case, though a less ironic one than it was.** One
file: importing `GATING` on line 205, declaring a second, independent public
name for the same `Acquire` on line 250 (`OBSERVE`), with no comment linking the
two — that link existed in an earlier revision of this file and has since been
edited out. Nothing is hidden from the author either way: the constant is
already in scope, imported for a different constant's (`OWN`'s) doctest three
constants down.

**`ring_batch` is the instructive one.** Its `claim_gated` takes an
`order : Ordering` parameter *and* hardcodes `Ordering::Acquire` for the two
gating reads, passing the parameter only to the claim itself:

```rust
pub fn claim_gated< P : SeqCell, C : SeqCell >
( producer : &P, consumer : &C, count : usize, capacity : Capacity, order : Ordering )
-> Result< BatchClaim, RingError >
{
  // …
  let at = producer.load( Ordering::Acquire );      // not `order`
  let behind = consumer.load( Ordering::Acquire );  // not `order`
  // …
  Ok( claim( producer, count, order ) )             // `order` used here only
}
```

That split is defensible — the gating read's ordering is not the caller's
business, which is the same argument `GATING` itself makes. It is also invisible:
the signature says the caller chooses the ordering, and for two of the three
atomic operations it does not.

### Why Prose Citation Is Not Enough

Two of the three sites are not ignorant. They are *correct and independent*.
That is worse than ignorant, because:

| | An author who never knew | An author who cited it |
|---|---|---|
| Would a review catch it? | Maybe — "isn't there a constant for this?" | No — the comment answers that question |
| Does the code look wrong? | Slightly | Not at all |
| Would changing the original propagate? | No | **No** |

Prose citation buys traceability for a human reading that file, and buys nothing
mechanical. The value still has to be changed in four places, and three of them
say "matching `ring_cursor`" while not matching it.

### What Would Actually Catch It

| # | Counter-measure | Catches | Cost |
|---|-----------------|---------|------|
| P1 | `tests/manual/readme.md` M1 — grep this crate's source for `64` | The cache-line fork, **here only** | A human, per release |
| P2 | A family-wide grep for `Ordering::Acquire` outside `ring_cursor` | The ordering fork, everywhere | A test in a crate that can see all of them — none currently can |
| P3 | `cargo +nightly udeps` at level 4 | The `ring_align` edge going unused *after* the fork | Already available, rarely run |
| P4 | Making the constant unavoidable — a type that carries it | Both, mechanically | A design change, and it is what already works |

**P4 is the one with evidence behind it.** Where the decision travels as a method
on a type the consumer holds, it has never forked: `ring_spsc` gets the
cache-line predicate through `CursorPair::on_distinct_lines` and has no literal.
Where it travels as a free constant, it has forked three times out of three
opportunities. [`integration/002`](../integration/002_who_reads_a_cursor.md)
measures this directly.

P2 is worth noting as unbuilt rather than rejected. It is a five-line test, and
the reason it does not exist is that no crate in the family depends on all the
others, so there is nowhere to put it that would see every site.

### CU41 — The Fork Is Not Hypothetical; It Already Happened One Crate Over

```
      claim.abs_diff( consume ) >= 64
```

A second copy of the cache-line constant, written as a literal, inside a method
whose name is shared with `ring_align`'s function.

**Finding.** This pitfall is documented from an instance rather than from
imagination, which changes what it is worth: the failure mode has an observed
occurrence, in the family's most heavily-worked crate, under a name that makes it
look like a call to the shared implementation. It is cited by pattern rather than
by line for the reason at
[`../integration/002`](../integration/002_who_reads_a_cursor.md) CU20.

---

### CU42 — The Detector Runs at a Level Ordinary Work Never Reaches

The second-order signal for this fork is `cargo +nightly udeps` reporting
`ring_align` unused once the wrapper is replaced by a literal.

**Finding.** That runs at verification level 4. Ordinary in-progress work runs
filtered tests, and task-completion gates run level 3. So the detector fires at a
level this project reaches rarely, and only if the author leaves the now-unused
dependency declared — an author who removes it in the same change has removed the
evidence too.

---

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_the_number_64_never_appears_here.md](../invariant/002_the_number_64_never_appears_here.md) | The same subject as a restriction on this crate |
| [../invariant/001_one_cursor_one_line.md](../invariant/001_one_cursor_one_line.md) | The five tests this change passes |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_four_dependencies_all_used.md](../integration/001_four_dependencies_all_used.md) | The `ring_align` edge this change removes |
| [../integration/002_who_reads_a_cursor.md](../integration/002_who_reads_a_cursor.md) | P4's evidence — where sharing survived and where it did not |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_gating_is_fixed_not_a_parameter.md](../decisions/001_gating_is_fixed_not_a_parameter.md) | The ordering constant, and why fixing it did not prevent the fork |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_forwarding_newtype.md](../pattern/001_the_forwarding_newtype.md) | The shape this change deletes |

### Pitfalls

| File | Relationship |
|------|--------------|
| [002_a_reading_that_consults_one_cursor.md](002_a_reading_that_consults_one_cursor.md) | The other trap — a correct-looking read that is not atomic |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:89, 143-144` | The constant and the type the change would rewrite |
| `ring_debug/src/lib.rs:66-73` | Restatement 1, with its citation |
| `ring_batch/src/lib.rs:306-322` | Restatement 2 and 3, inline, in a generic function that also takes an unused-for-gating `order` parameter |
| `ring_mpsc/src/lib.rs:205, 250` | Restatement 4 — the import and the independent fork; the doc-comment link between them has since been edited out |
| `ring_mpsc/src/lib.rs:860-866` | The cache-line version of the same fork |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:54-126` | The five tests that pass after the change |
| `tests/manual/readme.md` M1 | P1 — the only check that would notice |
| `tests/manual/readme.md` § closing note | The crate's own statement of this trap |
