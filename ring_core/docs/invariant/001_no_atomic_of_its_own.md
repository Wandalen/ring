# Invariant: This Crate Adds No Atomic of Its Own

### Scope

- **Purpose**: State the constraint that keeps [`ring_spsc`](../../../ring_spsc/readme.md)'s zero-read-modify-write assertion true once its publishes are routed through this crate.
- **Responsibility**: The invariant, why it belongs here rather than at the backend, what would break it, and how it is checked.
- **In Scope**: Atomics, `unsafe`, and shared mutable state introduced by `ring_core` itself.
- **Out of Scope**: The atomics each backend uses internally (→ [`ring_spsc` invariant/002](../../../ring_spsc/docs/invariant/002_no_lock_in_the_path.md), [`ring_mpsc` invariant/002](../../../ring_mpsc/docs/invariant/002_publication_ordering.md)); `crossbeam-queue`'s own, which are upstream.

### Invariant Statement

**`ring_core` contains no atomic type, no `unsafe` block, and no shared mutable
state.** Every synchronising operation on the path belongs to a backend.

#### Why it belongs here

`ring_spsc` asserts something unusually strong: **zero read-modify-writes
across an entire run**. Not "few", not "uncontended" — zero. That assertion is
the payoff of its single-producer cardinality, and it is the property its whole
design argument rests on.

Every publish through this crate reaches that ring. So an `AtomicUsize`
incremented here — a push counter, a high-water mark, a dropped-record tally —
would be an RMW on the path, and `ring_spsc`'s assertion would fail with
**nothing in `ring_spsc`'s own dependency tree to blame for it**. The
counter would be two crates away, in a composition layer that ring's tests do
not build.

That is the whole argument for stating this at the composition point rather
than trusting it: the failure is remote from its cause, and the crate that
detects it cannot see the crate that caused it.

The corollary is that **instrumentation is not merely unnecessary here, it is
excluded on purpose**. `ring_stats` exists for it, and is
deliberately not a dependency:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -c 'ring_stats' ring_core/Cargo.toml
```

Live output:

```
0
```

A consumer who wants counters composes `ring_stats` *around* this crate, where
the cost is visible in their own dependency list, rather than paying it
invisibly in everyone's.

#### What would break it

| Change | Why it looks reasonable | What it costs |
|---|---|---|
| A push/pop counter for diagnostics | Cheap, useful, "just one atomic" | `ring_spsc`'s zero-RMW assertion, remotely |
| A cached `len` to avoid a backend call | An optimisation | Same, plus a second source of truth for occupancy |
| An `Arc< AtomicBool >` closed flag | The surface looks incomplete without `is_closed` | Same, **and** a handle-local copy of a flag that `ring_shutdown` owns — the precise failure that crate exists to prevent |
| `default-features = true` on `crossbeam-queue` | It is the default | Turns on `crossbeam-utils/std` — thread-parking machinery a spin-only queue never calls. Measured: it does *not* change which crates are pulled, only their features (→ [`integration/001`](../integration/001_family_dependency_seam.md)) |

The third row is why this crate has **no `is_closed`**, and that absence is a
consequence of this invariant rather than an oversight
(→ [`integration/002`](../integration/002_handle_surface_divergence.md)).

The fourth is why the manifest declares
`default-features = false, features = [ "alloc" ]` — a line that reads as tidy
and is actually load-bearing.

### Enforcement Mechanism

Mechanical, and it must stay mechanical — this is not a property a behavioural
test can assert, because the failure it prevents surfaces in a *different
crate's* test suite:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -nE '\b(Atomic[A-Za-z]*|unsafe|Ordering::|fetch_[a-z]+|compare_exchange)\b' \
  ring_core/src/lib.rs | command grep -vE '^[0-9]+:[[:space:]]*//' || true
```

Live output:

```
```

**Expected: no output.** The filter's anchoring matters more than usual here:
this crate's module documentation argues about atomics at length, and an
unanchored grep reports that argument as an implementation. The one raw hit is
prose, on the line stating the invariant.

Gate G6 covers the `unsafe` half family-wide — only declared, justified crates
may opt out of the workspace deny, and this crate is not on that list.

### Violation Consequences

**The damage is remote from its cause, which is the whole reason the invariant is
stated here rather than trusted.**

| What breaks | Where it surfaces | Why the trail is cold |
|---|---|---|
| `ring_spsc`'s zero-RMW assertion | `ring_spsc`'s own suite | The offending atomic is two crates above, in a composition layer that ring's tests never build. Nothing in `ring_spsc`'s dependency tree names it |
| Occupancy agreement | Wherever a reader compares `len()` against a drain | A cached `len` is a second source of truth; the two disagree only under contention, so the failure is intermittent |
| `ring_shutdown`'s single-flag guarantee | Anywhere a second closed-flag is consulted | A handle-local `AtomicBool` is precisely the duplication `ring_shutdown` exists to prevent, and it would look like a completeness fix |

None of these is caught by a behavioural test in this crate — a crate cannot
assert that it did *not* add a cost. That asymmetry is why enforcement above is
mechanical and why the grep is the gate rather than a convention.

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_family_dependency_seam.md](../integration/001_family_dependency_seam.md) | The six dependencies declared and the pointed absence of `ring_stats` |
| [../integration/002_handle_surface_divergence.md](../integration/002_handle_surface_divergence.md) | `is_closed`'s absence, which row 3 explains |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_feature_gated_code_reads_as_uncovered.md](../pitfall/002_feature_gated_code_reads_as_uncovered.md) | The other property here that only a command, not a test, can check |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_crossbeam_queue_as_interim_backend.md](../workaround/001_crossbeam_queue_as_interim_backend.md) | Why `default-features = false` is row 4 rather than a style preference |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_spsc/docs/invariant/002_no_lock_in_the_path.md`](../../../ring_spsc/docs/invariant/002_no_lock_in_the_path.md) | The assertion this invariant protects |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md` | C5 — the grep above, and the note on why anchoring it matters here specifically |

### CO23 — The Only `Atomic` in the File Is the Sentence Saying There Is None

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
printf 'Atomic in code:  '; grep -vE '^\s*(//|///|//!)' src/lib.rs | grep -c 'Atomic'
printf 'Atomic in prose: '; grep -c 'Atomic' src/lib.rs
printf 'unsafe:          '; grep -c 'unsafe' src/lib.rs
printf 'ring_stats dep:  '; grep -c 'ring_stats' Cargo.toml
# The command above can legitimately exit nonzero -- a count of zero, a diff
# that differs -- and the sweep that keeps this output current skips any block
# that exits nonzero, so without this the block freezes instead of refreshing.
:
```

Live output:

```
Atomic in code:  0
Atomic in prose: 1
unsafe:          0
ring_stats dep:  0
```

**Note what the first line does and the obvious version does not.** Filtering
`grep -n`'s output rather than the file requires an anchor that survives
`grep -n` omitting the filename on a single-file match; get it wrong and the
comment on line 59 (was 48) counts as code. The recipe above filters the file.

Four zeros and a one. The one is line 59 (was 48; shifted by this session's
`Fix(decision_121_link_pointed_at_docsrs_not_the_ruling)` correction to the
module doc comment just above it, 2026-09-11) — "There is accordingly no
`AtomicUsize` anywhere below" — which is the claim, not a counterexample.

### CO24 — The Invariant Is Asserted Nowhere and Enforced by Review

`ring_spsc`'s zero-RMW assertion is what would eventually notice, and only for
publish paths that route through `ring_spsc`. An atomic added to the crossbeam
arm, or to a counter beside the dispatch, would break nothing.

The check is one `grep` — the first line of the recipe in CO23 — and no gate
runs it. Recorded rather than fixed because adding a gate is a change to
`bench_harness`, not to this crate, and the family has no convention yet for
per-crate source assertions.
