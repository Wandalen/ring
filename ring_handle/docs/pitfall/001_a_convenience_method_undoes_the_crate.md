# Pitfall: A Convenience Method Undoes the Crate

### Scope

- **Purpose**: Name the trap a crate built entirely from absences invites — that every individually reasonable addition destroys a guarantee, and none of them looks like a change to the design.
- **Responsibility**: The trap, the specific edits, what each one breaks, and what actually prevents them.
- **In Scope**: Additions to this crate's public surface, and the reasoning that produces them.
- **Out of Scope**: The invariants themselves (→ [`invariant/`](../invariant/readme.md)); the general practice (→ [Enforce by Withholding](../pattern/001_enforce_by_withholding.md)).

### Trap

**This crate's value is entirely in what it refuses to expose, and refusals do
not defend themselves.**

Every other crate in the family is defended by its own tests. Break
`ring_spsc`'s drain and 100 000 items stop arriving; break `ring_barrier`'s
minimum and a stalled-consumer test fails. Break `ring_handle` and **the test
suite goes green** — because adding a method cannot make a positive test fail.

The trap has a specific psychological shape, and naming it matters more than
listing the edits:

1. **Each addition is locally reasonable.** Nobody proposes "let me destroy the
   capability split." They propose "the `Producer` should be `Clone`, we need it
   in two places" — a sentence with a real problem behind it.

2. **The crate looks trivially simple.** Two types, one field each, a dozen
   forwarding methods. It reads as boilerplate, and boilerplate invites
   consolidation.

3. **The cost lands in another crate.** A `Clone` added here corrupts data in
   `ring_spsc`. The author of the edit is not reading `ring_spsc`'s invariant,
   and nothing in this crate's own files objects.

4. **The reviewer is reading a three-line diff.** `#[derive(Clone)]` is one
   line. `impl Deref` is three. Neither reads as an architectural change, and
   the diff shows what was added, never what stopped being guaranteed.

**The exposed population is this family's own authors, not its consumers.** A
consumer cannot add a method to `Producer`. Everyone who can, works on the
33-crate family and has a legitimate reason for the edit they are making.

### Failure

| # | The edit | The stated reason | What it actually breaks | Caught by |
|---|----------|-------------------|-------------------------|-----------|
| F1 | `#[derive(Clone)]` on `Producer` | "We need to publish from two places" | Two producers against a ring built for one — **silent data race**, and [`ring_spsc`'s whole correctness argument](../../../ring_spsc/docs/invariant/001_exactly_one_producer_one_consumer.md) is void | `tests/ui/producer_clones.rs`, and `consumer_clones.rs` for the mirror edit. **Not** the two cases the acceptance table specifies — these were added because F1 outranks F7 in likelihood |
| F2 | `impl Deref for Producer` | "Removes twelve forwarding methods" | Every backend capability reachable from both handles — the split becomes advisory | **Nothing currently** |
| F3 | `pub fn inner( &self ) -> &Backend` | "The bench harness needs direct access" | Same as F2, with a narrower entry point and a plausible justification | **Nothing currently** |
| F4 | A blocking `push` alongside `try_push` | "Non-tick threads need to wait" | **Deadlock inside a tick** the first time the ring is genuinely full — `ring_poll`'s exact scenario | **Nothing currently** — `ring_poll` has no `tests/ui` directory to hold such a case (→ HD39) |
| F5 | `try_push_timeout( d: Duration )` | "Bounded waiting isn't blocking" | Parking, renamed. Passes a naming audit, satisfies "returns a `Result`" | **Nothing currently** |
| F6 | A retry loop inside `try_push` | "Callers keep getting spurious failures" | Frame budget spent silently under load; no hang, no error, no signal | **Nothing currently** — `ring_poll`'s bounded-time test exists but names nothing in this crate, so it cannot see an edit made here (→ HD39) |
| F7 | A `drain` on `Producer` "for tests only" | "Tests need to inspect the ring" | The exact thing this crate's own specification forbids | `tests/ui/producer_drains.rs` — the acceptance table's own case |
| F8 | A stats counter in a forwarding method | "This is the natural chokepoint" | An RMW per operation, from a crate `ring_spsc` does not depend on — its no-RMW assertion fails for a reason nothing points at | `the_wrapper_costs_nothing`, a size assertion in this crate — no counting shim exists in `ring_spsc` (→ HD39) |

**The finding stands, and this crate acted on it.** As specified, the acceptance
criterion covered exactly the edit nobody makes — a `drain` on `Producer` is
obviously wrong to its author — while the edits that actually happen are
`Clone`, `Deref`, an accessor, and a timeout. F1 is now caught because the
compile-fail suite was widened past the two cases the table names; **F2, F3,
F4, F5 and F6 remain uncovered** — five of eight, not four; F4's cell named a
hedge that is unreachable since `ring_poll` has no `tests/ui` at all
(→ HD39) — and the reason is not oversight. A case can assert
that one named method is absent. It cannot assert that no method returns
`&Backend`, and `impl Deref` fails in a way that has no call site to point at
until someone uses it.

The original assessment, kept because its shape is the transferable part
(→ [Proven by Code That Must Not Compile](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md)'s
coverage gap).

**F5 is the most instructive because it survives every audit anyone would run.**
It is `try_`-named. It returns a `Result`. It has a documented timeout. A
reviewer checking "do all methods return `Result`?" passes it, and the tick
still misses its deadline
(→ [Nothing Reachable From a Handle Can Park](../invariant/002_no_parking_operation_is_reachable.md)'s W2).

**F8's failure is misattributed, which is its real cost.** The assertion that
fails is in `ring_spsc`. Its author looks at `ring_spsc`, `ring_claim`,
`ring_publish` — the crates `ring_spsc` depends on — and the RMW is in none of
them, because `ring_handle` is *above* it in the graph and does not appear in
its dependency list at all.

### Mitigation

**What does not work, listed first because it is what gets tried:**

- **A comment saying "do not add methods here."** The author of F1 has a real
  problem and a comment is not an answer to it.
- **Review discipline.** F2 and F4 are three-line diffs; the failure mode is
  that they look fine, and a reviewer who already knows to look for them did not
  need the mitigation.
- **Documenting the invariants.** Necessary and insufficient — F8's author never
  opens this crate's docs, because the edit is in a forwarding method and the
  breakage is two crates away.

**What actually works:**

| Mitigation | Covers | Cost |
|------------|--------|------|
| Widen the compile-fail suite: a case per withheld property, not per withheld method — `!Clone`, no `Deref`, no accessor, no `Sync` | F1, F2, F3, and F5's shape | **Done for `!Clone`** (two cases, both ends). Not done for the rest: `Deref` and an accessor have no absent *name* to write a case against |
| State the *consumer* of each guarantee in this crate's own docs, by name | F1's and F8's misattribution — the edit's author sees which crate they are about to break, without opening it | Prose, kept current |
| Assert each handle is the size of the `ring_core` handle it wraps | F8 and any other accumulated state, since a counter is a field | One line — `the_wrapper_costs_nothing`. Written as an equality rather than the predicted absolute 8 bytes, which was wrong: the handles measure 24 and 16 |
| Keep the forwarding methods hand-written and boring | F2 — a boilerplate-removal PR has to argue against an explicit choice rather than fill a vacuum | Ongoing tedium, deliberately |
| Name F5's shape explicitly: no method takes a `Duration`, a `Condvar`, or a waker | F5 | A stated rule, and one grep in CI |

**The first row is the only one that closes F1, and it is not currently
specified.** The acceptance table's row for this crate names two compile-fail
cases, both about missing methods. Widening it to withheld *properties* is a
change to a shared, family-grain document
([`bench_harness`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md)'s),
not a local decision — which is exactly why it is recorded here rather than
quietly done.

**Row two is cheap and underrated.** Most of the damage in this table is
misattribution: the edit is here, the failure is elsewhere, and the two are not
connected by anything the author reads. A table naming `ring_spsc` as the crate
that dies if `Producer` becomes `Clone` costs nothing and puts the consequence
in front of the person making the change.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_delegating_to_the_backend.md](../algorithm/002_delegating_to_the_backend.md) | F2, F6 and F8 as forbidden additions to the delegation path |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) | The absent-state table F8 would add a field to, and the `size_of` assertion that catches it |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_capability_follows_the_handle.md](../invariant/001_capability_follows_the_handle.md) | F1, F2, F3 and F7 as its V1–V4 |
| [../invariant/002_no_parking_operation_is_reachable.md](../invariant/002_no_parking_operation_is_reachable.md) | F4, F5 and F6 as its W1–W3 |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_proven_by_code_that_must_not_compile.md](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md) | Why F7 is caught and F1–F3 are not |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_enforce_by_withholding.md](../pattern/001_enforce_by_withholding.md) | The practice whose first consequence this pitfall works out in full |

### Sources

| File | Relationship |
|------|--------------|
| [`../pattern/001_enforce_by_withholding.md`](../pattern/001_enforce_by_withholding.md) | "One autocomplete away" — the trap's mechanism |
| [`../invariant/001_capability_follows_the_handle.md`](../invariant/001_capability_follows_the_handle.md) | The guarantees each edit removes |
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | The two compile-fail cases, and the properties they do not reach |

### Tests

| File | Relationship |
|------|--------------|
| `tests/ui/producer_clones.rs`, `tests/ui/consumer_clones.rs` | F1's detector — the mitigation table's first row, added beyond the acceptance criterion |
| `tests/handle_test.rs` | `the_wrapper_costs_nothing` — F8's detector, and the cheapest row in the table |

### HD39 — Three Caught-By Cells Name a Mechanism in Another Crate and None of the Three Exists

The Failure table's last column is the only place a reader learns which of the
eight edits would be stopped. Three of its eight cells point outside this
crate:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- F4 names a compile-fail case in ring_poll --'
ls ring_poll/tests/ui 2>&1 | sed 's|^|    |'
echo '  -- F6 names a bounded-time test in ring_poll; that test exists --'
command grep 'Duration::from_millis' ring_poll/tests/poll_test.rs | sed 's|^|    |'
echo '  -- and no file in ring_poll names this surface --'
printf '    ring_handle:: in ring_poll src+tests: %s\n' \
  "$( command grep -rF 'ring_handle::' ring_poll/src ring_poll/tests | wc -l )"
echo '  -- F8 names a counting shim in ring_spsc --'
printf '    files matching counting|shim in ring_spsc src+tests: %s\n' \
  "$( command grep -rEil 'counting|shim' ring_spsc/src ring_spsc/tests | wc -l )"
```

Live output:

```
  -- F4 names a compile-fail case in ring_poll --
    ls: cannot access 'ring_poll/tests/ui': No such file or directory
  -- F6 names a bounded-time test in ring_poll; that test exists --
            elapsed < Duration::from_millis(500),
  -- and no file in ring_poll names this surface --
    ring_handle:: in ring_poll src+tests: 0
  -- F8 names a counting shim in ring_spsc --
    files matching counting|shim in ring_spsc src+tests: 0
```

**F4's cell is hedged and the hedge is unreachable.** "`ring_poll`'s
compile-fail case, *if* it targets this surface" reads as a coverage claim with
one condition attached. `ring_poll` has no `tests/ui` directory at all, so
there is no case for the condition to be about — the cell is conditional on the
existence of a suite, not on its aim.

**F6's cell names a test that exists and cannot see this crate.** The
bounded-time assertion is real, at `poll_test.rs:167`. Nothing in `ring_poll`'s
source or tests names `ring_handle::`, so a retry loop added to a forwarding
method here cannot make that assertion fail. This is the one-way coupling
[`decisions/002`](../decisions/002_why_is_closed_is_absent.md)'s HD16 measures
from the other side: the two crates are named in each other's prose and joined
by nothing executable.

**F8's cell names a shim that was never written**, and F8 is nonetheless the
best-covered row in the table — by `the_wrapper_costs_nothing`, in this crate,
cited twice below in the Mitigation and Tests tables and not once in the column
that is supposed to answer the question.

The table's own summary already disagrees with it: "**F2, F3, F5 and F6 remain
uncovered**" contradicts F6's cell four lines above. The summary is right about
F6 and still wrong about F4. Counting from the code rather than either
statement: **F1 and F7 are covered by compile-fail cases, F8 by a size
assertion, and F2, F3, F4, F5 and F6 — five of eight — are covered by
nothing.**

**Disposition:** applied — the Failure table's F4, F6, and F8 "Caught by"
cells no longer point at mechanisms that don't exist or can't see this
crate; F4 and F6 now read "Nothing currently" with the reason, F8 now names
its real detector (`the_wrapper_costs_nothing`, in this crate), and the
summary paragraph above the table now counts five uncovered rows
(F2, F3, F4, F5, F6) instead of four, matching this section's own
Live output.
Now prints: `ring_handle:: in ring_poll src+tests: 0`

### HD40 — The One Mitigation Priced as Future Work Is Already Built, Wider Than Proposed, and Structurally Blind to F6

The Mitigation table's last row prices F5's defence as something to do: "Name
F5's shape explicitly: no method takes a `Duration`, a `Condvar`, or a waker ·
Cost: A stated rule, and one grep in CI." It exists, and it is not a grep:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the rule the mitigation table prices as a future cost --'
command grep -A2 'const FORBIDDEN' ring_handle/tests/handle_test.rs | sed 's|^|    |'
command grep 'include_str' ring_handle/tests/handle_test.rs | sed 's|^|    |'
echo '  -- an F6-shaped edit, checked against that same list --'
f6='while self.inner.try_push( record ).is_err() { core::hint::spin_loop(); }'
for n in 'thread::sleep' 'yield_now' '::park' 'park(' 'Condvar' 'Duration' 'Waker'; do
  case $f6 in ( *"$n"* ) printf '    HIT  %s\n' "$n" ;; ( * ) printf '    miss %s\n' "$n" ;; esac
done
```

Live output:

```
  -- the rule the mitigation table prices as a future cost --
        const FORBIDDEN: [&str; 7] =
            ["thread::sleep", "yield_now", "::park", "park(", "Condvar", "Duration", "Waker"];
    
        let source = include_str!("../src/lib.rs");
  -- an F6-shaped edit, checked against that same list --
    miss thread::sleep
    miss yield_now
    miss ::park
    miss park(
    miss Condvar
    miss Duration
    miss Waker
```

`no_parking_shaped_name_appears_in_the_source` reads this crate's own source
with `include_str!`, strips `//` comments, and fails on any of **seven** names —
the row's three plus `thread::sleep`, `yield_now`, `::park` and `park(`. It is
a test, not a CI grep, which is the stronger form: it runs wherever the suite
runs and its failure message points at
[`invariant/002`](../invariant/002_no_parking_operation_is_reachable.md).
Somebody built the row and did not come back to the Cost column, so the table
still advertises the work as unstarted. A reader planning the next increment
builds it twice.

**The row is right about what it covers, and that is the problem.** It claims
F5 and delivers F5. F6 — a retry loop inside `try_push` — is built from `while`,
`is_err` and at most a `spin_loop` hint, and the run above shows all seven names
miss it. Extending the array does not help: the minimal form is
`while self.inner.try_push( record ).is_err() {}`, which contains no
distinguishing token at all. **F5 parks by naming something; F6 parks by naming
nothing**, and a name list is the wrong instrument for the second.

With [HD39](#hd39--three-caught-by-cells-name-a-mechanism-in-another-crate-and-none-of-the-three-exists)
removing F6's only claimed detector, F6 is the one row in this file with no
coverage anywhere and no mitigation row that could acquire it. What would is a
bound the row does not propose — an assertion that a single `try_push` on a full
ring returns within a fixed number of backend operations, which is a cost
measurement rather than an audit
(→ [`non_functional_requirement/001`](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md)'s
HD33 on why the compile-fail instrument stops here).
