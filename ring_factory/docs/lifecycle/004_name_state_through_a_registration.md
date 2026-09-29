# Lifecycle: Name State Through a Registration

### Scope

- **Purpose**: Pin the ordering that makes a refused registration free of cleanup, and record the one interleaving in which a check-then-insert silently replaces another thread's ring.
- **Responsibility**: State the states, transitions, and behavioral invariants.
- **In Scope**: A name's states in the registry; the atomicity requirement; what a refusal must leave behind.
- **Out of Scope**: The registry's storage (→ [`ring_registry`](../../../ring_registry/readme.md)); the config's own states (→ [`lifecycle/003`](003_config_state_through_a_build.md)).

### States

**A name has three states and the middle one must not exist.**

| # | State | Meaning | Legal |
|---|-------|---------|-------|
| N0 | **Free** | No ring is registered under this name | Yes |
| N1 | **Claimed** | The name is taken but the ring it names is not yet complete | **No.** The state this machine is arranged to exclude |
| N2 | **Bound** | The name resolves to a ring that exists and is complete | Yes |

**N1 is the bug, drawn as a state so it can be argued about.** It arises from
the natural two-step spelling — reserve the name, then build — and it is
observable: a concurrent lookup between the two steps finds a name that resolves
to nothing, or worse, to a partially-initialised ring.

**The exclusion is achieved by ordering, not by locking.** Build first,
register second. Under that order the registry only ever receives a finished
ring, so there is no window in which N1 could be observed, and no cleanup path
is needed for a refusal — the ring that was built is simply dropped as an
ordinary local.

**The cost of that ordering is a wasted construction on the losing path.** A
build that loses a name race has allocated a backing buffer for nothing. That is
the right trade: name collisions are a configuration error, not a hot path, and
the alternative — reserving first — buys back one allocation in exchange for N1
being reachable.

### Transitions

| # | From → To | Trigger | Note |
|---|-----------|---------|------|
| Y1 | N0 → N2 | `build_named` completes: ring built, then inserted, and the insert found the name free | The success path. **One step at the registry, not two** |
| Y2 | N0 → N0 | `build_named` fails at the insert because another thread won the race | Returns `Err( NameTaken )`. The locally-built ring drops |
| Y3 | N2 → N2 | `build_named` on an already-bound name | Returns `Err( NameTaken )`. **The existing binding is untouched** |
| Y4 | N2 → N2 | Lookup by name | Read-only |
| Y5 | N0 → N0 | Lookup by name | Returns `None`. Not an error |
| Y6 | N2 → N0 | Deregistration | **Does not exist**, and should not without a decision (→ [`decisions/`](../decisions/readme.md)) |
| Y7 | N0 → N1 → N2 | Reserve-then-build | **Forbidden.** The interleaving this machine excludes |

**Y1's atomicity is the requirement, and it belongs to `ring_registry`.** This
crate must perform a single insert-if-absent and act on its answer — never
`contains()` followed by `insert()`. The two-call spelling is a
time-of-check-to-time-of-use race with a specific, silent outcome:

```text
thread A            thread B
contains("x") → false
                    contains("x") → false
insert("x", ringA)
                    insert("x", ringB)   ← ringA is replaced, no error anywhere
```

**Both threads get `Ok`, and A's handles now point at a ring nothing can find
by name.** No panic, no error, no diagnostic — A's ring keeps working through
the handles A holds, and the name silently means something else. That is worse
than either thread failing.

**Y3 is the guarantee that makes a name meaningful.** A registration that
replaced an existing one would make "look up by name" return whichever ring
registered most recently, which is the property a name exists to prevent
(→ [`api/002`](../api/002_the_named_build_surface.md)'s guarantee 2).

**Y6's absence is deliberate and is a gap, not a decision.** Nothing in this
crate's own registration surface describes removing a name, and nothing in
`ring_shutdown`'s close/reset/drain-all path says what happens to a *registered*
ring when it closes. So a name is
currently permanent and a closed ring stays findable — which is coherent, and is
not obviously what anyone intended.

### Behavioral Invariants

| # | Invariant | Holds because |
|---|-----------|---------------|
| Z1 | N1 is never observed | Build precedes register; the registry never sees an incomplete ring |
| Z2 | A failed `build_named` leaves the registry byte-identical to before the call | Y2 and Y3 both perform exactly one failed insert and no other write |
| Z3 | A failed `build_named` leaves no ring reachable by any route | The locally-built ring was never handed out and never inserted; it drops at the `return` |
| Z4 | Exactly one of N concurrent registrations of the same name succeeds | The registry's insert-if-absent is atomic. **Not this crate's to implement, and this crate must not attempt to help** |
| Z5 | Registration never alters the ring | It can only refuse the whole call (→ [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)'s V3) |

**Z5 is the invariant that keeps `invariant/001` true on the naming path.**
Information flows from shared state into `build_named` — the registry's contents
decide whether the call succeeds — and Z5 confines that flow to a single bit
that aborts or does nothing. Anything wider, such as deriving a capacity from
how many rings are already registered, makes the registry a second input and
`invariant/001` false.

**Z3 is the one a test can check and the one a naive implementation fails.** The
failure mode is not a leak in the memory sense — Rust drops the local — but a
*handover* leak: an implementation that constructs, hands the pair to the
caller, and then attempts registration returns `Err` alongside a working ring.
The caller now holds a functioning unnamed ring and an error saying the build
failed, which is the worst of both.

**Z4's second sentence is the actionable part.** The temptation is to add a
mutex, a double-check, or a retry loop in this crate to "make registration
safe." All three make it less safe by adding a second synchronisation point that
can disagree with the registry's own. The correct implementation here is one
call and one branch.

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_named_build_surface.md](../api/002_the_named_build_surface.md) | Y1–Y3 as operations; guarantees 1 and 2 are Z3 and Y3 |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_assembling_a_ring_from_a_validated_record.md](../algorithm/002_assembling_a_ring_from_a_validated_record.md) | F2 — the only genuine error edge, which is Y2 and Y3 |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_handle_pair_as_output.md](../data_structure/002_the_handle_pair_as_output.md) | H3 — ✅ resolved: the registry retains the owning `Split< T >` and lends `&mut` from `get_mut` |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_configuration_fully_determines_the_ring.md](../invariant/001_configuration_fully_determines_the_ring.md) | V3 and Z5 — the registry as a hidden second input |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_a_record_to_a_handle_pair.md](../lifecycle/001_from_a_record_to_a_handle_pair.md) | L5, T9, and C1 — this machine as a phase of the build arc |
| [../lifecycle/002_the_factory_outlives_nothing.md](../lifecycle/002_the_factory_outlives_nothing.md) | K6 — whether the registry is held or passed |

### State Machines

| File | Relationship |
|------|--------------|
| [003_config_state_through_a_build.md](003_config_state_through_a_build.md) | The machine that runs alongside this one on every named build |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_build_error.md](../type/002_build_error.md) | `NameTaken`, the sole outcome of Y2 and Y3 |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_registry/readme.md`](../../../ring_registry/readme.md) | Z4's owner |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | Z3 ✅ — `a_refused_registration_leaves_the_original_ring_intact` asserts the refusal *and* that lookup still resolves to the first ring, which is the half a `registry.len() == 1` check misses. Z4 ❌ **not written, and cannot be at this crate's grain:** `build_named` takes `&mut Registry`, so two threads cannot hold one registry to race for it — the borrow checker forecloses the scenario rather than the test failing to catch it. Z4's subject belongs to whatever type eventually shares a registry across threads, and no such type exists in the family |

### FC32 — Registration Is Atomic and the Name's Later States Are Not This Crate's to Pin

The ordering that makes a refusal free is `register`'s alone: one
insert-if-absent, never a check followed by an insert. That covers the
transition *into* the registered state. Every transition out of it belongs to
methods this crate never calls:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the registry public API, and which entries mutate --'
command grep -E '^  pub fn (register|remove|get_mut|new)\b' ring_registry/src/lib.rs | sed 's/(.*//'
echo '  -- which of them this crate calls --'
command grep 'registry\.' ring_factory/src/lib.rs | command grep -v '///'
echo '  -- and which the tests exercise --'
command grep -oE 'registry\.[a-z_]+\(' ring_factory/tests/factory_test.rs | sort -u | tr '\n' ' '
echo
```

Live output:

```
  -- the registry public API, and which entries mutate --
  pub fn new
  pub fn register
  pub fn get_mut
  pub fn remove
  -- which of them this crate calls --
    match registry.register( name, split )
  -- and which the tests exercise --
registry.contains( registry.get_mut( registry.is_empty( registry.len( registry.names( registry.remove( 
```

One call site in the whole crate. `remove` un-registers a name and `get_mut`
hands out a mutable borrow of a registered ring, and both are reachable by
anyone holding the registry — which, by the ruling that the registry is the
caller's, is always somebody other than this crate.

So the state machine documented here has one edge this crate owns and the rest
owned by whoever passed the `&mut Registry`. `a_removed_name_can_be_built_into_again`
asserts the round trip through an edge this crate cannot take, which is the
right test and is testing `ring_registry` through this crate's door.

The finding is the asymmetry between what the instance can pin and what it
describes: the atomicity requirement is enforceable here, because it is a
property of the one call; the name's later states are documented here and
enforceable nowhere, because the type that owns them is a re-exported argument
(→ [`api/002`](../api/002_the_named_build_surface.md) FC7).
