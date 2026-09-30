# Integration: The Declared Edges and the Reached Closure

### Scope

- **Purpose**: Map this crate's dependency surface against what it actually needs, and record the two anomalies — a return type reachable only through the registry, and two declared dependencies the build arc never touches.
- **Responsibility**: State the system description, integration points, error handling, and compatibility requirements.
- **In Scope**: The five declared edges; the reachable closure and the absentees, both as regenerable counts; `ring_handle`'s position.
- **Out of Scope**: The export Contract's rules (→ [`integration/002`](002_the_crate_the_export_surface_routes_through.md)); what each dependency does internally.

### System Description

**Five declared dependencies, nineteen of the thirty-three crates reachable,
fourteen not reachable at all — as of the last run of the recipe below.**

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[dependencies\]/,/^\[/p' ring_factory/Cargo.toml   # five path deps
cargo tree -p ring_factory --prefix none --no-dedupe \
  | awk '{print $1}' | command grep '^ring_' | sort -u | wc -l      # 19
comm -13 \
  <( cargo tree -p ring_factory --prefix none --no-dedupe \
       | awk '{print $1}' | command grep '^ring_' | sort -u ) \
  <( ls -d ring_* | xargs -n1 basename | sort )              # the 14 absent
```

Live output:

```
[dependencies]
ring_config = { path = "../ring_config" }
ring_core = { path = "../ring_core" }
ring_registry = { path = "../ring_registry" }
# Message 960 did not assign these two, and both are needed to write the
# signature: `ring_handle` owns `Split`, which is `build`'s return type, and
# `ring_types` owns `RingError`, which `BuildError::Unsupported` carries.
# `ring_handle` was reachable only transitively through `ring_registry`, which
# is not enough to name a type. Recorded as requirement 1 of
# `docs/integration/001` — a manifest bug, not a decision.
ring_handle = { path = "../ring_handle" }
ring_types = { path = "../ring_types" }
# Message 960 also assigned `ring_stats` and `ring_tls`; both are removed.
# `RingConfig` has no field that would configure staging or counters, so there
# was nothing for a build to do with either, and inventing surface to justify a
# manifest line is backwards. Closes `docs/decisions` Pending 7 by the second of
# its two branches. Restore them alongside the config fields, not before.

[lints]
19
ring_barrier
ring_batch
ring_bench
ring_consume
ring_debug
ring_event
ring_flush
ring_poll
ring_publish
ring_shutdown
ring_testkit
ring_tls
ring_trace
ring_wait
```

**Run these before trusting the numbers — this instance has already been wrong
once.** It was written against a closure of twenty-two and eleven, and
`ring_core`'s manifest was rewritten during the writing: `ring_batch` and
`ring_event` were dropped, `ring_config`, `ring_slot` and `ring_types` added,
and an optional `crossbeam` feature introduced for an optional third backend. `ring_event`
left the closure entirely; `ring_batch` stayed only because `ring_tls` still
reaches it.

**This is the second such shift inside one crate's documentation, and the first
was in a different crate.** `ring_flush`'s closure dropped from twenty-four to
twenty while it was being documented, when `ring_spsc` and `ring_mpsc` were
implemented and shed edges they no longer needed. Two independent occurrences is
enough to treat the counts as measurements rather than facts, which is why every
count in this instance ships with the command that regenerates it — and why this
file no longer carries one in its name.

**A third shift has since happened, and its cause is different from the first
two.** Twenty-one dropped to nineteen when *this* crate's manifest changed —
`ring_stats` and `ring_tls` were removed (requirement 2 below, closed) and
`ring_handle` and `ring_types` added (requirement 1, closed). Losing `ring_tls`
took `ring_batch` out of the closure with it, which is why the count moved by
two while the manifest moved by four. The first two shifts were something else
being written; this one is this crate acting on its own documentation. **The
counts are volatile in both directions and from both causes** — regenerate,
never quote.

| Declared | Needed for |
|----------|-----------|
| `ring_config` | The build argument. **The one dependency nothing could remove**, and now re-exported so a Contract-bound consumer can name it |
| `ring_core` | The composed backend `build` assembles, and the crate whose `DropOldest` refusal `BuildError::Unsupported` relays |
| `ring_registry` | The naming path. Re-exported as `Registry` |
| `ring_handle` | **Added.** `Split< S >` is `build`'s return type; a transitive path is not enough to name a type |
| `ring_types` | **Added.** `RingError` is `BuildError::Unsupported`'s payload |
| ~~`ring_stats`~~ | **Removed.** Nothing in the build arc, and no config field would give it one |
| ~~`ring_tls`~~ | **Removed.** Same |

**The fourteen unreachable crates:** `ring_barrier`, `ring_batch`, `ring_bench`,
`ring_consume`, `ring_debug`, `ring_event`, `ring_flush`, `ring_poll`,
`ring_publish`, `ring_shutdown`, `ring_testkit`, `ring_tls`, `ring_trace`,
`ring_wait`.

Most are correct absences — a factory has no business reaching a benchmark
harness or a debug facility, and `ring_flush` is a peer on the export Contract
rather than something to depend on. Two are not:

| Absent | Why it matters |
|--------|----------------|
| `ring_wait` | The `wait` field names a strategy from a crate this one cannot reach (→ [`pitfall/002`](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md)) |
| `ring_shutdown` | Its close path. A named ring that closes has a registry entry, and nothing connects the two (→ [`lifecycle/004`](../lifecycle/004_name_state_through_a_registration.md)'s Y6) |

**A third absence is new and is not a gap:** `ring_event` was reachable through
`ring_core` and no longer is. It left because `ring_core` was implemented and
turned out not to need it — the honest outcome, and the one that makes the
count's volatility easy to misread as a defect. A crate leaving a closure
because the code that was going to use it was written and did not is a design
question answered, not an edge lost.

**One trap in re-running the recipe, worth knowing before it produces a
contradiction.** `ring_event` is still declared in the family — by `ring_tls`,
which is in this closure — so a naive `grep` for its dependents suggests it
should be reachable from here. It is not, because the declaration is a
*dev*-dependency and dev-dependencies are not transitive:

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[dev-dependencies\]/,/^\[/p' ring_tls/Cargo.toml
cd ring_tls   && cargo tree --prefix none --no-dedupe | command grep -c '^ring_event'   # non-zero
cd ../ring_factory   && cargo tree --prefix none --no-dedupe | command grep -c '^ring_event'   # 0
```

Live output:

```
[dev-dependencies]
ring_event = { path = "../ring_event" }
ring_slot = { path = "../ring_slot" }
ring_store = { path = "../ring_store" }

[lints]
1
0
```

**So `cargo tree` run from two different crates disagrees about the same crate,
and both answers are correct.** Every count in this instance is a build closure
taken from `ring_factory/`; taken from a leaf it would include that
leaf's test-only edges. This is the second way these numbers mislead, after
volatility, and it is the one that survives re-running the recipe.

### Integration Points

| # | Point | Direction | Contract |
|---|-------|-----------|----------|
| E1 | `RingConfig` arrives by value | In | Pre-validated, partly clamped, immutable here (→ [`data_structure/001`](../data_structure/001_the_configuration_record_as_input.md)) |
| E2 | A backend is constructed | Out | Via `ring_core`, which composes `ring_spsc` or `ring_mpsc` |
| E3 | A handle pair is produced | Out | `ring_handle`'s two types, on a declared edge since the anomaly below was closed |
| E4 | A name is registered | Out, naming path only | One atomic insert-if-absent, never check-then-insert |
| E5 | `wait` is read | — | **Dead end.** No waiter can be constructed |
| E6 | `ring_stats` | — | Declared, unreferenced |
| E7 | `ring_tls` | — | Declared, unreferenced |

#### The anomaly, now closed: E3's type was reachable only through E4's crate

`ring_handle` **is** a declared dependency of this crate today. It was not, and
the recipe that found that is the one that now shows it fixed:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -c '^ring_handle = ' ring_factory/Cargo.toml
cargo tree -p ring_factory --no-dedupe | command grep -c 'ring_handle'
for m in ring_*/Cargo.toml; do
  command grep -q '^ring_handle = ' "$m" && basename "$( dirname "$m" )"
done
```

Live output:

```
1
2
ring_factory
ring_registry
```

**Expected: `1`, then `2`, then `ring_factory` and `ring_registry`.** The first
`1` is the declaration itself and is the line that closed the anomaly. The `2`
counts *paths* under `--no-dedupe`, not crates — direct, plus the one beneath
`ring_registry`, which also depends on `ring_handle`.

**All three read differently before the fix**: no declaration, one occurrence,
one dependent. That was the anomaly — the type `build` returns was reachable
only because the naming path's crate happened to depend on it, so removing
`ring_registry` from this manifest (a plausible refactor if naming moved
elsewhere) would have stopped `build`'s return type compiling, for a reason
appearing in neither manifest nor either crate's documentation. Declaring
`ring_handle` directly is what makes that refactor safe: E3 is now carried by a
declared edge, and removing `ring_registry` would cost the naming path (E4) alone
rather than also silently breaking the handle pair.

**This is load-bearing and undeclared, which is the worst combination.** The fix
is one line — declare `ring_handle` directly — and it is not merely tidiness:
the current arrangement makes an unrelated change break the crate's central
signature, and the error message would point at `ring_registry`.

**It is also the reverse of what the export Contract implies.** This family's own Contract ruling
says `ring_factory` "constructs them and hands back `ring_handle` values",
describing `ring_handle` as this crate's *output*. A crate's output type being an
undeclared transitive is not a design; it is what happens when four of the five
declared edges were chosen and one was inferred.

#### E6 and E7: two declared, neither used

`ring_stats` and `ring_tls` are in the manifest and the build arc touches
neither (→ [`lifecycle/001`](../lifecycle/001_from_a_record_to_a_handle_pair.md)'s
D5 and D6). Two readings, and they lead opposite ways:

| Reading | Consequence |
|---------|-------------|
| The manifest is aspirational — a ring should be built *with* stats attached and *with* a staging buffer | Then the build arc is missing two phases, and `RingConfig` is missing the fields to configure them |
| The manifest is wrong — these were declared by symmetry with the export Contract's five names | Then two lines should go, and this crate's closure shrinks |

**The second reading is suspicious in a specific way worth noting:** this
crate's five declared dependencies are `ring_config`, `ring_core`, `ring_tls`,
`ring_stats`, `ring_registry` — and the export Contract's five names are
`ring_factory`, `ring_handle`, `ring_tls`, `ring_flush`, `ring_types`. The
overlap is `ring_tls` alone, so it is not a straight copy; but a manifest with
exactly five entries, two of which are unused, in a family whose Contract has
exactly five names, is worth a second look rather than an assumption. Recorded
in [`decisions/`](../decisions/readme.md).

### Error Handling

| # | Seam | What can go wrong | Reported |
|---|------|-------------------|----------|
| E1 | Config arrival | Nothing. Pre-validated | — |
| E2 | Backend construction | `overflow == DropOldest` → `ring_core` refuses; allocation failure → abort | `Err( RingError::PolicyUnsupported )` from `ring_core`, **currently unpropagated** |
| E3 | Handle pair production | Nothing at runtime. Nothing at compile time either, now that `ring_handle` is declared directly — removing `ring_registry` costs E4 only | **Nothing** |
| E4 | Registration | Name taken | `Err( BuildError::NameTaken )` |
| E5 | `wait` read | The strategy is not honoured | **Nothing** |
| E6 | — | — | **Nothing** |
| E7 | — | — | **Nothing** |

**E2 changed under this instance and is now the most consequential row.** When
this table was first written, backend construction was infallible and the only
thing that could go wrong was an allocation abort Rust owns. `ring_core` is now
implemented, and `Ring::new` refuses one of the three overflow policies:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  pub fn new( config : &RingConfig ) -> Result< Self, RingError >' ring_core/src/lib.rs
```

Live output:

```
  pub fn new( config : &RingConfig ) -> Result< Self, RingError >
  {
    if config.overflow() == OverflowPolicy::DropOldest
    {
      return Err( RingError::PolicyUnsupported );
    }
```

So a legal, fully-validated `RingConfig` — `DropOldest` is `OverflowPolicy`'s
second variant and `ring_config` accepts it without complaint — reaches `E2` and
is refused there. **This crate has nowhere to put that refusal**, because the
unnamed `build` path returns a handle pair rather than a `Result`
(→ [`type/002`](../type/002_build_error.md)). Recorded as a pending, and it is
the one gap on this table with running code behind it rather than an absent
crate.

**Five of seven seams report nothing, and only two of those five are innocent.**
E1 and E3 genuinely have nothing to report. **E5, E6 and E7 are silences over
real gaps** — a field unhonoured and two dependencies whose purpose is unstated
— and they are indistinguishable at this table from the innocent rows.

**That is the shape worth carrying to the other crates in this family:** a seam
table with mostly-empty error columns reads as a clean design and can equally
mean the crate has no way to say what is wrong. Distinguishing the two requires
naming each silence, which is what the paragraphs above do — and E2 shows the
third case, where a silence was innocent when written and stopped being one
without the table changing.

### Compatibility Requirements

1. ✅ **`ring_handle` must be declared directly**, not reached through
   `ring_registry`. **Done** — and `ring_types` needed the same fix for the same
   reason, since `BuildError::Unsupported` carries a `RingError`. A transitive
   path lets a crate *link*; it does not let it *name*.
2. ✅ **`ring_config` must stay a dependency**, and its type must become
   Contract-legal somehow. **Done** by re-export — `pub use
   ring_config::RingConfig;`, option W3 of the three
   (→ [`decisions/001`](../decisions/001_the_owner_is_the_return_value.md)).
3. ✅ **No public signature names a crate outside the Contract's five.**
   `RingConfig` and `Registry` are re-exported, so a consumer names only
   `ring_factory`, `ring_handle` and `ring_types` — all three on the Contract.
   Asserted by `the_contract_surface_is_reachable_without_naming_a_non_contract_crate`.
4. **E4 stays a single atomic operation.** Adding a check-then-insert here would
   be a correctness regression invisible to every signature
   (→ [`lifecycle/004`](../lifecycle/004_name_state_through_a_registration.md)'s Z4).
5. ✅ **E6 and E7 are resolved in one direction or the other.** **The manifest
   lost two lines.** `RingConfig` has no field that would configure staging or
   counters, so the arc could not have gained the phases without the record
   gaining fields first — and inventing surface to justify a manifest line is
   backwards. Restoring them is a one-line change once the fields exist.
6. ✅ **E2's refusal must reach the caller.** `build` is fallible and relays it
   as `BuildError::Unsupported( RingError )`, keeping the ruling in the crate
   that makes it. The second option — refusing `DropOldest` here before
   delegating — was rejected because it duplicates a backend policy that a fourth
   backend could silently make wrong.
   → [`decisions/002`](../decisions/002_two_doors_not_one_that_routes.md).

**All of 1, 2, 3, 5 and 6 are now closed, and the order they were listed in
turned out to be right for a reason the list did not give.** Requirement 1 was
listed first as "the cheapest real improvement" and requirement 6 flagged as
"the most urgent". Both were true, and 1 was also **a precondition for 6**: the
error type `BuildError::Unsupported` carries could not be named until
`ring_types` was declared, which is the same missing-manifest-line problem as
`ring_handle`'s. The cheap fix was blocking the urgent one, and nothing in the
list said so.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | E1 and E3; requirement 2 and 3's subject |
| [../api/002_the_named_build_surface.md](../api/002_the_named_build_surface.md) | E4 |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/002_assembling_a_ring_from_a_validated_record.md](../algorithm/002_assembling_a_ring_from_a_validated_record.md) | E2 and E5, as consumption steps |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_handle_pair_as_output.md](../data_structure/002_the_handle_pair_as_output.md) | E3's type, and the D1/D2/D3 question that shapes it |

### Integrations

| File | Relationship |
|------|--------------|
| [002_the_crate_the_export_surface_routes_through.md](002_the_crate_the_export_surface_routes_through.md) | The Contract requirements 2, 3 and 5 answer to |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_a_record_to_a_handle_pair.md](../lifecycle/001_from_a_record_to_a_handle_pair.md) | D1–D7, the same edges seen as phases |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md) | E5 in full |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/004_name_state_through_a_registration.md](../lifecycle/004_name_state_through_a_registration.md) | E4's atomicity, requirement 4 |

### Sources

| File | Relationship |
|------|--------------|
| `Cargo.toml` | The five declared edges |
| [`ring_registry/Cargo.toml`](../../../ring_registry/Cargo.toml) | The one manifest that declares `ring_handle` |
| [`ring_flush/docs/pitfall/001_on_barrier_cannot_see_the_barrier.md`](../../../ring_flush/docs/pitfall/001_on_barrier_cannot_see_the_barrier.md) | The closure-volatility lesson this instance's recipes come from |

### Tests

| File | Relationship |
|------|--------------|
| `tests/factory_test.rs` | Requirement 1 is not testable — it is a manifest property, checkable by the `grep` above and by `cargo +nightly udeps`, and belongs to a gate. Requirements 3 and 6 **are** tested: `the_contract_surface_is_reachable_without_naming_a_non_contract_crate` and `both_paths_relay_the_same_refusal` |

### FC17 — Every Declared Edge Is Used, and Three Quarters of the Manifest Is About Edges That Are Not

The anomaly this instance was written to record — declared dependencies the
build arc never touches — is closed. All five are named in the source:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- declared edges, and mentions of each outside doc comments --'
for d in ring_config ring_core ring_registry ring_handle ring_types; do
  printf '  %-14s %s\n' "$d" \
    "$( command grep -vE '^\s*(///|//!)' ring_factory/src/lib.rs | command grep -c "$d" )"
done
echo '  -- manifest shape --'
printf '  comment lines: %s\n' "$( command grep -c '^#' ring_factory/Cargo.toml )"
printf '  dep lines:     %s\n' "$( command grep -c '^ring_' ring_factory/Cargo.toml )"
```

Live output:

```
  -- declared edges, and mentions of each outside doc comments --
  ring_config    1
  ring_core      1
  ring_registry  2
  ring_handle    1
  ring_types     1
  -- manifest shape --
  comment lines: 15
  dep lines:     5
```

Fifteen comment lines to five dependency lines. The comments are not
decoration — two of them record `ring_handle` and `ring_types` as a *manifest
bug* fixed rather than a design choice, and four record `ring_stats` and
`ring_tls` as edges deliberately removed, with the condition for restoring them.

That is the right thing to have written down and it makes the manifest the
crate's densest piece of history: three quarters of the file is about edges that
are not in it. The cost is that this is the only place any of it is recorded —
the removal note ends "Restore them alongside the config fields, not before",
and the config fields it means are `batch` and `wait`, which
[`algorithm/002`](../algorithm/002_assembling_a_ring_from_a_validated_record.md)
FC4 measures as read by nobody. Two documents state the same blocked condition
from opposite ends and neither points at the other.

### FC18 — Nineteen Crates in the Closure, Five Declared, and the Difference Is Invisible Here

```sh
cd "$(git rev-parse --show-toplevel)"
printf '  distinct crates in the closure: %s\n' \
  "$( cargo tree -p ring_factory --prefix none 2>/dev/null | sed 's/ v0.*//' | sort -u | wc -l )"
printf '  declared directly:              %s\n' \
  "$( command grep -c '^ring_' ring_factory/Cargo.toml )"
echo '  -- the closure --'
cargo tree -p ring_factory --prefix none 2>/dev/null | sed 's/ v0.*//' | sort -u | tr '\n' ' '
echo
```

Live output:

```
  distinct crates in the closure: 19
  declared directly:              5
  -- the closure --
ring_align ring_atomic ring_store ring_claim ring_config ring_core ring_cursor ring_factory ring_gating ring_handle ring_index ring_mpsc ring_overflow ring_registry ring_seqno ring_slot ring_spsc ring_stats ring_types 
```

Nineteen crates, of which this one declares five and can name five. The other
thirteen arrive transitively, and a breaking change in any of them reaches this
crate through a dependency it did not choose.

The number is worth having because it bounds what "the build arc" can mean.
Every claim in this corpus about what a build touches is a claim about a subset
of nineteen crates, and the walk in
[`algorithm/002`](../algorithm/002_assembling_a_ring_from_a_validated_record.md)
names seven of them. The remaining crates are in the closure and on no arc this
crate documents — not because they are unused, but because nothing here has
looked. Regenerating this number is the cheapest available check that the family
has not quietly widened underneath the corpus.
