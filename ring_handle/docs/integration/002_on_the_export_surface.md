# Integration: On the Export Surface

### Scope

- **Purpose**: State this crate's position on the family's five-crate Contract — it is one of the five — and account for what that costs, since it is the exact inverse of the position `ring_spsc` occupies.
- **Responsibility**: The boundary, what a consumer may name, the gate that enforces it, and the obligations that fall on a crate whose signatures are public.
- **In Scope**: The export boundary; gate G5; what changes here reach.
- **Out of Scope**: The dependency beneath (→ [One Dependency and the Backends Beneath It](001_one_dependency_and_the_backends_beneath.md)); the other four exported crates' own surfaces.

### System Description

A family-grain decision
restricted the family's external visibility to five crates:

```text
                      ┌─────────────── the Contract ───────────────┐
consumer  ──names──▶  │ ring_factory  ring_handle  ring_tls        │
                      │ ring_flush    ring_types                   │
                      └────────────────────────────────────────────┘
                                        │
                      ┌─────────────────┴──────────────────────────┐
                      │  28 internal crates — nothing outside      │
                      │  may name any of them (gate G5)            │
                      └────────────────────────────────────────────┘
```

**`ring_handle` is one of the five, and the consequence runs in exactly the
opposite direction from `ring_spsc`'s.**
This crate's design settles that internal crates' capabilities are reached
*through* this surface rather than by import — so this crate is where that
indirection terminates.

| | [`ring_spsc`](../../../ring_spsc/docs/integration/002_reached_through_the_export_surface.md) | `ring_handle` |
|---|---|---|
| Named in an outside manifest | Never — G5 fails the build | **Routinely.** That is its purpose |
| A signature change reaches | `ring_core`, `ring_handle` — then stops | **Every consumer of the family** |
| An open API question costs | A two-crate refactor | A breaking change, or a permanent wart |
| Its docs can leave shapes undecided | Cheaply, and do | **Expensively.** These docs commit |

**This is why [`api/001`](../api/001_producer_surface.md) and
[`api/002`](../api/002_consumer_surface.md) give committed signatures where
`ring_spsc`'s equivalents give three candidates each.** Same family, same
authors, same week — the difference is entirely which side of a five-name list
the crate falls on. The rationale states the value being protected: "the
Contract's entire value is that it is narrow — a five-crate seam is what makes
28 crates freely refactorable."

**Being one of the five is what makes the refactoring freedom of the other 28
possible, and this crate pays for it.** Every open question absorbed here is a
question 28 crates do not have to answer publicly.

### Integration Points

| # | Seam | Direction | What crosses it | What it requires |
|---|------|-----------|-----------------|------------------|
| B1 | A consumer's manifest | → This crate | `ring_handle = { ... }` | Nothing. This is the sanctioned form |
| B2 | A consumer's source | → This crate | `Producer< T >`, `Consumer< T >`, and the error types they return | That no public type here names an internal crate — G5 checks manifests, not signatures |
| B3 | [`ring_factory`](../../../ring_factory/readme.md) | → This crate | The handle pair it constructs and returns | That both crates are on the Contract, so the pair can be returned by name |
| B4 | [`ring_types`](../../../ring_types/readme.md) | → This crate | Shared discriminants a public signature may mention | That it is also on the Contract — which it is, by design |
| B5 | Gate `G5` | ↔ | `bench_harness/gate/declared/export_surface.txt` | That this crate stays on the list. Removing it would break every consumer |
| B6 | Gate `g3_features.sh` | ↔ | `tests/handle_test.rs`'s citation of this crate's declared feature identifier | That the crate→feature edge exists as a test citation, never as prose |

**B2 is the seam that G5 does not cover, and it is worth being precise about
the gap.** G5 reads dependency declarations: it fails the build if an outside
crate *names* `ring_claim` in its manifest. It does not read type signatures. A
`Producer` method returning `Result< (), ring_claim::ClaimError >` satisfies G5
completely — the consumer never names `ring_claim`, it just receives its type —
while defeating the purpose entirely: that consumer is now coupled to an
internal crate's error enum and cannot be shielded from its changes.

**So B2's requirement is a convention, not a gate.** Whether it should be a
gate is a real question and a family-grain one, since the check would live in
`bench_harness` alongside G5.

**And G5 itself is currently vacuous, which is worth stating plainly rather
than leaving to be discovered.**
[`bench_harness`'s own gate-non-vacuity invariant](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md)
records G5's status as "No crate outside the family depends on any `ring_*`
crate yet, so the confinement has nothing to confine" — it passes because there
is nothing to check, not because a boundary is being held.

The gate's own loop is where this is visible: `g5_export_surface.sh` walks every
`Cargo.toml` in the repository and skips manifests by path —
`case "$manifest" in */ring_*) continue ;; esac` — so every crate in the
family, `ring_bench` included, is exempt by construction. What remains is every
manifest outside `ring_*`, and none of them names a ring crate. No
external consumer is presently scheduled to arrive, so the boundary stays
declared rather than exercised. The first outside consumer is therefore
unplanned, and G5 stays vacuous until it arrives.

**This does not weaken anything above; it dates it.** Every consequence in the
Error Handling table is what G5 does once it has a consumer to check. Until
then the export boundary is upheld by the fact that nobody has crossed it, and
the first crate to depend on this family is also the first real test of the
Contract.

**B4 is the reason `ring_types` is on the Contract at all.** It supplies
the discriminants — `WaitKind`, `OverflowPolicy` — and a handle's configuration
surface has to be able to mention them. An exported crate whose signatures
require an unexported type is unusable, so the two memberships are joint.

**B3 makes the pair returnable.** `ring_factory` is on the Contract and returns
values of this crate's types; both being on the list is what lets that signature
be written at all.

### Error Handling

| Failure | Meaning | Consequence |
|---------|---------|-------------|
| An outside crate names an internal `ring_*` | The Contract is breached | **Build failure** at G5 — loud, mechanical, the intended outcome |
| A public signature here returns an internal crate's type | B2's gap | **Nothing fails.** The consumer is coupled to an internal type and neither gate notices |
| This crate's signature changes | The exported contract moves | **Every consumer breaks** — loudly, which is correct, and expensively |
| `tests/handle_test.rs` omits the feature citation | B6 | This crate's row reports unclaimed while the code is complete — a false negative, the safe direction |
| This crate is removed from `export_surface.txt` | B5 | Every consumer's build fails at G5 — loud, and the right failure for a wrong change |

**Row two is the only silent failure in this table**, and it is the one this
instance exists to name. Everything else about the export boundary is enforced
mechanically; type-level leakage is enforced by whoever is reading the diff.

**Row three is a feature.** A breaking change that breaks loudly is the correct
behaviour for an exported crate — the alternative is a compatibility shim, which
this family does not carry.

### Compatibility Requirements

1. **This crate stays on the export list.** Its whole role is to be nameable.
2. **No public type or error variant names a crate outside the five.** B2's gap
   is closed by convention; the convention is stated here so it can be cited.
3. **Signature changes are breaking and are treated as such.** No shim, no
   deprecated alias, no parallel v2 type.
4. **Additions to the surface are breaking in the direction nobody checks** —
   adding `Clone` to `Producer` breaks no caller and breaks `ring_spsc`
   (→ [A Convenience Method Undoes the Crate](../pitfall/001_a_convenience_method_undoes_the_crate.md)).
5. **`ring_types` stays on the Contract** as long as any signature here mentions
   its discriminants (B4).
6. **The claiming test cites this crate's declared feature identifier textually** (B6).
7. **Version movement is lockstep.**

**Requirement 4 is the one that distinguishes this crate from an ordinary
exported library**, where additive changes are safe by definition. Here the
public surface is defined by its absences, so an addition can be the breaking
change while a removal is merely visible.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | The signatures requirement 3 makes expensive to change, and its guarantee 5 is requirement 2 |
| [../api/002_consumer_surface.md](../api/002_consumer_surface.md) | Same, draining |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_two_handles_over_one_backend.md](../data_structure/001_two_handles_over_one_backend.md) | Why the D2 shape's lifetime parameter is expensive specifically because of this position |

### Integrations

| File | Relationship |
|------|--------------|
| [001_one_dependency_and_the_backends_beneath.md](001_one_dependency_and_the_backends_beneath.md) | The seam below; its requirement 6 is this instance's requirement 2 |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_proven_by_code_that_must_not_compile.md](../non_functional_requirement/001_proven_by_code_that_must_not_compile.md) | B6's citation, and the criterion that makes requirement 4 partially checkable |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_a_convenience_method_undoes_the_crate.md](../pitfall/001_a_convenience_method_undoes_the_crate.md) | Requirement 4 worked out — additions as the breaking changes |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_producer.md](../type/001_producer.md) | An exported type; its trait table is part of the public contract |
| [../type/002_consumer.md](../type/002_consumer.md) | Same |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/docs/invariant/001_gate_non_vacuity.md`](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md) | G5's current vacuity, and the family Contract this crate is on — "a five-crate seam is what makes 28 crates freely refactorable" |
| [`bench_harness/docs/acceptance/001_feature_reached_tests.md`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) | B6's rule — a crate claims a feature by testing it, never by asserting it in prose |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/handle_test.rs`](../../tests/handle_test.rs) | Cites this crate's declared feature identifier in its own text — B6, without which the feature reports unclaimed. Verified by `g3_features.sh`, which reports `all 1 features claimed by a test` |
| [`bench_harness`](../../../bench_harness/docs/acceptance/001_feature_reached_tests.md) gate `G5` | B5 — mechanical, and the only export check that runs automatically |

### HD23 — The Vacuity Argument Is Right and Both Pieces of Evidence for It Are Gone

The paragraph above supports its conclusion with a file path and a quoted line
of shell. Neither exists:

```sh
cd "$(git rev-parse --show-toplevel)"
printf '  the declared path this instance cites exists: %s\n' \
  "$( [ -f bench_harness/gate/declared/export_surface.txt ] && echo yes || echo no )"
printf '  the case/continue line it quotes, in the gate: %s\n' \
  "$( command grep -c 'ring_\*) continue' bench_harness/gate/g5_export_surface.sh )"
echo '  -- where the list actually lives, and how the gate reaches it --'
find bench_harness/gate/declared -name export_surface.txt | sed 's|^|    |'
command grep 'DECL="' bench_harness/gate/common.sh | sed 's|^|    |'
echo '  -- and how "outside the family" is actually decided --'
command grep 'family_members\|grep -qx -- "\$owner"' bench_harness/gate/g5_export_surface.sh
```

Live output:

```
  the declared path this instance cites exists: no
  the case/continue line it quotes, in the gate: 0
  -- where the list actually lives, and how the gate reaches it --
    bench_harness/gate/declared/ring/export_surface.txt
    DECL="$DECL_ROOT/$GATE_FAMILY"
  -- and how "outside the family" is actually decided --
mapfile -t members < <( family_members )
  printf '%s\n' "${members[@]}" | grep -qx -- "$owner" && continue
```

There is one `export_surface.txt` per family, reached as
`$DECL_ROOT/$GATE_FAMILY` — the ring family's is one of several, and the set
grows as families are added, which is why the recipe lists them rather than
naming a count. The confinement half no longer skips by path glob at all: it
reads `family_members` and skips a manifest whose owning crate name is on that
list.

**The conclusion survived the rewrite and its evidence did not.** G5's
confinement half is still vacuous — [`ring_factory`'s FC19](../../../ring_factory/docs/integration/002_the_crate_the_export_surface_routes_through.md)
measures that against the current script and finds `violations` empty by
construction — so a reader who trusts this paragraph reaches a true belief. They
reach it by following a path that 404s and looking for a line that is not there,
which is the failure mode a citation exists to prevent.

The generalizable part: **a paragraph whose conclusion outlives its evidence
reads exactly like one whose evidence is current.** Nothing in the corpus
distinguishes them, because the only signal is whether the quoted text still
matches, and no checker compares a quoted shell line to the file it came from.

### HD24 — B4's Justification for `ring_types` Membership Is Not Exercised Here

B4 says `ring_types` is on the Contract because "a handle's configuration
surface has to be able to mention" its discriminants, and calls the two
memberships joint. This crate cannot mention them:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- where ring_types sits in this manifest --'
command grep -E '^\[|^ring_types|^ring_core|^trybuild|^ring_config' ring_handle/Cargo.toml
echo '  -- and what the public signatures actually name --'
command grep -oE '^  pub (const )?fn [a-z_]+.*' ring_handle/src/lib.rs | sed 's|^  pub |    |'
printf '  ring_types anywhere in src/: %s\n' \
  "$( command grep -rc 'ring_types' ring_handle/src/lib.rs )"
```

Live output:

```
  -- where ring_types sits in this manifest --
[package]
[features]
[dependencies]
ring_core = { path = "../ring_core" }
[dev-dependencies]
ring_config = { path = "../ring_config" }
ring_types = { path = "../ring_types" }
trybuild = "1.0"
[lints]
  -- and what the public signatures actually name --
    const fn new( ring : Ring< T > ) -> Self
    fn ends( &mut self ) -> Ends< '_, T >
    fn split( &'a mut self ) -> ( Producer< 'a, T >, Consumer< 'a, T > )
    fn try_push( &mut self, record : T ) -> Result< (), T >
    fn try_push_batch( &mut self, records : &mut impl Iterator< Item = T > ) -> usize
    fn free_capacity( &self ) -> usize
    fn is_full( &self ) -> bool
    fn try_recv( &mut self ) -> Option< T >
    fn try_recv_batch( &mut self, out : &mut Vec< T > ) -> usize
    fn drain( &mut self ) -> Drain< '_, 'a, T >
    fn len( &self ) -> usize
    fn is_empty( &self ) -> bool
  ring_types anywhere in src/: 0
```

`ring_types` is a **dev-dependency**. Zero mentions in `src/`. Twelve public
methods, and the only non-`std`, non-local name across all of them is
`ring_core::Ring` in `Split::new`. `OverflowPolicy` appears once in the whole
source file, in a doc comment on `try_push` explaining that the *ring's* policy
decides — a type the caller never names and this crate never receives.

**So the joint-membership argument is sound and this crate is not its case.**
`ring_types` earns its place on the Contract through `ring_factory`, whose
`RingConfig` surface does take the discriminants; B4 attributes that to a
"handle's configuration surface", and handles have no configuration surface —
[`decisions/001`](../decisions/001_what_this_crate_is_for.md)'s N4 is precisely
that nothing here reaches configuration.

Worth recording because B4 reads as a dependency this crate has, and the
dev-dependency line is the only place that would correct it. Two of the five
Contract names are unreachable from every public signature in this crate; being
on the same list is not the same as being coupled.
