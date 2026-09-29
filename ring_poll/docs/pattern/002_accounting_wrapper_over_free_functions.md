# Pattern: Accounting Wrapper Over Free Functions

### Scope

- **Purpose**: Extract the shape used when an operation needs bookkeeping that not every caller wants: write the operation as a free function, and put the bookkeeping in a thin type that calls it.
- **Responsibility**: The two layers, what the split buys, the fidelity the wrapper's claim depends on, and the accounting hole the split opens.
- **In Scope**: The four free helpers and the four `Tick` methods that wrap them; what enforces the delegation claim and what leaves the counter bypassable.
- **Out of Scope**: What the counter fails to count (→ [`../algorithm/002`](../algorithm/002_what_an_attempt_costs.md)); why the tick has no terminal state (→ [`../lifecycle/001`](../lifecycle/001_the_states_a_tick_moves_through.md)).

### Problem

An operation has a cost worth measuring, and only some callers want the measurement.

Put the counter inside the operation and every caller pays for it — including the
one that already has its own accounting, and the one calling it once at startup.
Make the counter a parameter and every call site grows an argument it mostly
passes a placeholder for. Return the count alongside the result and the signature
gets worse for the caller who does not want it.

The specific version here: a scheduler wants one number per system per frame —
did this subsystem move anything — while a test, a setup path, or a one-shot
handshake wants the ring operation with nothing attached.

### Solution

**1. The operation is a free function, complete on its own.** `push_within`,
`push_batch_within`, `recv_within` and `drain_up_to` each take what they need,
return what happened, and hold no state. Every retry rule lives here, once.

**2. The bookkeeping is a small struct that calls them.** `Tick` holds the budget
and two running counts, and each of its four methods is the free call plus an
update:

```rust
pub fn recv< T : Send >( &mut self, consumer : &mut Consumer< '_, T > ) -> Option< T >
{
  let outcome = recv_within( consumer, self.budget );
  if outcome.is_some()
  {
    self.moved += 1;
  }
  outcome
}
```

`push_batch` is the one that does more than count a success — it wraps the
caller's iterator to learn how many records left it, so it can record what a
refusal destroyed as well as what got through (→ [`../algorithm/002`](../algorithm/002_what_an_attempt_costs.md)).
That is still only accounting: the free function it calls is unchanged, and so is
what it returns.

**3. The wrapper returns what the free function returned.** Not a wrapped type,
not a builder — the same `Result`, `Option` or `usize`. Adopting the accounting
layer costs a caller nothing at the call site except the receiver.

The type's own doc states the contract: *"Every method delegates to the free
function of the same shape and adds only the accounting, so there is one
implementation of each operation and one place a retry rule can be got wrong."*

### Applicability

| Condition | Why it matters |
|---|---|
| The bookkeeping is genuinely optional | If every caller needs it, put it in the operation and skip the layer |
| The operation is stateless | A stateful operation already has somewhere to keep a counter |
| The wrapper adds *only* bookkeeping | The moment it adds a policy decision, the two layers can disagree and the pattern's guarantee is gone |
| Callers may reasonably use both layers in one program | This is what makes the split worth having, and it is also where the counter leaks — see Consequences |

### Consequences

**What it buys is real.** One implementation of each retry rule; a wrapper small
enough to read in a glance; an accumulator with no allocation; and the option to
skip the layer entirely, which fifteen of this suite's tests take.

**The delegation claim is prose.** Nothing checks that `Tick::push` calls
`push_within` rather than reimplementing it, or that it passes the same budget.
Three of the four methods pass `self.budget`; `drain` passes `max` instead, for a
reason its own doc gives — so the pattern already has one live exception, stated
locally and not carried in the type-level sentence that says *every* method
→ PL39.

**The counter is bypassable, and the type hands you the key.** `Tick::budget()`
is public, so `push_within( &mut producer, record, tick.budget() )` is the
natural way to spend the tick's budget outside the tick. It compiles, performs
exactly the right ring operation, and does not count. `progress()` then reports
less than happened. That is deliberate and now stated at both doors rather than
at neither → PL40.

That second consequence generalizes past this crate: **any wrapper whose
accounting is optional can be bypassed, and the more faithfully it forwards, the
more natural the bypass looks.** The pattern's third part — return exactly what
the free function returns — is what makes adoption cheap and what makes
abandonment invisible.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'free operations:             %s\n' "$( command grep -oE '^pub fn [a-z_]+' src/lib.rs | sed 's/pub fn //' | tr '\n' ' ' )"
printf 'Tick methods wrapping them:  %s\n' "$( awk '/^impl Tick$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -oE 'pub fn [a-z_]+' | sed 's/pub fn //' | tr '\n' ' ' )"
printf 'wrappers calling a free fn:  %s\n' "$( awk '/^impl Tick$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -cE '= (push_within|push_batch_within|recv_within|drain_up_to)\(' || true )"
printf 'of those passing the budget: %s\n' "$( awk '/^impl Tick$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -c 'self.budget )' || true )"
printf 'sites incrementing moved:    %s\n' "$( command grep -c 'self.moved +=' src/lib.rs || true )"
printf 'guarded by is_ok/is_some:    %s\n' "$( awk '/^impl Tick$/{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -cE 'is_ok\(\)|is_some\(\)' || true )"
printf 'the budget accessor:         %s\n' "$( command grep -oE 'pub const fn budget\( &self \) -> Budget' src/lib.rs )"
printf 'setters or resets for moved: %s\n' "$( command grep -cE 'fn (set_moved|add_moved|record|reset)' src/lib.rs || true )"
printf 'free fns naming the bypass:  %s\n' "$( command grep -cE '^/// # Not Counted By A .Tick.' src/lib.rs || true )"
printf 'and the type-level section:  %s\n' "$( command grep -m1 -oE '# The Accounting Is Bypassable' src/lib.rs )"
printf 'the doctest count asserted:  %s\n' "$( command grep -oE 'tick.progress\(\).count\(\), [0-9]+' src/lib.rs )"
printf 'tests: Tick-only/free/both:  %s\n' "$( awk '/^fn /{name=$0;t=0;g=0} /Tick::|tick\./{t=1} /push_within\(|push_batch_within\(|recv_within\(|drain_up_to\(/{g=1} /^\}$/{if(name!=""){if(t&&g)b++;else if(t)o++;else if(g)r++;name=""}} END{printf "%d / %d / %d", o, r, b}' tests/poll_test.rs )"
```

Live output:

```
free operations:             push_within push_batch_within recv_within drain_up_to 
Tick methods wrapping them:  push push_batch recv drain 
wrappers calling a free fn:  3
of those passing the budget: 3
sites incrementing moved:    4
guarded by is_ok/is_some:    2
the budget accessor:         pub const fn budget( &self ) -> Budget
setters or resets for moved: 1
free fns naming the bypass:  4
and the type-level section:  # The Accounting Is Bypassable
the doctest count asserted:  tick.progress().count(), 3
tests: Tick-only/free/both:  8 / 15 / 1
```

### Patterns

| File | Relationship |
|------|--------------|
| [001_enforcement_by_dependency_graph.md](001_enforcement_by_dependency_graph.md) | The other pattern this crate carries — that one enforces a rule, this one shapes a surface |

### Algorithms

| File | Relationship |
|------|--------------|
| [`../algorithm/002_what_an_attempt_costs.md`](../algorithm/002_what_an_attempt_costs.md) | PL4 — what this wrapper's one counter leaves out |

### Data Structures

| File | Relationship |
|------|--------------|
| [`../data_structure/002_a_copy_accumulator.md`](../data_structure/002_a_copy_accumulator.md) | Why the accumulator is `Copy`, and what that costs |

### Lifecycles

| File | Relationship |
|------|--------------|
| [`../lifecycle/001_the_states_a_tick_moves_through.md`](../lifecycle/001_the_states_a_tick_moves_through.md) | PL29 — the wrapper has no state saying its frame ended |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Both layers: the four free functions and the four methods over them |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | Fifteen tests use the free layer alone, seven the wrapper alone, one both |

### PL39 — the delegation contract is a sentence, with one exception already in force

`Tick`'s doc says *"Every method delegates to the free function of the same shape
and adds only the accounting."* That sentence is what justifies the split: it is
why a reader can trust that `tick.push` retries the way `push_within` retries,
and why the retry rules only had to be got right once.

Nothing checks it. No test asserts that `Tick::push` and `push_within` behave
alike; no test calls both against the same ring and compares. A future edit that
gave `Tick::recv` a different budget, an extra attempt, or an early return would
compile, would pass all twenty-three tests, and would falsify the sentence
silently — because the three tests that exercise `Tick` assert counts, not
equivalence with the free layer.

The word *every* is also already too strong. Three methods pass `self.budget` to
their free function; `drain` passes `max` and no budget at all, for the reason its
own doc gives — a budget bounds retries, a drain limit bounds successes. That is
a good decision, documented where it happens. What is missing is that the
type-level sentence still says *every*, so a reader who takes it at face value
concludes that `Tick::new( Budget::new( 100 ) )` governs all four operations, and
for one of them it governs nothing → `../algorithm/` PL2 records the same
asymmetry from the retry side.

The cheap correction is to the sentence, not to the code: name the exception
where the rule is stated. The expensive one is a test per pair asserting the two
layers agree, which would make the split's central claim mechanical instead of
asserted. Neither is applied here, because *every* was written when the drain
took a budget or before the drain existed, and deciding which sentence is now
true is a decision about the contract rather than a documentation fix.

### PL40 — the accumulator is bypassable, and the type publishes the bypass

`Tick` counts what moves through it. `Tick::budget()` is a public accessor
returning the budget by value, and the free functions are public, so this is
legal, idiomatic, and wrong:

```rust
let mut tick = Tick::new( Budget::new( 4 ) );
push_within( &mut producer, record, tick.budget() )?;   // right budget, no count
assert_eq!( tick.progress(), Progress::None );          // and it holds
```

The ring operation is correct. The budget is the tick's own. The record is
published. And `moved` stays zero, because the only four sites that increment it
are inside the four methods, `moved` is private, and there is no setter — the
count cannot be repaired after the fact even by a caller who notices.

The consequence is the one `Progress` exists to prevent. A scheduler that backs
off on `Progress::None` — the pattern the enum is shaped for, with `then` to fold
across subsystems — will back off a subsystem that did work, and will keep doing
so for as long as that subsystem prefers the free layer. The failure was silent
in both directions: nothing warned at the call site, and the wrong answer is a
plausible one.

The suite did not reach it either. Some tests used `Tick` and nothing else, more
used the free functions and nothing else, and none used both. The two layers were
tested as two separate surfaces, which is exactly the arrangement in which a seam
between them goes unexamined.

This is not an argument against the pattern — the bypass is the same property
that lets most of this suite skip the bookkeeping, and that property is worth
having. It is an argument that the wrapper's guarantee needs stating in the
negative: `progress()` reports what moved *through this tick*, not what moved.
The type's doc said *"a running count of what moved"*, which is the reading that
fails.

**Disposition:** applied — each of the four free functions in `src/lib.rs` now
carries a `# Not Counted By A Tick` section saying the call moves records that no
`Tick` will observe, and `Tick` itself carries
`# The Accounting Is Bypassable`, which names `budget()` as the accessor that
makes the bypass idiomatic and restates `progress()` as what moved *through this
tick*. The bypass itself is untouched: making the free layer private would delete
the optionality this pattern exists to provide, and the finding says so.
`reaching_past_the_tick_leaves_its_count_short` in `tests/poll_test.rs` closes the
coverage half — it spends the tick's own budget through `push_within`, publishes a
record, and asserts `tick.progress()` is still `Progress::None`, so the seam is
now a pinned behaviour rather than an untested one. The type-level section reads:
Now prints: `# The Accounting Is Bypassable`
