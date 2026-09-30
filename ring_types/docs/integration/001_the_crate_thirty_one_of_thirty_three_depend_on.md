# Integration: The Crate Thirty-One of Thirty-Three Depend On

### Scope

- **Purpose**: Map this crate's position in the family — a leaf with no dependencies that 30 of the other 32 `ring_*` crates declare — and record which of its six exported names actually travels how far.
- **Responsibility**: Describe the system, name the integration points, and state the compatibility requirements.
- **In Scope**: The declaring set; per-type consumer counts; the acyclicity the leaf position buys.
- **Out of Scope**: `ring_registry`, the one whose absence has a story (→ [`integration/002`](002_the_registry_that_declined_the_shared_error.md)); what each type means (→ [`type/`](../type/)).

### System Description

**`ring_types` is tier 0 of this 33-crate family, and tier 0 has
exactly one member.** It declares no dependencies at all — not `error_tools`,
not `thiserror`, not another workspace crate:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[dependencies\]/,/^\[lints\]/p' ring_types/Cargo.toml
```

Live output:

```
[dependencies]


[lints]
```

**Thirty of the other thirty-two crates declare it.** The count and the
membership are both mechanical:

```sh
cd "$(git rev-parse --show-toplevel)"
for m in ring_*/Cargo.toml; do
  command grep -q '^ring_types' "$m" && basename "$( dirname "$m" )"
done | wc -l
```

Live output:

```
30
```

The three names absent from that list are `ring_types` (which cannot depend
on itself), `ring_registry` — the subject of
[`integration/002`](002_the_registry_that_declined_the_shared_error.md) — and
`ring_align`, which used to declare it.

**Correction (2026-09-28):** this section previously counted thirty-one
declaring crates and named only `ring_types` and `ring_registry` as absent.
`ring_align/Cargo.toml` dropped its `ring_types = { path = "../ring_types" }`
line in commit `ce60ae6e8` ("Remove unused dependencies from Cargo.toml
files") — the manifest's `[dependencies]` table is now empty and contains no
mention of `ring_types` at all, not even in a comment. The count is thirty,
the absent set is three, and this file's own title is left as its stable
name rather than rewritten every time a crate's manifest changes; the current
membership is whatever the recipe above prints, not what the heading says.

**The empty dependency list is the load-bearing property, not the type
definitions.** A family of 33 crates has a dependency forest that must stay
acyclic, and the cheapest way to guarantee it is a tier that cannot participate
in a cycle: a crate with no outgoing edges is in no cycle, by construction, and
every edge pointing at it is therefore safe to add without checking. That is
what the `lib.rs` header means by "acyclic by construction" — the forest is not
audited for cycles, it is shaped so the audit is unnecessary.

**It is also what forbids `error_tools` here**, in a workspace whose convention
is to use it. `RingError` is hand-written with a `Display` impl and a
`core::error::Error` impl because taking the dependency would give tier 0 an
outgoing edge and cost the property above. The cost is 20 lines of `match` arms
(`src/error.rs:162–182`), paid once.

### Integration Points

**Six exported names, and they do not travel equally.** Each row is a distinct
integration point with its own consumer set:

| Export | Kind | Consuming crates | What the consumer does with it |
|--------|------|-----------------:|-------------------------------|
| [`RingError`](../item/enum/002_ring_error.md) | Enum | 19 | Returns it, matches on it, classifies it |
| [`Seq`](../item/struct/002_seq.md) | Struct | 18 | Stores a position, compares two, advances one |
| [`Capacity`](../item/struct/001_capacity.md) | Struct | 17 | Sizes storage, derives a mask |
| [`OverflowPolicy`](../item/enum/003_overflow_policy.md) | Enum | 10 | Dispatches a full-ring decision |
| [`WaitKind`](../item/enum/001_wait_kind.md) | Enum | 7 | Selects a wait strategy, or refuses one |
| [`SlotIndex`](../item/struct/003_slot_index.md) | Struct | **3** | Indexes storage |

```sh
cd "$(git rev-parse --show-toplevel)"
for t in RingError Seq Capacity OverflowPolicy WaitKind SlotIndex; do
  n=$( command grep -rlE "\b$t\b" ring_*/src 2>/dev/null \
       | command grep -v '^ring_types/' | cut -d/ -f1 | sort -u | wc -l )
  printf '%-16s %s\n' "$t" "$n"
done
```

Live output:

```
RingError        19
Seq              18
Capacity         17
OverflowPolicy   10
WaitKind         7
SlotIndex        3
```

**`SlotIndex`'s three consumers are the finding in this table.** It reaches
`ring_batch`, `ring_store` and `ring_index` and nothing else, while `Seq` — the
type it is derived from — reaches eighteen. The asymmetry is not a defect: `Seq`
is what the family *reasons* about and `SlotIndex` is what one crate *folds* it
into, so the derived type stays near the fold. But it means the split those two
types exist to enforce is enforced at three call sites, not thirty
(→ [`data_structure/001`](../data_structure/001_two_position_types_and_the_fold_between_them.md)).

**Five names are re-exported from `lib.rs`; four modules are private.** Every
type lives in a private module and reaches consumers only through the crate
root, so `ring_types::error::RingError` is not a path any consumer can write:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep '^mod \|^pub use ' ring_types/src/lib.rs
```

Live output:

```
mod capacity;
mod error;
mod id;
mod policy;
pub use capacity::Capacity;
pub use error::RingError;
pub use id::{Seq, SlotIndex};
pub use policy::{OverflowPolicy, WaitKind};
```

That is one path per name, which is what lets the module layout be rearranged
without a consumer edit.

**None of the six is on the family's export Contract.** The export Contract names
five crates — `ring_factory`, `ring_handle`, `ring_tls`, `ring_flush`,
`ring_types` — and `ring_types` is one of them, so an external consumer *can*
name these types. But the four internal-facing ones travel mostly within the
family; what an external consumer needs is `RingError` (to handle a failure) and
the two policy enums (to configure a ring through `RingConfig`).

### Error Handling

**This crate cannot fail at integration time, and that is a design property
rather than an accident.** It has no runtime, no initialization, and no state:
every item is `const` or a plain data declaration. The only fallible operation
it exposes is
[`Capacity::new`](../item/associated_function/001_capacity_new.md), which
returns `Result< Capacity, RingError >` and whose two failure modes are both
configuration mistakes detected before any ring exists.

**The error it defines is the family's, not its own.** Nine variants, and only
two are constructed inside this crate:

| Variant | Constructed in this crate | Constructed elsewhere in the family |
|---------|---------------------------|-------------------------------------|
| `CapacityZero` | `src/capacity.rs:44` | 0 |
| `CapacityNotPowerOfTwo` | `src/capacity.rs:48` | 0 |
| `Full` | — | 36 |
| `Empty` | — | 16 |
| `Closed` | — | 5 |
| `BatchTooLarge` | — | 10 |
| `PolicyUnsupported` | — | 5 |
| `NameTaken` | — | **0** |
| `NameUnknown` | — | **0** |

```sh
cd "$(git rev-parse --show-toplevel)"
for v in CapacityZero CapacityNotPowerOfTwo Full Empty Closed \
         NameTaken NameUnknown BatchTooLarge PolicyUnsupported; do
  n=$( command grep -rn "RingError::$v" ring_*/src 2>/dev/null \
       | command grep -vc '^ring_types/' )
  printf '%-24s %s\n' "$v" "$n"
done
```

Live output:

```
CapacityZero             0
CapacityNotPowerOfTwo    0
Full                     36
Empty                    16
Closed                   5
NameTaken                0
NameUnknown              0
BatchTooLarge            10
PolicyUnsupported        5
```

**The two zeros are a real gap and are documented as one**
(→ [`pitfall/002`](../pitfall/002_two_name_errors_nothing_constructs.md)). The
two capacity variants reading zero is not — they are constructed at the single
validation site, which is in this crate, so an external count of zero is exactly
right.

### Compatibility Requirements

| # | Requirement | Why it binds | Currently |
|---|-------------|--------------|-----------|
| C1 | The dependency list stays empty | The acyclicity guarantee is "tier 0 has no outgoing edges", not "no cycle was found" | ✅ Held |
| C2 | `RingError` stays `Copy` and allocation-free | An error returned from the tick path must not allocate | ✅ Held; asserted by `error_is_copy` |
| C3 | `RingError` stays `#[ non_exhaustive ]` | Nine variants today, and the family is still adding backends | ✅ Held; `src/error.rs:43` |
| C4 | `Capacity`'s only constructor stays validating | 17 crates take the mask's validity on trust (→ [`invariant/001`](../invariant/001_every_capacity_has_a_valid_mask.md)) | ✅ Held; the field is private |
| C5 | `Seq` stays 64 bits | The non-wrapping guarantee is a width argument, not an arithmetic one | ✅ Held — the doc used to state the wrong reason and has since been corrected (→ [`pitfall/001`](../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md)) |
| C6 | Every export keeps one path through the crate root | Private modules; rearranging them must not be a breaking change | ✅ Held; `src/lib.rs:41–44` |
| C7 | No behaviour that dispatches on a policy lands here | The discriminant/handler split — discriminants here, handlers in `ring_wait`/`ring_overflow` | ✅ Held (→ [`pattern/001`](../pattern/001_discriminants_here_handlers_elsewhere.md)) |

**C1 and C7 are the two that a well-meaning change breaks.** C1 falls to any
edit that reaches for a helper crate — `error_tools` is the workspace
convention, and taking it here would be locally reasonable and globally
expensive. C7 falls to any edit that adds a method answering "what should
happen under this policy", which is a natural place to put it and the wrong
crate for it.

**C5 is the one whose stated justification used to be wrong.** The requirement
held and the reason in the source did not; a reader who audited C5 against
`src/id.rs:34–38` would have found a false claim and could have concluded the
requirement was softer than it is. The source has since been corrected to state
the width argument accurately.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | The six names as a surface, rather than as a set of edges |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_position_types_and_the_fold_between_them.md](../data_structure/001_two_position_types_and_the_fold_between_them.md) | `SlotIndex`'s three consumers, and why the number is not a defect |
| [../data_structure/002_the_error_enum_as_a_closed_copy_set.md](../data_structure/002_the_error_enum_as_a_closed_copy_set.md) | C2 and C3, as a layout rather than as a requirement |

### Integrations

| File | Relationship |
|------|--------------|
| [002_the_registry_that_declined_the_shared_error.md](002_the_registry_that_declined_the_shared_error.md) | The thirty-second crate — the one absent from the declaring set |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_every_capacity_has_a_valid_mask.md](../invariant/001_every_capacity_has_a_valid_mask.md) | C4 |
| [../invariant/002_tier_zero_depends_on_nothing.md](../invariant/002_tier_zero_depends_on_nothing.md) | C1, stated as an invariant with its enforcement mechanism |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_discriminants_here_handlers_elsewhere.md](../pattern/001_discriminants_here_handlers_elsewhere.md) | C7 |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md](../pitfall/001_seq_next_wraps_where_its_doc_says_it_saturates.md) | C5's stated reason |
| [../pitfall/002_two_name_errors_nothing_constructs.md](../pitfall/002_two_name_errors_nothing_constructs.md) | The two zeros in the constructor table |

### Sources

| File | Relationship |
|------|--------------|
| [`Cargo.toml`](../../Cargo.toml) | The empty `[dependencies]` C1 protects |
| [`src/lib.rs`](../../src/lib.rs) | Lines 36–44 — four private modules, four `pub use` lines |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ C2 asserted by `error_is_copy`; C4 by `capacity_rejects_zero_and_non_powers_of_two`. ❌ **C1, C3, C6 and C7 are not asserted by anything** — they are manifest and module-layout properties, and the suite has no way to see a dependency being added. The gate that could is `bench_harness`'s G5, which reads the export surface rather than the dependency list |

### TY29 — Thirty-One Crates Declare the Dependency and Thirty Use It

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_*/; do c=${c%/}; [ "$c" = ring_types ] && continue
  grep -q 'ring_types' $c/Cargo.toml 2>/dev/null || continue
  grep -qE '\b(Capacity|RingError|Seq|SlotIndex|OverflowPolicy|WaitKind|ring_types)\b' \
    $c/src/*.rs 2>/dev/null || echo "declared, unused in src: $c"
done
```

Live output:

```
declared, unused in src: ring_registry
```

One line of output, and it names the crate whose reason for the dependency was
an error type it went on to declare for itself
(→ [`../decisions/002`](../decisions/002_the_two_name_errors_and_the_two_crates_that_redeclared_them.md), TY8).

### TY30 — `RingError::Full` Is Constructed in Eight Crates and Four Variants in None

```sh
cd "$(git rev-parse --show-toplevel)"
for v in Full BatchTooLarge Empty Closed PolicyUnsupported CapacityZero \
         CapacityNotPowerOfTwo NameTaken NameUnknown; do
  printf '%-22s %d\n' "$v" \
    "$( grep -rn "RingError::$v" ring_*/src/*.rs | grep -v '^ring_types/' \
        | grep -vE ':[0-9]+: *(//|///|//!)' | cut -d/ -f1 | sort -u | wc -l )"
done
```

Live output:

```
Full                   8
BatchTooLarge          4
Empty                  4
Closed                 1
PolicyUnsupported      1
CapacityZero           0
CapacityNotPowerOfTwo  0
NameTaken              0
NameUnknown            0
```

The two capacity variants are constructed only inside `ring_types` — correctly,
since `Capacity::new` is the only validator — and the two name variants nowhere.
So of nine variants, five carry the family's traffic and four are declared for
crates that never construct them.

### TY31 — The Crate Is Depended On More Widely Than Any Single Name Is Used

Thirty declared dependents; `RingError`, the widest name, is in eighteen.
The difference is not idle dependencies — it is that most crates take one or two
of the six names. That is the shape a vocabulary crate should have, and it is
worth a number because the alternative reading — that thirty crates depend on
all of it — is what makes a tier-0 change look more expensive than it is.
