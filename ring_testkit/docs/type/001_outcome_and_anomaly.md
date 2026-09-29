# Type: Outcome And Anomaly

### Scope

- **Purpose**: Define the two values the fixture produces — what a run recorded, and what a check found wrong with it.
- **Responsibility**: Fields, variants, derives, and the choices behind each.
- **In Scope**: `Outcome`, `Anomaly`, `Step`.
- **Out of Scope**: The properties `Anomaly`'s variants report on (→ [`invariant/001`](../invariant/001_every_minted_record_is_somewhere.md)).

### Definition

Two types, and they divide the work between *what happened* and *what was wrong
with it*. `Outcome` is ten plain fields with no methods that judge them;
`Anomaly` is four variants, produced only by an audit the caller asks for
explicitly.

Keeping them apart is what lets a deliberately-broken run still report its
counters — the fixture never refuses to describe a run just because the run
violated a law.

#### `Outcome` — ten fields, no timings

| Field | Type | Records |
|---|---|---|
| `minted` | `u32` | Records the script created |
| `accepted` | `usize` | Offers the ring answered `Ok` |
| `refused_full` | `usize` | Offers refused for want of a slot |
| `refused_closed` | `usize` | Offers refused because the ring was shut |
| `refused_staging` | `usize` | Records a full staging buffer turned away |
| `received` | `Vec< u32 >` | What came out, in the order it came out |
| `published` | `Vec< u32 >` | What went in, in the order it was offered — not the order it was minted |
| `in_ring_at_end` | `usize` | Still in the ring when the script ended |
| `staged_at_end` | `usize` | Still in the buffer when the script ended |
| `closed_at_end` | `bool` | The shutdown's final state |

**Not a single field is a duration, a rate, or a thread count.** That is the
type's defining constraint: `Outcome` derives `PartialEq`, and two runs of one
script compare equal. A field that varied with scheduling would make every
comparison a flake, and the crate's whole claim is that the comparison holds.
→ [`non_functional_requirement/001`](../non_functional_requirement/001_two_runs_compare_equal.md).

#### Why `vanished` is a method

`accepted - received.len() - in_ring_at_end` is derived from three fields that
are already stored. An eleventh field would be a second copy of the same number,
free to disagree with them if a step updated one and forgot the others.

It saturates rather than wrapping. Under the invariant the subtraction cannot
underflow — `received` and `in_ring_at_end` are disjoint subsets of `accepted` —
but a `usize` subtraction that *could* underflow in a debug build is a panic
waiting for the first outcome constructed by hand, and this type is public
enough to be constructed by hand (the audit tests do exactly that).

#### Why `refused_full` and `refused_closed` are two fields

They are separately recoverable, and the distinction is the one a producer acts
on. `ring_shutdown` states it on `Refusal` itself:

> [`Refusal::Full`] clears when the consumer drains, [`Refusal::Closed`] never
> does.

A single `refused` count would make a test that expected "the ring was shut"
pass on a run where the ring was merely full — which is exactly the confusion
`a_closed_ring_refuses_with_a_reason_of_its_own` and
`a_flush_into_a_closed_ring_is_refused_as_closed` are written to prevent.

`refused_staging` is a third because those records **never reached the ring**.
Folding it into either of the others would attribute a buffer's refusal to a
ring that was never asked.

#### `Anomaly` — four variants

| Variant | Fields | Reports |
|---|---|---|
| `Unaccounted` | `minted`, `placed` | A minted record is in none of the five buckets |
| `Unminted` | `value`, `minted` | A record came out that was never created |
| `Overdelivered` | `accepted`, `out` | More records left the ring than the ring ever accepted |
| `OutOfOrder` | `previous`, `then` | Two records did not ascend — which covers duplicates |

Every variant carries **both** the offending value and the bound it broke, so a
failure message is complete without the reader having the run in front of them.
`Unminted { value : 3, minted : 3 }` says what came out and how many existed;
`Unminted { value : 3 }` would need the reader to go and find the second number.

`OutOfOrder` covering duplicates is argued in
[`invariant/001`](../invariant/001_every_minted_record_is_somewhere.md) R3.

#### Derives

| Type | Derives | Why |
|---|---|---|
| `Outcome` | `Debug, Clone, PartialEq, Eq` | `PartialEq` is the point of the type; `Debug` puts the whole run in a failure message; no `Copy` — it owns a `Vec` |
| `Anomaly` | `Debug, Clone, Copy, PartialEq, Eq` | `Copy` because every variant is two small numbers; `PartialEq` so a test can assert the exact anomaly rather than merely that one occurred. It also carries `#[ non_exhaustive ]` — the crate's only one, added in the same change as `Overdelivered` and deliberately before it, so the fourth variant was additive rather than breaking, and a fifth will be too |
| `Step` | `Debug, Clone, Copy, PartialEq, Eq` | `Copy` so a script can be iterated by value; `PartialEq` so `Script::steps()` can be asserted against a literal |
| `Script` | `Debug, Clone, PartialEq, Eq` | `Clone` costs nothing on a `Vec< Step >` and a `usize`, and its absence would be a surprising omission on a builder-shaped public type — not needed to run one script against several rings, since `run` takes `&self` and that needs no clone at all |

`Anomaly` also implements `Display` and `core::error::Error`, so a caller can
`?` it out of a test function rather than unwrapping it. Nothing in this crate
needs that; a consumer writing its own fixtures on top does.

### Validation

**Neither type validates on construction, and `Outcome`'s fields are `pub`.** A
fixture that refused to build an `Outcome` whose counters did not add up would be
unable to report the one case worth reporting, so validation is a separate,
opt-in call:

| Check | Called by | Returns |
|---|---|---|
| The accounting law over five buckets, then delivery against what was offered | `Outcome::audit` | `Ok( () )`, or the first `Anomaly` that fired |
| Delivery order and duplicates, given a list alone | `audit_received( &[ u32 ], u32 )` | The same `Anomaly` vocabulary, with no `Outcome` behind it |

`audit` is three checks, run in that order: the five-bucket accounting law
(`Unaccounted`), then the delivery ceiling (`Overdelivered` — more records out
than the ring ever accepted), then `audit_delivery_order`. That third step is
**private**: it walks `received` against `published` rather than against raw
mint values, which is the reading a script that stages and pushes needs, and it
is reachable only through `audit`. The public `audit_received` is the
list-alone variant that re-bases R3 onto ascending mint order instead.

Two properties of that split are worth stating, because both are easy to assume
the other way:

- **`audit` reports the first failure, not all of them.** A run that broke two
  of the three properties names one. The ordering is fixed rather than
  incidental, so a regression does not silently change which one a caller sees.
- **`vanished` is a method, not a field, precisely so it cannot go stale.** It is
  derived from `accepted`, `received`, and `in_ring_at_end` on every call, and a
  caller who mutates a `pub` field gets a recomputed answer rather than a
  remembered one.

An `Outcome` that fails `audit` is still a valid, comparable `Outcome` — it
derives `PartialEq` either way, and two runs of a broken script still compare
equal (→ [`non_functional_requirement/001`](../non_functional_requirement/001_two_runs_compare_equal.md)).

### Evidence

| # | Claim | Test |
|---|---|---|
| Y1 | Two runs produce equal `Outcome`s | `one_script_run_twice_produces_equal_outcomes` |
| Y2 | Every `Anomaly` prints its numbers | `every_anomaly_says_what_broke` |
| Y3 | `Anomaly` is a `core::error::Error` | same test |
| Y4 | An all-zero outcome is expressible and equal to an empty run | `an_empty_script_produces_an_empty_outcome` |
| Y5 | `Script::steps` compares against a literal | `a_script_reports_what_it_was_built_from` |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'Outcome public fields:         %s\n' "$( awk '/^pub struct Outcome/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -cE '^  pub ' || true )"
printf 'Anomaly variants:              %s\n' "$( awk '/^pub enum Anomaly/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -cE '^  [A-Z][a-zA-Z]+$|^  [A-Z][a-zA-Z]+ \{' || true )"
printf 'what Script derives:           %s\n' "$( command grep -B1 '^pub struct Script' src/lib.rs | command grep -oE 'derive\( [^)]*\)' )"
printf 'clone() call sites in src:     %s\n' "$( command grep -c '\.clone()' src/lib.rs || true )"
printf 'clone() call sites in tests:   %s\n' "$( cat tests/*.rs | command grep -c '\.clone()' || true )"
printf 'script.run( call sites:        %s\n' "$( command grep -c 'script.run(' tests/testkit_test.rs || true )"
printf 'how run takes its script:      %s\n' "$( command grep -m1 -oE 'pub fn run\( &self' src/lib.rs )"
printf 'what audit checks, in order:   %s\n' "$( awk '/pub fn audit\( &self/{f=1} f && /^  }$/{exit} f' src/lib.rs | command grep -oE 'Anomaly::[A-Za-z]+|audit_[a-z_]+' | tr '\n' ' ' )"
printf 'audit() assertions in tests:   %s\n' "$( cat tests/*.rs | command grep -c 'outcome.audit()' || true )"
printf 'the Unaccounted input:         %s\n' "$( awk '/fn an_outcome_that_lost_a_record/,/^}$/' tests/testkit_test.rs | command grep -E 'minted|accepted|received' | tr -d ' ' | tr '\n' ' ' )"
printf 'the Unminted input:            %s\n' "$( awk '/fn an_outcome_that_adds_up/,/^}$/' tests/testkit_test.rs | command grep -E 'minted|accepted|received' | tr -d ' ' | tr '\n' ' ' )"
```

Live output:

```
Outcome public fields:         10
Anomaly variants:              4
what Script derives:           derive( Debug, Clone, PartialEq, Eq )
clone() call sites in src:     0
clone() call sites in tests:   0
script.run( call sites:        23
how run takes its script:      pub fn run( &self
what audit checks, in order:   Anomaly::Unaccounted Anomaly::Overdelivered audit_delivery_order 
audit() assertions in tests:   11
the Unaccounted input:         minted:5, accepted:2, received:vec![0,1], assert_eq!(outcome.audit(),Err(Anomaly::Unaccounted{minted:5,placed:3})); 
the Unminted input:            minted:2, accepted:2, received:vec![0,4], assert_eq!(outcome.audit(),Err(Anomaly::Unminted{value:4,minted:2})); 
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_script_surface.md](../api/001_the_script_surface.md) | Where these values are produced and consumed |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_minted_record_is_somewhere.md](../invariant/001_every_minted_record_is_somewhere.md) | The properties each `Anomaly` variant reports on |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_two_runs_compare_equal.md](../non_functional_requirement/001_two_runs_compare_equal.md) | Why no field may be a timing |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_shutdown/src/lib.rs`](../../../ring_shutdown/src/lib.rs) | `Refusal`'s two arms, and the distinction the two refusal counts inherit |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | Y1–Y5 |

### TK47 — the reason given for `Script`'s `Clone` is not the reason it would be needed

The derives table justifies `Script`'s `Clone` as *"so one script can be run
against several rings without rebuilding it"*. That is not what makes running a
script against several rings possible.

`Script::run` takes `&self`. Running the same script against a second ring
requires no clone, no rebuild and no ownership transfer — it requires calling the
method again. The crate's own suite does this at twenty-three `script.run(` call
sites, including the test the whole determinism requirement rests on, which runs
one script against two rings back to back.

The measured count of `.clone()` in this crate is **zero** in `src/` and **zero**
across both test files. Nothing has ever cloned a `Script`, and under the current
signature nothing needs to.

The derive is still defensible — a builder-shaped public type that derives
`Debug`, `PartialEq` and `Eq` and not `Clone` is a surprising omission, and
`Clone` on a `Vec< Step >` and a `usize` costs nothing to provide. What is wrong
is the stated reason, which describes a limitation `&self` already removed and is
contradicted by [`../pattern/001`](../pattern/001_the_script_is_data_and_the_run_is_an_interpreter.md)'s
P3 four documents away, where the same `&self` is correctly named as the property
the central requirement rests on.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
command grep -m1 -F 'not needed to run one script against several rings, since' docs/type/001_outcome_and_anomaly.md
```

Live output:

```
| `Script` | `Debug, Clone, PartialEq, Eq` | `Clone` costs nothing on a `Vec< Step >` and a `usize`, and its absence would be a surprising omission on a builder-shaped public type — not needed to run one script against several rings, since `run` takes `&self` and that needs no clone at all |
```

**Disposition:** applied — the derives table now gives the actual reason
`Script` derives `Clone` (cheap on a `Vec< Step >` and a `usize`, avoids a
surprising omission on a builder-shaped type) instead of the `&self`-contradicted
claim that it enables running against several rings, matching the correct
reason already given four documents away in `pattern/001` P3.
Now prints: `not needed to run one script against several rings, since`

### TK48 — the order `audit` reports failures in is asserted and never observed

The Validation section states that `audit` *"reports the first failure, not all
of them"* and that *"the ordering is fixed rather than incidental, so a
regression does not silently change which one a caller sees"*. The order in the
source is `Anomaly::Unaccounted`, then the `Overdelivered` ceiling, then
`audit_delivery_order` — three checks, run in that sequence.

Observing an order requires an input that could produce more than one answer.
The suite has four `Outcome`s that fail an audit, and each violates exactly one
property:

- `minted : 5, accepted : 2, refused_full : 1, received : vec![ 0, 1 ]` — the
  accounting is short, and `[ 0, 1 ]` is within bounds, in published order, and
  no larger than `accepted`, so only `Unaccounted` can fire.
- `minted : 2, accepted : 2, received : vec![ 0, 4 ]` — the buckets sum to
  exactly `minted` and two records out of two accepted clears the ceiling, so
  `Unminted` is the only reachable answer.
- `minted : 2, accepted : 2, received : vec![ 0, 1 ], in_ring_at_end : 1` — the
  buckets still sum exactly, and `[ 0, 1 ]` is a clean delivery, so only the
  ceiling can fire.
- the same script run twice against one ring — six records out of three
  accepted, nothing delivered, so again only the ceiling can fire.

No input in either test file breaks two at once. So the claim that the ordering
is deliberate is true of the code and unverified by the suite, and the gap is
wider than a swap: with every failing input violating exactly one property, all
six orderings of the three checks leave every test passing.

`OutOfOrder` is not reachable through `audit` in this suite at all — every
assertion of it goes through `audit_received` on a bare slice, never through an
`Outcome`.

The fix is two tests, one per adjacent pair, which pins the whole order by
transitivity: an `Outcome` whose buckets are short *and* whose list contains an
unminted record, asserting `Unaccounted`; and one that breaches the ceiling
*and* carries an unminted record, asserting `Overdelivered`. The reason to want
them is exactly the reason the document gives for the ordering being fixed. A
property worth stating as deliberate is worth an assertion, particularly when
the document already tells a consumer they can rely on it.
