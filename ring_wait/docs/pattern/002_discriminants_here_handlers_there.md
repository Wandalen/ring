# Pattern: Discriminants Here, Handlers There

### Scope

- **Purpose**: Name the family's policy-enum pattern — the variants live in `ring_types`, the behaviour lives in a crate that may not be depended on — and measure how many handlers each enum actually has.
- **Responsibility**: State the pattern, census the three policy enums against it, and record where the one-handler assumption has already broken.
- **In Scope**: `WaitKind`, `OverflowPolicy`, and the crates that match on them.
- **Out of Scope**: Why the split was chosen for `WaitKind` — see [`decisions/002`](../decisions/002_the_discriminants_live_in_ring_types.md).

### The Shape

| Half | Lives in | Depended on by | Contains |
|------|----------|----------------|----------|
| the discriminants | `ring_types` | everything | the enum, `ALL`, cheap classifiers |
| the handlers | a leaf crate | only the crates that act | the `match` that does work |

The point of the split is that naming a policy must be cheaper than performing
it. A crate that stores a `WaitKind` in a config, logs one, or compares two needs
none of `std::thread`, and if the variants lived with the handlers it would drag
in `sleep` and `yield_now` to hold a one-byte value.

`ring_types` is Tier 0 and depends on nothing; `ring_wait` is Tier 4 and is
forbidden to three crates. The enum crosses that boundary and the handler does
not, which is the whole trick.

### The Census

```sh
cd "$(git rev-parse --show-toplevel)"

# every policy enum. `-n` is deliberately absent: a line number here would go
# stale on the next edit to `ring_types`, which is exactly what froze this block
command grep -r "^pub enum" ring_types/src/*.rs

# which src files actually name variants in code (not documentation). The
# printf is guarded by `if` rather than `&&`, so a final iteration with nothing
# to print leaves the loop's exit status at zero
for f in $( command grep -rl --include=*.rs "WaitKind::" /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ . /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/ | command grep "/src/" | sort ); do
  v=$( command grep -vE "^[[:space:]]*(//|///|//!)" "$f" | command grep -o "WaitKind::[A-Za-z]*" \
       | sort -u | tr '\n' ' ' )
  if [ -n "$v" ]; then printf '%-40s %s\n' "$f" "$v"; fi
done
```

Live output:

```
ring_types/src/error.rs:pub enum RingError
ring_types/src/policy.rs:pub enum WaitKind
ring_types/src/policy.rs:pub enum OverflowPolicy
ring_config/src/lib.rs              WaitKind::default 
ring_wait/src/lib.rs                WaitKind::None WaitKind::Park WaitKind::Spin WaitKind::Yield 
```

Run for both enums, the answer is:

| Enum | Declared | Variants | Crates naming **every** variant in code |
|------|----------|---------:|----------------------------------------|
| `WaitKind` | `policy.rs` | 4 | **1** — `ring_wait` |
| `OverflowPolicy` | `policy.rs` | 3 | **2** — `ring_overflow`, `ring_stats` |
| `RingError` | `error.rs` | 9 | not a policy — every crate constructs, none dispatches |

Everything else that appears to reference a variant is either documentation or a
single named value:

| File | What it actually names |
|------|------------------------|
| `ring_config/src/lib.rs` | `WaitKind::default`, `OverflowPolicy::default` — no variant at all |
| `ring_core/src/lib.rs` | `OverflowPolicy::DropOldest`, one variant, one branch |
| `ring_barrier`, `ring_claim`, `ring_shutdown` | `WaitKind::` in doc comments only |

The comment filter is doing real work in that table. Without it, six crates look
like they match on `WaitKind` and eleven like they match on `OverflowPolicy`.

### WT22 — The One-Handler Assumption Has Already Broken Once

`WaitKind` has exactly one handler and the pattern reads cleanly. `OverflowPolicy`
is ruled identically and has two:

| | `ring_overflow` | `ring_stats` |
|--|-----------------|--------------|
| Names | `DropNewest`, `DropOldest`, `Fail` | the same three, plus `ALL` |
| Does | performs the policy | counts events per policy |
| Is a leaf handler | yes | yes, of a different kind |

Neither is wrong. Counting *per policy* genuinely requires knowing the variants,
and a stats crate that could not name them could not bucket by them. But the
consequence is concrete: **adding a fourth `OverflowPolicy` variant requires
edits in two crates that neither compiler nor test connects**, and only
exhaustive-match errors in each will surface it — one at a time, in whatever
order the build happens to reach them.

`WaitKind` is one variant away from the same position. `ring_stats::record_wait`
(`ring_stats/src/lib.rs:312`) takes nanoseconds rather than a `WaitKind`,
so the second handler does not exist for waits **yet**; the moment anyone wants
per-strategy wait statistics, it will.

The mitigation the family already has is `ALL`:

```rust
// ring_types/src/policy.rs:58
pub const ALL : [ Self; 4 ] = [ Self::Spin, Self::Yield, Self::Park, Self::None ];

// ring_types/src/policy.rs:131
pub const ALL : [ Self; 3 ] = [ Self::DropNewest, Self::DropOldest, Self::Fail ];
```

A test that iterates `ALL` and asserts each variant is handled catches a missing
arm without needing to know where the handlers are. `ring_wait` uses it three
times ([`decisions/002`](../decisions/002_the_discriminants_live_in_ring_types.md)),
and `ring_stats` names it too — which is exactly why the second handler is
survivable rather than a trap.

### What the Pattern Costs

| Cost | Where it shows |
|------|----------------|
| A variant's behaviour is not next to its declaration | `WaitKind::Park`'s doc told readers it "blocks until a publisher signals"; it does not, and the explanation was 100 lines away in another crate ([`workaround/001`](../workaround/001_a_sleep_where_a_park_belongs.md)). The doc is now corrected, but only because someone read both crates — the distance is what let the two disagree for as long as they did |
| Two crates must agree on a classification | `is_non_blocking` in `ring_types` and `pause`'s `false` arm in `ring_wait` — asserted from both sides, `types_test.rs:160` and `wait_test.rs:102-108` |
| Adding a variant is an N-crate edit | N is 1 for `WaitKind` and 2 for `OverflowPolicy`, and nothing states N anywhere |

The middle row is the pattern working as designed: the duplication is
deliberate, it is checked from both ends, and neither check depends on the
other's crate. The first and third rows are the price.


### WT45 — The One Place the Enum Is Not Handled Variant by Variant

The split's discipline is that every variant gets its own arm in the handler. One
function in the handler crate breaks it, and that is why the ladder terminates.

```sh
cd "$(git rev-parse --show-toplevel)"
# arms per match over the enum
grep 'WaitKind::[A-Z][a-z]* =>\|WaitKind::[A-Z][a-z]* | WaitKind' ring_wait/src/lib.rs
```

Live output:

```
    WaitKind::Spin => Some( WaitKind::Yield ),
    WaitKind::Yield => Some( WaitKind::Park ),
    WaitKind::Park | WaitKind::None => None,
    WaitKind::Spin =>
    WaitKind::Yield =>
    WaitKind::Park =>
    WaitKind::None => false,
```

`pause` has four arms for four variants. `escalation_hint` has three: `Spin`,
`Yield`, and `Park | WaitKind::None` folded together.

The fold is deliberate and load-bearing. `Park` returns `None` because it is the
last rung; `WaitKind::None` returns `None` because it never escalates at all
(`none_never_escalates` pins it). Two different reasons producing the same answer,
and combining them is what makes `escalation_terminates_from_every_starting_point`
provable in a `while let` loop.

The cost is the one this pattern exists to prevent. Adding a fifth variant makes
`pause` fail to compile — the exhaustiveness check does its job — while
`escalation_hint` compiles unchanged, silently treating the new variant as
terminal, because a `|` arm absorbs whatever is added to it. So the crate's own
handler is non-exhaustive in exactly one place, and it is the place whose
correctness rests on which variants are terminal.

WT22 records the same failure mode arriving for the sibling enum by a different
route. This one is a single character of syntax inside a crate that otherwise
matches the pattern exactly.

### Patterns

| File | Relationship |
|------|--------------|
| [001_the_predicate_the_pause_and_the_budget.md](001_the_predicate_the_pause_and_the_budget.md) | The other pattern, about the loop rather than the enum |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_the_discriminants_live_in_ring_types.md](../decisions/002_the_discriminants_live_in_ring_types.md) | The decision this pattern generalises, with WT19 and WT22 |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | The dependency boundary the enum crosses and the handler does not |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_none_looks_exactly_once.md](../invariant/002_none_looks_exactly_once.md) | The classification both halves must agree on |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | The handler half, in full |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_a_sleep_where_a_park_belongs.md](../workaround/001_a_sleep_where_a_park_belongs.md) | What happens when the declaration's promise and the handler disagree |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/policy.rs:22,58,105,131` | Both policy enums and their `ALL` rosters |
| `ring_overflow/src/lib.rs` | `OverflowPolicy`'s first handler |
| `ring_stats/src/lib.rs:312,343-348` | Its second, and the wait counter that is not yet a third |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:56-65` | Exactly four discriminants exist |
| `tests/wait_test.rs:69-84` | Every discriminant has a handler that runs |
| `tests/wait_test.rs:102-108` | The classification, computed here and asserted in `ring_types` too |
| `ring_types/tests/types_test.rs:160` | The same classification from the declaring side |
