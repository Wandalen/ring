# API: The Script Surface

### Scope

- **Purpose**: Define the fixture's surface — a script, the steps it is made of, the outcome it produces, and the two helpers that reach a loom thread.
- **Responsibility**: Signatures, what each guarantees, and the three signature choices that are not obvious.
- **In Scope**: `Script`, `Step`, `Outcome`, `Anomaly`, `audit_received`, `audit_received_unordered`, `leak`, `leak_ends`.
- **Out of Scope**: What `vanished` means (→ [`pitfall/001`](../pitfall/001_neither_the_count_nor_the_list_alone.md)); the accounting law (→ [`invariant/001`](../invariant/001_every_minted_record_is_somewhere.md)).

### Abstract

Eleven operations across four types, and the shape is a builder feeding an
interpreter: `Script::new().then( … ).then( … )` describes what should happen,
`run` makes it happen against a ring the caller supplies, and `Outcome` reports
what did.

Three of the signatures are not the obvious choice — the record type is fixed at
`u32`, `run` takes a ring rather than a config, and there are two leak helpers
where one would seem to do. Each is explained below, because each looks like an
oversight until the reason is stated.

### Operations

| Operation | Signature | Returns |
|---|---|---|
| `Script::new` | `fn( usize ) -> Script` | An empty script with that staging-buffer size |
| `Script::then` | `fn( self, Step ) -> Script` | The script with one more step |
| `Script::steps` | `fn( &self ) -> &[ Step ]` | The steps, in order |
| `Script::stage_limit` | `fn( &self ) -> usize` | The staging-buffer size |
| `Script::run` | `fn( &self, &mut Ring< u32 > ) -> Outcome` | What happened |
| `Outcome::vanished` | `fn( &self ) -> usize` | Accepted, less delivered, less still held |
| `Outcome::audit` | `fn( &self ) -> Result< (), Anomaly >` | The first property that did not hold |
| `audit_received` | `fn( &[ u32 ], u32 ) -> Result< (), Anomaly >` | The same, for a list with no `Outcome` behind it |
| `audit_received_unordered` | `fn( &[ u32 ], u32 ) -> Result< (), Anomaly >` | The same again, with the order requirement dropped |
| `leak` | `fn( Ring< T > ) -> &'static mut Ring< T >` | A ring that outlives its scope |
| `leak_ends` | `fn( Ring< T > ) -> ( Producer< 'static, T >, Consumer< 'static, T > )` | Both ends, `'static` |

#### The record type is `u32`, and the script mints its own

`Script::run` takes a `Ring< u32 >`, not a `Ring< T >`. That is deliberate.

A generic script would have to be told how to produce records — a closure, a
seed, an iterator — and every one of those is a way for two runs to differ. The
fixture's entire claim is that the same script produces the same outcome, so
the records are consecutive `u32`s from `0`, minted by the script itself. A
script needs no input data, and two runs mint the same values by construction
rather than by the caller being careful.

**What it costs:** a caller whose real records are `MyEvent` cannot drive them
through a `Script`. They can drive the same *shape* of sequence and read the
outcome, which is what a fixture is for. → [`decisions/readme.md`](../decisions/readme.md) Pending 1.

#### `run` takes a ring rather than a config

The alternative — `run( &self, config : &RingConfig )` — would build the ring
itself and need `ring_config` as a real dependency rather than a dev-dependency.
Taking the ring instead means:

| | |
|---|---|
| The same script runs against any ring the caller can build | Including one from `Ring::new_crossbeam`, which this crate has never heard of |
| Overflow policy, capacity and backend are the caller's variables | Which is what makes [`pitfall/001`](../pitfall/001_neither_the_count_nor_the_list_alone.md)'s two-ring comparison expressible at all |
| The crate's dependency set stays at the three declared edges | → [`integration/001`](../integration/001_the_three_edges_and_the_one_that_is_missing.md) |

**The ring is left in whatever state the last step put it in.** A caller
comparing two runs must supply two rings; running twice on one compares a fresh
run against a used ring, and the outcomes will differ for a reason that is not
a bug.

#### There are two leak helpers, and one is not enough

`leak` gives the *ring* a `'static` lifetime. That is not sufficient to reach a
`loom::thread::spawn` closure, because `Ring::ends` returns an `Ends` that
`split` then borrows — so a `Producer` from a leaked ring is still bounded by
whatever local the `Ends` was bound to.

`leak_ends` performs **both** leaks and hands back the two ends. The failure
mode of using only `leak` is a borrow error whose message points at the local
rather than at the missing second leak, which is why the pair exists rather than
one function with a caveat.

Both leak. Nothing frees either allocation. That is acceptable for a model
execution running a two-slot ring, and it is why the crate is named a testkit.

### Error Handling

`run` does not fail — every step is defined on every ring state, and a step that
cannot do its work records a refusal instead. `audit` and `audit_received`
return [`Anomaly`](../type/001_outcome_and_anomaly.md), which is a report about a
finished run rather than a failure of the call.

### Compatibility Guarantees

| # | Guarantee | Evidence |
|---|---|---|
| A1 | One script on two equivalent rings produces equal outcomes | `one_script_run_twice_produces_equal_outcomes` |
| A2 | Ten runs agree, not just two | `ten_runs_of_one_script_all_agree` |
| A3 | `Push`/`Recv`/`Stage` equal their `Many( 1 )` forms | `the_single_record_steps_match_a_many_of_one` |
| A4 | A receive step stops at the first empty read — the count is a ceiling | `a_receive_step_stops_at_the_first_empty_read` |
| A5 | A script reports the steps and limit it was built from | `a_script_reports_what_it_was_built_from` |
| A6 | An empty script produces an all-zero outcome | `an_empty_script_produces_an_empty_outcome` |
| A7 | `leak_ends`' ends can be moved onto spawned threads | `leak_ends_produces_ends_that_can_be_moved_onto_spawned_threads` |
| A8 | A script runs identically against a leaked ring | `a_script_runs_the_same_against_a_leaked_ring` |

### Preconditions

| # | Precondition | On whom | If violated |
|---|---|---|---|
| B1 | Two runs to compare need two rings | The caller | The second outcome reflects a used ring, and the comparison fails for the wrong reason |
| B2 | `leak`/`leak_ends` are for tests | The caller | An allocation per call that is never freed |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'public items declared:        %s\n' "$( command grep -cE '^(pub (fn|struct|enum))|^  pub fn ' src/lib.rs )"
printf 'public fields on Outcome:     %s\n' "$( awk '/^pub struct Outcome$/{f=1} f && /^  pub [a-z_]+ :/{n++} f && /^}$/{exit} END{print n}' src/lib.rs )"
printf 'public fields on Script:      %s\n' "$( awk '/^pub struct Script$/{f=1} f && /^  pub [a-z_]+ :/{n++} f && /^}$/{exit} END{print n+0}' src/lib.rs )"
printf 'private fields on Script:     %s\n' "$( awk '/^pub struct Script$/{f=1} f && /^  [a-z_]+ :/{n++} f && /^}$/{exit} END{print n+0}' src/lib.rs )"
printf 'accessor call sites, all ring crates: %s\n' "$( command grep -rn '\.steps()\|\.stage_limit()' --include='*.rs' ../../ring | wc -l )"
printf 'field reads inside run:       %s\n' "$( awk '/pub fn run/{f=1} f && /self\.(steps|stage_limit)([^(]|$)/{n++} END{print n+0}' src/lib.rs )"
printf 'lines between the two structs: %s\n' "$( awk '/^pub struct Outcome$/{a=NR} /^pub struct Script$/{print NR-a; exit}' src/lib.rs )"
```

Live output:

```
public items declared:        15
public fields on Outcome:     10
public fields on Script:      0
private fields on Script:     2
accessor call sites, all ring crates: 4
field reads inside run:       2
lines between the two structs: 424
```

### Types

| File | Relationship |
|------|--------------|
| [../type/001_outcome_and_anomaly.md](../type/001_outcome_and_anomaly.md) | `Outcome`'s ten fields and `Anomaly`'s four variants |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_from_a_step_to_an_outcome.md](../algorithm/001_from_a_step_to_an_outcome.md) | What `run` does with each step |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_minted_record_is_somewhere.md](../invariant/001_every_minted_record_is_somewhere.md) | The law `audit` checks first |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_neither_the_count_nor_the_list_alone.md](../pitfall/001_neither_the_count_nor_the_list_alone.md) | Why `Outcome` carries both counts and records |
| [../pitfall/002_reopening_closes_first.md](../pitfall/002_reopening_closes_first.md) | `Step::Reopen`'s hidden close |
| [../pitfall/003_the_amortised_flush_has_no_ring.md](../pitfall/003_the_amortised_flush_has_no_ring.md) | Why `Step::Flush` is one push at a time |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The surface |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | 33 tests; A1–A8 each named above |
| `tests/exhaustive_test.rs` | 3 loom models, `cfg( loom )` only |

### TK5 — two accessors whose only readers are the test that asserts them

`Script::steps()` and `Script::stage_limit()` are `#[ must_use ]` public
accessors over two private fields. Across every `.rs` file in `ring/`,
`any crate root` and `the spike root` they are called **four** times, all four inside
`a_script_reports_what_it_was_built_from` and
`an_empty_script_produces_an_empty_outcome`, which exist to assert exactly that
the accessors return what the builder was given.

`Script::run` — the one function in the crate that needs both values — does not
use them. It reads `self.stage_limit` to size the staging buffer and iterates
`&self.steps` directly, because it is a method on the same type and the fields
are in scope.

So the encapsulation the accessors provide is not consumed by anything, inside
the crate or out ([`002`](002_the_surface_no_crate_has_taken.md) TK7: there is
no "out"). They are not wrong — a consumer building a script from a helper
function has no other way to look at it — but the pair currently forms a closed
loop: two functions and two tests that keep each other alive, and nothing else
touching either.

### TK6 — two encapsulation policies, one file

`Outcome` exposes **ten** `pub` fields and no accessors. `Script`, declared 424
lines later in the same file, exposes **zero** and reaches its two fields through
`steps()` and `stage_limit()`.

Both choices are defensible on their own. `Outcome` is a record of what happened
— a caller wants `outcome.accepted` without ceremony, and `PartialEq` over the
whole struct is the comparison the crate's central claim rests on. `Script` is a
builder whose `then` is consuming, and private fields keep a half-built script
from being mutated behind `run`'s back.

What is not written down anywhere is that the file holds both rules, or which
applies to a type added later. A reader who learns the convention from `Outcome`
and adds a public field to `Script` breaks nothing the compiler will complain
about; a reader who learns it from `Script` and writes accessors for an eleventh
`Outcome` field produces a struct that is inconsistent with its own ten
neighbours. The crate has one file and two answers.

**The tenth field has since arrived.** `published` was added as a bare `pub`
field with no accessor — `Outcome`'s rule, not `Script`'s. That the author
happened to pick the consistent one is not evidence the ambiguity closed:
nothing on the page told them which of the two rules applied.
