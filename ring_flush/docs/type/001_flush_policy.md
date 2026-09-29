# Type: Flush Policy

### Scope

- **Purpose**: Define the exported three-variant value that is this crate's entire public reason to exist, and fix the validation rules that keep an invalid configuration from degrading into a different policy at runtime.
- **Responsibility**: Give the definition, the trait obligations, and every validation rule with its failure mode.
- **In Scope**: The variants; their traits; construction-time validation.
- **Out of Scope**: How a policy is evaluated (→ [Evaluating a Policy at an Append](../algorithm/001_evaluating_a_policy_at_an_append.md)); what a drive reports (→ [Flush Outcome](002_flush_outcome.md)).

### Definition

Three variants, one carrying a parameter:

```rust
pub enum FlushPolicy
{
  /// Flush when the buffer cannot accept the record being appended.
  OnFull,
  /// Flush when the driver is told the stage barrier has been reached.
  OnBarrier,
  /// Flush when `n` records have accumulated since the last flush.
  OnBatch( usize ),
}
```

**The asymmetry is load-bearing.** `OnFull` and `OnBarrier` name conditions
determined elsewhere — by the buffer's capacity and by the consumer's schedule.
`OnBatch( n )` names a condition this crate owns outright, which is why it is
the only variant with a parameter, the only one with state
(→ [Policy Arming and Firing](../lifecycle/004_policy_arming_and_firing.md)),
and the only one that can be misconfigured
(→ [Two Batch Sizes That Must Not Diverge](../pitfall/002_two_batch_sizes_that_must_not_diverge.md)).

**Trait obligations:**

| Trait | Status | Why |
|-------|--------|-----|
| `Copy`, `Clone` | **Required** | A policy is consulted per append; passing it by value must cost nothing |
| `Debug` | **Required** | It appears in benchmark output and in the flush log's cause field |
| `PartialEq`, `Eq` | **Required** | The test asserts which policy was in effect |
| `Send`, `Sync` | **Automatic** | It is a plain enum; no interior mutability |
| `Default` | **Withheld** | There is no defensible default — that is the whole point (→ [`pattern/001`](../pattern/001_policy_as_a_value.md)'s applicability, last row) |
| `Hash` | Withheld | No use; `OnBatch`'s parameter would make it a poor key anyway |
| `serde` derives | **Open** | Needed if benchmark configurations are files rather than code. Not yet decided (→ [`decisions/`](../decisions/readme.md)) |

**`Default` is withheld deliberately and it is the most likely trait to be
added by mistake.** A default policy is a policy nobody chose, applied wherever
someone wrote `..Default::default()` — which recreates, in one derive, exactly
the "publication point nobody designed" state this crate exists to prevent. It
would also make [`pitfall/002`](../pitfall/002_two_batch_sizes_that_must_not_diverge.md)'s
G5 the normal case.

### Validation

Validation happens at construction, and every rule fails loudly rather than
adjusting the value.

| # | Rule | Applies to | Failure if unvalidated |
|---|------|-----------|------------------------|
| N1 | `n >= 1` | `OnBatch` | `OnBatch( 0 )` fires on every append — silently `OnEveryRecord`, a fourth policy |
| N2 | `n <=` buffer capacity in records | `OnBatch` | Can never fire; the buffer fills and behaves as `OnFull` (→ [trigger exclusivity](../invariant/001_a_policy_fires_only_at_its_trigger.md)'s V2) |
| N3 | A buffer has exactly one policy | all | Two policies on one buffer means call-site order decides — [the publication-point invariant](../invariant/002_publication_point_is_designed_not_inherited.md)'s C3 |
| N4 | `OnBarrier` requires a driver that announces barriers | `OnBarrier` | **Not statically checkable.** A policy configured `OnBarrier` and driven without the announcement never fires (→ [the pitfall](../pitfall/001_on_barrier_cannot_see_the_barrier.md)'s F1) |

**N1 and N2 are checkable and must be checked at construction.** Both turn a
configuration mistake into a loud failure instead of a silent policy
substitution, which is the single most valuable thing validation does here —
every unvalidated case in this table degrades into *a different, working
policy*, and a working program producing a benchmark verdict about the wrong
policy is worse than a program that refuses to start.

**N2 requires knowing the buffer's capacity**, so the policy cannot be
validated in isolation — it is validated when bound to a buffer, not when the
enum value is created. That means `FlushPolicy` itself is always constructible
and the validation lives at the binding site, which is a real weakness: a
`FlushPolicy` value in hand carries no guarantee it is valid for the buffer it
is about to be used with.

**N4 cannot be enforced by this crate at all,** and it is the one that matters
most. It is recorded here rather than omitted precisely because an unenforceable
rule that nobody wrote down is indistinguishable from no rule.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_policy_enum.md](../data_structure/001_the_policy_enum.md) | The same value examined as a structure — layout, size, and cost of consultation |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_a_policy_fires_only_at_its_trigger.md](../invariant/001_a_policy_fires_only_at_its_trigger.md) | Its V2 is what N1 and N2 prevent |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_policy_as_a_value.md](../pattern/001_policy_as_a_value.md) | Why this is an enum rather than a trait, and what that costs |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_two_batch_sizes_that_must_not_diverge.md](../pitfall/002_two_batch_sizes_that_must_not_diverge.md) | N2's neighbour — the claim-width coupling validation cannot see |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_policy_arming_and_firing.md](../lifecycle/004_policy_arming_and_firing.md) | Why only `OnBatch` carries state |

### Types

| File | Relationship |
|------|--------------|
| [002_flush_outcome.md](002_flush_outcome.md) | The companion exported type; together they are this crate's public vocabulary |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_types/readme.md`](../../../ring_types/readme.md) | Owns the family's shared discriminants; this crate's policy enum is local, not shared |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | `OnFull`'s pair — `on_full_fires_when_the_buffer_is_full` is M1 (fires at capacity, cause `Full`, nothing logged below it) and `on_full_ignores_barriers_and_counts` is M2 (seven announced barriers and seven appends below capacity produce zero entries). Together they are the only variant whose trigger this crate can both cause and observe |
| `tests/flush_test.rs` | N1 and N2 — `an_unusable_batch_size_is_refused_at_binding`, with `a_batch_size_equal_to_capacity_is_accepted` pinning N2's boundary as inclusive. N3 needs no test: `Flusher::new` takes the buffer by value, so ownership enforces it at compile time. N4 remains **untested and untestable here**, as this row said — a driver that never announces is indistinguishable from one whose barrier never arrives |

### FL45 — Six Types Carry One Derive Line, and the Table Reads It as Five Rulings About This Type

Every derive in the crate, and the four justifications the table gives for one of them:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every derive and what it decorates --'
awk '/^#\[ derive/{ d = $0; next } d && /^pub (enum|struct)/ { printf "    %-16s %s\n", $3, d; d = "" }' \
  ring_flush/src/lib.rs
echo '  -- what this instance says each trait is there for --'
awk '/^### FL/{ exit } /^\| `?(Copy|Debug|PartialEq|Hash|Default)/{ printf "    %s\n", $0 }' \
  ring_flush/docs/type/001_flush_policy.md | sed -E 's/^(.{0,124}).*/\1/'
```

Live output:

```
  -- every derive and what it decorates --
    FlushPolicy      #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
    FlushCause       #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
    FlushOutcome     #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
    ConfigError      #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
    FlushEntry       #[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
    FlushLog         #[ derive( Debug, Clone, Default, PartialEq, Eq ) ]
    Flusher<         #[ derive( Debug ) ]
  -- what this instance says each trait is there for --
    | `Copy`, `Clone` | **Required** | A policy is consulted per append; passing it by value must cost nothing |
    | `Debug` | **Required** | It appears in benchmark output and in the flush log's cause field |
    | `PartialEq`, `Eq` | **Required** | The test asserts which policy was in effect |
    | `Default` | **Withheld** | There is no defensible default — that is the whole point (→ [`pattern/001`](../pattern/001_
    | `Hash` | Withheld | No use; `OnBatch`'s parameter would make it a poor key anyway |
```

The trait table gives four different reasons for four different traits, each
argued from something specific to `FlushPolicy`: `Copy` because a policy is
consulted per append, `Debug` because it appears in benchmark output and the
log's cause field, `PartialEq`/`Eq` because a test asserts which policy was in
effect, `Hash` withheld because `OnBatch`'s parameter would make a poor key.

**The same five derives appear on all four exported enums and on `FlushEntry`.**
`ConfigError` is not consulted per append. `FlushCause` is the field the `Debug`
argument is about, not a thing appearing beside it. `FlushEntry` is a record, not
a value passed by copy. The derive line is a house style applied uniformly, which
is the right thing to do and is not five decisions about this type.

**The one type that departs from it proves the table's `Default` row overstates
its case.** `FlushLog` adds `Default`, and correctly — an empty log is the only
sensible starting state. The table says of `Default` that "there is no defensible
default — that is the whole point," phrased as though it were a property of the
crate rather than of the policy enum specifically. It is a property of the policy
enum specifically, and the structure that stores the policy's evidence derives the
trait one screen below.

Nothing here is wrong in the code. What is wrong is the direction of inference:
the table reads deliberate per-trait reasoning off a uniform line, and a reader
who removed `Copy` from `ConfigError` on the strength of "it is consulted per
append, and this is not" would be reasoning from a justification that never
applied to it.

### FL46 — The Trait Table Sends the Open `serde` Question to a Record That Explicitly Declines to Hold It

The pointer, its destination, and the dependency the question is about:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the row --'
awk '/^### FL/{ exit } /^\| `serde` derives \|/{ printf "    %s\n", $0 }' \
  ring_flush/docs/type/001_flush_policy.md | sed -E 's/^(.{0,132}).*/\1/'
echo '  -- what the destination says --'
command grep -E 'deliberately not recorded|not filed as' ring_flush/docs/decisions/readme.md \
  | sed -E 's/^(.{0,120}).*/    \1/'
echo '  -- and how much serde exists in the family --'
printf '    ring_* manifests naming serde: %s\n' "$( command grep -l 'serde' ring_*/Cargo.toml 2>/dev/null | wc -l )"
printf '    ring_* sources naming serde:   %s\n' "$( command grep -rl 'serde' ring_*/src 2>/dev/null | wc -l )"
printf '    ring_* tests naming serde:     %s\n' "$( command grep -rl 'serde' ring_*/tests 2>/dev/null | wc -l )"
```

Live output:

```
  -- the row --
    | `serde` derives | **Open** | Needed if benchmark configurations are files rather than code. Not yet decided (→ [`decisions/`](
  -- what the destination says --
    ### One question deliberately not recorded here
    (→ [`type/001`](../type/001_flush_policy.md)'s trait table). It is not filed as
  -- and how much serde exists in the family --
    ring_* manifests naming serde: 0
    ring_* sources naming serde:   0
    ring_* tests naming serde:     1
```

The table marks `serde` **Open** — "Not yet decided" — and points the reader at
the decisions record. That record has a section titled "One question deliberately
not recorded here," and `serde` is its subject. The reasoning there is sound: a
policy that serialises while the `RingConfig` containing it does not is useless,
so the real question is whether the family's configuration serialises, and it
belongs where that is answered.

**So the link resolves to a refusal rather than to a pending item**, and the two
documents describe the same question in incompatible statuses. "Open, not yet
decided" invites a reader to go and decide it; "deliberately not recorded here,
because it is not local" tells them the opposite. The correct status for the
table is neither — it is *out of scope for this crate*, which is a third thing
and is the one the decisions record actually establishes.

**No crate in the family depends on `serde`** — zero manifests, zero sources, and
the single test-tree mention is one line of prose in another crate's manual test
plan, quoting `cargo tree` output. That makes the disagreement cheap today and is exactly why it
has persisted: a
status field nobody acts on can hold two values for as long as nobody acts on it.
The row is worth correcting rather than deleting, because the underlying
observation is right and the crate that owns the answer should be findable from
here.
