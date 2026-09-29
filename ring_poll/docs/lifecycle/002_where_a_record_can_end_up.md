# Lifecycle: Where A Record Can End Up

### Scope

- **Purpose**: Follow one record through the four operations and record every state it can finish in, including the states from which the caller is not told where it went.
- **Responsibility**: The four terminal outcomes, which return value reports each, and which of those a caller can discard without a warning.
- **In Scope**: `Result< (), T >`, `Option< T >`, the two `usize` counts, and the `#[ must_use ]` placement across all eighteen public functions.
- **Out of Scope**: The tick's own lifecycle (→ [`001`](001_the_states_a_tick_moves_through.md)); what a budget bounds while the record is in flight (→ [`../invariant/002`](../invariant/002_a_budget_bounds_attempts_not_time.md)).

### The paths

A record enters through one of four operations and finishes in one of four
states. The operation determines which report the caller gets:

| Operation | Record ends up | Caller is told | Told by |
|---|---|---|---|
| `push_within` | in the ring, **or** back with the caller | exactly which | `Result< (), T >` |
| `push_batch_within` | some prefix in the ring; the rest **consumed from the iterator and dropped** | how many arrived — not what was lost | `usize` |
| `recv_within` | with the caller, or still in the ring | exactly which | `Option< T >` |
| `drain_up_to` | appended to `out`, or still in the ring | how many arrived | `usize`, and `out.len()` |

Three of the four are lossless. `push_batch_within` is not, and it is the one
whose report is a bare count → PL31.

### What each return value can express

```text
  push_within        Ok(())      ─ the ring has it
                     Err( rec )  ─ you have it back        ← total accounting

  recv_within        Some( rec ) ─ you have it
                     None        ─ the ring still has it   ← total accounting

  drain_up_to        n           ─ n are in `out`
                                   the rest are in the ring ← total accounting

  push_batch_within  n           ─ n are in the ring
                                   ??? — the iterator moved past
                                   records this call destroyed
```

The asymmetry is not in how many records move. It is that three operations leave
the record somewhere the caller can still reach, and one does not.

### Which of these a caller may silently ignore

`#[ must_use ]` appears ten times in the crate. All ten are on value-type
accessors and constructors. None is on an operation:

| Carries `#[ must_use ]` | Does not |
|---|---|
| `Budget::once`, `new`, `attempts` | `push_within`, `push_batch_within` |
| `Progress::of`, `is_made`, `count`, `then` | `recv_within`, `drain_up_to` |
| `Tick::new`, `budget`, `progress` | `Tick::push`, `push_batch`, `recv`, `drain` |

`Result` and `Option` are `#[ must_use ]` in the standard library, so
`push_within`, `recv_within`, `Tick::push` and `Tick::recv` are covered by their
return types regardless. The four that return a bare `usize` are not covered by
anything → PL32.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'must_use attributes:       %s\n' "$( command grep -c '#\[ must_use' src/lib.rs || true )"
printf 'public fns and methods:    %s\n' "$( command grep -cE '^(pub|  pub) (const )?fn ' src/lib.rs || true )"
printf 'fns carrying must_use:     %s\n' "$( awk '/#\[ must_use/{m=1;next} m&&/fn [a-z_]+/{ l=$0; sub(/.*fn /,"",l); sub(/[(<].*/,"",l); printf "%s ", l; m=0 }' src/lib.rs )"
printf 'the four ring operations:  %s\n' "$( command grep -oE '^pub fn [a-z_]+' src/lib.rs | sed 's/^pub fn //' | tr '\n' ' ' )"
printf 'of those, marked must_use: %s\n' "$( command grep -B1 -E '^pub fn [a-z_]+' src/lib.rs | command grep -c 'must_use' || true )"
printf 'Tick ops marked must_use:  %s\n' "$( awk '/^impl Tick$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -B1 -E '^  pub fn (push|push_batch|recv|drain)' | command grep -c 'must_use' || true )"
printf 'returning a bare usize:    %s\n' "$( command grep -oE '^-> usize' src/lib.rs | wc -l )"
printf 'returning Result or Option:%s\n' "$( command grep -cE '^-> (Result|Option)' src/lib.rs || true )"
printf 'batch consumes via:        %s\n' "$( command grep -oE 'try_push_batch|next\(\)' src/lib.rs | sort -u | tr '\n' ' ' )"
printf 'fns taking that iterator:  %s\n' "$( command grep -c 'records : &mut impl Iterator' src/lib.rs || true )"
printf 'src saying eat/destroy:    %s\n' "$( command grep -ciE 'eaten|eats|destroy|consumes and drops' src/lib.rs || true )"
printf 'tests saying it:           %s\n' "$( command grep -ciE 'eaten|eats|destroy|consumes and drops' tests/poll_test.rs || true )"
printf 'tests pinning the loss:    %s\n' "$( command grep -c 'records.next()' tests/poll_test.rs || true )"
printf 'what they assert is left:  %s\n' "$( command grep -A1 'records.next()' tests/poll_test.rs | command grep -oE 'Some\( [0-9] \)' | tr '\n' ' ' )"
printf 'the budget each spent:     %s\n' "$( command grep -B12 'records.next()' tests/poll_test.rs | command grep -oE 'Budget::(once\(\)|new\( [0-9] \))' | tr '\n' ' ' )"
printf 'batch doctests, and case:  %s / %s\n' "$( awk '/^\/\/\/ Publish from .records./{f=1} f&&/^pub fn push_batch_within/{exit} f&&/^\/\/\/ ```$/{n++} END{print n/2}' src/lib.rs )" "$( awk '/^\/\/\/ Publish from .records./{f=1} f&&/^pub fn push_batch_within/{exit} f' src/lib.rs | command grep -oE 'assert_eq!\( published, [0-9] \)' )"
printf 'a usize discarded in tests:%s\n' "$( command grep -nE '^ *(push_batch_within|drain_up_to)\(' tests/poll_test.rs | cut -d: -f1 | sed 's/^/tests\/poll_test.rs:/' | tr '\n' ' ' )"
```

Live output:

```
must_use attributes:       11
public fns and methods:    20
fns carrying must_use:     once new attempts of is_made count then new budget progress lost 
the four ring operations:  push_within push_batch_within recv_within drain_up_to 
of those, marked must_use: 0
Tick ops marked must_use:  0
returning a bare usize:    2
returning Result or Option:2
batch consumes via:        try_push_batch 
fns taking that iterator:  2
src saying eat/destroy:    5
tests saying it:           14
tests pinning the loss:    5
what they assert is left:  Some( 6 ) Some( 5 ) Some( 5 ) Some( 2 ) Some( 4 ) 
the budget each spent:     Budget::new( 3 ) Budget::once() Budget::once() Budget::once() Budget::new( 3 ) 
batch doctests, and case:  1 / assert_eq!( published, 5 )
a usize discarded in tests:tests/poll_test.rs:458 tests/poll_test.rs:809 
```

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_the_states_a_tick_moves_through.md](001_the_states_a_tick_moves_through.md) | The counter that all four operations feed, and its own missing terminal state |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/002_the_roster_as_a_public_constant.md`](../api/002_the_roster_as_a_public_constant.md) | The other place a stated promise is wider than the check credited with keeping it honest |

### Items

| File | Relationship |
|------|--------------|
| [`../item/002_what_the_crate_does_not_declare.md`](../item/002_what_the_crate_does_not_declare.md) | PL27 — why the failure is a returned record rather than an error type |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_a_budget_bounds_attempts_not_time.md`](../invariant/002_a_budget_bounds_attempts_not_time.md) | What bounds the attempts a record's transit costs |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md`](../pitfall/002_a_bigger_budget_is_not_a_bigger_batch.md) | The related misreading of `push_batch_within`'s count |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The four signatures and every `#[ must_use ]` placement |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `push_batch_within_eats_one_record_per_attempt` and `push_batch_within_spends_a_second_attempt_after_a_productive_first` — the pair that prices the loss; `drain_up_to_stops_at_the_limit` |

### PL31 — the destructive case is understood, tested, and absent from the published docs

Three of the four operations account for every record. `push_within` returns the
record on refusal. `recv_within` returns `None` and leaves it in the ring.
`drain_up_to` reports a count and leaves the rest in the ring. `push_batch_within`
takes `&mut impl Iterator< Item = T >` and advances it, so a record pulled from
the iterator and refused by the ring is in none of those places — it is gone, and
the return value is a count of what *did* arrive.

The crate knows this precisely. Two tests pin it, differing only in budget:

```rust
// Budget::once()   → records.next() == Some( 5 )
//   "one attempt: 0-3 published, 4 eaten by the refusal, 5 still pending"
// Budget::new( 3 ) → records.next() == Some( 6 )
//   "two attempts, two records eaten — 4 by the first refusal, 5 by the second"
//   (three were allowed; the early-stop rule ended it after two)
```

and a doc comment above them states the rule outright: *"a budget of N against a
full ring destroys N records rather than one"*, with the reason —
`ring_core::Producer::try_push_batch` pulls before it can know there is room —
and the sharp observation that *"the iterator is the only place that cost is
visible"*, since the published count and the consumer's length are identical
whether one attempt ran or two.

That is a better account than this document would have written. The finding is
where it lives. `cargo doc` publishes `src/lib.rs`; it does not publish
`tests/poll_test.rs`. Searching the source for any of *eaten*, *eats*, *destroy*,
or *consumes and drops* returns zero hits; searching the test file returns six.

What a caller reading the rustdoc for `push_batch_within` gets instead is:
*"Publish from `records` in batches, retrying within `budget`, never parking.
Returns how many records were published. Stops early on an attempt that moves
nothing…"* — a careful paragraph about the early-stop optimisation, followed by a
single doctest that publishes five records into an eight-slot ring and asserts
all five arrived. The refusal path, which is the only path where anything is
destroyed, appears in neither the prose nor the example.

The interaction is what makes this worth recording rather than filing as a doc
gap. The published sentence recommends the budget — *"retrying within `budget`"*
— and the unpublished sentence says the budget is exactly what multiplies the
loss. A caller who reads the rustdoc, sees retries are supported, and raises the
budget to improve throughput against a contended ring is following the
documentation to increase silent data loss linearly in the parameter it told them
to raise. `Tick::push_batch` inherits all of it under one line — *"[`push_batch_within`]
against this tick's budget"* — and supplies the budget automatically.

Moving those two sentences from the test file into the rustdoc costs nothing and
changes no behaviour. It is recorded here rather than applied because the source
edit belongs with the source, and because the shape generalises: this crate keeps
its sharpest reasoning in test doc comments, which is excellent for a reader who
opens the test file and invisible to everyone else.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -F "A refused attempt consumes and drops that record" ring_poll/src/lib.rs
```

Live output:

```
/// the ring has room. A refused attempt consumes and drops that record, so a
```

**Disposition:** applied — `push_batch_within`'s rustdoc in `src/lib.rs` (the
same fix recorded under [`../algorithm/002`](../algorithm/002_what_an_attempt_costs.md)
PL3) now states the destructive case directly, moving the substance of the
test file's doc comment into the published documentation rather than leaving
it recorded only here as deferred. Now prints: `A refused attempt consumes and drops that record`

### PL32 — `#[ must_use ]` is on all ten accessors and none of the eight operations

Ten `#[ must_use ]` attributes in the crate, all on the same kind of thing:
`Budget`'s three constructors and accessor, `Progress`'s four, and `Tick::new`,
`budget`, `progress`. These are pure, cheap, and side-effect-free — discarding
one is meaningless and the attribute correctly says so.

Not one of the eight ring operations carries it. Four are covered anyway:
`push_within` and `Tick::push` return `Result`, `recv_within` and `Tick::recv`
return `Option`, and both are `#[ must_use ]` in the standard library, so
ignoring them is already a warning.

The other four are not covered by anything. `push_batch_within`, `drain_up_to`,
`Tick::push_batch` and `Tick::drain` return a bare `usize`, so
`push_batch_within( &mut producer, &mut source, budget );` compiles clean with no
diagnostic. The suite already does this once — `drain_up_to( &mut consumer, &mut
out, 8 );` at `tests/poll_test.rs:288` — and there it is correct, because the
records are in `out` and the next line asserts on `out` directly. A discarded
count is not automatically a bug.

Where it matters is the two batch entry points, and the reason is PL31's, one
step further on. The count is not what reveals the loss — the test file's own
observation is that *"the published count and the consumer's length are identical
whether one attempt ran or two"*, so the count is precisely the value that
**cannot** tell you a record was destroyed. What a discarded count costs is the
one signal the caller has that the call ran at all against a ring that refused:
a `0` return, or a return short of what was fed in, is the prompt to go look at
the iterator, which is where the damage is visible.

So the attribute is present on ten functions whose results are inert, and absent
from the two whose results are the only prompt to check for silent data loss. The
placement follows the shape of the value — accessors return data, operations do
work — rather than the cost of ignoring it, which for a `usize` returned by a
batch push is the difference between noticing a refusal and not.

One attribute on `push_batch_within` and one on `Tick::push_batch` would close
it, with no signature change and no cost to any correct caller — `tests/poll_test.rs:288`
would keep compiling, because it discards a `drain_up_to` count and not a batch
count. That the same attribute on `drain_up_to` would be closer to noise is the
point rather than an aside: the two functions have identical return types and
opposite consequences for ignoring them, which is exactly the distinction a rule
based on return shape cannot make.
