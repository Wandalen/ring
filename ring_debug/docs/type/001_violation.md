# Type: Violation

### Scope

- **Purpose**: Define the value a check reports, and record why it carries numbers rather than a message.
- **Responsibility**: The variants, the data each holds, and the two design decisions behind the shape.
- **In Scope**: `Violation`, `Cursor`.
- **Out of Scope**: What produces each variant (→ [`api/001`](../api/001_the_check_surface.md)); what each variant means about the ring (→ [`invariant/001`](../invariant/001_cursor_invariants_over_a_live_ring.md)).

### Definition

```rust
pub enum Violation
{
  ConsumerAheadOfProducer { producer : Seq, consumer : Seq },
  ProducerLappedConsumer  { producer : Seq, consumer : Seq, capacity : usize },
  CursorWentBackwards     { cursor : Cursor, was : Seq, now : Seq },
  ReadingsDisagree        { pending : usize, free : usize, capacity : usize },
}

pub enum Cursor { Producer, Consumer }
```

| Variant | Invariant | Produced by |
|---|---|---|
| `ConsumerAheadOfProducer` | D1 | `check`, `Watch::new`, `Watch::observe` |
| `ProducerLappedConsumer` | D2 | `check`, `Watch::new`, `Watch::observe` |
| `CursorWentBackwards` | D3 | `Watch::observe` only |
| `ReadingsDisagree` | — | `check_ends` only |

#### Every variant carries its numbers, not a message

**A caller matches on state; a report formats it.** The two are different jobs
and the crate does only the first, because the second depends on where the report
is going — a test assertion wants the values, a log line wants prose, a debugger
wants neither. Carrying a `String` would force the prose choice on every caller
and make the values unrecoverable afterwards.

The consequence is that `assert_eq!` against a whole `Violation` is the natural
test shape for asserting which violation was reported — five of the suite's
twenty-six tests do exactly that. A message-carrying variant would have forced
substring matching, which passes for the wrong reasons.

**`Display` is the other half of the bargain, and is tested as such.**
`a_violation_reports_the_numbers_it_was_derived_from` asserts that each variant's
rendering contains its own operands — the failure it exists to prevent is a
`Display` impl that reduces a precise finding to "cursor invariant violated".

#### Why `Cursor` is an enum and not a `bool` or a `&str`

`CursorWentBackwards` without a discriminant is nearly useless — "a cursor moved
backwards" leaves the reader where they started, and which one it was is the
whole diagnostic. A `bool` would encode it unreadably (`is_producer : true`), and
a `&'static str` would let a typo through the compiler. The enum costs one type
and makes the wrong value unrepresentable.

`Cursor` implements `Display` so a report reads `consumer cursor went backwards`
rather than `Consumer cursor went backwards`; `the_cursor_names_are_distinct`
pins both renderings, which is trivial and is the one thing the variant is
useless without.

#### Derives

| Derive | On | Why |
|---|---|---|
| `Debug` | Both | Test failure output; `assert_eq!` requires it |
| `Clone`, `Copy` | Both | Every field is `Copy`; a diagnostic that borrowed would be awkward in exactly the contexts diagnostics are used |
| `PartialEq`, `Eq` | Both | The assertion shape the crate is designed around |
| `Hash` | `Cursor` only | Not currently used — see below |
| `core::error::Error` | `Violation` | `check( pair )?` in a caller returning `Box< dyn Error >` |

**`Hash` on `Cursor` is currently used by nothing**, and is kept only because a
two-variant fieldless enum deriving `Hash` costs nothing and its absence is the
kind of omission that gets discovered at the moment someone wants a
`HashMap< Cursor, _ >`. It is recorded here rather than left silent, because an
unused derive is exactly the sort of thing a later reader is right to question.
The same question is open on `ring_types::Backend`'s `Hash`
(→ deferred blocker (w)); if that one is resolved by deletion, this one should be
reconsidered on the same grounds.

### Validation

The type validates nothing on construction and cannot — a `Violation` is a
*report of a broken invariant*, so refusing to build one for holding impossible
numbers would refuse exactly the case it exists to describe.

What is worth stating instead is which variants a given caller can actually
receive, because three of the four are unreachable from some entry points:

| Variant | Reachable from `check` | from `Watch::observe` | from `check_ends` |
|---|---|---|---|
| `ConsumerAheadOfProducer` | ✅ | ✅ | ❌ — D1 is invisible to derived readings |
| `ProducerLappedConsumer` | ✅ | ✅ | ❌ |
| `CursorWentBackwards` | ❌ — needs a baseline | ✅ | ❌ |
| `ReadingsDisagree` | ❌ | ❌ | ✅ — its only source |

A caller matching all four arms on a `check` result is writing two arms that
cannot fire. That is not an error, and the enum is deliberately not split into
three smaller ones per entry point: one vocabulary for one subject is worth more
than exhaustive matches that carry no dead arms
(→ [`api/001`](../api/001_the_check_surface.md)'s three-groups argument).

**The numbers in each variant are unvalidated by construction and load-bearing
by contract** — a `ConsumerAheadOfProducer` always carries the two sequences that
made it true, so a reader never has to re-observe a ring that has since moved on.

### Errors

`Violation` is itself the error type. There is no separate error for "the check
could not run": every input either satisfies the invariants or does not, and
there is no third outcome to represent.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug
echo '-- the suite, by assertion shape --'
printf '  tests:                                    %s\n' "$( command grep -c '^#\[ test \]' tests/debug_test.rs )"
printf '  naming a Violation variant at all:        %s\n' "$( awk '/^fn [a-z_]+\(\)/{f=$2} /Violation::/{print f}' tests/debug_test.rs | sort -u | wc -l )"
printf '  asserting one as a whole value:           %s\n' "$( awk '/^fn [a-z_]+\(\)/{f=$2} /Err\( *Violation::/{print f}' tests/debug_test.rs | sort -u | wc -l )"
printf '  the Display test asserts through:         %s\n' "$( command grep -oE 'rendered\.[a-z_]+' tests/debug_test.rs | sort -u )"
echo '-- what each needle set actually pins, by mutation --'
python3 - << 'EOF'
def render( name, o ) :
  if name == 'ConsumerAheadOfProducer' :
    return f"consumer at {o['consumer']} is ahead of producer at {o['producer']} - the ring reads as empty"
  if name == 'ProducerLappedConsumer' :
    d = o[ 'producer' ] - o[ 'consumer' ] if o[ 'producer' ] > o[ 'consumer' ] else 0
    return f"producer at {o['producer']} is {d} ahead of consumer at {o['consumer']}, past a capacity of {o['capacity']}"
  if name == 'CursorWentBackwards' :
    return f"{o['cursor']} cursor went backwards, from {o['was']} to {o['now']}"
  return f"pending {o['pending']} plus free {o['free']} is not the capacity {o['capacity']}"

cases = \
[
  ( 'ConsumerAheadOfProducer', { 'producer' : 3,  'consumer' : 9 },                        [ '3', '9' ] ),
  ( 'ProducerLappedConsumer',  { 'producer' : 30, 'consumer' : 0, 'capacity' : 8 },        [ '30', '8' ] ),
  ( 'CursorWentBackwards',     { 'cursor' : 'consumer', 'was' : 15, 'now' : 4 },           [ 'consumer', '15', '4' ] ),
  ( 'ReadingsDisagree',        { 'pending' : 2, 'free' : 3, 'capacity' : 16 },             [ '2', '3', '16' ] ),
]

def passes( name, o, needles ) :
  r = render( name, o )
  return all( n in r for n in needles )

for name, ops, needles in cases :
  print( f"  {name}  needles {needles}" )
  print( f"    renders: {render( name, ops )}" )
  loose = []
  for k in ops :
    m = dict( ops ); m[ k ] = 'producer' if k == 'cursor' else 77
    if passes( name, m, needles ) : loose.append( k )
  print( f"    a wrong value still passes for: {', '.join( loose ) if loose else 'nothing'}" )
  keys = [ k for k in ops if isinstance( ops[ k ], int ) ]
  swaps = []
  for i in range( len( keys ) ) :
    for j in range( i + 1, len( keys ) ) :
      a, b = keys[ i ], keys[ j ]
      m = dict( ops ); m[ a ], m[ b ] = ops[ b ], ops[ a ]
      if render( name, m ) != render( name, ops ) and passes( name, m, needles ) :
        swaps.append( f"{a}<->{b}" )
  print( f"    an exchange of operands still passes for: {', '.join( swaps ) if swaps else 'nothing'}" )
EOF
```

Live output:

```
-- the suite, by assertion shape --
  tests:                                    28
  naming a Violation variant at all:        12
  asserting one as a whole value:           5
  the Display test asserts through:         rendered.contains
-- what each needle set actually pins, by mutation --
  ConsumerAheadOfProducer  needles ['3', '9']
    renders: consumer at 9 is ahead of producer at 3 - the ring reads as empty
    a wrong value still passes for: nothing
    an exchange of operands still passes for: producer<->consumer
  ProducerLappedConsumer  needles ['30', '8']
    renders: producer at 30 is 30 ahead of consumer at 0, past a capacity of 8
    a wrong value still passes for: consumer
    an exchange of operands still passes for: producer<->consumer, producer<->capacity, consumer<->capacity
  CursorWentBackwards  needles ['consumer', '15', '4']
    renders: consumer cursor went backwards, from 15 to 4
    a wrong value still passes for: nothing
    an exchange of operands still passes for: was<->now
  ReadingsDisagree  needles ['2', '3', '16']
    renders: pending 2 plus free 3 is not the capacity 16
    a wrong value still passes for: nothing
    an exchange of operands still passes for: pending<->free, pending<->capacity, free<->capacity
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_check_surface.md](../api/001_the_check_surface.md) | Which entry point produces which variant |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_cursor_invariants_over_a_live_ring.md](../invariant/001_cursor_invariants_over_a_live_ring.md) | D1–D3, and V1–V4's consequences — what each variant means |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The definitions |
| [`ring_types/src/id.rs`](../../../ring_types/src/id.rs) | `Seq` — the carried sequence type |

### Tests

| File | Relationship |
|------|--------------|
| `tests/debug_test.rs` | `a_violation_reports_the_numbers_it_was_derived_from` — every variant renders its own operands; `a_violation_propagates_as_an_error` — the `Error` impl; `the_cursor_names_are_distinct` — `Cursor`'s two renderings |

### DB41 — the argument for the value-carrying design claims every test asserts on a whole value, and five of twenty-six do

The case for carrying numbers rather than a message closes with a consequence:
*"`assert_eq!` against a whole `Violation` is the natural test shape, and every
test in `tests/debug_test.rs` uses it."* The suite has 26 tests. Twelve name a
`Violation` variant at all, and five assert one as a whole value.

The other twenty-one assert something else — that a check returned `Ok`, that a
baseline was left alone, that a rendering contains a substring — and none of them
is wrong to. The overstatement matters because the sentence is doing work: it is
the evidence offered for the design decision, and a reader checking it finds a
claim about the whole suite supported by under a quarter of it.

**The precise version is stronger than the loose one.** Every test that asserts
*which violation was reported* asserts the whole value, which is the property the
design buys and the only population the claim needed to be about.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F 'test shape for asserting which violation was reported' ring_debug/docs/type/001_violation.md
```

Live output:

```
test shape for asserting which violation was reported — five of the suite's
```

**Disposition:** applied — the design-decision paragraph no longer claims every
test in `tests/debug_test.rs` uses `assert_eq!` against a whole `Violation`; it
now scopes the claim to tests asserting which violation was reported and states
the real count — five of the suite's twenty-six. Now prints:
`test shape for asserting which violation was reported`

### DB42 — the test that pins the rendering cannot tell an operand from its neighbour

`a_violation_reports_the_numbers_it_was_derived_from` checks that each variant's
rendering contains a list of expected substrings. Its docstring names the failure
it exists to prevent: a `Display` impl that *"throws the numbers away"*.

It does prevent that. It cannot prevent the adjacent failure, because
`str::contains` is blind to position: exchange two operands in a rendering and
every needle still matches. The mutation table above models the four arms of the
impl and finds **all four cases survive an operand exchange** —

- `ConsumerAheadOfProducer` reading *"consumer at 3 is ahead of producer at 9"*,
  which reverses the diagnosis;
- `CursorWentBackwards` reading *"from 4 to 15"*, which describes a cursor moving
  forwards — the one thing the variant exists to deny;
- `ReadingsDisagree` swapping `pending` and `free`, which are the two numbers a
  reader is trying to tell apart;
- `ProducerLappedConsumer` under any of its three exchanges, including the one
  that reports the capacity as the producer.

A fourth gap is narrower and in the same direction: the lapped case's needles are
`[ "30", "8" ]` for a value whose operands are `producer 30, consumer 0,
capacity 8`. The consumer is never pinned, and `"30"` is ambiguous between the
producer and the distance the arm derives from it, so a rendering that dropped the
producer and kept the subtraction would pass unchanged.

Recorded as coverage rather than a defect because the `Display` impl is currently
correct and the test's own stated goal is met. The gap is between that goal and
the property worth having: **for a diagnostic, an operand in the wrong place is
worse than an operand missing — the first misdirects an investigation and the
second only stalls it, and the guard is built to catch the second.**
