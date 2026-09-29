# API: The Tick-Path Surface

### Scope

- **Purpose**: Enumerate everything `ring_poll` exposes and, for each item, record who chooses its cost — this crate or the caller.
- **Responsibility**: The surface, the guarantees it makes, the guarantees it deliberately does not make, and the items settled as absent.
- **In Scope**: All exported items; the four compatibility guarantees and their enforcement strength; the one failure mode that reaches a caller through this surface without originating in it.
- **Out of Scope**: Why no operation here parks (→ [`../invariant/001`](../invariant/001_no_parking_operation_on_the_tick_path.md)); what a budget does and does not bound (→ [`../invariant/002`](../invariant/002_a_budget_bounds_attempts_not_time.md)).

### Abstract

**Half this surface has a cost the caller sets, and the surface says which
half.** Every entry below carries a "who chose it" column, because a helper that
quietly accepted a `usize` without saying so would look like it was making the
latency promise itself. It is not.

### Operations

| Item | Signature | Cost bounded by | Who chose it |
|---|---|---|---|
| `PARKING_CRATES` | `[ &str; 3 ]` | — | The family's dependency graph |
| `Budget::once` | `() -> Budget` | 1 ring operation | This crate |
| `Budget::new` | `( usize ) -> Budget` | `attempts` ring operations | **The caller** |
| `Budget::attempts` | `( self ) -> usize` | — | — |
| `Progress::of` | `( usize ) -> Progress` | — | — |
| `Progress::is_made` / `count` / `then` | — | — | — |
| `push_within` | `( &mut Producer, T, Budget ) -> Result< (), T >` | `budget` | The caller |
| `push_batch_within` | `( &mut Producer, &mut impl Iterator, Budget ) -> usize` | `budget`, and stops early | The caller |
| `recv_within` | `( &mut Consumer, Budget ) -> Option< T >` | `budget` | The caller |
| `drain_up_to` | `( &mut Consumer, &mut Vec< T >, usize ) -> usize` | `max` | The caller |
| `Tick::new` / `budget` / `progress` | — | — | — |
| `Tick::push` / `push_batch` / `recv` / `drain` | mirrors the free functions | the tick's budget | The caller, once, at `Tick::new` |

The "who chose it" column is the one worth reading. Half this surface has a cost
the caller sets, and a helper that quietly accepted a `usize` without saying so
would look like it was making the latency promise itself. It is not
([`../invariant/002`](../invariant/002_a_budget_bounds_attempts_not_time.md)).

#### Settled — absent

| Item | Why it is not here |
|---|---|
| `push_blocking`, `recv_blocking` | The whole point. They exist in `ring_wait` for threads outside the tick |
| A `Duration` parameter anywhere | Would require reading a clock per attempt, which costs more than the attempt. A budget is the cheap bound; see [`../decisions/001`](../decisions/001_should_the_roster_be_generated.md) for the roster question, and `ring_bench` for whether the clock read is actually the expensive part |
| `push_all_or_nothing` | The ring offers no rollback. A partial batch cannot be un-published, so an all-or-nothing helper would have to buffer the whole iterator first — which is the caller's decision to make, with the caller's allocator |
| An error type | `Result< (), T >` carries the record; there is nothing an error enum would add that `Err( record )` does not already say |

### Error Handling

**There is no error type, and that is the design.** `push_within` returns
`Result< (), T >` (`src/lib.rs:286`) rather than `Result< (), Error >`: a failed
push hands the record back so the caller can decide — retry next tick, drop it,
or route it elsewhere. An error enum would add a name for the one thing that can
go wrong and lose the payload doing it.

`recv_within` returns `Option< T >`; `push_batch_within` and `drain_up_to`
return a `usize` count and report a short result rather than a failure. None of
the four can fail in a way that needs describing.

#### The trap that is not this crate's

Under `OverflowPolicy::DropNewest` — `RingConfig`'s default — a push into a full
ring reports `Ok` having discarded the record. That surfaces through
`push_within` and `Tick::push` unchanged, because they report what
`ring_core::Producer::try_push` reports. The behaviour is documented once, where
it was first found, in
[`ring_shutdown/docs/pitfall/002`](../../../ring_shutdown/docs/pitfall/002_ok_does_not_mean_kept_under_drop_newest.md),
and this crate's suite asserts it holds here too rather than restating why
([`push_within_under_drop_newest_reports_success_and_keeps_nothing`](../../tests/poll_test.rs)).

### Compatibility Guarantees

| # | Guarantee | Holds by |
|---|---|---|
| 1 | No operation here parks, yields, or sleeps | **Construction** — `ring_wait` is not a dependency, so the calls that do those things cannot be named ([`../invariant/001`](../invariant/001_no_parking_operation_on_the_tick_path.md)) |
| 2 | Every call returns after a finite number of ring operations | **Construction** — every loop is bounded by a `usize` fixed before it starts |
| 3 | A failed push returns the record | **Construction** — the error type *is* `T`; there is no variant that discards it |
| 4 | `Progress::Made( 0 )` is never produced by this crate | **Convention** — the enum variant is public, so a caller can still spell it. `Progress::of` is the constructor that cannot ([`../type/002`](../type/002_progress_has_no_zero_made.md)) |

Guarantee 4 is the only one on this list that a caller can break, and it is
listed as convention rather than quietly promoted, on the same principle as
`ring_shutdown`'s advisory-close row.

### Algorithms

| File | Relationship |
|------|--------------|
| [`../algorithm/001_bounded_retry.md`](../algorithm/001_bounded_retry.md) | The loop every budget-taking entry above runs |

### Integrations

| File | Relationship |
|------|--------------|
| [`../integration/001_family_dependency_seam.md`](../integration/001_family_dependency_seam.md) | The declared edges that make guarantee 1 hold by construction |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/001_no_parking_operation_on_the_tick_path.md`](../invariant/001_no_parking_operation_on_the_tick_path.md) | Guarantee 1, and the measurement behind it |
| [`../invariant/002_a_budget_bounds_attempts_not_time.md`](../invariant/002_a_budget_bounds_attempts_not_time.md) | Why the "who chose it" column exists — this crate bounds attempts, not latency |

### Patterns

| File | Relationship |
|------|--------------|
| [`../pattern/001_enforcement_by_dependency_graph.md`](../pattern/001_enforcement_by_dependency_graph.md) | How guarantee 1 is enforced without a single runtime check |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`../pitfall/001_non_parking_is_not_bounded_latency.md`](../pitfall/001_non_parking_is_not_bounded_latency.md) | The misreading guarantee 1 invites |
| [`../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md`](../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md) | The misreading the `Budget` parameter invites |

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_budget_clamps_to_one.md`](../type/001_budget_clamps_to_one.md) | The parameter half this surface takes |
| [`../type/002_progress_has_no_zero_made.md`](../type/002_progress_has_no_zero_made.md) | Guarantee 4's subject, and why it is convention-strength |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Every item in the Operations table; `:253` states the `Result< (), T >` rationale in the source itself; `:286` and `:599` are the two signatures carrying it |

### What the surface derives, and what it does not

| Type | Derives | Missing | Consequence |
|---|---|---|---|
| `Budget` | `Debug Clone Copy PartialEq Eq PartialOrd Ord Hash` | — | sortable, hashable, usable as a map key |
| `Progress` | `Debug Clone Copy PartialEq Eq` | `PartialOrd Ord Hash` | two results compare equal-or-not and nothing else → PL5 |
| `Tick` | `Debug Clone Copy` | `PartialEq Eq` | two ticks cannot even be compared for equality |

`Budget` is the input and carries every ordering trait; `Progress` is the output
and carries none. The asymmetry runs the wrong way for a scheduler, which sets
one budget and then compares many results.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'top-level pub items:      %s\n' "$( command grep -cE '^pub (struct|enum|const|fn) ' src/lib.rs )"
printf 'pub fn, all impls:        %s\n' "$( command grep -cE '^ *pub (const )?fn ' src/lib.rs )"
printf 'of those const:           %s\n' "$( command grep -cE '^ *pub const fn ' src/lib.rs )"
printf 'must_use attributes:      %s\n' "$( command grep -c '#\[ must_use \]' src/lib.rs || true )"
printf 'non_exhaustive:           %s\n' "$( command grep -c 'non_exhaustive' src/lib.rs || true )"
printf 'derive lines, in order:   %s\n' "$( command grep -oE '^#\[ derive\( [^)]*' src/lib.rs | sed 's/^#\[ derive( //' | tr '\n' '/' )"
printf 'Budget ordering traits:   %s\n' "$( awk -v n1="$( command grep -n -m1 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash ) ]' src/lib.rs | cut -d: -f1 )" 'NR==n1' src/lib.rs | command grep -oE 'PartialOrd|Ord|Hash' | wc -l )" | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
printf 'Progress ordering traits: %s\n' "$( awk -v n1="$( command grep -n -m1 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]' src/lib.rs | cut -d: -f1 )" 'NR==n1' src/lib.rs | command grep -oE 'PartialOrd|Ord|Hash' | wc -l )" | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
printf 'Tick equality traits:     %s\n' "$( awk -v n1="$( command grep -n -m1 -F '#[ derive( Debug, Clone, Copy ) ]' src/lib.rs | cut -d: -f1 )" 'NR==n1' src/lib.rs | command grep -oE 'PartialEq|Eq' | wc -l )" | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
printf 'tests in the suite:       %s\n' "$( command grep -c '^#\[ test \]' tests/poll_test.rs || true )"
printf 'doc examples in src:      %s\n' "$( expr "$( command grep -c '^/// ```$' src/lib.rs )" / 2 )"
printf 'PARKING_CRATES arity:     %s\n' "$( command grep -oE '\[ &str; [0-9]+ \]' src/lib.rs )"
```

Live output:

```
top-level pub items:      8
pub fn, all impls:        20
of those const:           12
must_use attributes:      11
non_exhaustive:           0
derive lines, in order:   Debug, Clone, Copy, PartialEq, Eq, PartialOrd, Ord, Hash /Debug, Clone, Copy, PartialEq, Eq /Debug, Clone /
Budget ordering traits:   3
Progress ordering traits: 0
Tick equality traits:     0
tests in the suite:       32
doc examples in src:      8
PARKING_CRATES arity:     [ &str; 3 ]
```

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | The suite covering this surface, including `push_within_under_drop_newest_reports_success_and_keeps_nothing` for the inherited trap |
| [`src/lib.rs`](../../src/lib.rs) | The doc examples, run as doc tests |

Counts for both live in the Regenerate block above rather than in this table,
because a number written into prose goes stale silently and a measured one does
not → PL6.

### PL5 — the input type is ordered and hashable, the output type is neither

`Budget` derives eight traits including `PartialOrd`, `Ord` and `Hash`.
`Progress` derives five and none of those three. `Tick` derives three and stops
before `PartialEq`.

The direction is backwards for the caller this crate is written for. A scheduler
sets a budget once — one value, at `Tick::new` — and then handles a `Progress`
per subsystem per frame, which is the value it actually wants to aggregate. It
cannot sort them, cannot `max()` over them, cannot key a map by one, and cannot
put one in a `HashSet`. It can only ask whether two are equal.

`Progress::then` exists precisely because folding results is the expected
operation, so the need is acknowledged in the surface — `then` combines two into
one. What is missing is everything else a fold might want: an ordering to pick
the largest, a hash to group by outcome. The workaround is `count()`, which
unwraps to a `usize` that has all three traits, so every caller that needs to
order results converts out of the type first. A type whose users routinely
convert out of it to do the ordinary thing is one trait list short.

Nothing here is broken, and adding `PartialOrd` to `Progress` would raise a real
question — is `None` less than `Made( 1 )`, or incomparable? That question is
worth answering deliberately rather than by derive, and this finding is the
record that it has not been asked.

### PL6 — a stale count in a cross-reference table survived the tests that changed it

Until this revision the Tests table above read *"22 tests covering the surface"*
and *"8 doc examples"*, and the file ended with a bare `cargo nextest run -p
ring_poll     # 22 tests` block repeating the figure a third time.

The suite has 23. The number was correct when written and drifted the moment a
test was added, which is the normal fate of a count typed into prose. Nothing
detected it: the corpus recipe checker executes recipes and compares nothing, the
test suite does not read its own documentation, and the third copy of the figure
sat inside a `sh` block that looks executable and asserts nothing.

The doc-example figure was and still is right, which is the more interesting
half — two counts written the same way at the same time, one drifted and one did
not, and a reader has no way to tell which is which. That is the argument for
measuring rather than for correcting: a corrected number is a number that will
drift again.

The replacement is the Regenerate block, whose output is regenerated from the
files themselves. The general form of the problem — documentation counts with no
mechanical link to what they count — is [`../pitfall/`](../pitfall/readme.md)'s
subject in this crate and appears across the corpus wherever a table names a
figure instead of deriving one.
