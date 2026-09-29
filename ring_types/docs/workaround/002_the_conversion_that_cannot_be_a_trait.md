# The Conversion That Cannot Be a Trait

### Scope

- **Purpose**: Record why `Capacity::new` is an inherent constructor rather than the `TryFrom< usize >` impl its signature otherwise describes, what the family gives up for that, and the toolchain change that would retire it.
- **Responsibility**: State the language constraint, the cost measured across the thirty dependents, and a deletion condition that fails at a toolchain bump rather than needing a judgement.
- **In Scope**: `Capacity::new`'s `const fn` + `Result` signature, and the absence of any `From`/`TryFrom` impl in the crate.
- **Out of Scope**: The validation the constructor performs — see [`algorithm/001`](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md); `NonZeroUsize` as an alternative encoding — see [`../decisions/readme.md`](../decisions/readme.md), P1.

### The Constraint

`Capacity::new` has exactly the shape the standard library reserves for
`TryFrom`: one argument, a `Result`, and an error type declared alongside it.

```rust
// ring_types/src/capacity.rs:40
pub const fn new( slots : usize ) -> Result< Self, RingError >
```

**It cannot be that impl and stay `const`.** Trait impls cannot be `const` on
stable Rust — `const_trait_impl` is unstable
([#67792](https://github.com/rust-lang/rust/issues/67792)) — so the choice is
between a `const fn` that consumers can call in a `const` item and a
`TryFrom` impl that composes with `?`, `try_into()`, and every generic bound
written against the standard trait. The crate took `const`.

### What the Crate Gives Up, Measured

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'From/TryFrom impls in ring_types:     '
grep -rhoE '^impl.*(From|TryFrom)' ring_types/src/*.rs | wc -l
printf 'try_into()/TryFrom uses family-wide:  '
grep -r 'try_into\|TryFrom' ring_*/src/*.rs | wc -l
```

Live output:

```
From/TryFrom impls in ring_types:     0
try_into()/TryFrom uses family-wide:  0
```

Zero and zero. The family has no `try_into()` call site because there is nothing
to call it on: every construction of every type in this crate names its
constructor explicitly. That is a cost paid by all thirty dependents and it
is invisible at each of them individually — nobody misses a conversion they
cannot write.

### Why This Is a Workaround and `Seq::ZERO` Is Not

[`readme.md`](readme.md) rules that a costless language limitation is not a
workaround, and files `Seq::ZERO` under design justification on exactly that
ground: the compensation reads better than what it replaces. This one goes the
other way.

| | `Seq::ZERO` | `Capacity::new` |
|--|-------------|-----------------|
| Language limitation | `Default::default()` is not `const`-callable | Trait impls cannot be `const` |
| Compensation | An associated constant | An inherent constructor |
| Compared to the unavailable form | **Better** — `Seq::ZERO` is clearer than `Seq::default()` at every call site | **Worse** — loses `?` composition, `try_into()`, and any `TryFrom` bound |
| Cost if the limitation lifted | None; nobody would migrate | The impl becomes writable and every construction site could shorten |

### Removal Trigger

`const_trait_impl` stabilises, **or** the crate decides `const`-constructibility
is not worth the trait. The second is a decision rather than a trigger and
belongs in [`../decisions/`](../decisions/readme.md) if anyone raises it; the
first is checkable:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types
rustc --version
```

Live output:

```
rustc 1.97.1 (8bab26f4f 2026-07-14)
```

Adding the impl without dropping `const` fails to compile on any toolchain that
prints a stable version above. That is the deletion condition: the day it
compiles, this instance is obsolete.

### The Constraint Is Not Only This Crate's

Every crate in the family that wraps a validated integer meets the same wall.
Nothing else in `ring_*/src` declares a `TryFrom` impl either, and the
reason is not that nobody wanted one — it is that tier 0 set the pattern in the
one crate everything else copies.

### TY17 — The Family Has No `try_into()` Call Site

`Capacity::new` cannot be `TryFrom< usize >` and remain `const fn`, so the
standard fallible-conversion vocabulary is unavailable for the type it fits
best. Across thirty dependents there are zero `try_into()` uses and zero
`TryFrom` bounds — a cost that is uniform, permanent under the current
toolchain, and invisible at every individual call site.

**Disposition:** declined — this instance's own Removal Trigger names the
fix: `const_trait_impl` stabilising on stable Rust
(`https://github.com/rust-lang/rust/issues/67792`), an upstream language
change outside this repository, not a doc or source edit available in a
corpus disposition pass.

### TY18 — The Crate Declares No Conversion Impl at All

No `From`, no `TryFrom`, on any of its four types. This is the same absence that
leaves the family's six error enums unable to compose
(→ [`../decisions/001`](../decisions/001_one_error_type_is_a_rule_the_family_does_not_keep.md),
TY3): the two are usually read as separate gaps, and both follow from tier 0
never having written a conversion for anything.

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_tier_zero_depends_on_nothing.md`](../invariant/002_tier_zero_depends_on_nothing.md) | The rule that rules out a derive-based escape, as it does for [`001`](001_hand_written_all_arrays_stand_in_for_variant_enumeration.md) |
