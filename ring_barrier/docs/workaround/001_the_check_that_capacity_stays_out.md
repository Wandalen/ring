# Workaround: The Check That Capacity Stays Out

### Scope

- **Purpose**: Record why the crate's central claim is guarded by a grep at all, and why that grep now proves less than it did.
- **Responsibility**: State the check, the weakening its own plan documents, where the load-bearing assertion moved to, and what remains uncovered.
- **In Scope**: Manual check B1, and what it can and cannot see.
- **Out of Scope**: The invariant itself — see [`invariant/002`](../invariant/002_capacity_never_enters_the_arithmetic.md).

### What Is Being Worked Around

The claim is an **absence**: no capacity anywhere in the barrier's arithmetic
([`invariant/002`](../invariant/002_capacity_never_enters_the_arithmetic.md)).
Absences are not testable by calling things — every test of a correct
implementation and every test of one that quietly clamps to a capacity would
agree on every input where the clamp does not bind. The workaround is to test the
*source text* instead of the behaviour:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -E "capacity|Capacity" \
  || echo '(no mention of capacity survives the comment filter)'
# control: the identical expression without the comment filter
grep -cE "capacity|Capacity" ring_barrier/src/lib.rs
```

Live output:

```
(no mention of capacity survives the comment filter)
6
```

**Expected: no output.** Run today it produces none — `Capacity` is not
imported, not named, and does not appear even in a doc example.

### BR10 — The Check Passes for a Weaker Reason Than It Used To

This is stated in the plan itself rather than discovered here, and it is the
honest part of the check:

> **Expected:** no output. `Capacity` is not imported and does not appear at all,
> not even in a doc example — which is a weaker result than it was before the
> signature changed, and worth saying so. While `Barrier::over` took a
> `&GatingSet`, every doc example here had to build one, so a capacity was in the
> file and this check was reading real restraint. Now the constructor takes a
> bare `&[ PaddedCursor ]` and there is nothing to build; the check passes
> because capacity has no way in, not because it was kept out.
>
> — `tests/manual/readme.md` § B1

| | Before the signature change | Now |
|--|-----------------------------|-----|
| `Barrier::over` takes | `&GatingSet` | `&[ PaddedCursor ]` |
| A doc example must build | a `GatingSet`, so a `Capacity` | a `Vec< PaddedCursor >` |
| So a `Capacity` in the file is | present, and deliberately kept out of the *arithmetic* | absent, with no route in at all |
| The grep therefore reads | restraint | **the type signature** |

A check that passes because the thing it forbids is unreachable is not worthless
— it would fire the moment someone adds the import — but it is no longer
evidence that anyone is being careful. The distinction matters because the
[`decisions/002`](../decisions/002_a_slice_rather_than_an_aggregate.md) change
that weakened this check was made for an unrelated reason
([`pattern/001`](../pattern/001_the_borrowed_view_and_the_owned_set.md)), and
nobody noticed the check's value had changed until the plan said so.

### Where the Load-Bearing Assertion Moved

```rust
// tests/barrier_test.rs:185-193
let set = GatingSet::new( cap( 4 ), 1 );
set.cursor( 0 ).unwrap().store( Seq( 1_000 ), Ordering::Release );

assert_eq!( set.headroom( Seq( 1_000 ) ), 4, "the producer is clamped to one lap" );
assert_eq!(
  Barrier::over( set.cursors() ).available( Seq::ZERO ),
  1_000,
  "the consumer is not clamped at all"
);
```

**4 against 1,000, over the same cursor, in the same test.** That is the claim
the grep used to make and can no longer make on its own — and it is a real
assertion, not a restatement: any clamp introduced into `available` moves the
second number from 1,000 to at most 4, and the test fails loudly.

The capacity is `4` and the position is `1_000` for a reason. A clamp introduced
at *any* granularity — one lap, one capacity, one anything derived from a
four-slot ring — cannot produce 1,000. A test that used a capacity of 8 and a
position of 9 would fail against a lap clamp and pass against several others.

### What Is Still Uncovered

| | Caught by |
|--|-----------|
| `use ring_types::Capacity` added to the file | **B1's grep** |
| A clamp using a hard-coded number rather than a `Capacity` | `available_ignores_capacity_entirely` |
| A clamp added to `admits` but not `available` | `admits_and_available_never_disagree`, which asserts the two agree |
| A clamp added to **both** `admits` and `available` consistently | `available_ignores_capacity_entirely` |
| A capacity reaching `wait_for`'s **return** rather than its arithmetic | **nothing** — `wait_for` returns `self.frontier()`, and no test clamps a frontier |
| A capacity reaching `ring_cursor::slowest` one crate down | **nothing here** — that is `ring_cursor`'s check to own |

The last two rows are the gap. B1's grep covers this file only, and the fold it
guards is not in this file ([`algorithm/001`](../algorithm/001_the_frontier_in_two_delegations.md)
— two delegations, both outward). The invariant this crate states is
enforceable here only for the arithmetic this crate performs, which is one
`saturating_sub` and one comparison.

### Why This Shape Rather Than a Test

| Alternative | Why not |
|-------------|---------|
| A behavioural test only | Every clamp that does not bind on the tested inputs passes; the 4-vs-1000 test works precisely because it was designed to bind |
| A `#![ deny ]` lint | There is no lint for "does not mention a type" |
| A type-level guarantee | The arithmetic is `u64` and a `Capacity` is a `usize` newtype — a cast makes the mixing legal, and [`type/001`](../type/001_a_u64_distance_and_a_usize_headroom.md) shows 13 files in the family already cast |
| **A grep plus a binding test** | What is done — the grep catches the import, the test catches the arithmetic |

The pairing is the actual workaround: neither half is sufficient, and the plan
records which half is currently load-bearing.

### BR50 — The Whole Discriminating Power of the Guard Is One Filter

The word `capacity` appears six times in `src/lib.rs` and not once in code. All
six are `//!` module documentation, arguing at length about why capacity belongs
to `ring_gating` and not here — the crate discusses the thing it excludes far
more than crates that use it.

So the guard's `grep -vE "^[[:space:]]*//"` is not incidental tidying, it is the
check. Drop that one filter and the guard reports six matches and fails, on a
crate that is entirely correct, for the reason the crate is correct: it argues
its case in prose because it has nothing to say in code.

That failure direction is the right one — a dropped filter produces a false
alarm somebody investigates, not a silent pass. It is worth recording precisely
because it is the opposite of what B1 does (BR10), where the check passes for a
structural reason rather than because the property holds, and of the family's
loop guard, which passed for 33 crates without being able to fail. Three guards
over the same crate family, two of which cannot fail and one of which cannot
quietly succeed.

```sh
cd "$(git rev-parse --show-toplevel)"
# every mention, with the comment prefix that decides whether the guard sees it
grep -n "capacity\|Capacity" ring_barrier/src/lib.rs | cut -c1-24 | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
# with the filter, and without it
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs | grep -cE "capacity|Capacity"
grep -cE "capacity|Capacity" ring_barrier/src/lib.rs
```

Live output:

```
//! A gating answer i
//! the next slot it 
//! A barrier answer 
//! the two would mea
//! carries the capac
//! are neither owned
0
6
```

### Workarounds

| File | Relationship |
|------|--------------|
| [002_ring_gating_as_a_dev_dependency.md](002_ring_gating_as_a_dev_dependency.md) | The dependency that makes the 4-vs-1000 test writeable |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_frontier_in_two_delegations.md](../algorithm/001_the_frontier_in_two_delegations.md) | The fold that lives outside this file's grep |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_slice_rather_than_an_aggregate.md](../decisions/002_a_slice_rather_than_an_aggregate.md) | The change that weakened the check |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_capacity_never_enters_the_arithmetic.md](../invariant/002_capacity_never_enters_the_arithmetic.md) | The claim being guarded |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_borrowed_view_and_the_owned_set.md](../pattern/001_the_borrowed_view_and_the_owned_set.md) | Why the signature changed at all |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_returning_the_request_instead_of_the_frontier.md](../pitfall/002_returning_the_request_instead_of_the_frontier.md) | The family's other structural grep, and its window problem |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_u64_distance_and_a_usize_headroom.md](../type/001_a_u64_distance_and_a_usize_headroom.md) | Why a type-level guarantee would not hold |

### Sources

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` § B1 | The check, and its own account of the weakening |
| `ring_barrier/src/lib.rs` | The file the grep covers — no `Capacity` anywhere |
| `ring_gating/src/lib.rs:323` | The clamp on the other side, `advanced_by( capacity )` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:179-194` | 4 against 1,000, over one cursor |
| `tests/barrier_test.rs:196-221` | The two readings asserted to agree |
