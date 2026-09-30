# Integration: The Family Dependency Seam

### Scope

- **Purpose**: Account for every dependency this crate declares, and — equally — for the ones it deliberately does not, since a composition layer's dependency list is its design.
- **Responsibility**: Each edge, why it exists, and what its absence would mean; the one external dependency and the flags it is pinned behind; the crates a reader would expect here and will not find.
- **In Scope**: `Cargo.toml`'s `[dependencies]` and `[features]`.
- **Out of Scope**: The surface reconciliation with `ring_handle` (→ [`integration/002`](002_handle_surface_divergence.md)); why `crossbeam-queue` was chosen at all (→ [`workaround/001`](../workaround/001_crossbeam_queue_as_interim_backend.md)).

### System Description

`ring_core` is a composition layer, so its dependency list *is* its design —
there is almost nothing here that is not a backend, a vocabulary, or the one
shared policy resolution. Six in-house edges, one external edge pinned behind two
flags, and a set of absences that are load-bearing rather than incidental.

### Integration Points

| Crate | Why | Removable? |
|---|---|---|
| `ring_types` | `RingError`, the family's shared error vocabulary | No — the constructor's `Err` |
| `ring_config` | `RingConfig`, and `is_multi_producer()` as the backend discriminator | No — construction reads it |
| `ring_spsc` | Backend 1 | No |
| `ring_mpsc` | Backend 2 | No |
| `ring_slot` | `TypedSlot`, the element type the two in-house rings store | **Only if both in-house backends went** — it is theirs, not this crate's |
| `ring_overflow` | `would_resolve`, the shared policy resolution after every backend arm | No — it is the one place the policy is applied |

`ring_slot` is the interesting row: this crate never constructs a `TypedSlot`
for its own sake. It appears because `ring_spsc::Ring` and `ring_mpsc::Ring` are
slot-based and a record must live in something that exists before it does
(→ [`data_structure/001`](../data_structure/001_three_way_storage_enum.md),
asymmetry 1). A crossbeam-only build would not need it at all — but there is no
such build, and manufacturing one would mean a fourth configuration to test for
no caller's benefit.

#### The one external edge

```toml
crossbeam-queue = { version = "0.3", optional = true, default-features = false, features = [ "alloc" ] }
```

Every one of those four settings is load-bearing:

- **`optional`** — with `default = []`, a consumer who does not ask for the
  feature does not compile, download, or audit this crate.
- **`default-features = false`** — this removes a *feature*, not a crate, and
  the distinction is worth stating precisely because the obvious reading is
  wrong. Measured, both configurations resolve the same two packages:

  | | `crossbeam-queue` features | `crossbeam-utils` features |
  |---|---|---|
  | default | `default`, `std`, `alloc` | **`std`** |
  | ours | `alloc` | **none** |

  `crossbeam-utils` is an unconditional dependency of `crossbeam-queue` — it
  supplies `CachePadded` and `Backoff`, which `ArrayQueue` needs in every
  configuration, and no flag here can drop it. What the flag actually drops is
  `crossbeam-utils/std`, and with it the thread-parking machinery a
  spin-only queue never calls (→ [`invariant/001`](../invariant/001_no_atomic_of_its_own.md),
  row 4).
- **`features = [ "alloc" ]`** — `ArrayQueue` allocates its buffer once at
  construction, so `alloc` is the minimum that compiles. It is not `std`.
- **`version = "0.3"`** — the current major. A `0.x` dependency's minor bumps
  are breaking by cargo's rules, so this pin is tighter than it looks.

#### What is pointedly absent

| Crate | Why a reader expects it | Why it is not here |
|---|---|---|
| `ring_stats` (185) | Counters are the obvious thing to add at a composition point | An `AtomicUsize` here breaks `ring_spsc`'s zero-RMW assertion two crates away (→ [`invariant/001`](../invariant/001_no_atomic_of_its_own.md)). Compose it *around* this crate |
| `ring_shutdown` (184) | The surface has no `is_closed` and looks incomplete | Liveness is that crate's to own; a local copy of its flag is the precise failure it exists to prevent |
| `ring_handle` (176) | It specifies this exact surface | It sits *above* this crate. An edge here would invert the layering and create a cycle |
| A logging crate | Diagnostics | Nothing on the path may allocate or block |

**The absences are checkable, not merely stated:**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -cE '^(ring_stats|ring_shutdown|ring_handle) ' ring_core/Cargo.toml
```

Live output:

```
0
```

The grep anchors to column 0 rather than searching for the bare name, so this
paragraph — which contains all three names — cannot inflate its own count.

### Error Handling

Only one edge carries errors across the seam, and it carries them in one
direction:

| Edge | What crosses | Handled how |
|---|---|---|
| `ring_types` | `RingError` | Returned from `with_config` verbatim — this crate re-raises the family vocabulary rather than defining its own |
| `ring_overflow` | `would_resolve`'s verdict | Not an error at all: a policy decision resolved into `Ok`/`Err( record )` after the backend arm returns |
| `ring_spsc`, `ring_mpsc`, `crossbeam-queue` | Refusals | Never errors — a full backend returns its own refusal shape, which the dispatch converts into `Err( record )` |

**No dependency's error type is wrapped, prefixed, or re-rendered here.** A
composition layer that added its own error enum would give the family a second
vocabulary for the same conditions, which is exactly what depending on
`ring_types` is for.

### Compatibility Requirements

All six in-house edges point *downward* — this crate depends on leaves and is
depended on by composers. Nothing in `[dependencies]` depends on `ring_core` in
turn, which is what keeps the family a DAG. The crates that will consume this
one (`ring_handle`, `ring_factory`, `ring_bench`) are not yet built; each will
add edges *into* this crate when it lands, never out of it.

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_no_atomic_of_its_own.md](../invariant/001_no_atomic_of_its_own.md) | Why `ring_stats` is absent, and why `default-features = false` is not a style preference |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_three_way_storage_enum.md](../data_structure/001_three_way_storage_enum.md) | Asymmetry 1, which is why `ring_slot` is an edge |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_crossbeam_queue_as_interim_backend.md](../workaround/001_crossbeam_queue_as_interim_backend.md) | Why the external edge exists at all, and the condition for removing it |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` | C5 — the grep for atomics |
| `tests/manual/readme.md` | C9 — every claim on this page, run as four commands; it is where the `default-features` correction above came from |

### CO19 — Six Path Dependencies, Four Imported by Name

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
printf 'path deps:   '; grep -cE '^ring_[a-z_]+ = \{ workspace = true' Cargo.toml
printf 'use lines:   '; grep -cE '^use ' src/lib.rs
echo '--- reached by full path ---'
grep -oE 'ring_(mpsc|spsc|slot|overflow|config|types)::' src/lib.rs | sort | uniq -c
```

Live output:

```
path deps:   6
use lines:   4
--- reached by full path ---
      8 ring_config::
      6 ring_mpsc::
      1 ring_overflow::
      1 ring_slot::
      5 ring_spsc::
      4 ring_types::
```

Six path dependencies; four `use` declarations. `ring_mpsc` and `ring_spsc` are
never imported — every reference spells the crate name in full, because both
export types whose names (`Producer`, `Consumer`, `Ends`) collide with this
crate's own.

**The collision is the reason and it is worth naming**, since a reader scanning
the `use` block for the dependency list will find two thirds of it.

### CO20 — The Optional Dependency Is the Only One Without a Path

`crossbeam-queue = { version = "0.3", optional = true, default-features =
false, features = [ "alloc" ] }` — the sole external crate this composition
layer pulls in, and it is off by default.

`default-features = false` with `alloc` is deliberate: it keeps the dependency
usable in a no-std-with-alloc build, matching what the rest of the family
assumes. Recorded because it is the only place in this crate where an external
version constraint exists at all, and therefore the only place a version bump
can reach.
