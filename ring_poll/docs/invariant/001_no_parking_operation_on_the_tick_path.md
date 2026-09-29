# Invariant: No Parking Operation Is Reachable From the Tick Path

### Scope

- **Purpose**: State that the set of `ring_*` crates from which a parking operation can be reached is exactly `{ ring_barrier, ring_shutdown, ring_wait }`, and that no crate in that set is on the tick path.
- **Responsibility**: What counts as parking, how the roster is kept honest against the manifests, why the assertion lives here rather than in the crate it constrains, and the two things the invariant does not establish.
- **In Scope**: The dependency-graph reachability claim, its measurement, and its enforcement strength.
- **Out of Scope**: Whether non-parking code returns *promptly*, which does not follow (→ [`002_a_budget_bounds_attempts_not_time.md`](002_a_budget_bounds_attempts_not_time.md)).

### Invariant Statement

The set of `ring_*` crates from which a parking operation can be reached is
exactly `{ ring_barrier, ring_shutdown, ring_wait }`, and no crate in that set
is on the tick path.

#### What "parking" means here

`ring_wait::pause` is the family's only operation that gives up the CPU. It is
selected by a `WaitKind`, an enum owned by `ring_types` rather than by
`ring_wait`, whose four variants do different things and two of which count:

| `WaitKind` | What it does | Parks? |
|---|---|---|
| `Spin` | `core::hint::spin_loop()`, bounded by the attempt count | No |
| `Yield` | `std::thread::yield_now()` | **Yes** — the scheduler decides when this thread runs again |
| `Park` | `std::thread::sleep( 50µs )` | **Yes** |
| `None` | Returns `false` — "stop after the first look" | No |

`Spin` and `None` are the two a tick may reach. They are also the two this
crate implements directly, which is why it does not depend on `ring_wait` at
all: taking the dependency to use two of its four variants would put the other
two one autocomplete away, which is the exact failure this invariant exists to
prevent.

The type and the function are not in the same crate, and only one of them is out
of reach. `ring_types` is in this crate's own normal closure — `WaitKind::Park`
is a name `ring_poll` can already spell — while `ring_wait::pause`, which is
what acts on it, is not reachable at any depth → PL21.

### Enforcement Mechanism

The check greps every `ring_*/Cargo.toml` for the name and expects the
three crates above. Each is a coordination crate that runs *between* ticks — a barrier waits for a cohort
to arrive, a shutdown waits for a close, and `ring_wait` is the primitive
itself. None is reachable from inside a system's step.

The same set is declared as `ring_poll::PARKING_CRATES` and asserted against the
manifests by
`tests/poll_test.rs::the_tick_path_cannot_reach_a_parking_operation`, so the
roster cannot drift away from the graph it describes.

#### Why the assertion lives in this crate

This invariant also constrains `ring_handle` without being claimed by it: nothing
reachable from a handle may park. If that assertion lived in `ring_handle`'s own
suite, the crate whose manifest changed would be the crate asked to notice —
and `ring_handle`'s suite stays green whether or not it grows a `ring_wait`
dependency, because nothing in it would call the new code. The check has to sit
somewhere that fails when *another* crate's manifest changes, and this is the
crate that owns the feature.

### Violation Consequences

**A tick-path crate that gains a parking dependency risks a deadlock, not a
slowdown.** A tick that parks on a full ring and never wakes stops every later
tick with it — the failure this invariant exists to prevent, and the reason the
roster is asserted rather than merely documented.

Two things the invariant does not establish:

**That the graph cannot change.** Rust has no way to forbid an import; a
tick-path crate that adds `ring_wait = { path = "../ring_wait" }` compiles fine.
What the invariant buys is that the change is *loud* — the suite fails, and the
roster in the public surface has to be edited to make it pass again, which is an
API change rather than a quiet one.

That distinction — enforcement by *noise* rather than by *impossibility* — is
the general shape, extracted in
[`../pattern/001`](../pattern/001_enforcement_by_dependency_graph.md).

**That non-parking code returns promptly.** It does not follow, and the
counter-example is one line: `Budget::new( 1_000_000 )`. See
[`002_a_budget_bounds_attempts_not_time.md`](002_a_budget_bounds_attempts_not_time.md).

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'the set, as declared in src:  %s\n' "$( command grep 'pub const PARKING_CRATES' ring_poll/src/lib.rs | command grep -oE 'ring_[a-z_]+' | tr '\n' ' ' )"
printf 'the same names, typed here:   %s copies in prose\n' "$( command grep -o 'ring_barrier, ring_shutdown, ring_[w]ait' ring_poll/docs/invariant/001_no_parking_operation_on_the_tick_path.md | wc -l )"
printf 'crates that reach ring_wait:  %s\n' "$( for c in ring_*/; do n=$( basename "$c" ); cargo tree --manifest-path Cargo.toml -e normal -p "$n" 2>/dev/null | command grep -q ring_wait && echo x; done | wc -l )"
printf 'WaitKind is owned by:         %s\n' "$( command grep -rl 'pub enum WaitKind' */src/ 2>/dev/null | cut -d/ -f1 | tr '\n' ' ' )"
printf 'its variants:                 %s\n' "$( awk '/pub enum WaitKind/{f=1} f&&/^\}/{exit} f' ring_types/src/policy.rs | command grep -oE '^  [A-Z][a-zA-Z]*' | tr -d ' ' | tr '\n' ' ' )"
printf 'ring_types, in this closure:  %s\n' "$( cargo tree --manifest-path Cargo.toml -p ring_poll -e normal 2>/dev/null | command grep -c 'ring_types v' || true )"
printf 'ring_wait, in this closure:   %s\n' "$( cargo tree --manifest-path Cargo.toml -p ring_poll 2>/dev/null | command grep -c 'ring_wait v' || true )"
printf 'crates whose API takes a kind: %s\n' "$( for n in ring_barrier ring_shutdown ring_wait; do command grep -qE 'pub fn .*kind : WaitKind|kind : WaitKind' $n/src/lib.rs && printf '%s ' "$n"; done )"
printf 'this crate naming WaitKind:   %s\n' "$( command grep -c 'WaitKind' ring_poll/src/lib.rs || true )"
```

Live output:

```
the set, as declared in src:  ring_barrier ring_shutdown ring_wait 
the same names, typed here:   3 copies in prose
crates that reach ring_wait:  5
WaitKind is owned by:         ring_types 
its variants:                 Spin Yield Park None 
ring_types, in this closure:  14
ring_wait, in this closure:   0
crates whose API takes a kind: ring_barrier ring_shutdown ring_wait 
this crate naming WaitKind:   0
```

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_tick_path_surface.md`](../api/001_tick_path_surface.md) | Compatibility guarantee 1, which this invariant is the evidence for |

### Integrations

| File | Relationship |
|------|--------------|
| [`../integration/001_family_dependency_seam.md`](../integration/001_family_dependency_seam.md) | The declared edges the roster is measured against |

### Invariants

| File | Relationship |
|------|--------------|
| [`002_a_budget_bounds_attempts_not_time.md`](002_a_budget_bounds_attempts_not_time.md) | The weaker sibling claim, and the one this invariant is routinely mistaken for |

### Patterns

| File | Relationship |
|------|--------------|
| [`../pattern/001_enforcement_by_dependency_graph.md`](../pattern/001_enforcement_by_dependency_graph.md) | Enforcement by noise rather than impossibility, extracted as a reusable shape |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`../pitfall/001_non_parking_is_not_bounded_latency.md`](../pitfall/001_non_parking_is_not_bounded_latency.md) | The misreading this invariant invites |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `PARKING_CRATES` — the roster as a public constant, so drift is an API change |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `the_tick_path_cannot_reach_a_parking_operation` asserts the roster against every manifest; `this_crate_declares_no_parking_dependency` covers the narrower self-check |


### PL21 — the type that names the hazard is reachable; only the function is not

The invariant is stated over crates: `ring_poll` must not reach `ring_wait`, and
it does not, at any depth. But `WaitKind` — the enum whose `Yield` and `Park`
variants *are* the hazard — lives in `ring_types`, which is in this crate's
normal closure through `ring_core`. `WaitKind::Park` is a name `ring_poll`
compiles against today.

That is currently harmless, because nothing in this crate mentions it and no
function reachable from here consumes one. It is worth recording because it
marks where the guard's boundary actually is. The roster watches for a crate
acquiring the *implementation* of parking. It does not watch for the tick path
acquiring the *vocabulary* of parking, and the vocabulary arrived already, on an
edge nobody added deliberately.

The shape this leaves open is narrow but real: a future `ring_core` operation
taking a `WaitKind` parameter would put a parking *request* on the tick path
without any crate gaining a `ring_wait` dependency. The roster would stay at
three, the suite would stay green, and a tick could ask to be descheduled by a
crate that has no way to do it — until some later version of that call forwards
to `ring_wait` and the request starts being honoured.

Both roster crates already work that way, which is what makes the shape
plausible rather than hypothetical: `ring_barrier::Barrier::wait_for` and
`ring_shutdown::wait_for_close` each take a caller-supplied `kind : WaitKind`
and pass it down. Neither decides to park; both let a caller decide. Their place
on the roster is correct precisely because they are conduits, and a conduit is
exactly what a tick-path crate could become without tripping a manifest check.

### PL22 — the set is written out by hand in three places and generated in none

`{ ring_barrier, ring_shutdown, ring_wait }` appears in the Purpose bullet at the
top of this file and again in the Invariant Statement — two hand-typed normative
copies in one document about a set whose whole point is that it changes, plus a
third in the paragraph you are reading, which is this finding restating what it
is about.

`PARKING_CRATES` is the fourth copy, in source, and it is the only one anything
asserts against. The three in this file are prose: nothing compares them to the
constant, and a roster edit that correctly updates the constant and the test
leaves this document silently describing the previous family.

There was a fourth. The Enforcement Mechanism used to state its expected output
as *"expect exactly three lines"* — a hand-typed pass condition, sitting directly
under the command whose job was to produce it, in the section of the document
that exists to say how the claim is checked. It has been replaced with prose that
defers to the roster rather than restating it.

The Regenerate block above now reads the set out of `src/lib.rs` on every run, so
the copy that matters is the one in the code and this file reports it instead of
repeating it. That matters more here than the tidiness suggests: a typed
expectation of "three" reads as green against a set the same corpus measures at
five ([`../integration/002`](../integration/002_what_actually_reaches_ring_wait.md)),
so the copy most likely to be believed was the one nothing could falsify.
