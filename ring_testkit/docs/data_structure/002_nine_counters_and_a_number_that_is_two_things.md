# Data Structure: Nine Counters And A Number That Is Two Things

### Scope

- **Purpose**: Define the shape a run is reported in — seven scalars, two lists, one flag and one derived reading — and the two places that shape leaks an assumption.
- **Responsibility**: The field layout, why the derived value is a method, the one field whose width differs from its neighbours, and the arithmetic that cannot report a negative.
- **In Scope**: `Outcome`'s ten fields and `Outcome::vanished`.
- **Out of Scope**: The variants of `Anomaly` (→ [`type/001`](../type/001_outcome_and_anomaly.md)); the checks over these values (→ [`algorithm/002`](../algorithm/002_the_four_passes_of_an_audit.md)).

### Layout

```text
Outcome
├─ minted           : u32        how many records the script created
├─ accepted         : usize      offers the ring answered Ok
├─ refused_full     : usize      refused, ring full
├─ refused_closed   : usize      refused, ring closed
├─ refused_staging  : usize      refused by a full staging buffer
├─ received         : Vec< u32 > records that came back out, in order
├─ published        : Vec< u32 > records the ring took, in the order offered
├─ in_ring_at_end   : usize      still in the ring when the script ended
├─ staged_at_end    : usize      still in the staging buffer
└─ closed_at_end    : bool       the ring's final state

derive( Debug, Clone, PartialEq, Eq )
vanished( &self ) -> usize       derived, not stored
```

Seven scalars, two lists, one flag. Nothing here is a timing, a pointer or a
handle — which is the property `PartialEq` rests on, and the reason two runs of
one script can be compared with `assert_eq!` rather than field by field.

### The tenth value is a method

`vanished` is `accepted - received.len() - in_ring_at_end`, and it is the reading
the crate exists for: records the ring said it took, did not deliver, and does
not hold. It is derived rather than stored, which is
[`decisions/readme.md`](../decisions/readme.md) Closed 3 — a tenth field would be
a second copy of a number three other fields already determine, free to disagree
with them if a step updated one and forgot the rest.

The cost of deriving it is that `PartialEq` does not see it. Two `Outcome`s that
compare equal necessarily agree on `vanished`, because it is a function of three
fields that are compared; but an `Outcome` constructed by hand can hold any
combination of the three, and the method will compute something from them
regardless.

### The one field that is not a `usize`

`minted` is a `u32`; the six other counts are `usize`. That is not an
oversight — records *are* `u32`s, minted consecutively from `0`, so the counter
and the next record's value are the same number:

```text
let record = minted;
minted += 1;
```

The consequence is that `minted` carries two facts at once. As a count it says
how many records the run created; as a bound it says no legitimate record may
have a value `>= minted`, which is precisely what `audit_received`'s provenance
pass tests. Both readings are correct *only* because minting is consecutive from
zero — a change to the minting scheme would silently break the second while the
field's own doc comment, which records only the first, kept reading true.

The width difference is paid at the one place both readings meet:
`Outcome::audit` casts `self.minted as usize` twice in five lines to compare it
against the sum of its neighbours.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'fields on Outcome:            %s\n' "$( awk '/^pub struct Outcome$/{f=1} f && /^  pub [a-z_]+ :/{n++} f && /^}$/{exit} END{print n}' src/lib.rs )"
printf 'field types, in order:        %s\n' "$( awk '/^pub struct Outcome$/{f=1} f && /^  pub [a-z_]+ :/{ sub( /^.*: /, "" ); sub( /,$/, "" ); printf "%s ", $0 } f && /^}$/{exit}' src/lib.rs )"
printf 'fields not usize:             %s\n' "$( awk '/^pub struct Outcome$/{f=1} f && /^  pub [a-z_]+ :/ && !/usize/{ sub( /^  pub /, "" ); sub( /,$/, "" ); printf "%s; ", $0 } f && /^}$/{exit}' src/lib.rs )"
printf 'casts of minted in audit:     %s\n' "$( awk '/pub fn audit\( &self/{f=1} f && /^  }$/{exit} f' src/lib.rs | command grep -c 'minted as usize' || true )"
printf 'the vanished expression:      %s\n' "$( awk '/pub fn vanished/{f=1} f && /self\.accepted/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'Outcomes built by hand in tests: %s\n' "$( command grep -c 'Outcome$' tests/testkit_test.rs || true )"
printf 'assertions naming the other zero: %s\n' "$( command grep -c 'Anomaly::Overdelivered' tests/testkit_test.rs || true )"
```

Live output:

```
fields on Outcome:            10
field types, in order:        u32 usize usize usize usize Vec< u32 > Vec< u32 > usize usize bool 
fields not usize:             minted : u32; received : Vec< u32 >; published : Vec< u32 >; closed_at_end : bool; 
casts of minted in audit:     2
the vanished expression:      self.accepted.saturating_sub( self.received.len() + self.in_ring_at_end )
Outcomes built by hand in tests: 4
assertions naming the other zero: 4
```

### Types

| File | Relationship |
|------|--------------|
| [../type/001_outcome_and_anomaly.md](../type/001_outcome_and_anomaly.md) | Each field's meaning, one by one |

### Data Structures

| File | Relationship |
|------|--------------|
| [001_the_script_as_a_flat_step_list.md](001_the_script_as_a_flat_step_list.md) | The shape that produces these values |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_the_four_passes_of_an_audit.md](../algorithm/002_the_four_passes_of_an_audit.md) | The four passes that read these fields |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_neither_the_count_nor_the_list_alone.md](../pitfall/001_neither_the_count_nor_the_list_alone.md) | Why the counts and the list are both stored |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `Outcome`, `Outcome::vanished` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | `an_outcome_that_lost_a_record_fails_the_audit`, `a_ring_with_room_vanishes_nothing_under_either_policy` |

### TK11 — the reading that cannot report a negative reports zero instead

`vanished` is written `self.accepted.saturating_sub( self.received.len() + self.in_ring_at_end )`.
A plain subtraction would panic on underflow in a debug build and wrap in a
release one; saturation avoids both. What it returns instead is **`0`**, and `0`
is the value that means *nothing was destroyed* — the healthy answer, produced by
the one arithmetic path that only runs when the numbers are impossible.

An `Outcome` where delivered-plus-held exceeds accepted is not reachable from a
single `Script::run`. It is reachable two other ways, both established in this
crate:

| Route | Established by |
|---|---|
| Running one script twice on the same ring — records from the first run are delivered by the second | `Script::run`'s own doc warns against it; the crate doc example does it |
| Constructing an `Outcome` by hand — all ten fields are `pub` | four tests in the suite build one this way, including `an_outcome_that_lost_a_record_fails_the_audit` |

In either case `vanished()` answers `0` and `audit()` is the check that might
catch it, which inverts the crate's own reading order: `vanished` is documented
as *the* reading that says records were destroyed, and it is the reading that
goes quiet first when the numbers stop making sense.

A `checked_sub` returning `Option< usize >` would make the impossible case
namable at the cost of an `unwrap` at every call site. That was a trade worth
stating; it was stated nowhere.

**Disposition:** applied — to the other method, and the `checked_sub` is
**declined**. `vanished` keeps its `saturating_sub` and its `usize` return,
because an `Option` there costs an `unwrap` at every call site to report a state
that call site cannot act on; what it gained is a `# `0` Is Two Different Answers`
doc section naming both readings and pointing at the method that separates them.
The separation is `audit`, which now runs an over-delivery pass before anything
else and returns `Anomaly::Overdelivered { accepted, out }` — so the impossible
case is namable, and it is named where a caller is already looking for anomalies
rather than where it is reading a count. Both routes in the table above are now
covered by a test:
`an_outcome_that_delivered_more_than_it_accepted_is_caught` builds the `Outcome`
by hand, and `a_script_run_twice_on_one_ring_produces_an_outcome_that_fails_its_audit`
reaches the same state through the mistake `Script::run`'s own doc warns about,
with no cooperation from the caller beyond making it. What this does not buy:
`vanished()` still answers `0` in both cases, so any consumer reading that number
without calling `audit()` first sees exactly what it saw before — the ambiguity is
reported, not removed. Now prints:
`assertions naming the other zero: 4`

### TK12 — one field, two facts, one doc comment

`minted` is documented as *"How many records the script created."* It is also the
exclusive upper bound on legitimate record *values* — the entire content of
`audit_received`'s provenance pass is `value >= minted`, and `Outcome::audit`
hands the field straight to it.

The two readings coincide only because minting is `let record = minted; minted += 1;`
from zero. Nothing enforces that: it is a property of two lines inside `run`,
stated in `Step`'s enum-level doc comment and not in the field's, and not checked
anywhere. A minting scheme that skipped values, started at one, or restarted per
step would leave `minted` correct as a count and wrong as a bound, and the only
symptom would be `audit_received` returning `Unminted` for records that were
minted.

It is also the crate's only `u32` count among seven `usize` neighbours, which is
the visible trace of the dual role — the field is typed after the records it
bounds rather than after the things it counts.
