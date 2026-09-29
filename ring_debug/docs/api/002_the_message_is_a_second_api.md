# API: The Message Is a Second API

### Scope

- **Purpose**: Treat the two `Display` impls as a callable surface in their own right — what they promise, what pins them, and what a caller is entitled to read out of them.
- **Responsibility**: The rendered form of `Violation` and `Cursor`, the coverage it has, and the one claim it makes on another crate's behalf.
- **In Scope**: `impl Display for Violation`'s four arms; `impl Display for Cursor`'s two; `a_violation_reports_the_numbers_it_was_derived_from`.
- **Out of Scope**: The data the variants carry (→ [`type/001`](../type/001_violation.md)); the entry points that produce them (→ [`api/001`](001_the_check_surface.md)).

### Abstract

`Violation` carries numbers so a caller can match on state, and renders prose so a
caller can print one. [`api/001`](001_the_check_surface.md) documents the first
half; this documents the second, because the two have different callers, different
guarantees, and — as it turns out — very different coverage.

A rendered message is the surface a human actually meets. Nothing in the family
declares it stable, and one of the four messages states a fact about a *different
crate's* arithmetic.

### Operations

| Impl | Arms | Renders |
|---|---|---|
| `Display for Cursor` | 2 | `producer` / `consumer` — the words, lowercase, no punctuation |
| `Display for Violation` | 4 | One sentence per variant, naming every number the variant carries |

`Display for Cursor` exists so the backwards-cursor message can interpolate
`{cursor}` and read as English. It is the only place `Cursor` reaches a human,
and the only `Display` impl in the crate whose exact output is asserted by
equality (`the_cursor_names_are_distinct`).

#### The four sentences

| Variant | Shape of the message | Numbers named |
|---|---|---|
| `ConsumerAheadOfProducer` | `consumer at C is ahead of producer at P — the ring reads as empty and permits a claim` | Both, plus a consequence |
| `ProducerLappedConsumer` | `producer at P is D ahead of consumer at C, past a capacity of N` | Three carried, plus `D`, which is derived |
| `CursorWentBackwards` | `<cursor> cursor went backwards, from W to N` | Both, plus the end's name |
| `ReadingsDisagree` | `pending P plus free F is not the capacity N` | All three |

**Three of the four render only what they carry. The lapped arm computes.** `D` is
`producer.0.saturating_sub( consumer.0 )` — a fourth number that exists nowhere in
the value and is manufactured at format time.

### Compatibility Guarantees

| # | Guarantee | Evidence |
|---|---|---|
| A6 | Every message names the numbers its variant carries | `a_violation_reports_the_numbers_it_was_derived_from`, by substring containment |
| A7 | `Cursor` renders as exactly `producer` and `consumer` | `the_cursor_names_are_distinct`, by equality |
| A8 | Rendering allocates only through the formatter | No arm builds an intermediate `String`; every arm is one `write!` |

**A6 is weaker than it sounds and A7 is stronger.** A6 is checked with
`rendered.contains( needle )` over a list of digit strings, so the sentences
themselves — word order, punctuation, the em dash, the word "capacity" — are not
pinned by anything. A7 is an `assert_eq!` against the literal words, because two
words that must differ is the whole content of that impl.

### Preconditions

| # | Precondition | On whom | If violated |
|---|---|---|---|
| B4 | The rendered form is read by humans, not parsed | The caller | Nothing in this crate promises stability, and A6's test would not notice a rewrite |
| B5 | `ConsumerAheadOfProducer` is rendered for a ring using the family's arithmetic | The caller | The message's second clause is a claim about `ring_seqno`, and is simply wrong for a ring that computes its readings differently |

**B5 is unusual enough to state as a precondition rather than leave implicit.** The
message does not say "the cursors are ordered wrongly" — it says what the
consequence *is*, in a ring this crate does not own and does not depend on. That
is the most useful thing it could tell a reader, and it is the sentence most
likely to become false without anything failing.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
S=ring_debug/src/lib.rs
T=ring_debug/tests/debug_test.rs
echo '-- every format string, comments excluded --'
command grep -m1 -A44 -F 'impl fmt::Display for Violation' $S \
  | command grep -vE '^ *//' | command grep -oE '"[^"]+"' | sed 's/^/  /'
echo '-- two interpolation styles in one impl --'
printf '  inline-captured {name}: %s   positional {}: %s\n' \
  "$( command grep -m1 -A44 -F 'impl fmt::Display for Violation' $S | command grep -vE '^ *//' | command grep -oE '\{[a-z_]+\}' | wc -l )" \
  "$( command grep -m1 -A44 -F 'impl fmt::Display for Violation' $S | command grep -vE '^ *//' | command grep -oE '\{\}' | wc -l )"
echo '-- the one derived number, and the idiom it is derived with --'
command grep -E '\.(checked|saturating)_sub\(' $S | command grep -vE '^ *//' | sed 's/^ */  /'
printf '  arms that refuse to fabricate a distance: %s\n' \
  "$( command grep -c 'a lap report its own cursors contradict' $S )"
echo '-- what the suite asserts about the messages, and against what --'
command grep -E 'rendered\.contains|assert_eq!\( Cursor|assert_eq!\( corrupt' $T \
  | command grep -vE '^ *///' | sed 's/^ */  /'
printf '  tests asserting the empty-and-claimable consequence against ring_seqno: %s\n' \
  "$( command grep -c 'fn the_d1_message_still_describes_what_the_arithmetic_does' $T )"
echo '-- the lapped fixture, and whether its subtraction is observable --'
command grep -E 'ProducerLappedConsumer \{ producer : Seq\(' $T | sed 's/^ */  /'
python3 -c 'p,c=30,5; print(f"  producer {p}, consumer {c}: distance -> {p-c}, producer.0 -> {p}, distinguishable: {p-c != p}")'
```

Live output:

```
-- every format string, comments excluded --
  "consumer at {} is ahead of producer at {} — the ring reads as empty and permits a claim"
  "producer at {} is {ahead} ahead of consumer at {}, past a capacity of {capacity}"
  "producer at {} is behind consumer at {} — a lap report its own cursors contradict"
  "{cursor} cursor went backwards, from {} to {}"
-- two interpolation styles in one impl --
  inline-captured {name}: 3   positional {}: 8
-- the one derived number, and the idiom it is derived with --
  match producer.0.checked_sub( consumer.0 )
  let Some( pending ) = producer.0.checked_sub( consumer.0 )
  arms that refuse to fabricate a distance: 1
-- what the suite asserts about the messages, and against what --
  assert_eq!( corrupt.free_slots(), 8, "a corrupt ring did not report itself empty" );
  assert_eq!( corrupt.pending(), 0 );
  assert!( rendered.contains( needle ), "{violation:?} rendered as {rendered:?}" );
  assert!( rendered.contains( "contradict" ), "rendered as {rendered:?}" );
  !rendered.contains( "0 ahead" ),
  assert_eq!( corrupt.pending(), 0, "a D1 pair reads as empty" );
  assert_eq!( corrupt.free_slots(), 8, "and as entirely free" );
  assert!( rendered.contains( "reads as empty" ), "rendered as {rendered:?}" );
  assert!( rendered.contains( "permits a claim" ), "rendered as {rendered:?}" );
  assert_eq!( Cursor::Producer.to_string(), "producer" );
  assert_eq!( Cursor::Consumer.to_string(), "consumer" );
  tests asserting the empty-and-claimable consequence against ring_seqno: 1
-- the lapped fixture, and whether its subtraction is observable --
  Violation::ProducerLappedConsumer { producer : Seq( 30 ), consumer : Seq( 5 ), capacity : 8 },
  producer 30, consumer 5: distance -> 25, producer.0 -> 30, distinguishable: True
```

### APIs

| File | Relationship |
|------|--------------|
| [001_the_check_surface.md](001_the_check_surface.md) | The data surface these messages render |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_violation.md](../type/001_violation.md) | The four variants and their payloads |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_saturating_arithmetic_reports_health.md](../pitfall/001_saturating_arithmetic_reports_health.md) | The idiom the lapped arm formats with, and the reason this crate exists |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Both `Display` impls |

### Tests

| Test | Relationship |
|------|--------------|
| `a_violation_reports_the_numbers_it_was_derived_from` | A6 — containment over four fixtures |
| `the_cursor_names_are_distinct` | A7 — the only equality assertion on rendered output |
| `a_violation_propagates_as_an_error` | The `Error` impl that makes the message reachable through `?` |

### DB27 — one error message states a fact about a crate this one does not depend on

`ConsumerAheadOfProducer` renders as *"consumer at C is ahead of producer at P —
the ring reads as empty and permits a claim"*. The clause after the dash is not a
restatement of the violation; it is
[`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md)'s
measurement, compiled into a string literal.

It is also the most valuable sentence in the crate. A reader who sees "consumer
ahead of producer" learns that two numbers are ordered wrongly; a reader who sees
the second clause learns why that matters and what it costs them. The finding is
not that the sentence is there — it is that **the crate's most load-bearing claim
lives where nothing can check it.** `ring_seqno` is not a dependency
([`decisions/001`](../decisions/001_four_edges_not_two.md)'s four edges do not
include it), the claim is asserted by no test, and A6's containment check passes
on the digits alone.

If `ring_seqno`'s arithmetic stopped saturating tomorrow, `pitfall/001` would be
re-measured and this sentence would not. It would go on telling every reader a
consequence that had stopped being true, in the crate whose subject is readings
that quietly stop being true.

**The claim did not need moving, it needed a witness.** Deleting the clause would
cost the crate its most useful sentence, and importing `ring_seqno` to assert on it
would add a fifth edge to `decisions/001`'s four for the sake of one string. What
was missing was a test that builds the D1 pair the message describes, asks
`ring_seqno` what it says about it, and asserts the answer against the words.

**Disposition:** applied — `the_d1_message_still_describes_what_the_arithmetic_does`
constructs a lapped pair through `ring_core`, asserts that `pending()` is 0 and
`free_slots()` is the full capacity and `may_claim()` holds — which is
`pitfall/001`'s measurement, re-taken — and only then asserts the rendered message
contains *reads as empty* and *permits a claim*. The sentence and the arithmetic
now fail together instead of drifting apart, and no dependency was added: the
behaviour is reached through the `ring_core` edge the crate already has. Now
prints: `tests asserting the empty-and-claimable consequence against ring_seqno: 1`

### DB28 — the crate's only derived number is computed with the idiom the crate exists to catch, and its one fixture cannot see it

The lapped message manufactures a distance: `producer.0.saturating_sub(
consumer.0 )`. It is the only arithmetic in the crate that is not part of a check,
and it is written **saturating** — in the crate whose founding measurement is that
saturating arithmetic reports health it cannot support.

Here it is defensible: the D1 guard has already run, so `producer >= consumer`
holds and the subtraction cannot underflow. That is exactly
[`pattern/001`](../pattern/001_the_guard_that_makes_the_next_line_legal.md)'s
positional soundness (DB9), reappearing in a formatter where no guard is visible
at all — and `Violation` has public variant fields and no `#[ non_exhaustive ]`
(DB4), so a caller can hand-build `ProducerLappedConsumer { producer : Seq( 5 ),
consumer : Seq( 9 ), capacity : 8 }` and get *"producer at 5 is 0 ahead of consumer
at 9"*. Nothing rejects it; the saturation absorbs it into a sentence that reads as
a measurement.

The coverage was the sharper half. The one fixture that rendered this arm was
`producer : Seq( 30 ), consumer : Seq( 0 )` — and 30 − 0 is 30, so the derived
number was numerically identical to a field the message already printed. **The
whole subtraction could have been replaced by `producer.0` and every test in the
suite would still have passed.** The fixture was chosen to be obviously past a
capacity of 8; the zero came along for free, and it deleted the only opportunity
to observe the one computation the crate performs outside a check.

Both halves were fixed, and the second one is why the first could be. Moving the
fixture's consumer off zero makes the subtraction observable; only then is there
any point in changing what it computes.

**Disposition:** applied — the formatter matches on `checked_sub` and, when the
cursors contradict the variant, says so rather than saturating a fabricated
distance into a sentence that reads as a measurement; the lapped fixture is
`consumer : Seq( 5 )`, so 30 − 5 is 25 and the derived number is no longer a copy
of a field already printed; and `a_lap_report_that_contradicts_itself_does_not_fabricate_a_distance`
hand-builds the inverted variant DB4 leaves reachable and asserts the rendering
neither claims *0 ahead* nor stays silent about the contradiction. Now prints:
`arms that refuse to fabricate a distance: 1`
