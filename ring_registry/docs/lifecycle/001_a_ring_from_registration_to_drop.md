# Lifecycle: A Ring From Registration to Drop

### Scope

- **Purpose**: Trace where a ring's ownership sits at each moment, from before registration to after the registry is gone.
- **Responsibility**: The stages, the transitions between them, and the two that transfer ownership rather than lend it.
- **In Scope**: One ring's passage through one registry; what drops it and when.
- **Out of Scope**: The operations' signatures (→ [`api/001`](../api/001_the_registry_surface.md)); the ownership invariant itself (→ [`invariant/001`](../invariant/001_one_name_one_ring.md)).

### Lifecycle Phases

| # | Stage | Owner | The ring's records |
|---|-------|-------|--------------------|
| K1 | **Unregistered** | The caller | Whatever the caller put there |
| K2 | **Registered** | The registry | Preserved; unreachable except through `get_mut` or `remove` |
| K3 | **Borrowed** | The registry, lent mutably | Readable and writable through the borrow |
| K4 | **Removed** | The caller again | Preserved across the transfer |
| K5 | **Dropped** | Nobody | Dropped, each one, transitively |

### Phase Transitions

| # | From | To | Trigger | Ownership |
|---|------|----|---------|-----------|
| L1 | K1 | K2 | `register`, name free | **Moves** to the registry |
| L2 | K1 | K1 | `register`, name taken | **Stays** with the caller — returned in the error |
| L3 | K2 | K3 | `get_mut` | Lent; the registry still owns it |
| L4 | K3 | K2 | The borrow ends | Lent back |
| L5 | K2 | K4 | `remove` | **Moves** to the caller |
| L6 | K2 | K5 | The registry is dropped | Dropped by the registry |
| L7 | K4 | K5 | The caller drops it | Dropped by the caller |
| L8 | K3 | K5 | Assignment through the borrow `get_mut` returns | Destroyed in place — no `remove` |
| L9 | K3 | K4 | `core::mem::replace` through the same borrow | **Moves** to the caller, without `remove` |

**L1, L2 and L5 are the three that move ownership, and each is asserted.** L1 by
the registry answering `get_mut` afterwards; L2 by
`a_refused_registration_hands_the_ring_back`, which uses the returned ring; L5 by
`a_removed_ring_carries_its_records_to_its_new_owner`, which drops the registry
first and observes zero drops before dropping the removed ring and observing all
five.

**L2 is the transition that a simpler signature would have deleted.** With
`register` returning a bare `RegistryError`, the ring moved in and was dropped —
so K1 would have led to K5 directly, via a naming mistake, with no stage in
between and nothing returned.

**L6 is transitive and is the acceptance criterion's third clause.** The registry
drops each `Split< T >`, each `Split` drops its `Ring< T >`, and each `Ring`
drops the records still unread in it. Only the first step is this crate's; the
rest is `ring_core`'s, measured here because this is where the claim is made — unless `T::drop` unwinds, in which case a ring later in the map may never be reached at all.

#### The unregistration path exists, and it is what makes "live" meaningful

The criterion says a second registration under a **live** name is refused — which
implies names can stop being live. L5 is that path, and without it a registry
would be append-only: a name taken once could never be reused, and the refusal
would be permanent rather than a condition.

`removing_a_name_frees_it_for_reuse` is the test, and it registers a
capacity-4 ring, removes it, and registers a capacity-64 one under the same name.
The differing capacity is what makes it a genuine second registration rather than
a no-op.

#### What has no stage here

| # | Not modelled | Why |
|---|---|---|
| N1 | A ring registered under two names | Excluded by ownership — `Split< T >` is not `Clone`, so there is nothing to register twice |
| N2 | A ring outliving its registry while still registered | Excluded by ownership — K2's owner is the registry, so L6 is unconditional |
| N3 | A partially-dropped registry | Not representable unless `T::drop` unwinds — `HashMap`'s own `Drop` runs to completion otherwise |

**N1 and N2 are enforced by the type system rather than by this crate**, and are
listed because a reader looking for the code that prevents them will not find
any. There is none; `Split< T >` deriving no `Clone` is the whole mechanism.

### Dependencies

The lifecycle needs remarkably little from outside, and the shortness of this
list is the crate's design rather than an omission:

| Needed | For which stage | Why it cannot be avoided |
|---|---|---|
| `ring_handle::Split< T >` | K1–K5 | It is the stored value; the map's whole type parameter |
| A `String` key | K2 onward | The name is owned, not borrowed — a registry holding `&str` keys could not outlive what it was built from |
| `T : ` nothing at all | every stage | There is no bound on `T`. The registry hashes a name and moves a value; it never reads one |

**Nothing supplies the drop.** L6 and L7 are `Drop` impls the registry does not
write — `HashMap`'s, then `Split`'s, then `Ring`'s — which is why the transitive
claim in L6 is measured here rather than asserted.

The caller supplies one thing the type system cannot: the decision to `remove`
before re-registering. That is the third row of the table above in spirit —
a prerequisite `Cargo.toml` cannot express (→ [`invariant/001`](../invariant/001_one_name_one_ring.md)).

### Cleanup Requirements

**There is exactly one obligation, and it is not the caller's:** every registered
ring's records are dropped exactly once, whether the registry is dropped whole
(L6) or the ring was removed first (L5 then L7) — again unless `T::drop` unwinds, in which case a ring later in the map may be dropped zero times rather than exactly once.

| Path | Who drops the records | Obligation |
|---|---|---|
| Registry dropped with rings in it | The registry, transitively | Each `Split` once — no leak, no double drop |
| Ring removed, then dropped | The caller | The same records, unchanged by the transfer |
| Registration refused | The caller | The ring never entered, so nothing changed hands |

`a_removed_ring_carries_its_records_to_its_new_owner` asserts the second row
against the first by ordering the drops: it drops the *registry* first and
observes zero record drops, then drops the removed ring and observes all five.
Testing them in that order is what distinguishes "the records moved" from "the
records were dropped twice and the count happened to match".

**No stage requires an explicit close, flush, or unregister call.** A caller who
simply drops the registry leaks nothing, which is the property that makes the
registry usable as a field in a longer-lived structure rather than something with
a teardown protocol of its own.

### Failure

| # | Failure | Consequence |
|---|---------|-------------|
| Q1 | L2 elided — refusal drops the ring | A naming mistake destroys unread records (→ [the pitfall](../pitfall/001_insert_would_have_replaced_silently.md)) |
| Q2 | L5 lends instead of moving | The caller cannot outlive the registry with their ring; `remove` becomes a strange `get_mut` |
| Q3 | L6 leaves a ring alive | A leak, invisible to the borrow checker |
| Q4 | L5 followed by L6 dropping it anyway | The registry drops what it no longer owns |

---

## The Stage That Was Ruled Out, and the Two Moves With No Row

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim is
# a parallel ugrep that emits hits in completion order. Patterns against this
# file are anchored at column 0 and carry no line numbers: the findings below
# quote these same rows, and would otherwise match themselves.
i=ring_registry/docs/lifecycle/001_a_ring_from_registration_to_drop.md
echo '  -- the stage the model calls unrepresentable, and the claims resting on it --'
command grep -h '^| N3 |\|^\*\*L6 is transitive\|^\*\*There is exactly one obligation' "$i" | cut -c1-96 | sed 's/^/    /'
echo '  -- every transition the model gives out of the borrowed stage --'
command grep -h '^| L[34] |\|^\*\*L1, L2 and L5' "$i" | cut -c1-96 | sed 's/^/    /'
echo '  -- what the crate requires of T, in every impl header it writes --'
command grep '^impl\|^pub struct\|^#\[ derive' ring_registry/src/lib.rs | sed 's/^/    src\/lib.rs:/'
printf '    impl headers placing any bound on T: %s\n' \
  "$( command grep -c '^impl< T : ' ring_registry/src/lib.rs || true )"
echo '  -- the two transitions out of the borrowed stage the table was missing --'
command grep -h '^| L[89] |\|^| K2 |' "$i" | cut -c1-96 | sed 's/^/    /'
```

Live output:

```
  -- the stage the model calls unrepresentable, and the claims resting on it --
    **L6 is transitive and is the acceptance criterion's third clause.** The registry
    | N3 | A partially-dropped registry | Not representable unless `T::drop` unwinds — `HashMap`'s
    **There is exactly one obligation, and it is not the caller's:** every registered
    | N3 | A partially-dropped registry | Not representable unless `T::drop` unwinds — `HashMap`'s
  -- every transition the model gives out of the borrowed stage --
    | L3 | K2 | K3 | `get_mut` | Lent; the registry still owns it |
    | L4 | K3 | K2 | The borrow ends | Lent back |
    **L1, L2 and L5 are the three that move ownership, and each is asserted.** L1 by
  -- what the crate requires of T, in every impl header it writes --
    src/lib.rs:#[ derive( Debug, Clone, PartialEq, Eq ) ]
    src/lib.rs:impl fmt::Display for RegistryError
    src/lib.rs:impl core::error::Error for RegistryError {}
    src/lib.rs:#[ derive( Debug ) ]
    src/lib.rs:pub struct Registry< T >
    src/lib.rs:impl< T > Default for Registry< T >
    src/lib.rs:impl< T > Registry< T >
    impl headers placing any bound on T: 0
  -- the two transitions out of the borrowed stage the table was missing --
    | K2 | **Registered** | The registry | Preserved; unreachable except through `get_mut` or `remov
    | L8 | K3 | K5 | Assignment through the borrow `get_mut` returns | Destroyed in place — no `re
    | L9 | K3 | K4 | `core::mem::replace` through the same borrow | **Moves** to the caller, without
```

A `T` with no bound may panic on drop. Three rings, four unread records each,
one record's `Drop` panicking exactly once, the whole teardown under
`catch_unwind`:

```rust
let mut registry = Registry::new();
for name in [ "a", "b", "c" ] { registry.register( name, loaded( 4 ) ).unwrap(); }

let unwound = std::panic::catch_unwind
(
  std::panic::AssertUnwindSafe( move | | drop( registry ) )
)
.is_err();
```

Its output, identical on two runs:

```
    3 rings, 4 unread records each: 12 records, len 3
    dropping it, with the 3rd record's Drop panicking once:
      the panic escaped the registry's drop: true
      Drop bodies entered: 4 of 12
      records never dropped at all: 8
```

---

### RG29 — The One Stage Ruled Out as Unrepresentable Is Two Lines of `T` Away

N3 excludes a partially-dropped registry from the model on the grounds that
`HashMap`'s own `Drop` runs to completion. That is true of a `HashMap` whose
values drop without panicking, and the crate accepts values that do not.

`T` carries no bound anywhere — not on the struct, not on either `impl`, and the
Dependencies table names that absence as a feature ("`T : ` nothing at all").
A record type whose `Drop` panics is therefore squarely inside the accepted input
space, and one panicking drop leaves **eight of twelve records never entered at
all**. The boundary is exactly the map: drop glue inside the affected ring
finishes its own four, then the panic escapes `Ring`'s drop and `HashMap`'s
element loop stops where it stands, leaking the two rings it had not reached
along with the table's own allocation.

Three of this document's claims turn on N3 being true. L6's transitivity — "each
`Ring` drops the records still unread in it" — holds for the one hop under the
panic and fails at every hop after it. The Cleanup Requirements' "exactly one
obligation… every registered ring's records are dropped exactly once" becomes
*at most* once. And Q3 ("L6 leaves a ring alive | A leak, invisible to the borrow
checker") is listed as a hypothetical failure of the implementation when it is a
reachable behaviour of the current one.

**Finding.** Recorded as a **latent hazard**, and a mild one by intent: nothing
in this family pushes panicking types through a registry today, and no reasonable
`T` panics on drop. It matters because the crate has gone out of its way to
accept *every* `T`, and the widest input space is the one whose edges must be
stated. Two repairs, both text: replace N3's "Not representable" with the
condition that actually holds — not representable unless `T::drop` unwinds — and
put the same qualifier on L6 and on the cleanup obligation. Either that, or state
that unwinding `T` is out of scope, which is a smaller claim than "no bound at
all" and should then be said in the Dependencies table too.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m 1 '^| N3 |' ring_registry/docs/lifecycle/001_a_ring_from_registration_to_drop.md
```

Live output:

```
| N3 | A partially-dropped registry | Not representable unless `T::drop` unwinds — `HashMap`'s own `Drop` runs to completion otherwise |
```

**Disposition:** applied — took the stated repair over the alternative: N3's row
now reads "Not representable unless `T::drop` unwinds — `HashMap`'s own `Drop`
runs to completion otherwise", and the same qualifier was appended to L6's
paragraph and to the Cleanup Requirements paragraph, both naming the same
panic-mid-teardown consequence this finding measured. Each addition extends an
existing line rather than inserting a new one, so no other citation into this
file shifted. Now prints:
`A partially-dropped registry | Not representable unless`

---

### RG30 — Five Transitions Move Ownership; the Table Has Three

The transition table gives K3 exactly one exit: L4, "The borrow ends | Lent
back". And the paragraph under it is explicit about the count — L1, L2 and L5 are
"the three that move ownership, and each is asserted."

A `&mut Split< T >` is an assignable place, and
[RG21](../invariant/001_one_name_one_ring.md) measures what that permits.
`*registry.get_mut( "events" ).unwrap() = fresh;` sends the borrowed ring from K3
straight to K5 — dropped, records destroyed, `len` unchanged — without passing
through K4, which the model says is the only stage a registered ring can reach on
its way out. `core::mem::replace` through the same borrow sends it to K4 instead,
where `remove` is supposed to be the sole road. So there are five ownership
moves, not three, and the two with no row are the two the model would rule out if
it named them.

The stage table has a matching slip. K2's records are "Preserved; unreachable
except through `get_mut`", while L5 two rows below hands the whole ring to the
caller — `remove` reaches them too, and reaches them by moving rather than
lending, which is a stronger form of reachable.

**Finding.** Recorded as a **misleading doc**: the model's value is that it
enumerates, and a reader who counts the ownership moves gets three. The repair is
two rows and one word — L8 (K3 → K5, assignment through the borrow, the ring is
destroyed in place) and L9 (K3 → K4, `core::mem::replace`, ownership leaves
without `remove`), with K2's "except through `get_mut`" widened to "through
`get_mut` or `remove`". Both new rows carry a Q-row consequence for free: L8 is
Q1's damage arriving through a door Q1 does not watch. Whether the borrow should
be assignable at all is a surface question and belongs with
[RG7](../api/002_the_receiver_split_and_the_sweep_it_forbids.md); this document's
job is only to say that it is.

**Disposition:** applied — added L8 (K3 → K5, assignment through the borrow,
destroyed in place) and L9 (K3 → K4, `core::mem::replace`, moves without
`remove`) to the Phase Transitions table, and widened K2's row to "except
through `get_mut` or `remove`". Now prints: `Destroyed in place`

---

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_registry_surface.md](../api/001_the_registry_surface.md) | A4 and A5 — L2 and L5 as per-call guarantees |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_one_name_one_ring.md](../invariant/001_one_name_one_ring.md) | R2 — the ownership property these transitions preserve; V3 and V4 as Q3 and Q4 |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_insert_would_have_replaced_silently.md](../pitfall/001_insert_would_have_replaced_silently.md) | Q1, and F2 — the `remove` signature that would have collapsed L5 into L6 |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | L1–L5 |
| [`ring_handle/src/lib.rs`](../../../ring_handle/src/lib.rs) | `Split< T >` — no `Clone`, which is N1 and N2's enforcement |

### Tests

| File | Relationship |
|------|--------------|
| `tests/registry_test.rs` | L2 — `a_refused_registration_hands_the_ring_back`; L3/L4 — `a_retrieved_ring_keeps_what_was_written_to_it`; L5 — `removing_a_name_frees_it_for_reuse`, `a_removed_ring_carries_its_records_to_its_new_owner`; L6 — `dropping_the_registry_drops_every_record_still_in_every_ring` |
