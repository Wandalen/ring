# Pattern: The Named Ordering Constant

### Scope

- **Purpose**: Record the family's convention of binding each memory ordering to a named `const` documented at its declaration, and place this crate's `PUBLISH` among the eleven that exist.
- **Responsibility**: State the pattern, give the full census, show that four of the seven names are used by two crates with no dependency edge between them, and record what nothing checks.
- **In Scope**: `const PUBLISH` (`src/lib.rs:60-67`) and the imported `GATING`.
- **Out of Scope**: Why `Release`/`Acquire` rather than `SeqCst` — the ordering choice itself, covered at the declaration and in [`lifecycle/001`](../lifecycle/001_a_slot_from_claim_to_visibility.md).

### The Pattern

1. Every memory ordering used by a crate is bound to a **named `const`** at
   module level, never written inline at a call site.
2. The constant's name states the **role** — what the operation is for — not the
   ordering's own name.
3. The doc comment names the **pairing partner**, states what weakening it would
   produce, and on which architecture the weakening would be invisible.

This crate's instance, `src/lib.rs:60-67`:

```rust
/// The ordering a publication is made visible at.
///
/// `Release`, paired with the consumer's `Acquire` read of the same cursor:
/// that pairing is the entire happens-before edge between a producer's slot
/// writes and a consumer's reads of them. Weakening it to `Relaxed` produces a
/// ring that works on x86, where the hardware supplies the ordering the code
/// failed to ask for, and races on aarch64.
const PUBLISH : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
```

Seven lines of documentation for one line of code, and all three parts are
present: role (`PUBLISH`, not `RELEASE`), partner (the consumer's `Acquire`
read), and the failure mode with its architecture (`Relaxed` → works on x86,
races on aarch64).

The other half of the pair is not declared here — it is imported.
`src/lib.rs:57` takes `GATING` from `ring_cursor`, where it is `pub const GATING
: Ordering = Ordering::Acquire` (`ring_cursor/src/lib.rs:89`). So a single
compare-exchange in `try_publish` carries one locally-named ordering and one
imported one:

```rust
self.cursor.compare_exchange( start, end, PUBLISH, GATING ).map( | _ | end )
```

The asymmetry is deliberate and `tests/manual/readme.md § P3` explains it:

> A failed exchange published nothing, so it needs no release — but it did read
> the cursor, and the value it returns is what the caller retries against.

### The Local Check

`tests/manual/readme.md § P3` is what keeps the pattern from decaying into inline
orderings:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -E "Ordering::|GATING|PUBLISH"
```

Live output:

```
use ring_cursor::{ PaddedCursor, SeqCell, GATING };
const PUBLISH : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
    self.cursor.load( GATING )
    self.cursor.compare_exchange( start, end, PUBLISH, GATING ).map( | _ | end )
```

**Expected: four lines** — the `use` importing `GATING`, the `const PUBLISH`
binding, `GATING` in `published()`'s load, and the exchange taking both. Run
today it produces exactly those four. The plan states the reason at `:89-91`:

> An ordering chosen at the point of use is an ordering nobody will find when
> they go looking for why the ring races.

The check is cheap and structural: it does not verify the ordering is *correct*,
only that there is exactly one place to look for it. P1's loom mutation is the
behavioural counterpart — weaken `PUBLISH` to `Relaxed` and the model fails —
and `:97` records that this one is free by comparison.

### PB34 — Eleven Constants, Seven Names, and Four Names Shared Across Crates With No Edge Between Them

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rE '^\s*(pub )?const [A-Z_]+ *: *(core::sync::atomic::)?Ordering' */src/*.rs
```

Live output:

```
ring_claim/src/lib.rs:const CLAIM_SUCCESS : core::sync::atomic::Ordering = core::sync::atomic::Ordering::AcqRel;
ring_consume/src/lib.rs:const COMMIT : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
ring_cursor/src/lib.rs:pub const GATING : Ordering = Ordering::Acquire;
ring_debug/src/lib.rs:const OBSERVE : Ordering = Ordering::Acquire;
ring_mpsc/src/lib.rs:pub const PUBLISH : Ordering = Ordering::Release;
ring_mpsc/src/lib.rs:pub const OBSERVE : Ordering = Ordering::Acquire;
ring_mpsc/src/lib.rs:pub const COMMIT : Ordering = Ordering::Release;
ring_mpsc/src/lib.rs:pub const OWN : Ordering = Ordering::Relaxed;
ring_publish/src/lib.rs:const PUBLISH : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
ring_spsc/src/lib.rs:pub const OWN : Ordering = Ordering::Relaxed;
ring_spsc/src/lib.rs:pub const HANDOFF : Ordering = Ordering::Release;
```

Eleven named ordering constants exist family-wide, in seven crates:

| Name | Crate | Value | Visibility | Line |
|------|-------|-------|:----------:|-----:|
| `GATING` | `ring_cursor` | `Acquire` | `pub` | `87` |
| `CLAIM_SUCCESS` | `ring_claim` | `AcqRel` | private | `76` |
| **`PUBLISH`** | **`ring_publish`** | **`Release`** | **private** | **`67`** |
| `PUBLISH` | `ring_mpsc` | `Release` | `pub` | `221` |
| `OBSERVE` | `ring_debug` | `Acquire` | private | `73` |
| `OBSERVE` | `ring_mpsc` | `Acquire` | `pub` | `234` |
| `COMMIT` | `ring_consume` | `Release` | private | `67` |
| `COMMIT` | `ring_mpsc` | `Release` | `pub` | `255` |
| `OWN` | `ring_mpsc` | `Relaxed` | `pub` | `271` |
| `OWN` | `ring_spsc` | `Relaxed` | `pub` | `191` |
| `HANDOFF` | `ring_spsc` | `Release` | `pub` | `206` |

Four names appear twice — `PUBLISH`, `OBSERVE`, `COMMIT`, `OWN` — and **no
dependency edge connects either member of any pair**:

| Pair | Edge between them | Checked in |
|------|:-----------------:|------------|
| `ring_publish` ↔ `ring_mpsc` | **none** | `ring_mpsc/Cargo.toml:13-20` declares eight deps; `ring_publish` is not one |
| `ring_debug` ↔ `ring_mpsc` | **none** | `ring_debug/Cargo.toml` declares `ring_core`, `ring_cursor`, `ring_types`, `ring_atomic`, `ring_config` |
| `ring_consume` ↔ `ring_mpsc` | **none** | `ring_consume/Cargo.toml` declares `ring_types`, `ring_cursor`, `ring_barrier`, `ring_seqno` |
| `ring_spsc` ↔ `ring_mpsc` | **none** | `ring_spsc/Cargo.toml` declares `ring_store`, `ring_config`, `ring_cursor`, `ring_slot`, `ring_types` |

So the shared vocabulary is **convention held in prose, not in code**. Nothing
imports another crate's `PUBLISH`; nothing asserts the two are equal; nothing
would notice if one changed. Each pair agrees today because two authors reached
the same name for the same role and wrote the same value under it.

Three observations follow:

1. **`ring_mpsc` holds four of the eleven** and shares three of its four names
   with crates it does not depend on. It rebuilt the vocabulary along with the
   mechanism — the same pattern
   [`integration/002`](../integration/002_the_two_crates_that_declined.md) § PB6
   records for the constant's *text*, where `ring_mpsc:224-237` reproduces this
   crate's x86/aarch64 sentence verbatim and adds a doctest this crate lacks.
2. **Visibility splits on tier, not on role.** The four private constants —
   `ring_publish::PUBLISH`, `ring_consume::COMMIT`, `ring_claim::CLAIM_SUCCESS`
   and `ring_debug::OBSERVE` — belong to crates that apply their own ordering
   internally. The seven public ones belong to crates whose callers construct
   atomic operations themselves and therefore need the vocabulary. `ring_cursor
   ::GATING` is public for the same reason and is the one constant this crate
   imports rather than declares.
3. **The name is the load-bearing part.** `PUBLISH` and `COMMIT` are both
   `Release`; `GATING`, `OBSERVE` are both `Acquire`; `OWN` is `Relaxed` twice.
   The eleven constants carry only four distinct ordering values, so the seven
   role names carry all the information a reader gets — which is the pattern's
   entire point, and also why nothing mechanical can check it.

### What This Pattern Does Not Buy

| Claim it might look like | Actually |
|--------------------------|----------|
| The orderings are correct | unchecked by the pattern; checked by loom, and only for this crate's own pair |
| The two `PUBLISH` constants agree | true today, guaranteed by nothing |
| An ordering cannot be written inline | only within `ring_publish`, and only because `§ P3` is run by hand |
| The pairing documented is the pairing in effect | the doc comment names the partner in prose; no type, test or lint connects them |

The last row is the sharpest. `PUBLISH` says it is *"paired with the consumer's
`Acquire` read of the same cursor"* — and the consumer's read is
`ring_barrier`'s, reached through `Barrier::over( publisher.cursor() )` in a test
file, in a crate this one does not depend on. The pairing is real, and the only
thing that establishes it is
`tests/handshake_test.rs:77-165`'s loom model, which exercises both halves under
a memory model weak enough to break them
([`workaround/001`](../workaround/001_the_loom_seam_and_its_only_user.md)).

### PB52 — No Compare-Exchange in the Family Takes Both Orderings From One Place

```sh
cd "$(git rev-parse --show-toplevel)"
# the ordering arguments at every genuine call site
grep -r 'compare_exchange(' --include=*.rs ring_*/src/ \
  | grep -vE 'fn compare_exchange' | grep -vE ':\s*///'
# where each of those names is declared
grep -r '^const \(PUBLISH\|CLAIM_SUCCESS\)' \
  ring_publish/src/lib.rs ring_claim/src/lib.rs
grep 'pub const GATING' ring_cursor/src/lib.rs
```

Live output:

```
ring_atomic/src/lib.rs:      .compare_exchange( current.0, new.0, success, failure )
ring_atomic/src/lib.rs:    self.cell.compare_exchange( current, new, success, failure )
ring_claim/src/lib.rs:      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
ring_claim/src/lib.rs:      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
ring_cursor/src/lib.rs:    self.0.get().compare_exchange( current, new, success, failure )
ring_publish/src/lib.rs:    self.cursor.compare_exchange( start, end, PUBLISH, GATING ).map( | _ | end )
ring_publish/src/lib.rs:const PUBLISH : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
ring_claim/src/lib.rs:const CLAIM_SUCCESS : core::sync::atomic::Ordering = core::sync::atomic::Ordering::AcqRel;
pub const GATING : Ordering = Ordering::Acquire;
```

Three genuine call sites, and all three read the same way: a crate-local
constant for the success ordering, and `GATING` — `ring_cursor`'s, imported —
for the failure ordering. `ring_publish` passes `( PUBLISH, GATING )` with
`PUBLISH` declared in its own source; `ring_claim` passes
`( CLAIM_SUCCESS, GATING )` twice, with `CLAIM_SUCCESS` declared in its own.
The remaining hits are the forwarding implementations, which take both
orderings as parameters and name neither.

So the family has no site where both orderings come from the same place. The
split is not arbitrary — it tracks who owns which guarantee. The success
ordering makes *this crate's* writes visible, so it is this crate's word; the
failure ordering re-reads a cursor whose visibility rules belong to
`ring_cursor`, so it is `ring_cursor`'s. Two crates, two constants, one call.

It is also why the shared-vocabulary problem this instance records has a floor.
Whatever drifts among the eleven named constants, `GATING` cannot: it has
exactly one declaration, and every failure path in the family imports that one
rather than restating it. The names that duplicate are the success halves —
each crate's own, each written independently — and those are precisely the ones
nothing compares.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_padded_cursor_and_nothing_else.md](../data_structure/001_one_padded_cursor_and_nothing_else.md) | The cursor both constants are applied to |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_two_crates_that_declined.md](../integration/002_the_two_crates_that_declined.md) | The verbatim reproduction of this constant's own rationale |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_frontier_moves_only_by_compare_exchange.md](../invariant/001_the_frontier_moves_only_by_compare_exchange.md) | The one operation both constants are passed to |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_slot_from_claim_to_visibility.md](../lifecycle/001_a_slot_from_claim_to_visibility.md) | The transition the `Release`/`Acquire` edge orders |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_what_a_publication_costs.md](../non_functional_requirement/001_what_a_publication_costs.md) | Why `Release` rather than `SeqCst` costs nothing on x86 and aarch64 |

### Patterns

| File | Relationship |
|------|--------------|
| [001_try_and_loop_over_compare_exchange.md](001_try_and_loop_over_compare_exchange.md) | The other pattern the same exchange instantiates |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_loom_seam_and_its_only_user.md](../workaround/001_the_loom_seam_and_its_only_user.md) | The only thing that checks the pairing rather than documenting it |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:57,60-67` | The imported half and the declared half |
| `ring_cursor/src/lib.rs:89` | `GATING`, the only ordering constant this crate imports |
| `ring_mpsc/src/lib.rs:224-287` | Four constants, three names shared with crates it does not depend on |
| `ring_claim/src/lib.rs:76` | `CLAIM_SUCCESS` — `AcqRel`, the family's only one |
| `ring_spsc/src/lib.rs:198-213` | `OWN` and `HANDOFF`, the single-producer vocabulary |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md § P3` | Four lines, no inline `Ordering::` anywhere |
| `tests/manual/readme.md § P1` | The mutation that weakens `PUBLISH`, and the loom failure it produces |
| `tests/handshake_test.rs:77-165` | The pairing exercised under a memory model that can break it |
