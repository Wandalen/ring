# Data Structure: Three Values, Three Shapes

### Scope

- **Purpose**: Record the shape each of this crate's three value types takes — newtype, enum, struct — what each shape buys, and where the three stop agreeing with each other.
- **Responsibility**: Field visibility, construction routes, derived traits, and `Default` coverage across `Budget`, `Progress` and `Tick`.
- **In Scope**: The declarations themselves and what they permit a caller to build.
- **Out of Scope**: What the values mean (→ [`../type/001`](../type/001_budget_clamps_to_one.md), [`../type/002`](../type/002_progress_has_no_zero_made.md)); the accumulator's mutation hazard (→ [`002`](002_a_copy_accumulator.md)).

### Abstract

Three types, three shapes, one rule each. `Budget` is a newtype guarding an
invariant, `Progress` is an enum making one state unspellable through its
constructor, and `Tick` is a struct with two private fields. The shapes are
well chosen. What they do not do is agree on which traits a caller gets.

### The three declarations

| Type | Shape | Fields | Constructible directly? |
|---|---|---|---|
| `Budget` | tuple newtype over `usize` | one, private | no — the field is private |
| `Progress` | two-variant enum | `Made( usize )` payload, public | **yes** — `Progress::Made( 0 )` is spellable |
| `Tick` | struct | two, both private | no |

Two of the three close construction completely. The enum cannot: variants of a
public enum are public, so `Progress::of` is a recommended door rather than the
only one, which is why the crate's fourth compatibility guarantee is convention
strength rather than construction strength
([`../api/001`](../api/001_tick_path_surface.md)).

### What each shape is for

`Budget( usize )` exists so that "at least one attempt" is a property of the
type and not of every call site. `Budget::new( 0 )` returns `Self( 1 )`, and
because the field is private there is no second route to a zero. The invariant
survives `Copy`, survives `Clone`, and cannot be broken by a struct update
because there is no public field to update.

`Progress` is an enum rather than a `usize` so that "nothing moved" is a state
and not a magic number. `Made( usize )` carries the count in the variant that
has one, which is what makes `Progress::None` self-describing.

`Tick` is a struct because it holds two unrelated things — the budget it was
given, and what it has accumulated — and neither is derivable from the other.

### Where they stop agreeing

| Trait | `Budget` | `Progress` | `Tick` |
|---|---|---|---|
| `Debug Clone Copy` | ✓ | ✓ | ✓ |
| `PartialEq Eq` | ✓ | ✓ | — |
| `PartialOrd Ord Hash` | ✓ | — | — |
| `Default` | ✓ hand-written | — | ✓ hand-written |

Two hand-written `Default` impls and one type without. `Progress` is the type
with the most obvious default of the three — `Progress::None` is the identity
element of `then`, and the type already models "nothing" as a first-class state
→ PL9.

The `Default` impls that do exist both point at `Budget::once()`, one directly
and one through the other, so the crate has a single default policy expressed
twice → PL10.

### Evidence

| # | Claim | Test |
|---|---|---|
| D1 | `Budget::new( 0 )` yields one attempt | `a_budget_is_at_least_one_attempt` |
| D2 | `Budget::default()` is `Budget::once()`, and the order/hash derives behave | `a_budget_defaults_to_a_single_attempt` |
| D3 | `Tick::default()` matches `Tick::new( Budget::once() )` | `a_default_tick_is_a_single_attempt` |
| D4 | `Progress::of( 0 )` is `None`, not `Made( 0 )` | `progress_of_zero_is_no_progress` |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll
printf 'type declarations:        %s\n' "$( command grep -oE '^pub (struct|enum) [A-Za-z]+' src/lib.rs | sed 's/^pub //' | tr '\n' ' ' )"
printf 'private fields, Budget:   %s\n' "$( command grep -cE '^pub struct Budget\( usize \);' src/lib.rs || true )"
printf 'private fields, Tick:     %s\n' "$( awk '/^pub struct Tick/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -cE '^  [a-z_]+ :' || true )"
printf 'public enum variants:     %s\n' "$( awk '/^pub enum Progress/{f=1} f && /^}$/{exit} f' src/lib.rs | command grep -oE '^  [A-Z][A-Za-z]*' | tr -d ' ' | tr '\n' ' ' )"
printf 'Default impls:            %s\n' "$( command grep -oE '^impl Default for [A-Za-z]+' src/lib.rs | sed 's/^impl Default for //' | tr '\n' ' ' )"
printf 'types without Default:    %s\n' "$( for t in Budget Progress Tick; do command grep -q "^impl Default for $t\$" src/lib.rs || echo "$t"; done | tr '\n' ' ' )"
printf 'both defaults resolve to: %s\n' "$( awk '/^impl Default for/{f=1} f && /fn default/{g=1} g && /Self::/{ print; g=0; f=0 }' src/lib.rs | tr -d ' ' | tr '\n' ' ' )"
printf 'derive lines:             %s\n' "$( command grep -cE '^#\[ derive' src/lib.rs || true )"
printf 'Eq on Tick:               %s\n' "$( awk -v n1="$( command grep -m1 -F '#[ derive( Debug, Clone, Copy ) ]' src/lib.rs | cut -d: -f1 )" 'NR==n1' src/lib.rs | command grep -oE 'Eq' | wc -l )"
printf 'tests naming a Default:   %s\n' "$( command grep -oE '^fn [a-z_]*default[a-z_]*' tests/poll_test.rs | sed 's/^fn //' | tr '\n' ' ' )"
printf 'and what they assert:     %s\n' "$( command grep -oE '(Budget|Tick)::default\(\)[a-z.()]* *, *[A-Za-z:()]+' tests/poll_test.rs | tr '\n' '/' )"
```

Live output:

```
type declarations:        struct Budget enum Progress struct Tick 
private fields, Budget:   1
private fields, Tick:     3
public enum variants:     Made None 
Default impls:            Budget Tick 
types without Default:    Progress 
both defaults resolve to: Self::once() Self::new(Budget::once()) 
derive lines:             3
Eq on Tick:               0
tests naming a Default:   a_budget_defaults_to_a_single_attempt a_default_tick_is_a_single_attempt 
and what they assert:     Budget::default(), Budget::once()/Tick::default().budget(), Budget::once()/
```

### Data Structures

| File | Relationship |
|------|--------------|
| [002_a_copy_accumulator.md](002_a_copy_accumulator.md) | `Tick`'s second field, and what `Copy` does to it |

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_budget_clamps_to_one.md`](../type/001_budget_clamps_to_one.md) | The invariant the newtype shape exists to hold |
| [`../type/002_progress_has_no_zero_made.md`](../type/002_progress_has_no_zero_made.md) | The state the enum shape cannot quite make unspellable |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_tick_path_surface.md`](../api/001_tick_path_surface.md) | The trait table read as a surface question rather than a shape one |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | All three declarations and both `Default` impls |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | D1–D4 |

### PL9 — the type that models "nothing" as a state is the one without a `Default`

`Budget` and `Tick` each carry a hand-written `Default`. `Progress` does not,
and it is the only one of the three for which the right default is not a
judgement call: `Progress::None` means nothing moved, `Progress::of( 0 )`
produces it, and `then` treats it as the identity — `x.then( None )` is `x`.

The two types that got a `Default` both had to *choose* one. `Budget::default()`
is `once()`, which is a policy decision the doc comment defends. `Tick::default()`
is a tick on that budget, inheriting the same decision. Neither default is
forced by the type; both are opinions.

`Progress`'s default is forced by the type and is absent. The practical cost is
small — `Progress::None` is three characters longer than `Progress::default()` —
but it shows up wherever a generic bound wants `Default`, and it means a struct
holding a `Progress` field cannot itself derive `Default`. A scheduler
accumulating per-subsystem progress into a `#[ derive( Default ) ]` struct is
exactly the caller this crate is written for, and it is the caller that hits it.

### PL10 — one default policy, written down twice, checked in one direction

`impl Default for Budget` returns `Self::once()`. `impl Default for Tick`
returns `Self::new( Budget::once() )` — not `Self::new( Budget::default() )`.

Both are correct today and they are correct for the same reason, which is that
`Budget::default()` *is* `Budget::once()`. Written the way they are, that shared
reason is a coincidence the compiler does not know about. Changing
`Budget::default` to something else — a larger tick-path budget, say — leaves
`Tick::default` silently pinned to `once()`, and the type whose whole job is to
carry a budget would stop agreeing with the budget type's own default.

The two tests that cover it repeat the same shortcut.
`a_budget_defaults_to_a_single_attempt` asserts
`Budget::default() == Budget::once()`, and `a_default_tick_is_a_single_attempt`
asserts `Tick::default().budget() == Budget::once()` — again `once()`, not
`Budget::default()`. Its own doc comment says *"The default tick is a single
attempt, matching the default budget"*, so the sentence describes the delegation
and the assertion below it pins the literal. Both tests would keep passing after
`Budget::default` changed, and the tick's default would be the thing that
silently stopped matching.

Writing `Self::new( Budget::default() )` would make the delegation structural
and cost nothing. It is not applied here because it is a source change rather
than a documentation one, and the crate's defaults are settled — what is
recorded is that the link between them is by value rather than by reference.
