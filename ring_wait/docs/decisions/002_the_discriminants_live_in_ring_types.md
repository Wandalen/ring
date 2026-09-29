# Decision: The Discriminants Live in `ring_types`

### Scope

- **Purpose**: Record the split that puts `WaitKind`'s four variants in one crate and their four handlers in another, what it buys, and what it costs this crate.
- **Responsibility**: State the ruling, show what each side holds, quantify the price of the boundary, and name the two things that would collapse it.
- **In Scope**: The `ring_types` / `ring_wait` division for `WaitKind`.
- **Out of Scope**: The same shape as a reusable pattern with a second instance — see [`pattern/002`](../pattern/002_discriminants_here_handlers_there.md).

### The Ruling

A ruling elsewhere places the
discriminants in `ring_types` and the handlers in `ring_wait`. Both crates state
it in their own module documentation — `ring_wait/src/lib.rs:7-12` and
`ring_types/src/policy.rs:4-7` — and `ring_types`' says why it matters to
it specifically:

> Only the discriminants live here. The handlers that act on them are
> `ring_wait` and `ring_overflow` respectively — the split
> `docs/decision/121_workstream_008_contract_gaps_ruled.md` § 5 rules, and the
> reason this crate's own description ends "no ring logic".

### What Each Side Holds

| | `ring_types::policy` | `ring_wait` |
|--|---------------------|-------------|
| The four variants | `Spin`, `Yield`, `Park`, `None` (`:21-34`) | — |
| The variant order | `ALL`, `:58` — asserted, because it is on the wire | — |
| "Which one is safe on a tick" | `is_non_blocking`, `:81-88` | — |
| What each variant *does* | — | `pause`, `:112-146` |
| What to try instead | — | `escalation_hint`, `:83-91` |
| The loop that uses it | — | `wait_until`, `:179-195` |
| Dependencies pulled in | none | `ring_cursor`, and `std` |

The asymmetry is the whole point. `is_non_blocking` is in `ring_types` and
`pause` is here, even though they answer the same question — the first as a
property of a value, the second as behaviour. `tests/wait_test.rs:69-84` asserts
they agree:

```rust
let keep_going = pause( kind, 0 );
assert_eq!( keep_going, !kind.is_non_blocking(), … );
```

That assertion exists because the split makes disagreement *possible*. Two
crates hold two encodings of one fact, and only a test spanning both keeps them
aligned.

### WT19 — The Boundary Costs One Byte

```
probe: core::mem::size_of::< WaitKind >() = 1
```

`WaitKind` is a fieldless four-variant enum deriving
`Debug, Clone, Copy, PartialEq, Eq, Hash, Default`
(`ring_types/src/policy.rs:21`). Passing it by value across the boundary
costs one byte and no indirection, which is what makes the split free at the
call site: every one of this crate's six functions takes `kind : WaitKind` by
value, and none of them takes a reference to one.

That is not incidental. The stated benefit of the split is that *"a `WaitKind`
is a configuration value that travels through a `RingConfig` and into a struct
field without dragging a thread parking implementation behind it"* (`:10-12`).
A one-byte `Copy` enum can sit in a config struct, be compared, be hashed, and
be defaulted; a trait object or a function pointer could not do all four, and a
struct holding a strategy implementation could not be `Copy`.

`RingConfig` is where that cash is collected — `with_wait` at
`ring_config/src/lib.rs:89` is a `const fn`, which it could not be if the
strategy were anything but a plain discriminant.

### What the Split Costs This Crate

| Cost | Consequence |
|------|-------------|
| Cannot rename a variant | `Park` sleeps under a name it does not own ([`decisions/001`](001_park_sleeps_rather_than_parking.md)) |
| Cannot add a variant | a fifth would need a `ring_types` change and would leave this crate silently one handler short |
| Cannot make the enum non-exhaustive from here | the `match` in `pause` (`:114-145`) is exhaustive by construction, and is the only thing that fails when a variant is added |
| Two crates must agree on "non-blocking" | asserted by a test in this crate, not by a type |

The second row is why `tests/wait_test.rs:56-65` asserts the discriminant *set*
in a crate that does not declare it. Its own module documentation defends the
apparent duplication at `:11-19`: `ring_types` asserts the enum's shape as a
fact about that crate; this file asserts it as a **precondition** of "one handler
per discriminant", which is meaningless without knowing how many discriminants
there are. The two assertions would survive each other's deletion and mean
different things.

### The Sibling Enum Already Has Two Matchers

`OverflowPolicy` is the same shape under the same ruling, so it is the control
for what happens to a discriminant enum over time. The finding is WT22, recorded
in [`pattern/002`](../pattern/002_discriminants_here_handlers_there.md) where the
two handlers are compared side by side; what it is doing here is supplying the
evidence that this decision has a cost that arrives later rather than at the
boundary:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rl 'WaitKind::Spin\s*=>' */src/*.rs
grep -rl 'OverflowPolicy::[A-Za-z]*\s*=>' */src/*.rs
```

Live output:

```
ring_wait/src/lib.rs
ring_overflow/src/lib.rs
ring_stats/src/lib.rs
```

| Enum | Crates exhaustively matching its variants |
|------|-------------------------------------------|
| `WaitKind` | 1 — `ring_wait` |
| `OverflowPolicy` | **2** — `ring_overflow`, and `ring_stats` at `:287-292` and `:343-348` |

`ring_stats`' two matches are not handlers; they select a counter
(`record_drop`, `dropped`). But they are exhaustive matches on the enum in a
crate that is not its named handler, and they are what "one handler per
discriminant" drifting looks like in practice: not a rival implementation, a
dispatch table that happens to enumerate the same variants.

The property `tests/wait_test.rs:69-84` asserts is per-crate and looks only at
`ring_wait`, so the equivalent drift here would be invisible to it. Today
`WaitKind` has no second matcher; nothing in the test suite would say if it
gained one.

### What Would Collapse the Split

Two changes, neither currently wanted:

1. **A strategy that needs state.** A back-off with memory, an adaptive
   strategy, or a real park with a registration all need something a
   discriminant cannot carry. The moment one arrives, `WaitKind` stops being a
   sufficient configuration value and the split has to be re-argued.
2. **A second matcher on `WaitKind`**, of the kind `OverflowPolicy` already
   has — at which point "one handler per discriminant" becomes a property with
   no owner and no check.

That both enums are ruled the same way and have already diverged on this point
is what makes the split a pattern with an observable failure mode rather than a
one-off ([`pattern/002`](../pattern/002_discriminants_here_handlers_there.md)).


### WT31 — Two Bounded-Retry Crates, Two Defaults, a Factor of 1024 Apart

`ring_wait` and `ring_poll` are the family's two crates whose subject is a
bounded retry loop. Each ships a default budget. They differ by three orders of
magnitude and neither mentions the other.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -B2 'pub const DEFAULT_SPINS' ring_wait/src/lib.rs | tail -3
grep -A4 'pub const fn once' ring_poll/src/lib.rs
# does either default cite the other? zero is the answer, and `grep -c` exits 1 saying it
grep -c 'ring_poll' ring_wait/src/lib.rs || true
grep -c 'DEFAULT_SPINS' ring_poll/src/lib.rs || true
```

Live output:

```
/// assert_eq!( ring_wait::DEFAULT_SPINS, 1024 );
/// ```
pub const DEFAULT_SPINS : usize = 1024;
  pub const fn once() -> Self
  {
    Self( 1 )
  }

0
0
```

`DEFAULT_SPINS` is 1024. `Budget::once()` is 1. Both are the documented default
for "how many times should this try before giving up", and the ratio between them
is 1024.

The difference is correct and it is the whole point of the split. `ring_poll` is
the tick-path crate: a tick has a deadline, so its default is one attempt and its
module documentation says a million attempts "never deadlocks and will still blow
a frame budget". `ring_wait` is for callers with no deadline, where 1024 spins is
a reasonable wait before reporting back.

What is worth recording is that the two constants carry no reference to each
other in either direction — the greps return zero both ways. The two defaults
encode the family's central distinction (tick path versus not) in two numbers in
two crates, and the only thing tying them together is that WT17 already found
their *clamps* written twice for the same reason. A reader who finds one default
has no path to the other, and no indication that picking between them is the same
choice as picking between the two crates.

### Decisions

| File | Relationship |
|------|--------------|
| [001_park_sleeps_rather_than_parking.md](001_park_sleeps_rather_than_parking.md) | The variant this crate cannot rename |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_a_crate_with_no_type_of_its_own.md](../data_structure/001_a_crate_with_no_type_of_its_own.md) | The three types this crate borrows rather than declares |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | `ring_types` as one of the two dependencies |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | The four handlers the split leaves here |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_none_looks_exactly_once.md](../invariant/002_none_looks_exactly_once.md) | The one variant the split exists to protect |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_discriminants_here_handlers_there.md](../pattern/002_discriminants_here_handlers_there.md) | The same split, twice, and what makes it a pattern |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_one_return_type_and_the_one_must_use.md](../type/002_one_return_type_and_the_one_must_use.md) | `RingError`, the other type crossing the same boundary |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/policy.rs:1-7,21-34,58,81-88` | The module note, the enum, `ALL`, and `is_non_blocking` |
| `ring_config/src/lib.rs:89,167` | Where a one-byte discriminant pays for itself |
| `ring_stats/src/lib.rs:343-348` | The sibling enum's second matcher |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:11-19` | Why asserting the discriminant set here is not duplication |
| `tests/wait_test.rs:56-65` | The set, in discriminant order |
| `tests/wait_test.rs:69-84` | `pause` and `is_non_blocking` agree — the two encodings, checked against each other |
| `tests/wait_test.rs:102-108` | Exactly one variant is non-blocking |
