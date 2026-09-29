# API: Shutdown Surface

### Scope

- **Purpose**: Fix the whole surface in one table, and mark per item whether its promise holds by construction or by convention — the distinction the rest of this crate's documentation turns on.
- **Responsibility**: Method sets, signatures, error shape, and the guarantee column.
- **In Scope**: `Shutdown`, `Stopped`, `Guarded`, `Refusal`, `Wake`, `reset`, `wait_for_close`, `for_space_or_close`.
- **Out of Scope**: The drain procedure (→ [`algorithm/001`](../algorithm/001_drain_to_empty.md)); why the token types exist (→ [`type/`](../type/readme.md)).

### Abstract

Eleven entry points across five types, and one column that matters more than
the signatures: whether each promise holds **by construction** — the wrong call
does not compile — or **by convention**, where a caller who skips a step gets no
warning. Six rows are construction, one is convention, four are ungraded (`—`),
and the split is the crate's honest account of where its guarantees stop.

### Operations

| Item | Signature | Guarantee holds by |
|---|---|---|
| The flag | `Shutdown::new() -> Self` | — |
| Read it | `is_closed( &self ) -> bool` | — |
| Stop | `close( &self ) -> Stopped< '_ >` | **construction** — the only way to obtain a `Stopped` |
| Ask | `admit( &self ) -> Result< (), RingError >` | convention — the caller must call it |
| Wrap a producer | `guard( &self, Producer< 'a, T > ) -> Guarded< 'a, T >` | **construction** — `Guarded`'s only push checks |
| Recover records | `Stopped::drain_all( &self, &mut Consumer, &mut Vec< T > ) -> usize` | **construction** — unreachable without a close |
| Empty without a sink | `Stopped::discard_all( &self, &mut Consumer ) -> usize` | **construction** — same |
| Reopen | `Stopped::reopen( self )` | **construction** — consumes the token |
| Teardown in one call | `reset( &Shutdown, &mut Consumer ) -> usize` | **construction** — composes the three |
| Join a waiter | `wait_for_close( &Shutdown, WaitKind, usize ) -> Result< usize, RingError >` | — |
| Wait with two exits | `for_space_or_close( &CursorPair, &Shutdown, WaitKind, usize ) -> Result< Wake, RingError >` | — |

### Error Handling

Three failure shapes appear on this surface, and they are deliberately three
rather than one — a refused push, a refused admission, and a wait that gave up
are different events and a caller responds to each differently:

| Shape | Where | Means | Carries the record? |
|---|---|---|---|
| `Refusal< T >` | `Guarded::try_push` | The ring was full, or it was closed | ✅ both arms |
| `RingError` | `admit` | The ring is closed and the caller asked politely first | n/a — nothing was handed over |
| `RingError` | `wait_for_close`, `for_space_or_close` | The bounded spin ran out of attempts | n/a |

**No entry point panics, and none returns a bare `bool` for a failure.** A
`bool` would collapse "full" and "closed" into one answer, and the two demand
opposite responses: retry the first, stop retrying the second. That distinction
is [`type/002`](../type/002_refusal_carries_the_record.md)'s whole subject.

`RingError` is `ring_types`' — this crate defines no error type of its own, so a
caller composing several family crates handles one vocabulary rather than four.

### Compatibility Guarantees

1. **A refusal never destroys the record.** `Guarded::try_push` returns
   [`Refusal< T >`](../type/002_refusal_carries_the_record.md), and both arms
   carry the record. This extends `ring_core`'s own refusal contract by one
   case rather than replacing it.

2. **`close` is idempotent and always yields the token.** Teardown is reached
   from more than one path — the normal end of a run, and an unwind through a
   guard — and neither path should have to know whether it is first. A `close`
   that returned `Option< Stopped >` would force every caller to handle a
   second close as an error case that is not one.

3. **`reset` ends open, however it started.** Its contract is the state
   afterwards, not the transition: a caller resetting an already-closed ring
   gets an open one, which is what "fit for the next run" means.

4. **Nothing on this surface allocates or parks.** `drain_all` appends into a
   caller-supplied `Vec`; the two waits are `ring_wait`'s bounded spin, which
   returns rather than blocking. This is what makes the surface reachable from
   inside a tick, where nothing may allocate or block.

#### The guarantee column is the point

Six rows read **construction** (Stop, Wrap a producer, Recover records, Empty
without a sink, Reopen, Teardown in one call), and each is a place where the
wrong call does not compile rather than misbehaving at runtime:

- A drain that did not follow a close cannot be written — `drain_all` is a
  method on `Stopped`, and `close` is the only constructor.
- A drain that follows a *reopen* cannot be written either — `reopen` takes
  `self` by value, so the proof is gone.
- A `Guarded` cannot publish into a closed ring — its only push checks first.

One row reads **convention**, and it is the crate's central limitation:
`admit` must be called to have any effect, and a caller holding a
raw `ring_core::Producer` never calls it (→
[`pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)).
The remaining four rows are ungraded — `new`, `is_closed`, `wait_for_close`
and `for_space_or_close` state no compile-time-versus-convention promise at
all.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/api
printf 'rows in the surface table:     %s\n' "$( awk -F'\\|' '/^\| /&&( $4 ~ /construction|convention/ || $4 ~ /^ — / || $4 == " — " ){ n++ } END{ print n+0 }' 001_shutdown_surface.md )"
printf 'of those, construction:        %s\n' "$( awk -F'\\|' '/^\| /&&$4 ~ /construction/{ n++ } END{ print n+0 }' 001_shutdown_surface.md )"
printf 'of those, convention:          %s\n' "$( awk -F'\\|' '/^\| /&&$4 ~ /convention/{ n++ } END{ print n+0 }' 001_shutdown_surface.md )"
printf 'of those, ungraded:            %s\n' "$( awk -F'\\|' '/^\| /&&$4 == " — " { n++ } END{ print n+0 }' 001_shutdown_surface.md )"
printf 'the abstract summary line:     %s\n' "$( awk '/^### Regenerate/{exit} {print}' 001_shutdown_surface.md | command grep -ohE 'Four rows are construction, two are convention' | tr '\n' ' ' )"
printf 'the later section repeats:     %s\n' "$( awk '/^### Regenerate/{exit} {print}' 001_shutdown_surface.md | command grep -ohE '(Four|Two) rows read .{0,3}(construction|convention)' | tr '\n' ' ' )"
printf 'the cross-ref row here cites:  %s\n' "$( awk '/^### Types/{f=1} f&&/^### Invariants/{exit} f' 001_shutdown_surface.md | command grep -ohE 'behind [a-z]+ of the [a-z]+ .{0,4}construction' )"
printf 'the module index cites:        %s\n' "$( command grep -ohE 'behind [a-z]+ of the [a-z]+ .{0,4}construction' ../readme.md )"
printf 'items named in the table:      %s\n' "$( awk -F'\\|' '/^\| /{ print $3 }' 001_shutdown_surface.md | command grep -ohE '[a-z_]+\(' | tr -d '(' | sort -u | wc -l )"
printf 'public fn names in src:        %s\n' "$( command grep -ohE '^ *pub (const )?fn [a-z_]+' ../../src/lib.rs | awk '{ print $NF }' | sort -u | wc -l )"
printf 'in src, absent from the table: %s\n' "$( comm -13 <( awk -F'\\|' '/^\| /{ print $3 }' 001_shutdown_surface.md | command grep -ohE '[a-z_]+\(' | tr -d '(' | sort -u ) <( command grep -ohE '^ *pub (const )?fn [a-z_]+' ../../src/lib.rs | awk '{ print $NF }' | sort -u ) | tr '\n' ' ' )"
```

Live output:

```
rows in the surface table:     11
of those, construction:        6
of those, convention:          1
of those, ungraded:            4
the abstract summary line:     
the later section repeats:     
the cross-ref row here cites:  behind three of the six **construction
the module index cites:        behind three of the six **construction
items named in the table:      11
public fn names in src:        21
in src, absent from the table: drain_all_bounded free_capacity into_inner into_record is_blocked is_ready reason shutdown try_push try_push_batch 
```

### Types

| File | Relationship |
|------|--------------|
| [../type/001_stopped_proof_token.md](../type/001_stopped_proof_token.md) | The mechanism behind three of the six **construction** rows |
| [../type/002_refusal_carries_the_record.md](../type/002_refusal_carries_the_record.md) | Guarantee 1's error shape |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_exactly_one_liveness_flag.md](../invariant/001_exactly_one_liveness_flag.md) | Why `is_closed` lives here and nowhere else in the family |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_close_is_advisory_to_an_unguarded_producer.md](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md) | The two **convention** rows, worked out as a trap |
| [../pitfall/002_ok_does_not_mean_kept_under_drop_newest.md](../pitfall/002_ok_does_not_mean_kept_under_drop_newest.md) | Why guarantee 1's `Full` arm is unreachable under the default policy |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_core/docs/api/001`](../../../ring_core/docs/api/001_producer_surface.md) | The producer surface `Guarded` wraps, and whose refusal contract guarantee 1 extends |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `the_three_operations_hand_back_a_ring_fit_for_the_next_run` — the reached-test |
| `tests/shutdown_test.rs` | `close_is_idempotent_and_admit_reports_it` — guarantee 2 |
| `tests/shutdown_test.rs` | `reset_discards_and_leaves_the_ring_open` — guarantee 3 |
| `tests/shutdown_test.rs` | `a_guarded_producer_refuses_a_closed_ring_and_returns_the_record` — guarantee 1 |


### SD5 — The Document's Own Summary of Its Own Table Is Wrong, Twice

The Abstract says the split is *four construction, two convention*. The section
below the table repeats it: *"Four rows read **construction**…"* and *"Two rows
read **convention**…"*.

The table has eleven rows. Six read **construction** — Stop, Wrap a producer,
Recover records, Empty without a sink, Reopen, Teardown in one call. One reads
**convention** — Ask. Four read `—`.

Both numbers are wrong in both places, and they are wrong in opposite
directions: the crate under-counts what it guarantees by construction and
over-counts what it leaves to convention. That is the least likely direction
for a rounding error and the most likely one for a document that was written
against an earlier, smaller table and never re-read against the current one.

The claim is not decorative, and it has already propagated. This column is what
the Scope line calls *"the distinction the rest of this crate's documentation
turns on"*, and the wrong denominator is now cited twice more, in two files:
this document's own Types cross-reference row and the crate's reading-order
table in [`../readme.md`](../readme.md) both introduce
[`type/001`](../type/001_stopped_proof_token.md) as *"the mechanism behind three
of the four **construction** rows"* — a fraction whose denominator does not
exist. So one uncounted table has produced four wrong statements across two
files, three of which agree with each other and none of which agrees with the
table.

Nothing checks it. The counts are prose beside a table in the same file, and no
checker in the corpus compares two statements inside one document — `shape.py`
counts directories, `vocabulary.py` counts finding IDs, `citations.py` resolves
links, `recipes.py` runs commands. A claim about a table four lines above it is
outside all four.

**Disposition:** applied — the Abstract, the guarantee-column section, this
document's own Types cross-reference row, and `../readme.md`'s reading-order
row all now state the table's real counts (six construction, one convention,
four ungraded). Now prints: `behind three of the six **construction`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/api
printf 'abstract:  %s\n' "$( awk '/^### Regenerate/{exit} {print}' 001_shutdown_surface.md | command grep -ohE 'Six rows are construction, one is convention' )"
printf 'cross-ref: %s\n' "$( awk '/^### Types/{f=1} f&&/^### Invariants/{exit} f' 001_shutdown_surface.md | command grep -ohE 'behind [a-z]+ of the [a-z]+ .{0,4}construction' )"
printf 'readme:    %s\n' "$( command grep -ohE 'behind [a-z]+ of the [a-z]+ .{0,4}construction' ../readme.md )"
```

Live output:

```
abstract:  Six rows are construction, one is convention
cross-ref: behind three of the six **construction
readme:    behind three of the six **construction
```

### SD6 — The "Whole Surface" Table Omits the Push It Grades and the Method That Revokes the Grade

The Scope line promises to *"fix the whole surface in one table."* The table
names eleven functions. The crate declares twenty-one public function names.

Ten are absent, and they are not a random ten. Among them:

- **`try_push`** — the row *"Wrap a producer … **construction** — `Guarded`'s
  only push checks"* is a claim *about* `try_push`, and `try_push` has no row of
  its own. The graded operation is the one the table never lists.
- **`try_push_batch`** — the second push, with its own distinct behaviour
  (returns `0` without consuming the iterator on a closed ring).
- **`into_inner`** — the method that hands the raw `Producer` back and thereby
  ends the guarantee three of the six **construction** rows rest on. It is
  important enough that the crate devotes a whole decision instance to whether
  it should exist ([`decisions/001`](../decisions/001_should_into_inner_exist.md)),
  and it does not appear on the surface the decision is about.
- **`into_record`** — the payload-recovery method, which
  [`item/001`](../item/001_the_declared_surface_and_its_attributes.md) shows is
  the one return value the compiler lets a caller drop.
- **`drain_all_bounded`** — the budgeted drain added when
  [`pitfall/001`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md)'s
  SD42 was dispositioned. `drain_all` has a row of its own; its budgeted sibling,
  same receiver and same purpose, does not — so this one omission is not
  explained by the rule below.

The omissions are consistent — the table lists *operations a caller performs on
the shutdown* and skips accessors and producer-side methods — so the selection
rule is defensible for nine of the ten. It is the promise that is wrong: a table that stops at
eleven of twenty-one cannot be "the whole surface", and calling it that is what
makes a reader stop looking for `into_inner`.
