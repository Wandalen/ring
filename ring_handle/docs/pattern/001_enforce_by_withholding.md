# Pattern: Enforce by Withholding, Not by Checking

### Scope

- **Purpose**: Give the practice this crate is built from its general form — make the disallowed operation unrepresentable rather than detectable — and state honestly what it costs and where it fails.
- **Responsibility**: The problem, the practice, its applicability limits, and its consequences.
- **In Scope**: The practice as applied here and beyond; the conditions under which it pays and the ones under which it does not.
- **Out of Scope**: This crate's two specific applications (→ [`invariant/`](../invariant/readme.md)); the acceptance criterion that keeps it enforced (→ [Proven by Code That Must Not Compile](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md)).

### Problem

A restriction has to be stated somewhere and enforced somewhere, and the two
are usually different places.

"Only the barrier drains." "Don't block inside a tick." "One producer per ring."
Each is written in a design document, and each is violated by code that compiles
cleanly, passes review, and works under test. The gap between the two locations
is where the violations live, and it has three properties that make it
unusually hard to close:

1. **The document is not read at the call site.** The author of the offending
   line is looking at an API, not at a rulebook.
2. **A runtime check has a cost and a coverage problem.** It runs in release
   builds where it is dead weight, or it is compiled out where it is needed, and
   either way it fires *after* the wrong thing happened.
3. **A violation often is not observable at all.** A second producer against a
   single-producer ring corrupts data on ARM and passes on x86-64
   (→ [`ring_spsc` invariant/001](../../../ring_spsc/docs/invariant/001_exactly_one_producer_one_consumer.md)).
   There is nothing for a check to catch, most of the time.

This crate's own tick-path surface names property 1's consequence precisely: the
wrong call is "one autocomplete away." Nobody decides to violate the
restriction; the editor offers it.

### Solution

**Do not make the disallowed operation detectable. Make it unrepresentable — by
partitioning the capability across values, and giving each caller only the value
whose capability it is entitled to.**

Three parts, and the third is the one that is usually skipped:

1. **Split the capability into distinct types.** Not one type with a mode flag —
   two types. A flag is checkable at runtime; a type is checkable at the call
   site.

2. **Withhold every route around the split.** No accessor returning the
   underlying object, no `Deref`, no public field, no `From` conversion between
   the two. This is a list of absences and it must be maintained as
   deliberately as any list of features
   (→ [Delegating an Operation to the Backend](../algorithm/002_delegating_to_the_backend.md)).

3. **Add a test that fails when the withheld thing reappears.** This is the part
   that makes the pattern survive maintenance. An ordinary test suite cannot
   express "this method does not exist" — adding a method never turns a positive
   test red. A compile-fail case can, and it is the only mechanism that can.

**Part 3 is what separates this pattern from merely writing a narrow API.** A
narrow API is a snapshot of one author's discipline; a narrow API plus a
compile-fail suite is a constraint with a detector. Without part 3, the pattern
decays silently and by increments, each of which looks like an improvement.

### Applicability

| Situation | Apply? |
|-----------|--------|
| The restriction partitions capabilities cleanly (this may publish, that may drain) | **Yes** — the canonical case, and what this crate is |
| Violations are silent, rare, or platform-dependent | **Yes** — runtime detection has nothing to detect most of the time; this is the strongest argument for the pattern |
| The restriction is contextual rather than structural ("only during a tick") | **Partly.** A type can carry the context (a token only the tick can produce), but the pattern degrades: contexts nest, and the token becomes a parameter threaded through everything |
| The restriction depends on runtime values ("only if capacity > 0") | **No.** Types cannot see runtime values. A check is correct here, and dressing it up as a type produces machinery that carries a phantom parameter and no additional safety |
| The API has one caller, in the same crate | **Rarely worth it.** The cost is two types and a compile-fail suite; the benefit is protection against a future author who may be the same person that afternoon |
| The capability split would multiply combinatorially (five capabilities → 32 types) | **No.** Use a capability parameter or a check. The pattern is for partitions of two or three, not for a lattice |
| The restriction is genuinely advisory — violating it is legal but discouraged | **No.** Making the compiler reject a legal program to express a preference is how a codebase acquires types nobody can name a reason for |

**Row four is the honest limit and the one most often crossed.** A great deal of
type-level machinery exists to express constraints that are actually runtime
conditions, and it produces types that carry a phantom parameter, a builder to
construct them, and no additional safety. The test is whether the compiler can
actually decide the question — if the answer depends on a value, it cannot.

**Row three is where this crate itself sits partway.** "Only the barrier drains"
is contextual. This crate makes it *possible* to enforce, by concentrating the
draining capability in one movable value; it does not enforce it, because
nothing stops that value being moved somewhere else
(→ [The Barrier Holds the Consumer](../lifecycle/002_the_barrier_holds_the_consumer.md)).
That is the pattern working as specified, and it is worth being precise that
"makes enforcement possible" is a weaker claim than "enforces."

### Consequences

- **The crate becomes mostly negative space, and reviews get harder.** What this
  crate does is best described by what it does not have, and an absence cannot
  be reviewed by reading a diff. A change that adds three lines can undo the
  whole design, and it looks like a three-line change.

- **Forwarding boilerplate accumulates, and the labour-saving fix is fatal.**
  Every backend operation needs a hand-written method on the appropriate handle.
  The obvious remedy — a `Deref` to the backend — restores every capability to
  both handles in three lines
  (→ [Capability Follows the Handle](../invariant/001_capability_follows_the_handle.md)'s V4).

- **Compile-fail tests are brittle in a specific, manageable way.** Their pinned
  stderr breaks on compiler upgrades, and the fix — regenerating the expected
  output — is also exactly how a genuine regression gets accepted. The
  distinguishing question is whether the program is still *rejected*, not whether
  the message matches.

- **Coverage is partial and the gaps are invisible.** Part 3's test asserts the
  specific withheld method is withheld. It says nothing about a newly derived
  `Clone`, a new `Sync` impl acquired from a changed field, or a new accessor —
  each of which defeats the pattern along an axis no existing case covers
  (→ [Proven by Code That Must Not Compile](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md)).

- **Error messages are worse than a check's.** A runtime check says "consumer
  handle required." The compiler says "no method named `drain` found for struct
  `Producer`," which is correct, arrives earlier, and explains less. Where the
  restriction is non-obvious, the type needs documentation the check would have
  carried in its message.

- **It composes badly with dynamic dispatch.** Two capability types cannot be
  stored in one collection without a trait object, and a trait object that
  covers both capabilities has re-merged them. This is the pattern's real
  boundary in a system that wants to hold heterogeneous handles.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_splitting_a_ring_into_two_ends.md](../algorithm/001_splitting_a_ring_into_two_ends.md) | Part 1 applied — the one point where the capability is partitioned |
| [../algorithm/002_delegating_to_the_backend.md](../algorithm/002_delegating_to_the_backend.md) | Part 2 applied — the forwarding boilerplate, and the `Deref` that would undo it |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | This pattern applied along the capability axis |
| [../invariant/002_no_parking_operation_is_reachable.md](../invariant/002_no_parking_operation_is_reachable.md) | The same pattern applied along the blocking axis, showing it composes across axes |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_barrier_holds_the_consumer.md](../lifecycle/002_the_barrier_holds_the_consumer.md) | Applicability row three worked out — where the pattern enables enforcement without performing it |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_proven_by_code_that_must_not_compile.md](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md) | Part 3, stated as this crate's binary acceptance criterion |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_convenience_method_undoes_the_crate.md](../pitfall/001_a_convenience_method_undoes_the_crate.md) | The first consequence worked out — the three-line changes that undo the design |

### Sources

| File | Relationship |
|------|--------------|
| [`../invariant/001_capability_follows_the_handle.md`](../invariant/001_capability_follows_the_handle.md) | "Answered by ownership rather than by convention" — the pattern, stated as intent |
| [`../invariant/002_no_parking_operation_is_reachable.md`](../invariant/002_no_parking_operation_is_reachable.md) | "Enforced by what is exposed, not by a rule in a document" — the pattern stated a second time, independently |
| [`ring_poll/readme.md`](../../../ring_poll/readme.md) | A restriction of exactly the shape the problem section describes |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/ui/producer_drains.rs`](../../tests/ui/producer_drains.rs) | Part 3 in its minimal form — the pattern's own detector |
| [`tests/ui/producer_try_clones.rs`](../../tests/ui/producer_try_clones.rs) | Part 2's absence list, asserted rather than assumed — for one entry. A compile-fail case names a single thing that must not exist; "the list is complete" has no name, so the pattern's strongest claim is the one it cannot detect |

### HD20 — The Suite Covers Two of the Three Axes This Instance Calls Uncovered

The fourth consequence names three ways the pattern decays and says no case
catches any of them:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the three axes the consequence bullet calls uncovered --'
for axis in 'Clone' 'Sync' 'accessor'; do
  case $axis in
    Clone )    pat='clone' ;;
    Sync )     pat='shared_across_threads|send|sync' ;;
    accessor ) pat='deref|inner|as_ref|accessor' ;;
  esac
  printf '  %-9s %s\n' "$axis" \
    "$( ls ring_handle/tests/ui/*.rs | sed 's|.*/||;s|\.rs$||' \
         | command grep -iE "$pat" | tr '\n' ' ' )"
done
echo '  -- out of --'
printf '  total ui cases: %s\n' "$( ls ring_handle/tests/ui/*.rs | wc -l )"
```

Live output:

```
  -- the three axes the consequence bullet calls uncovered --
  Clone     consumer_clones producer_clones producer_try_clones 
  Sync      producer_shared_across_threads 
  accessor  
  -- out of --
  total ui cases: 7
```

Three cases target the `Clone` axis and one targets `Sync`. Four of the suite's
seven exist for exactly the two decay modes the bullet says nothing covers.

**The bullet is fair about its own subject and wrong in its closing clause.**
"Part 3's test asserts the specific withheld method is withheld" is true of
`producer_drains` read alone; "an axis no existing case covers" is a claim about
the whole suite, and for two of the three axes it is false. `consumer_clones`,
`producer_clones` and `producer_try_clones` were written precisely because a
derived `Clone` defeats the split, and `producer_shared_across_threads` exists
because a `Sync` impl acquired from a changed field does.

The third axis is the real one, and [`algorithm/002`](../algorithm/002_delegating_to_the_backend.md)'s
HD3 measures it from the other side: no case names `Deref`, `inner`, `as_ref` or
any accessor shape. So the honest version of this bullet is narrower and
sharper — one gap, not three — and understating the suite this way costs
something specific: a maintainer reading it has no reason to check whether the
gap they care about is already closed, and for two of three it is.
