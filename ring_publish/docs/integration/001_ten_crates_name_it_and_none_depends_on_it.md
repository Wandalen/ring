# Integration: Ten Crates Name It and None Depends On It

### Scope

- **Purpose**: Record this crate's actual position in the family's dependency graph — two dependencies, four dev-dependencies, zero dependents — and the ten crates that discuss it in prose while depending on none of it.
- **Responsibility**: Establish the graph facts by command, distinguish the dev-dependency edges from the real ones, and locate `ring_publish` against its three Tier 5 siblings.
- **In Scope**: Every edge into and out of this crate, and the prose references that are not edges.
- **Out of Scope**: *Why* the two crates that could have depended on it did not — see [`integration/002`](002_the_two_crates_that_declined.md).

### The Edges

```sh
cd "$(git rev-parse --show-toplevel)"

# out — what this crate depends on, and what its tests depend on
awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_publish/Cargo.toml \
  | grep -oE '^ring_[a-z_]+'
awk '/^\[dev-dependencies\]/{f=1;next} /^\[/{f=0} f' ring_publish/Cargo.toml \
  | grep -oE '^ring_[a-z_]+'

# in — every manifest in the family that names this crate at all, with its section
for m in */Cargo.toml; do
  awk -v f="$m" '/^\[/{s=$0} /ring_publish/{printf "%s  %s  %s\n", f, s, $0}' "$m"
done
```

Live output:

```
ring_cursor
ring_types
ring_barrier
ring_claim
ring_consume
ring_gating
ring_mpsc/Cargo.toml  [package]  # Eight, not the seven this manifest was scaffolded with. `ring_publish` and
ring_publish/Cargo.toml  [package]  name = "ring_publish"
```

| Direction | Section | Crates |
|-----------|---------|--------|
| out | `[dependencies]` | `ring_types`, `ring_cursor` |
| out | `[dev-dependencies]` | `ring_claim`, `ring_consume`, `ring_barrier`, `ring_gating` |
| out | `[target.'cfg(loom)'.dev-dependencies]` | `loom = "0.7"` |
| **in** | — | **none** |

The last command finds exactly two manifests containing the string
`ring_publish`. One is this crate's own `name =` field. The other is
`ring_mpsc/Cargo.toml:8-11` — a **comment**, in the `[package]` section,
explaining that this crate was removed from that dependency list:

> Eight, not the seven this manifest was scaffolded with. `ring_publish` and
> `ring_consume` are gone and `ring_atomic`, `ring_slot` and `ring_types` are
> new — see decision 124. Publication here is a per-slot stamp this crate owns,
> so there is no published cursor for either removed crate to act on.

So the family's only manifest reference to this crate is a record of its
deletion from that manifest.

### PB1 — The Only Tiered Primitive With No Dependents

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_*/; do n=$( basename "$c" )
  t=$( grep -m1 -oE 'Tier [0-9]+' "$c/src/lib.rs" 2>/dev/null )
  d=$( command grep -rl --include=Cargo.toml "^${n} = " . 2>/dev/null | command grep -v "/${n}/" | wc -l )
  printf "%-16s %-8s dependents=%s\n" "$n" "$t" "$d"
done
```

Live output:

```
ring_align       Tier 1   dependents=1
ring_atomic      Tier 2   dependents=5
ring_barrier     Tier 5   dependents=2
ring_batch       Tier 2   dependents=1
ring_bench                dependents=0
ring_store      Tier 2   dependents=4
ring_claim       Tier 5   dependents=2
ring_config      Tier 1   dependents=12
ring_consume     Tier 5   dependents=1
ring_core                 dependents=10
ring_cursor      Tier 3   dependents=10
ring_debug                dependents=0
ring_event       Tier 2   dependents=1
ring_factory              dependents=2
ring_flush                dependents=2
ring_gating      Tier 4   dependents=4
ring_handle               dependents=3
ring_index       Tier 1   dependents=2
ring_mpsc                 dependents=2
ring_overflow    Tier 1   dependents=1
ring_poll                 dependents=0
ring_publish     Tier 5   dependents=0
ring_registry             dependents=1
ring_seqno         Tier 1   dependents=4
ring_shutdown             dependents=1
ring_slot        Tier 1   dependents=7
ring_spsc                 dependents=2
ring_stats       Tier 1   dependents=2
ring_testkit              dependents=0
ring_tls         Tier 2   dependents=4
ring_trace       Tier 2   dependents=0
ring_types       Tier 0   dependents=31
ring_wait        Tier 4   dependents=2
```

Six of the 33 crates have zero dependents:

| Crate | Tier | What it is |
|-------|------|------------|
| `ring_bench` | — | Comparative write-path measurements |
| `ring_debug` | — | Runtime invariant checks over a live ring |
| `ring_poll` | — | Non-blocking progress helpers |
| `ring_testkit` | — | Determinism-test fixtures |
| `ring_trace` | Tier 2 | Optional sequence-operation trace log |
| **`ring_publish`** | **Tier 5** | **Publication of claimed slots to consumers** |

Five of the six are tooling, harnesses, or explicitly optional. `ring_publish`
is a tiered write-path primitive — the load-bearing kind — and it is the only
one of those with nothing above it. Its three Tier 5 siblings all have
dependents: `ring_barrier` 2, `ring_claim` 2, `ring_consume` 1.

This is not a defect and it is not neglect. It is the measurable shape of
[`integration/002`](002_the_two_crates_that_declined.md): the two crates that
would have been its consumers each considered it and each declined, for
*opposite* stated reasons, and both wrote down which.

### PB2 — Ten Crates Discuss It, In Prose, Depending On None Of It

```sh
cd "$(git rev-parse --show-toplevel)"
# `grep` here is a `ugrep` shim whose output order is not stable between runs,
# so the sort is what makes this block comparable to the one recorded below
command grep -rl 'ring_publish' */src/*.rs | command grep -v ring_publish/ | sort
```

Live output:

```
ring_atomic/src/lib.rs
ring_barrier/src/lib.rs
ring_claim/src/lib.rs
ring_consume/src/lib.rs
ring_cursor/src/lib.rs
ring_event/src/lib.rs
ring_index/src/lib.rs
ring_mpsc/src/lib.rs
ring_spsc/src/lib.rs
ring_testkit/src/lib.rs
```

Ten crates name this one in their module or item documentation, and every one of
the mentions is a comment — no crate in the family names it in code, which is
what zero dependents means read from the other side:

| Crate | Where | What it says |
|-------|-------|--------------|
| `ring_atomic` | `:59-60` | `ring_publish/tests/handshake_test.rs` is what uses the family's `loom` seam, and the command that runs it |
| `ring_barrier` | `:36`, `:43` | `Publisher::cursor` is a barrier dependency; the handshake here is what forced `Barrier`'s signature to take a bare slice |
| `ring_claim` | `:18`, `:22`, `:106`, `:281`, `:298`, `:339` | "`ring_publish` is the second half"; why `Claim`'s accessors are public; which cursor is which |
| `ring_consume` | `:10` | "The feature's reached-test wires all four operations together in `ring_publish/tests/handshake_test.rs`" |
| `ring_cursor` | `:80` | Named among the five crates that import `Cursor` to decide whether a slot is safe to touch — the reason the accessor is public at all |
| `ring_event` | `:152` | `Publisher::publish` is *not* a slot-shaped publish path: it moves a cursor over sequence numbers and never touches a slot |
| `ring_index` | `:13` | Named among the four crates that work in sequence space end to end and never fold an index |
| `ring_mpsc` | `:15-21` | Why it is deliberately not used here ([`integration/002`](002_the_two_crates_that_declined.md)) |
| `ring_spsc` | `:10`, `:20-22` | Why its absence is the crate's thesis ([`integration/002`](002_the_two_crates_that_declined.md)) |
| `ring_testkit` | `:59` | Named among the crates whose test surface this one complements |

Ten prose references, zero dependency edges. The asymmetry is worth stating
plainly because it is the reverse of the usual failure: this is not a crate
nobody knows about, it is a crate everybody has an opinion about.

Two of the ten references are load-bearing in the other direction —
`ring_barrier:40-44` records that this crate's handshake test is what made it
change a signature, and `ring_atomic:59-60` records that this crate's test is
the only consumer of that crate's `loom` seam
([`workaround/001`](../workaround/001_the_loom_seam_and_its_only_user.md)). Two
crates changed shape for a test that lives here, in a crate neither depends on.

### PB3 — The Four Dev-Dependencies Are the Reached-Test, Not the Crate

`ring_publish/Cargo.toml:12-14` states the reason in the manifest itself:

> Tests only. `tests/handshake_test.rs` is feature 170's reached-test, which is
> the whole four-operation handshake — so it needs the other three operations'
> crates. None of them depends on this one, so the graph stays acyclic.

The library uses neither `ring_claim`, `ring_consume`, `ring_barrier` nor
`ring_gating`, and must not: publishing is a cursor advance and a contiguity
test, and every one of those four is a different operation in the same
handshake. `tests/manual/readme.md § P5` is the check that keeps the two
sections honest, and it is deliberately **two commands** rather than one:

```sh
cd "$(git rev-parse --show-toplevel)"
# The library's own dependencies, against the library.
comm -23 \
  <( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_publish/Cargo.toml \
     | grep -oE "^ring_[a-z_]+" | sort -u ) \
  <( grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )

# The dev-dependencies, against the tests.
comm -23 \
  <( awk '/^\[dev-dependencies\]/{f=1;next} /^\[/{f=0} f' ring_publish/Cargo.toml \
     | grep -oE "^ring_[a-z_]+" | sort -u ) \
  <( grep -hvE "^[[:space:]]*//" ring_publish/tests/*.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )
```

Live output:

```

```

`:118-120` records why: *"Sharing one command between them — as the first draft
of this check did — reports the four `[dev-dependencies]` as unused, because the
library indeed does not use them and must not."* A single-command version of
this check does not merely miss a defect; it reports four false ones, on a crate
whose whole distinguishing feature is which section its edges are in.

### The Dependency That Was Removed Rather Than Given a Use

`src/lib.rs:7-10`:

> `ring_seqno` was scaffolded into this crate's manifest before the
> implementation existed and is not among them. Publishing is a cursor advance
> and a contiguity test; it computes no distances, no free slots and no
> minimum. The same over-declaration was found and removed in `ring_claim`.

Two crates were scaffolded with a `ring_seqno` edge and neither turned out to need
one — the two halves of the same feature, over-declared the same way, caught by
the same check. `ring_seqno` is Tier 1 arithmetic (`pending`, `free_slots`,
distances); publication computes none of those. The end of a publication is
`start.advanced_by( len as u64 )`, which is `ring_types`' own method on `Seq`,
not a `ring_seqno` call ([`type/001`](../type/001_a_seq_a_usize_and_the_one_cast.md)).

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_compare_exchange_that_refuses.md](../algorithm/001_the_compare_exchange_that_refuses.md) | What the two dependencies are actually used for |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_six_methods_and_no_caller.md](../api/001_six_methods_and_no_caller.md) | The surface nothing calls, item by item |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_four_cursors_of_the_handshake.md](../data_structure/002_the_four_cursors_of_the_handshake.md) | What the four dev-dependencies contribute to the reached-test |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_refused_rather_than_reordered.md](../decisions/001_refused_rather_than_reordered.md) | The choice `ring_mpsc` declined over |

### Integrations

| File | Relationship |
|------|--------------|
| [002_the_two_crates_that_declined.md](002_the_two_crates_that_declined.md) | Why the dependent count is zero, in the declining crates' own words |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_four_operation_handshake.md](../lifecycle/002_the_four_operation_handshake.md) | The test the four dev-dependencies exist for |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_loom_seam_and_its_only_user.md](../workaround/001_the_loom_seam_and_its_only_user.md) | `ring_atomic`'s seam, and why this crate's test is its only consumer |
| [../workaround/002_four_dev_dependencies_that_look_like_a_cycle.md](../workaround/002_four_dev_dependencies_that_look_like_a_cycle.md) | What the dev-dependency section costs the family's readability |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/Cargo.toml:8-22` | Both dependency sections, the explaining comment, and the `loom` target |
| `ring_publish/src/lib.rs:5-10` | The declared dependencies and the removed one |
| `ring_mpsc/Cargo.toml:8-11` | The family's only other manifest mention — a record of removal |
| `ring_barrier/src/lib.rs:30-44` | The signature this crate's test changed |
| `ring_atomic/src/lib.rs:59-60` | The `loom` seam's named consumer |
| `ring_claim/src/lib.rs:18,22,106,281,298,339` | The six references from the other half of the feature |
| `ring_consume/src/lib.rs:10` | The reached-test, named from the consumer side |
| `ring_cursor/src/lib.rs:80` | This crate among the five that import `Cursor` to read a slot's safety |
| `ring_event/src/lib.rs:152` | The publish path this crate's is explicitly *not* |
| `ring_index/src/lib.rs:13` | This crate among the four that never fold an index |
| `ring_testkit/src/lib.rs:59` | This crate among the test-surface set |

### Tests

| File | Relationship |
|------|--------------|
| `tests/handshake_test.rs` | The file the four dev-dependencies exist for |
| `tests/manual/readme.md § P5` | Every declared dependency is actually used — two commands, two sections |
| `tests/manual/readme.md § P6` | `loom` declared only under `[target.'cfg(loom)']`, never plainly |
