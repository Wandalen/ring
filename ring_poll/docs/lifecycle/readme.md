# Lifecycle Doc Definition

### Scope

- **Purpose**: The two things in this crate that have a beginning and an end — the tick, and the record passing through it — and what the type system says about each.
- **Responsibility**: `Tick`'s states and its missing terminal transition; a record's four possible destinations and which of them the caller is told about.
- **In Scope**: Construction, accumulation, reading, and the absence of an end; the four return types and the `#[ must_use ]` placement across them.
- **Out of Scope**: What a budget bounds during transit (→ [`../invariant/002`](../invariant/002_a_budget_bounds_attempts_not_time.md)); why `Copy` makes a stale tick easy to make (→ [`../data_structure/002`](../data_structure/002_a_copy_accumulator.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The States A Tick Moves Through](001_the_states_a_tick_moves_through.md) | One constructor, an accumulating middle, a repeatable read, and no terminal state at all | 🔄 |
| 002 | [Where A Record Can End Up](002_where_a_record_can_end_up.md) | Four destinations, three of them recoverable, and the one the published docs do not mention | 🔄 |

**Two lifecycles, opposite problems.** `001` is about an object that cannot end:
`Tick` has no `Drop`, no consuming method, no `reset`, so "the frame is over" is a
caller convention the type does not represent. `002` is about an object that ends
four different ways: a record finishes in the ring, with the caller, in a `Vec`,
or destroyed — and the operation determines which of those the caller can find
out about.

They are separate documents because they fail on different edits. `001` goes
stale if a method is added that takes `self` by value; `002` goes stale if a
return type changes or a `#[ must_use ]` is added. Neither change would touch the
other file.

The four findings share one shape: something true and important is established
somewhere other than where a caller would look for it. The tick's single-frame
lifetime is a convention with no statement; its budget/counter coupling is
demonstrated by a workaround in the suite rather than described; the batch
operation's destructiveness is documented thoroughly in a test file `cargo doc`
does not publish; and `#[ must_use ]` is placed by return shape rather than by
what ignoring the value costs.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/lifecycle
printf 'instances:                  %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:    %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:    %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'Tick constructors:          %s\n' "$( awk '/^impl Tick$/{f=1} f&&/^\}$/{exit} f' ../../src/lib.rs | command grep -c 'pub const fn new' || true )"
printf 'Tick terminal operations:   %s\n' "$( command grep -cE 'fn (reset|finish|close)|^impl.*Drop' ../../src/lib.rs || true )"
printf 'record destinations:        %s\n' "$( command grep -oE '^-> (Result< \(\), [A-Z] >|Option< [A-Z] >|usize)' ../../src/lib.rs | sed 's/^-> //' | tr '\n' ' ' )"
printf 'of those, recoverable:      %s\n' "$( command grep -cE '^-> (Result|Option)' ../../src/lib.rs || true )"
printf 'must_use on operations:     %s\n' "$( command grep -B1 -E '^pub fn [a-z_]+' ../../src/lib.rs | command grep -c 'must_use' || true )"
printf 'must_use on accessors:      %s\n' "$( command grep -c '#\[ must_use' ../../src/lib.rs || true )"
printf 'loss described in src:      %s\n' "$( command grep -ciE 'eaten|eats|destroy|consumes and drops' ../../src/lib.rs || true )"
printf 'loss described in tests:    %s\n' "$( command grep -ciE 'eaten|eats|destroy|consumes and drops' ../../tests/poll_test.rs || true )"
```

Live output:

```
instances:                  2
finding headings inside:    4
rows in the table below:    4
each instance has a recipe: 2
Tick constructors:          1
Tick terminal operations:   1
record destinations:        Result< (), T > usize Option< T > usize 
of those, recoverable:      2
must_use on operations:     0
must_use on accessors:      11
loss described in src:      5
loss described in tests:    14
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL29 | a per-frame object with no representation of the frame ending | **latent hazard** | `Tick` is documented as *"one system's turn on the ring"* yet has no `Drop`, no method taking `self` by value, and no `reset`, so a tick hoisted out of a per-frame loop — an ordinary optimisation on a `Copy` type — accumulates `moved` for the program's lifetime and reports `Made( n )` forever, permanently silencing a scheduler that backs off on `Progress::None`; both tests that build a working tick read `progress()` and keep going afterwards, so the non-terminal read is deliberate and the gap is that nothing states a tick is meant to live one frame. |
| PL30 | one constructor sets both the ceiling and the counter, and the suite pays for it | n/a — observation | `Tick::new` is the only writer of `budget` and the only way to zero `moved`, so neither field can be varied without discarding the other — and `a_tick_counts_nothing_when_nothing_moved` already builds a second `Tick::default()` at `tests/poll_test.rs:613` purely to obtain a fresh counter, the same move that at frame scale throws away a budget; `Copy` makes both `with_budget` and `reset` one-liners, and nothing documents that a budget is fixed for the tick's life. |
| PL31 | the destructive path is tested precisely and published nowhere | **misleading doc** | Two tests pin that a refused `push_batch_within` attempt eats the record it pulled — `Budget::once()` leaves `Some( 5 )`, a larger budget leaves `Some( 6 )` — under a doc comment stating *"a budget of N against a full ring destroys N records rather than one"*, but `src/lib.rs` contains zero occurrences of *eaten*, *destroy* or *consumes and drops* against six in the test file, so the published rustdoc describes only the early-stop optimisation and a doctest where all five records fit, while recommending the very budget that scales the unmentioned loss linearly. |
| PL32 | `#[ must_use ]` placed by return shape rather than by cost of ignoring | n/a — unenforced | All ten `#[ must_use ]` attributes sit on inert value accessors and none on the eight ring operations; `Result` and `Option` cover four of them incidentally, leaving `push_batch_within`, `drain_up_to` and their two `Tick` forwarders discardable in silence — defensible for `drain_up_to`, whose records are in `out` and which the suite discards correctly at `tests/poll_test.rs:288`, and not for the batch pushes, where a short count is the caller's only prompt to inspect the iterator that PL31's loss is visible in. |
