# The Backend Discrimination Surface

### Scope

- **Purpose**: Take the three items that exist so a caller can tell one backend from another — `Backend`, `Ring::backend`, `Producer::try_clone` — and measure who uses them, because the module documentation's central argument depends on someone doing so.
- **Responsibility**: State what the three are for, what calls them, and what the gap costs.
- **In Scope**: The one public enum and two associated functions, measured across every `src/` and every `tests/` in the family.
- **Out of Scope**: The `free_capacity` asymmetry these exist to disambiguate (→ [`../pitfall/001`](../pitfall/001_free_capacity_carries_two_contracts.md), [`../decisions/002`](../decisions/002_free_capacity_keeps_one_signature_over_two_contracts.md)).

### The Argument These Three Items Serve

The module documentation states the problem plainly: `free_capacity` is one
signature over two contracts, binding at SPSC and advisory elsewhere, and *"the
caller cannot see which one it has"*. It then names the remedy —
`try_clone` returning `None` is *"the one machine-checkable way to tell the
backends apart"* — and `Backend`'s own rustdoc says it is *"how a caller learns
which column applies"*.

**Three items, one job: let a caller discover the contract it was handed.**

### Who Uses Them

```sh
cd "$(git rev-parse --show-toplevel)"
DEP="ring_bench ring_debug ring_factory ring_flush ring_handle ring_poll ring_shutdown ring_testkit"
for m in Backend 'backend()' try_clone; do
  A=0; C=0; T=0
  for c in $DEP; do
    A=$(( A + $( grep -c "$m" $c/src/*.rs ) ))
    C=$(( C + $( cat $c/src/*.rs | grep -vE '^\s*(//|///|//!)' | grep -c "$m" ) ))
    T=$(( T + $( cat $c/tests/*.rs 2>/dev/null | grep -c "$m" ) ))
  done
  printf '%-12s src(all) %2d   src(code) %2d   tests %2d
' "$m" "$A" "$C" "$T"
done
```

Live output:

```
Backend      src(all)  1   src(code)  0   tests  3
backend()    src(all)  0   src(code)  0   tests  3
try_clone    src(all)  3   src(code)  0   tests  4
```

**Zero production uses of all three, across thirty-two crates.** What exists
instead:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r 'Backend\|backend()\|try_clone' ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null \
  | grep -v '^ring_core/' | sed 's/:  */:/'
```

Live output:

```
ring_bench/src/lib.rs://! `ring_core::Producer::try_clone` is the operation that would, and
ring_handle/src/lib.rs://! | `try_clone` is **withheld** | `ring_core::Producer::try_clone` exists and can succeed on an MPSC backend. A `ring_handle::Producer` cannot be duplicated at all |
ring_handle/src/lib.rs:/// producer, which is the difference from `ring_core::Producer::try_clone`.
ring_testkit/src/lib.rs:/// # Backend-blind
ring_bench/tests/bench_test.rs:/// whose `Ends::split` yields one producer with no `try_clone` beside it.
ring_factory/tests/factory_test.rs:use ring_core::{Backend, Ring};
ring_factory/tests/factory_test.rs:assert_eq!(single.backend(), Backend::Spsc);
ring_factory/tests/factory_test.rs:assert_eq!(multi.backend(), Backend::Mpsc);
ring_handle/tests/handle_test.rs://! is no `Clone` and no `try_clone` — and a test cannot call a method that does
ring_handle/tests/handle_test.rs:let backend = ring.backend();
ring_handle/tests/ui_test.rs:/// | `producer_try_clones` | `ring_core`'s duplication is not forwarded | This crate — N2, and the `Deref` detector |
ring_handle/tests/ui_test.rs:cases.compile_fail("tests/ui/producer_try_clones.rs");
```

Read the list rather than the count. Every `src/` line is a doc comment, and all
three say the same thing — `ring_handle` withholds duplication, `ring_bench`
names the operation it does not perform. The test lines are `ring_factory`
asserting the selector picks the right backend and `ring_handle` reading a
backend it then does not branch on.

**So the discrimination surface is consumed by two test files and three doc
comments explaining that the crate in question does not use it.** No library in
the family asks which contract it has.

### What the Gap Costs

It does not cost correctness today, and saying so is the point of measuring it:
the four crates that call `free_capacity` all treat the reading as advisory,
which the module documentation says *"is always correct"*. The cost is that the
argument for keeping `free_capacity` uniform rests on an escape hatch nobody
takes, so the hatch's own correctness is asserted by ring_core's tests alone —
and a change to `try_clone`'s SPSC behaviour would break no caller and be caught
by no dependent's suite.

### CO30 — The Remedy for the Documented Hazard Has No User

This is the finding the instance exists for, stated as a single sentence: the
crate's hardest documented problem has a documented remedy, and the remedy has
zero production callers across thirty-two crates.

Nothing is broken. Every caller took the safe reading by default
(→ [`../decisions/002`](../decisions/002_free_capacity_keeps_one_signature_over_two_contracts.md), CO16),
so the escape hatch was never needed. What that costs is confidence: the hatch's
correctness rests on this crate's own tests, and a regression in it would be
caught by no dependent's suite.

### CO31 — `ring_factory` Tests the Enum Nobody Imports

`ring_factory/tests/factory_test.rs:28` — `use ring_core::{ Backend, Ring };`,
then lines 125 and 126 assert `single.backend() == Backend::Spsc` and
`multi.backend() == Backend::Mpsc`.

**This is the enum's entire purpose realized in a test rather than in a
consumer.** `ring_factory`'s own `src/` builds rings and never asks which one it
built. The assertion is worth having — it pins the selector's behaviour — but it
verifies a surface whose intended user does not exist.

### CO32 — A `backend()` Result Is Bound and Not Branched On

`let backend = ring.backend();` — the one `backend()` call outside
`ring_factory`, and the value is used for reporting rather than for control flow.

That is the whole observed usage pattern of the discrimination surface in this
family: assert it in one place, print it in another, branch on it nowhere. A
surface designed to make a caller change behaviour has never yet changed one.
