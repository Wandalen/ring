# Workaround: `ring_gating` as a Dev-Dependency

### Scope

- **Purpose**: Record why the crate this one is paired with is a dev-dependency, what that buys, and the one-line edit that would undo it.
- **Responsibility**: Give the three tests that need it, the coupling the split prevents, and a measured account of which half of the guarding check survives the edit.
- **In Scope**: The `[dev-dependencies]` placement, and manual check B4.
- **Out of Scope**: The ownership rule the split enforces — see [`pattern/001`](../pattern/001_the_borrowed_view_and_the_owned_set.md).

### The Position

```toml
# ring_barrier/Cargo.toml
[dependencies]
ring_types  = { path = "../ring_types" }
ring_cursor = { path = "../ring_cursor" }
ring_wait   = { path = "../ring_wait" }

# Tests only. Three of them assert the relationship between this crate's answers
# and `ring_gating`'s over one set of cursors — see the test file's own header on
# why a stand-in would assert nothing. The library itself never names it.
[dev-dependencies]
ring_gating = { path = "../ring_gating" }
```

Two crates implement one feature from opposite ends
([`integration/001`](../integration/001_three_dependencies_and_one_dependent.md)),
and the library half of this one does not name the other at all. The tests do,
three times.

### Why the Tests Cannot Use a Stand-In

```rust
// tests/barrier_test.rs:33-36
// what they assert is precisely the relationship between the two crates'
// answers: that the same cursors read from both sides give opposite answers on
// an empty set, and differently-clamped answers on a full one. Asserting that
// against a hand-rolled stand-in would be asserting it against nothing.
```

The three:

| Test | Line | Asserts | Against a stand-in |
|------|-----:|---------|--------------------|
| `available_ignores_capacity_entirely` | `:188` | `headroom == 4` while `available == 1_000`, one cursor | asserts my own stand-in clamps — nothing about `ring_gating` |
| `an_empty_barrier_and_an_empty_gating_set_answer_oppositely` | `:257` | `headroom == 8` while `available == 0`, empty set | same |
| `a_barrier_over_a_gating_set_reads_that_set_and_not_a_copy` | `:447` | `barrier.frontier() == set.slowest()`, same objects | same, and it is the *identity* of the cursors that is under test |

The third is the one that most needs a real `GatingSet`: what it asserts is that
`Barrier::over( set.cursors() )` reads the set's live cursors rather than a
snapshot of them ([`pattern/001`](../pattern/001_the_borrowed_view_and_the_owned_set.md)).
A stand-in built to be read live would make the assertion circular.

### What the Split Prevents

`ring_gating` does not depend on `ring_barrier`, so there is no cycle forcing
the split. And `ring_seqno` — `ring_gating`'s third dependency — is already
transitive here through `ring_cursor`:

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_barrier ring_gating ring_cursor; do
  echo -n "$c: "
  awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring/$c/Cargo.toml \
    | grep -oE '^ring_[a-z_]+' | tr '\n' ' '; echo
done
# ring_barrier: ring_types ring_cursor ring_wait
# ring_gating:  ring_types ring_cursor ring_seqno
# ring_cursor:  ring_types ring_seqno ring_atomic ring_align
```

Live output:

```
ring_barrier: ring_types ring_cursor ring_wait 
ring_gating: ring_types ring_cursor ring_seqno 
ring_cursor: ring_types ring_seqno ring_atomic ring_align 
```

So promoting the dependency would add **no new crate to the build** — the cost
is not compile time or graph size. What it would add is a *direct edge*, and
with it the ability to write `Barrier::over( &GatingSet )` again. That is the
signature that made `ring_publish/tests/handshake_test.rs` unwriteable, and
undoing it is what manual check B2 exists to prevent. The two crates are
siblings over `ring_cursor`; the edge would make them a chain.

### BR14 — Half the Guard Survives the Re-Promotion, Half Goes Silent

B4 is two commands, and the plan names this exact hazard:

> `ring_gating` is the one easy to *re-promote*: three tests assert the
> relationship between this crate's answers and that crate's over one set of
> cursors, and the shortest way to make them compile is to move the dependency
> back up, which would quietly restore the coupling B2 exists to prevent.
>
> — `tests/manual/readme.md` § B4

Simulated — the manifest with `ring_gating` moved up and `[dev-dependencies]`
emptied — the two halves behave differently:

| B4 half | Compares | Output after the re-promotion | Verdict |
|---------|----------|-------------------------------|---------|
| 1 | `[dependencies]` against `src/lib.rs` | **`ring_gating`** | **fires** — the library declares a dependency it never names |
| 2 | `[dev-dependencies]` against `tests/*.rs` | *(nothing)* | **silent** — an empty section has no left-hand input, so `comm -23` prints nothing |

Half 2's "expected: no output" is satisfied by two different states: every
dev-dependency is used, *or* there are no dev-dependencies at all. That is a
check whose passing condition includes its own subject having vanished.

Half 1 catches the edit, so the guard holds overall — but only because the same
edit is visible from the other side. If a future change gave the library a
genuine `ring_gating` use for some unrelated reason, half 1 would go quiet too
and nothing would be watching.

Reproduce the simulation on a copy — never on the real manifest:

```sh
cd "$(git rev-parse --show-toplevel)"
sed 's/^\[dev-dependencies\]/[dev-deps-REMOVED]/' ring_barrier/Cargo.toml \
  | sed 's|^ring_wait = .*|&\nring_gating = { path = "../ring_gating" }|' > ./-b4sim.toml
comm -23 \
  <( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ./-b4sim.toml \
     | grep -oE "^ring_[a-z_]+" | sort -u ) \
  <( grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )
# ring_gating
```

Live output:

```
ring_gating
```

### Why B4 Is Two Commands and Not One

A single `comm` over the whole manifest against `src/lib.rs` reports
`ring_gating` as unused — which it correctly is, and must be. The check would
then be permanently red for the correct configuration, and a permanently-red
check is one that gets ignored or deleted. Splitting it by section is what makes
"no output" mean something in both halves.

That is the shape of the workaround: the natural single check has the wrong
answer for the right state, so it is split into two checks whose union has the
right answer — at the cost of half 2 having a degenerate pass mode.

### What This Costs

| Cost | Detail |
|------|--------|
| No compile-time pairing | Nothing in the type system says these two are one feature — [`pattern/001`](../pattern/001_the_borrowed_view_and_the_owned_set.md) |
| The pairing lives in prose | The manifest comment, the test file header, and `src/lib.rs:30-44` all argue it; none of them fail |
| A degenerate pass mode | B4 half 2, above |
| The relationship is only asserted three times | Three tests over three cursor configurations — empty, clamped, and shared-identity |

### BR51 — The Manifest Comment Is the Only Place the Arrangement Is Explained

`Cargo.toml` carries three lines of prose under `[dev-dependencies]` saying
why `ring_gating` is there and why a stand-in would assert nothing. That comment
is load-bearing documentation living in a file no rustdoc renders, no test
reads, and no gate checks.

The same reasoning appears nowhere in `src/lib.rs`, whose module documentation
discusses the relationship between the two crates for forty lines without
mentioning that one is compiled into the other's tests. A reader who never opens
the manifest sees two crates that are carefully independent; the manifest is
where the exception is recorded.

```sh
cd "$(git rev-parse --show-toplevel)"
cat ring_barrier/Cargo.toml | tail -8
# the same reasoning in the module documentation
grep -c "dev-dep\|dev dependency" ring_barrier/src/lib.rs \
  || echo '(the module documentation never mentions the arrangement)'
```

Live output:

```
# Tests only. Three of them assert the relationship between this crate's answers
# and `ring_gating`'s over one set of cursors — see the test file's own header on
# why a stand-in would assert nothing. The library itself never names it.
[dev-dependencies]
ring_gating = { path = "../ring_gating" }

[lints]
workspace = true
0
(the module documentation never mentions the arrangement)
```

### Workarounds

| File | Relationship |
|------|--------------|
| [001_the_check_that_capacity_stays_out.md](001_the_check_that_capacity_stays_out.md) | The check whose load-bearing test needs this dependency |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/002_the_slices_three_provenances.md](../data_structure/002_the_slices_three_provenances.md) | `set.cursors()` as one of the three |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_a_slice_rather_than_an_aggregate.md](../decisions/002_a_slice_rather_than_an_aggregate.md) | The signature the re-promotion would make writeable again |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_three_dependencies_and_one_dependent.md](../integration/001_three_dependencies_and_one_dependent.md) | The four edges, and the one that is dev-only |
| [../integration/002_the_dependency_that_is_not_ring_seqno.md](../integration/002_the_dependency_that_is_not_ring_seqno.md) | The other edge this crate does not have |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_capacity_never_enters_the_arithmetic.md](../invariant/002_capacity_never_enters_the_arithmetic.md) | What the 4-vs-1000 test asserts |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_borrowed_view_and_the_owned_set.md](../pattern/001_the_borrowed_view_and_the_owned_set.md) | The ownership rule, and the test that produced it |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_two_empty_answers_look_like_a_bug.md](../pitfall/001_the_two_empty_answers_look_like_a_bug.md) | The opposite-answers test, and why prose was not enough |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/Cargo.toml` | The placement, and its comment |
| `tests/manual/readme.md` § B4 | Both commands, and the re-promotion hazard |
| `ring_barrier/src/lib.rs:30-44` | The module's own argument for the split |

### Tests

| File | Relationship |
|------|--------------|
| `tests/barrier_test.rs:28-36` | The header on why a stand-in asserts nothing |
| `tests/barrier_test.rs:179-194` | Clamped against unclamped |
| `tests/barrier_test.rs:248-263` | Opposite answers on an empty set |
| `tests/barrier_test.rs:436-449` | The set and not a copy |
