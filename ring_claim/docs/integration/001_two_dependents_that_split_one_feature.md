# Integration: Two Dependents That Split One Feature

### Scope

- **Purpose**: Record this crate's position in the family's dependency graph — three dependencies, two dependents in two different manifest sections — and the fact that its two dependents adopted opposite halves of the same feature.
- **Responsibility**: Establish the edges by command, separate the real dependency from the dev-dependency, and place `ring_claim` against its three Tier 5 siblings.
- **In Scope**: Every edge into and out of this crate, and the prose references that are not edges.
- **Out of Scope**: What the incoming edges actually *use* — see [`integration/002`](002_four_predicates_and_the_one_that_is_called.md).

### The Edges

```sh
cd "$(git rev-parse --show-toplevel)"

# out — what this crate depends on
awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_claim/Cargo.toml \
  | grep -oE '^ring_[a-z_]+'

# in — every manifest in the family that names this crate, with its section
for m in */Cargo.toml; do
  awk -v f="$m" '/^\[/{s=$0} /ring_claim/{printf "%s  %s  %s\n", f, s, $0}' "$m"
done
```

Live output:

```
ring_types
ring_cursor
ring_gating
ring_claim/Cargo.toml  [package]  name = "ring_claim"
ring_mpsc/Cargo.toml  [dependencies]  ring_claim = { path = "../ring_claim" }
ring_publish/Cargo.toml  [dev-dependencies]  ring_claim = { path = "../ring_claim" }
```

| Direction | Section | Crates |
|-----------|---------|--------|
| out | `[dependencies]` | `ring_types`, `ring_cursor`, `ring_gating` |
| **in** | `[dependencies]` | `ring_mpsc` |
| **in** | `[dev-dependencies]` | `ring_publish` |

Three manifests contain the string `ring_claim`: this crate's own `name =`
field, and the two above. That is the whole graph.

### CL1 — The Only Tier 5 Primitive Whose Two Dependents Are in Different Sections

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_*/; do n=$( basename "$c" )
  t=$( grep -m1 -oE 'Tier [0-9]+' "$c/src/lib.rs" 2>/dev/null )
  real=$( for m in */Cargo.toml; do
            awk -v p="$n" '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f && $0 ~ "^"p" = "{print}' "$m"
          done | wc -l )
  dev=$( for m in */Cargo.toml; do
           awk -v p="$n" '/^\[dev-dependencies\]/{f=1;next} /^\[/{f=0} f && $0 ~ "^"p" = "{print}' "$m"
         done | wc -l )
  printf "%-16s %-8s real=%s dev=%s\n" "$n" "$t" "$real" "$dev"
done | grep 'Tier 5'
```

Live output:

```
ring_barrier     Tier 5   real=1 dev=1
ring_claim       Tier 5   real=1 dev=1
ring_consume     Tier 5   real=0 dev=1
ring_publish     Tier 5   real=0 dev=0
```

The four Tier 5 primitives divide on this:

| Crate | Real dependents | Dev-only dependents | What that means |
|-------|----------------:|--------------------:|-----------------|
| `ring_barrier` | 2 | 0 | used, plainly |
| `ring_consume` | 1 | 0 | used, plainly |
| `ring_publish` | 0 | 0 | never used ([`ring_publish/docs/integration/001`](../../../ring_publish/docs/integration/001_ten_crates_name_it_and_none_depends_on_it.md)) |
| **`ring_claim`** | **1** | **1** | **used once, and tested-against once** |

The dev-only edge is `ring_publish`'s, and it exists for exactly one file:
`ring_publish/tests/handshake_test.rs`, the claim/publish handshake's
reached-test. `ring_publish/Cargo.toml:12-14` states it:

> Tests only. `tests/handshake_test.rs` is feature 170's reached-test, which is
> the whole four-operation handshake — so it needs the other three operations'
> crates. None of them depends on this one, so the graph stays acyclic.

So the two halves of the claim/publish handshake are joined by an edge that
exists only under `cargo test`, pointing from the second half to the first. In
a release build they have never been linked together.

### CL2 — `ring_mpsc` Took the Claim Half and Rejected the Publish Half

`ring_mpsc/Cargo.toml:8-11` is the family's one manifest comment that
explains a dependency list rather than declaring one:

> Eight, not the seven this manifest was scaffolded with. `ring_publish` and
> `ring_consume` are gone and `ring_atomic`, `ring_slot` and `ring_types` are
> new — see decision 124. Publication here is a per-slot stamp this crate owns,
> so there is no published cursor for either removed crate to act on.

Both halves of the handshake were scaffolded into that manifest. One survived.
The asymmetry is not a preference for this crate — it is a consequence of what
each half *is*:

| Half | Shape | Survived `ring_mpsc`'s design? |
|------|-------|-------------------------------|
| claim | a cursor advanced by compare-exchange under a gate | **yes** — `ring_mpsc:203` `use ring_claim::Claimer` |
| publish | a cursor advanced by compare-exchange under a contiguity test | no — replaced by a per-slot stamp |

A published *cursor* requires publications to become visible in sequence order,
which is what `ring_publish` enforces by refusing out-of-order advances. A
per-slot stamp does not: each producer stamps its own slot and the consumer
scans. `ring_mpsc` chose the second, so it had a claim to keep and nothing for
the publisher to advance.

The claim half survived because there is no per-slot alternative to it.
Exclusivity is a property of the *range allocation*, not of any one slot, and
the only place it can be established is at the cursor.

### CL3 — Three Dependencies, and One of Them Carries This Crate's Arithmetic

`src/lib.rs:5` declares `ring_types`, `ring_cursor`, `ring_gating`. `src/lib.rs:7-13`
records what was removed and why:

> `ring_seqno` was scaffolded into this crate's manifest before the
> implementation existed and is not among them: every piece of sequence
> arithmetic claiming needs is either `ring_types::Seq`'s own (`advanced_by`,
> and the `Ord` that makes range containment a comparison rather than a
> subtraction) or already inside `ring_gating::GatingSet::headroom`, which
> reaches `ring_seqno` on this crate's behalf.

That last clause is the interesting one, and it is checkable:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^ring_' ring_gating/Cargo.toml
grep 'ring_seqno' ring_gating/src/lib.rs
```

Live output:

```
ring_types = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }
ring_seqno = { path = "../ring_seqno" }
//! Depends on `ring_types`, `ring_cursor`, `ring_seqno`.
      ring_seqno::free_slots( producer, slowest, self.capacity )
```

The middle line the old quote carried — `` `ring_seqno::slowest` returns `None` for an
empty set rather than `Seq::ZERO` `` — no longer matches this grep: `ring_gating`'s
own module doc now attributes `slowest` to `ring_cursor`, not `ring_seqno`
(`ring_gating/src/lib.rs:24`), so the live output above is one line shorter.
The claim this recipe exists to check is unaffected — `ring_seqno::free_slots` is
still the one real call.

`ring_gating` does depend on `ring_seqno`, and `GatingSet::headroom` is where the
distance arithmetic happens. So this crate reaches Tier 1 arithmetic through
exactly one call, in exactly one predicate, and declares no edge to it. The
same over-declaration was found and removed from `ring_publish`, whose module
doc names this crate when recording it — the two halves of the claim/publish
handshake were scaffolded the same way and corrected by the same check
(`tests/manual/readme.md § C5`).

Both removals were possible because neither half computes distances. Claiming
asks *how much room is there* and delegates; publishing asks *is my start the
current frontier* and compares.

### The Prose References That Are Not Edges

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -rl 'ring_claim' */src/*.rs | command grep -v ring_claim/
```

Live output:

```
ring_batch/src/lib.rs
ring_consume/src/lib.rs
ring_cursor/src/lib.rs
ring_gating/src/lib.rs
ring_index/src/lib.rs
ring_mpsc/src/lib.rs
ring_publish/src/lib.rs
ring_spsc/src/lib.rs
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/demo_orbital_rail/src/beat_s11.rs
```

| Crate | Where | What it says |
|-------|-------|--------------|
| `ring_mpsc` | `:5`, `:46`, `:187`, `:239` | the real dependent — and `:46` explains the CAS loop to its own readers |
| `ring_publish` | `:7-10`, `:152` | the removed `ring_seqno` edge, found in both crates by the same check |
| `ring_spsc` | `:10`, `:17-18` | why a single-producer ring needs none of it |

`ring_spsc:17-18` is the sharpest of the three, because it agrees with this
crate's own reasoning and then declines it anyway:

> `ring_claim` moves the producer cursor by compare-exchange, because two
> producers may race for the same range.

One producer cannot race itself. The crate that says so is the one that
demonstrates the whole justification for this crate's existence by negation:
remove the second producer and the compare-exchange loop, the gate inside the
retry, and the `must_use` all become unnecessary at once.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_the_gate_inside_the_retry.md](../algorithm/001_the_gate_inside_the_retry.md) | What the three dependencies are actually used for |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seventeen_items_and_nothing_that_drops_silently.md](../api/001_seventeen_items_and_nothing_that_drops_silently.md) | The surface the two dependents reach, item by item |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_borrow_that_is_half_the_type.md](../data_structure/002_the_borrow_that_is_half_the_type.md) | The borrow that shaped `ring_mpsc`'s public API |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_compare_exchange_rather_than_fetch_add.md](../decisions/001_compare_exchange_rather_than_fetch_add.md) | The choice `ring_spsc` does not have to make |

### Integrations

| File | Relationship |
|------|--------------|
| [002_four_predicates_and_the_one_that_is_called.md](002_four_predicates_and_the_one_that_is_called.md) | What the outgoing edge to `ring_gating` actually uses |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_range_from_grant_to_publication.md](../lifecycle/001_a_range_from_grant_to_publication.md) | The handshake the dev-dependency edge exists to test |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim/Cargo.toml` | The three declared dependencies |
| `ring_claim/src/lib.rs:1-23` | The tier, the dependencies, and the removed one |
| `ring_mpsc/Cargo.toml:8-11` | The manifest comment recording the dependency-change rationale |
| `ring_mpsc/src/lib.rs:46, 50, 56` | The real dependent's `use` and its two explanations |
| `ring_publish/Cargo.toml:12-16` | The dev-dependency section, and the reason for it |
| `ring_publish/src/lib.rs:7-10` | The same `ring_seqno` removal, in the other half |
| `ring_spsc/src/lib.rs:10, 17-18` | The crate that agrees with the reasoning and declines the crate |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md § C5` | Every declared dependency is used, and `ring_seqno` stays out |
| `ring_publish/tests/handshake_test.rs` | The one file the dev-dependency edge exists for |
