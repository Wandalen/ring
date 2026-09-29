# Integration: A Decision on the Export Surface

### Scope

- **Purpose**: Account for what it means that one of the five names a consumer outside the family may import is a *policy* rather than a thing — and what that obliges of this crate's API that the other four do not carry.
- **Responsibility**: State the boundary, the obligations exporting imposes, the confinement mechanism, and its current force.
- **In Scope**: The five-name surface; this crate's position on it; G5's status.
- **Out of Scope**: The declared and transitive dependency graph (→ [Two Dependencies and the Barrier It Cannot See](001_two_dependencies_and_the_barrier_it_cannot_see.md)).

### System Description

**Five of thirty-three crates are nameable from outside the family.** The list
is declared, not inferred:

```sh
cd "$(git rev-parse --show-toplevel)"
cat bench_harness/gate/declared/ring/export_surface.txt
```

Live output:

```
# Ring family export surface — the only ring_* crates a consumer outside
# ring_* may name as a dependency.
#
# Ruled by docs/decision/121_workstream_008_contract_gaps_ruled.md § 4, which
# upholds the five-crate Contract set by decision/120 and rules that features
# 171 (SPSC), 172 (MPSC) and 181 (registry) are reached *through* this surface
# rather than by importing ring_spsc, ring_mpsc or ring_registry directly:
# ring_factory constructs them and hands back ring_handle values.
#
# The other 28 crates are internal to the family and freely refactorable.
# Widening this file widens the workstream's contract — do not add a name
# without a decision that says so.

ring_factory
ring_handle
ring_tls
ring_flush
ring_types
```

The file's own header states what admission means, and the sentence to hold onto
is the last one:

> Widening this file widens the workstream's contract — do not add a name
> without a decision that says so.

**This crate is on that list and it is the odd one out.** The other four are
things a consumer holds:

| Name | What a consumer gets | Kind |
|------|---------------------|------|
| `ring_factory` | A constructor | A thing that makes things |
| `ring_handle` | A producer or consumer handle | A thing |
| `ring_tls` | A staging buffer | A thing |
| `ring_types` | Shared vocabulary | Definitions |
| **`ring_flush`** | **A choice about when publication happens** | **A decision** |

**A consumer importing `ring_flush` is not acquiring a capability; it is
accepting an obligation.** It gets `FlushPolicy` — three variants and no
machinery — and in exchange it must drive
([`pattern/002`](../pattern/002_driven_not_self_firing.md)), must announce
barriers if it chose `OnBarrier` ([`api/002`](../api/002_the_driver_surface.md)),
and must retry a rejected final drain
([`lifecycle/002`](../lifecycle/002_from_configuration_to_the_final_drain.md)'s
W3). Three obligations, none enforceable. That asymmetry is what being a
decision on the surface means, and it is the reason this crate's documentation
keeps arriving at obligations-it-cannot-check from different directions.

### Integration Points

| # | Point | Obligation this crate takes on |
|---|-------|-------------------------------|
| X1 | `FlushPolicy` is public and stable | Adding a variant is a breaking change for every consumer's `match`; the enum is `#[non_exhaustive]` or it is frozen (→ [`type/001`](../type/001_flush_policy.md)) |
| X2 | `FlushOutcome` is public and stable | Same, and it is the only channel through which a rejection is visible |
| X3 | The driver surface is public | `drive`, `drive_at_barrier`, `drain_final` — three names a consumer's code will be shaped around |
| X4 | The three obligations are documented, not enforced | **This instance and the API instances are the enforcement** |
| X5 | `ring_tls`'s primitives remain public | The consumer can bypass this crate entirely; the surface does not prevent it |

**X5 is the structural hole in the surface itself and it is worth stating
here rather than only in the invariant.** Both `ring_tls` and `ring_flush` are
exported. A consumer may therefore hold a buffer and call seal, drain and reset
directly, publishing outside any policy, without importing anything undeclared
and without tripping any gate. The export surface confines *which crates* may be
named; it says nothing about which operations within them are appropriate.

**As built the hole is half-closed, by ownership rather than by the surface.**
`Flusher::new` takes the `TlsBuffer` **by value**, so a buffer that has been
bound to a driver is unreachable — there is no borrow to call `drain()` or
`flush_into` on, and no accessor returns one. What a consumer can still do is
never bind a buffer at all and publish from it directly.

| | Reachable? |
|---|---|
| Bypass a buffer already bound to a `Flusher` | **No** — the buffer was moved |
| Keep a second, unbound buffer and publish from it | Yes. Unchanged by anything here |

So X5's obligation shrinks from "any buffer, at any time" to "a buffer the
consumer deliberately withheld," which is a decision visible at the binding site
rather than an accident available anywhere. It is still not a gate, and no gate
checks it.

### The confinement mechanism, and its current force

**G5 is the gate that enforces the surface.** It walks every `Cargo.toml` in the
repository and rejects any manifest outside the family that names a `ring_*`
crate not on the list — with family crates exempted:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -E 'family_members|grep -qx -- "\$owner"' bench_harness/gate/g5_export_surface.sh
```

Live output:

```
mapfile -t members < <( family_members )
  printf '%s\n' "${members[@]}" | grep -qx -- "$owner" && continue
```

The exemption is a lookup against the family's declared membership list. **This
instance previously quoted a path glob here**, with a line number, and that line
no longer exists — see FL19, which measures the gap and records why the quotation
outlived the code.

**And G5 is currently vacuous.**
[`bench_harness`'s gate-non-vacuity invariant](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md)
records its status directly: "No crate outside the family depends on any
`ring_*` crate yet, so the confinement has nothing to confine." The gate passes
because there is nothing to check.

**Nothing outside the family is presently positioned to become a first
consumer.** Every crate verified alongside this one is a family crate,
exempt by the same rule, and no external consumer is scheduled — the
boundary stays declared rather than exercised until one arrives.

So the surface is presently a declaration rather than a constraint. That is not
a defect: the file exists so that the first crate to import from the family
lands against a written boundary instead of establishing one by accident. But a
reader who sees "G5 passes" should not read it as "the boundary held."

### Error Handling

| Failure | Detected by | Notes |
|---------|-------------|-------|
| An outside crate imports `ring_spsc` directly | G5 — **once an outside consumer exists** | The confinement's whole purpose; vacuous today |
| A consumer imports `ring_flush` and never drives | **Nothing** | The buffer fills; `OnFull` fires; the other two never do |
| A consumer calls `ring_tls`'s primitives directly | **Nothing** | X5; no gate covers operation-level use |
| A new `FlushPolicy` variant is added | Downstream compile errors, unless `#[non_exhaustive]` | X1 |
| `export_surface.txt` is widened without a decision | Review only — the file's own header is the rule | No script checks for a matching decision |

**The last row is a gap in the gate's own governance** and it is the family's,
not this crate's. The header states "do not add a name without a decision that
says so"; nothing verifies that a name added has one. Worth recording where
family-level concerns live rather than here.

### Compatibility Requirements

| # | Requirement | Consequence of breaking it |
|---|-------------|---------------------------|
| Z1 | `FlushPolicy`'s three variants keep their meanings | Every consumer's configuration silently changes behaviour |
| Z2 | `FlushOutcome` distinguishes `NotTriggered` from `TriggeredEmpty` | Collapsing them makes a mis-scheduled driver indistinguishable from an idle one (→ [`type/002`](../type/002_flush_outcome.md)) |
| Z3 | `drive` and `drive_at_barrier` stay separate names | Merging them into a boolean parameter destroys the greppability that is D1/D2's stated reason for existing |
| Z4 | Adding a variant requires a decision | Same rule the surface file states for adding a name |
| Z5 | The crate stays free of a `Drop` flush | A consumer's teardown behaviour would change without any API change |

**Z5 is the compatibility requirement most likely to be broken by a
well-intentioned fix**, because it looks like an internal improvement rather
than a surface change. Adding `Drop` alters when a consumer's records become
visible, with no signature difference to review — an ABI-invisible behavioural
break on an exported crate.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_policy_surface.md](../api/001_the_policy_surface.md) | X1 and X2's surface; the absences it refuses to fill |
| [../api/002_the_driver_surface.md](../api/002_the_driver_surface.md) | X3 and Z3 |

### Integrations

| File | Relationship |
|------|--------------|
| [001_two_dependencies_and_the_barrier_it_cannot_see.md](001_two_dependencies_and_the_barrier_it_cannot_see.md) | The inbound graph; `ring_handle`'s row is this surface seen from the dependency side |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_publication_point_is_designed_not_inherited.md](../invariant/002_publication_point_is_designed_not_inherited.md) | X5 as a standing restriction; its P5 is the gate that does not exist |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_policy_as_a_value.md](../pattern/001_policy_as_a_value.md) | Why a decision is exportable at all — a value crosses a crate boundary; a schedule does not |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_flush_policy.md](../type/001_flush_policy.md) | X1's subject |
| [../type/002_flush_outcome.md](../type/002_flush_outcome.md) | X2's and Z2's |

### Sources

| File | Relationship |
|------|--------------|
| [`bench_harness/gate/declared/ring/export_surface.txt`](../../../bench_harness/gate/declared/ring/export_surface.txt) | The five names and the widening rule, quoted above |
| [`bench_harness/docs/invariant/001_gate_non_vacuity.md`](../../../bench_harness/docs/invariant/001_gate_non_vacuity.md) | G5's recorded vacuity |
| [`ring_handle/docs/integration/002_on_the_export_surface.md`](../../../ring_handle/docs/integration/002_on_the_export_surface.md) | The sibling exported crate's account of the same boundary |

### Tests

| File | Relationship |
|------|--------------|
| `tests/flush_test.rs` | Z5, in two parts and not in the form specified. For `FlushPolicy`, `the_policy_is_a_value` asserts `!needs_drop`, which is stronger than checking for a `Drop` impl — it also rejects a variant whose *payload* has one. For the flusher the specified check is impossible and would have been the wrong instrument anyway: an optional log owns a `Vec`, so `needs_drop` is true regardless. What Z5 was protecting is behavioural, so `dropping_a_driver_with_records_staged_publishes_nothing` asserts that instead |

### FL19 — The Mechanism Quoted Here Was Replaced by the Thing the Script Says a Path Test Gets Wrong

The line this instance displayed as G5's exemption is not in G5:

```sh
cd "$(git rev-parse --show-toplevel)"
G=bench_harness/gate/g5_export_surface.sh
echo '  -- the quoted form --'
printf '    occurrences of a path-glob exemption: %s\n' "$( command grep -c 'ring_\*) continue' "$G" )"
echo '  -- what decides membership instead --'
command grep -E 'family_members|grep -qx -- "\$owner"' "$G" | sed -E 's/^(.{0,96}).*/\1/'
echo '  -- and the reason the script gives for not using a path test --'
awk '/Outside the family|path glob|smoke binary|whole dependency list/ { printf "    %d: %s\n", NR, substr( $0, 3 ) }' "$G"
```

Live output:

```
  -- the quoted form --
    occurrences of a path-glob exemption: 0
  -- what decides membership instead --
mapfile -t members < <( family_members )
  printf '%s\n' "${members[@]}" | grep -qx -- "$owner" && continue
  -- and the reason the script gives for not using a path test --
    12: "Outside the family" is decided by the declared membership list, not by a
    13: path glob. The orbital family's own demo is named `demo_orbital_rail` and
    15: class the family's own smoke binary as an external consumer and demand its
    16: whole dependency list be exported.
```

Zero occurrences. The exemption is `grep -qx` against the family's declared
membership list, and the script's header explains the choice: a `<prefix>_*`
path test would class a family's own demo binary as an external consumer and
"demand its whole dependency list be exported."

**The quoted line was not merely outdated; it was the specific design the script
rejected, with the rejection written above the code.** So the instance did not
lag behind a refactor — it recorded a mechanism that the file it cites argues
against, ten lines above the code it cites.

**The line number is what made it credible.** `22:  case "$manifest" in ...`
carries the format of `grep -n` output: a number, a colon, source. Nothing else
in this corpus produces that shape by hand, so the shape itself reads as
provenance — this was run, at some point, against some version of the file. That
is probably true, and it is exactly why the block survived: a reader checking it
would need to open the script, and a reader who trusts the format will not.

The reusable shape: **a pasted result is most durable when it looks most
measured.** A hand-written summary invites checking. A line-numbered extract
answers the question that would have prompted the check.

### FL20 — The Family's Own Consumer Cannot Be Built Without Naming a Crate the Surface Excludes

The surface names five crates; the one program that uses them needs a sixth:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the declared surface --'
command grep -v '^#' bench_harness/gate/declared/ring/export_surface.txt | command grep . | sed 's/^/    /'
echo '  -- what this crate'\''s constructor requires --'
awk '/^  pub fn new/{ i = 1 } i { printf "    %d: %s\n", NR, $0 } i && /-> Result/{ exit }' \
  ring_flush/src/lib.rs
command grep -E '^use ring_(core|tls|types)' ring_flush/src/lib.rs
echo '  -- and what ring_bench had to declare, in its own words --'
command grep 'Contract defect' ring_bench/Cargo.toml | sed -E 's/^(.{0,150}).*/\1/'
echo '  -- manifests outside ring_* naming any ring_* crate --'
printf '    %s\n' "$( command grep -rl '^ring_[a-z_]* = ' --include=Cargo.toml . 2>/dev/null | command grep -cvE '/ring_|\.claude/worktrees' )"
```

Live output:

```
  -- the declared surface --
    ring_factory
    ring_handle
    ring_tls
    ring_flush
    ring_types
  -- what this crate's constructor requires --
    408:   pub fn new
    409:   (
    410:     buffer : TlsBuffer< T >,
    411:     producer : Producer< 'a, T >,
    412:     policy : FlushPolicy,
    413:   )
    414:   -> Result< Self, ConfigError >
use ring_core::Producer;
use ring_tls::TlsBuffer;
use ring_types::RingError;
  -- and what ring_bench had to declare, in its own words --
#                it is a Contract defect, not a preference.
  -- manifests outside ring_* naming any ring_* crate --
    1
```

`Flusher::new` takes a `ring_core::Producer`. `ring_core` is not on the surface,
and this crate re-exports neither the type nor any way to obtain one — so a
consumer that named only the five declared crates could import `ring_flush` and
be unable to construct a `Flusher`.

**This is a different hole from X5 and the difference matters.** X5 is about what
a consumer *may* do that the surface does not forbid — hold a `TlsBuffer` and
publish outside any policy. This is about what a consumer *must* do that the
surface does forbid. One is unenforced permission; the other is a required
violation, and only the second makes the declared set wrong rather than
incomplete.

**`ring_bench` is the existing witness.** Its manifest declares `ring_core` and
its comment states why in the crate's own words: the staging candidate cannot be
constructed without that line, and it is "a Contract defect, not a preference."
That comment has been sitting in a manifest since the benchmark was written.

**G5 cannot see any of this, and the reason is not the vacuity this instance
already records.** `ring_bench` is a declared family member, so its manifest is
skipped before any dependency is examined — the exemption that makes intra-family
refactoring free is the same exemption that hides the family's own proof that the
surface is too small. Zero manifests outside the family name a `ring_*` crate at
all, so the gate's other half has nothing to check either. The gate is vacuous
*and* blind, and the two failures are independent: fixing the first, by acquiring
a real outside consumer, would not surface this, because that consumer would fail
G5 rather than reveal why the surface is short.

The instance's closing sentence — "a reader who sees G5 passes should not read it
as the boundary held" — is right, and understated. The boundary is already known
not to hold, in writing, by the crate that broke it.
