# Integration: The Only Consumer of Two Contract Names

### Scope

- **Purpose**: Measure how many crates in the family actually depend on each of the five names the export Contract publishes, and record what falls out — two names with exactly one consumer, and the name this crate uses most and cannot name.
- **Responsibility**: State the consumer count per Contract name, from the manifests rather than from the Contract document.
- **In Scope**: The five exported names; every `ring_*` manifest; which of them this crate declares.
- **Out of Scope**: Which nine edges this crate declares and why three were added late (→ [`001`](001_declared_edges_and_the_three_that_were_missing.md)); the Contract's membership, which is decided at the family level rather than by this crate; what the ceiling costs a comparison (→ [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md)).

### What the Contract Publishes, and Who Takes It

The export Contract names five crates. Inside the family, their consumer counts
are not remotely comparable.

| Exported name | Declared by | Count |
|---------------|-------------|-------|
| `ring_types` | thirty crates | 30 |
| `ring_tls` | `ring_bench`, `ring_flush`, `ring_testkit` | 3 |
| `ring_handle` | `ring_factory`, `ring_registry` | 2 |
| `ring_factory` | `ring_bench` | **1** |
| `ring_flush` | `ring_bench` | **1** |

`ring_bench` declares four of the five. The one it does not declare is
`ring_handle`, and that is the one this document is mostly about.

### Sources

| File | Relationship |
|------|-----------------|
| `Cargo.toml` | The nine declared edges, measured rather than described |
| [`src/lib.rs`](../../src/lib.rs) | Seven prose mentions of a crate that appears in no `use` and no path |

### Integrations

| File | Relationship |
|------|--------------|
| [`001_declared_edges_and_the_three_that_were_missing.md`](001_declared_edges_and_the_three_that_were_missing.md) | The nine edges from this side, and the two Contract names that do not compose |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`../pitfall/001_the_door_caps_what_the_structure_does_not.md`](../pitfall/001_the_door_caps_what_the_structure_does_not.md) | The consequence of the undeclared edge — a ceiling imposed by a crate this manifest never names |

### Decisions

| File | Relationship |
|------|--------------|
| [`../decisions/001_five_candidates_for_four_named_paths.md`](../decisions/001_five_candidates_for_four_named_paths.md) | Why a candidate exists specifically to reach past the Contract |

### Tests

| Test | Relationship |
|------|--------------|
| `the_contract_door_caps_a_multi_producer_structure_at_one_producer` | Exercises the undeclared crate's `Ends::split` without naming it |
| `every_candidate_declares_a_name_and_a_ceiling` | The ceiling table, whose two door-imposed rows point at the undeclared crate |
| `a_policy_refusal_names_the_crate_that_refused` | The two Contract names whose errors this crate relays |

### BN19 — Two of the Five Exported Names Have Exactly One Consumer, and It Is This Crate

Consumer counts, per exported name, measured from the manifests:

```sh
cd "$(git rev-parse --show-toplevel)"
for exported in ring_types ring_tls ring_handle ring_factory ring_flush
do
  consumers="$( command grep -l "^$exported = " ring_*/Cargo.toml 2>/dev/null | sed 's|/Cargo.toml||' | tr '\n' ' ' )"
  count="$( command grep -l "^$exported = " ring_*/Cargo.toml 2>/dev/null | wc -l )"
  printf '  %-14s %2s  %s\n' "$exported" "$count" "$consumers"
done
printf '  crates in the family: %s\n' "$( ls -d ring_*/ | wc -l )"
```

Live output:

```
  ring_types     30  ring_atomic ring_barrier ring_batch ring_bench ring_store ring_claim ring_config ring_consume ring_core ring_cursor ring_debug ring_event ring_factory ring_flush ring_gating ring_handle ring_index ring_mpsc ring_overflow ring_poll ring_publish ring_seqno ring_shutdown ring_slot ring_spsc ring_stats ring_testkit ring_tls ring_trace ring_wait 
  ring_tls        3  ring_bench ring_flush ring_testkit 
  ring_handle     2  ring_factory ring_registry 
  ring_factory    1  ring_bench 
  ring_flush      1  ring_bench 
  crates in the family: 33
```

**A compatibility guarantee is graded by whoever depends on it, and for two of
these five names that is one crate.** `ring_factory` and `ring_flush` are named
on the same Contract as `ring_types`, which thirty crates take. The
guarantee is written identically for all five; the exposure differs by a factor
of thirty.

That matters here specifically because this crate is a benchmark harness, and a
harness is the *worst* single consumer to be graded by. It exercises the happy
path of each candidate once per run and does not care about ergonomics,
composability, or error chaining — the properties a real consumer would notice
first. Both gaps this crate did find in the Contract were found by *trying to
build something*, not by measuring anything:
[`001`](001_declared_edges_and_the_three_that_were_missing.md)'s
`Flusher::new`/`build` incompatibility surfaced at the first compile, and the
producer ceiling surfaced at the first multi-producer workload.

The general shape: **"exported" is a property of the Contract document; "used"
is a property of the manifests, and nothing keeps the two in step.** Two names
have been on the Contract since it was defined and have been compiled
against, from outside their own crate, exactly once.

### BN20 — The Contract Crate This One Leans On Hardest Is the One It Cannot Name

`ring_handle` supplies the type every Contract candidate is built through, and
appears nowhere in this crate's manifest or code:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- what the manifest declares --'
sed -n '/^\[dependencies\]/,/^$/p' Cargo.toml
echo '  -- where ring_handle appears in this crate --'
command grep -rn 'ring_handle' src/ tests/ Cargo.toml | sed -E 's/^(.{0,104}).*/\1/' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- and how many of those are code rather than prose --'
printf '    prose (doc comments) : %s\n' "$( command grep -rn 'ring_handle' src/ tests/ | command grep -c '//' )"
printf '    code (use or path)   : %s\n' "$( command grep -rn 'ring_handle' src/ tests/ | command grep -vc '//' )"
echo '  -- what the Contract door actually hands back --'
command grep -n 'pub fn build<' ../ring_factory/src/lib.rs | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  -- what the manifest declares --
[dependencies]
ring_factory = { path = "../ring_factory" }
ring_tls = { path = "../ring_tls" }
ring_flush = { path = "../ring_flush" }
ring_stats = { path = "../ring_stats" }
ring_spsc = { path = "../ring_spsc" }
ring_mpsc = { path = "../ring_mpsc" }
# Message 960 did not assign these three. Each is needed to write a signature,
# which is the same reason `ring_factory` had to add two — see that crate's
# `docs/integration/001`. Third occurrence in the family, and the first where
# the missing name is on the export Contract's own argument list:
#
#   ring_core  — `ring_flush::Flusher::new` takes a `ring_core::Producer` and
#                `ring_flush` re-exports neither it nor a way to build one, so
#                the staging candidate cannot be constructed without this line.
#                Recorded as this crate's `docs/integration/001` requirement 1;
#                it is a Contract defect, not a preference.
#   ring_slot  — `ring_spsc::Ring< S >` and `ring_mpsc::Ring< S >` are generic
#                over `Slot`, and `TypedSlot< T >` is the only implementor this
#                benchmark exercises (the other, `BytesSlot< N >`, round-trips
#                through no path measured here — see ring_event's own
#                non_functional_requirement/001 § EV34). The two direct
#                candidates cannot name their own ring type without it.
#   ring_types — `RingStats::record_drop` takes an `OverflowPolicy`, so the
#                per-run counters feature 185 supplies cannot be recorded
#                without naming the policy vocabulary.
ring_core = { path = "../ring_core" }
ring_slot = { path = "../ring_slot" }
ring_types = { path = "../ring_types" }

  -- where ring_handle appears in this crate --
src/lib.rs://! build, which is the configuration most readers of these docs are in. `ring_factory::bu
src/lib.rs://! `ring_handle` deliberately does not re-expose it.
src/lib.rs:  /// it: `ring_factory::build`, returning a `ring_handle::Split`.
src/lib.rs:  /// dispatch `ring_core` and `ring_handle` add: same ring, same single
src/lib.rs:  /// | [`ContractRing`](Self::ContractRing) | 1 | `ring_handle::Ends::split`, which yiel
src/lib.rs:  /// | `OffTheShelf` | 1 | `ring_handle` again; `ArrayQueue` itself is multi-producer |
tests/bench_test.rs:  // `ring_handle` — the crate that actually imposes the `1` on `ContractRing`,
tests/bench_test.rs:/// do with the ring: `ring_factory::build` hands back a `ring_handle::Split`,
  -- and how many of those are code rather than prose --
    prose (doc comments) : 8
    code (use or path)   : 0
  -- what the Contract door actually hands back --
  pub fn build< S : Send >( &self, cfg : RingConfig ) -> Result< Split< S >, BuildError >
```

Every mention is a doc comment. `run_contract_ring` binds
`Factory.build::< Record >( … )`, then calls `.ends()` and `.split()` on the
result — three `ring_handle` operations reached entirely through inference on a
return type, with no `use`, no path, and no manifest edge.

**So the crate's largest finding rests on a crate its manifest does not name.**
The producer ceiling that caps `ContractRing`, `TlsOverRing` and `OffTheShelf`
at one producer is imposed by `ring_handle::Ends::split`, which yields a
producer with no `try_clone` beside it. A reader who opens `Cargo.toml` to find
where that constraint comes from finds nine edges and none of them is the
answer. The `producer_ceiling` doc table names `ring_handle` twice, in prose, in
a crate that does not depend on it.

This is not a defect to fix — adding the edge would declare a dependency the
code does not have, and `ring_factory` re-exporting `Split` is `ring_factory`'s
call, not this crate's. It is the reason the doc table exists in that form:
**a constraint arriving through a return type has no manifest row to be recorded
in, so the only place left to record it is prose, where nothing checks it.**

The counterpart is worth stating from the other side. `ring_bench` declares
`ring_core`, `ring_slot` and `ring_types` precisely *because* those types had to
be written out in signatures — a type you must name becomes an edge, and a type
you can infer does not. The nine-edge manifest is a record of which of this
crate's dependencies happened to be un-inferrable, and it reads like a record of
which ones matter.
