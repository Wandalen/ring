# Decisions — ring_debug

Open questions and the closed ones worth keeping the reasoning for. Every entry
is either **Closed** with what settled it, or **Pending** with what would settle
it — nothing is left as a preference.

Two of them are recorded at full length as their own records, because their
reasoning is longer than a paragraph and is cited from elsewhere in the corpus:

| ID | Name | Rules | Status |
|----|------|-------|--------|
| 001 | [Four Edges, Not Two](001_four_edges_not_two.md) | What this crate is allowed to depend on | accepted |
| 002 | [A Watch Does Not Latch](002_a_watch_does_not_latch.md) | Whether the instrument remembers a fault | pending |

## Closed

### Closed 1 — Does this crate have a reason to exist beyond belt-and-braces?

**Yes, and it was measured rather than argued.** The question was live: a family
whose sequences are monotonic by construction does not obviously need a runtime
checker for monotonicity, and "check the invariants" reads as ceremony.

What settled it is the reading in
[`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md). A
D1-corrupted ring does not merely fail to report the corruption — it reports
`free_slots = capacity`, `pending = 0`, `may_claim = true`, which is a fresh
empty ring, and the last of those is the reading a producer acts on. The failure
is silent, safe-looking, and lands on the one value that decides whether unread
slots get overwritten.

Recorded because the alternative conclusion — "the type system already ensures
this" — is reasonable-sounding and wrong, and someone will reach it again.

### Closed 2 — Should `check` and `Watch` be one entry point?

**No.** D3 costs the caller state and a baseline taken before the corruption; D1
and D2 cost neither. Folding them together produces either a hidden global or a
`check` that appears to check everything and silently does nothing about D3.
The split is in [`api/001`](../api/001_the_check_surface.md).

### Closed 3 — Should `ring_seqno` be made defensive instead?

**No**, and this is the decision the crate is the consequence of. A `checked_sub`
in `Seq::distance_to` would put a branch and an error path on the family's
most-executed read, to catch a state a correct program never reaches. The
pitfall's P4 states it; [`nfr/001`](../non_functional_requirement/001_absent_unless_called.md)
is the set of constraints that keep this crate the cheaper choice it claims to be.

### Closed 4 — Two dependency edges or four?

**Four.** The initial design assigned `ring_debug → ring_core, ring_cursor`.
Reading a cursor at all needs `ring_atomic::SeqCell` in scope, because
`PaddedCursor`'s `load`/`store` are trait methods; and `Seq`/`Capacity` appear in
every public signature, which needs `ring_types`.

Worth recording because the two-edge design was not careless — it assumed the
*derived* readings would suffice, and the whole content of
[`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md) is that
they do not. The missing edges are a consequence of the finding, not an oversight
corrected.

## Pending

### Pending 1 — Should `ring_core` expose `position()` on its ends?

**What is undecided:** not whether a ring end should expose its position at
all — `ring_spsc`'s already do, on both ends — but the narrower question of
whether the wrapper should forward what the wrapped type already exposes.

**Why it matters:** without it, `check` and `Watch` are unreachable from the
family's top-level type, and the only check a `ring_core` caller can run is
`check_ends` — which cannot see D1, because it is built from the readings that
mask it. That is
[`integration/001`](../integration/001_reaching_the_cursors_of_a_live_ring.md)'s
J4, and it is the crate's central limitation.

**Why it is not decided here:** it widens the public surface of one of the five
exported crates (gate G5), for the benefit of a consumer that is not itself
exported. The decision belongs in `ring_core`'s own docs, not in a diagnostic
crate's. Recorded as deferred blocker (hh).

**What would settle it:** a second consumer wanting the raw sequences, or a
concrete corruption investigation that stalls on their absence. Either turns a
speculative widening into a demanded one.

### Pending 2 — Should a `Watch` latch a fault?

**What is undecided:** whether a `Watch` that has once reported a violation
should keep reporting one, even after the ring returns to a valid state.

**Current behaviour:** it does not — T6 in
[`lifecycle/001`](../lifecycle/001_from_one_observation_to_a_sequence.md)
is a real transition. A cursor written wrongly and then written correctly leaves
a watch that passes again.

**The argument each way:** a latch would mean the instrument remembers, which is
what an investigator usually wants. Not latching means the instrument reports
only what it can currently observe, which is what a *checker* should do — and
what "recovered" means is a question about the caller's model of the ring, not
about the cursors.

**What would settle it:** an actual investigation where the un-latched behaviour
lost information that mattered. Adding a latch on speculation would be a guess
about how the crate gets used, made before it has been used.

### Pending 3 — Is `Hash` on `Cursor` worth keeping?

**What is undecided:** the derive is used by nothing.

**Why it is still there:** a two-variant fieldless enum deriving `Hash` costs
nothing, and its absence is discovered at the moment someone wants a
`HashMap< Cursor, _ >`.

**Why it is recorded rather than left silent:** an unused derive is exactly what
a later reader is right to question, and finding no reasoning is worse than
finding this one. The same question is open on `ring_types::Backend`'s `Hash`
(deferred blocker (w)); if that is resolved by deletion, this should be
reconsidered on the same grounds rather than surviving by inattention.

## Sources

| File | Relationship |
|------|--------------|
| [`../pitfall/001_saturating_arithmetic_reports_health.md`](../pitfall/001_saturating_arithmetic_reports_health.md) | Closed 1 and Closed 3's measurement |
| [`../integration/001_reaching_the_cursors_of_a_live_ring.md`](../integration/001_reaching_the_cursors_of_a_live_ring.md) | Pending 1's cost |
| [`../lifecycle/001_from_one_observation_to_a_sequence.md`](../lifecycle/001_from_one_observation_to_a_sequence.md) | Pending 2's T6 |
| [`../type/001_violation.md`](../type/001_violation.md) | Pending 3's derive |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_debug/docs/decisions
printf 'records:                  %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### DB[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| DB[0-9]+ ' readme.md )"
printf 'closed / pending entries: %s / %s\n' \
  "$( command grep -c '^### Closed ' readme.md )" "$( command grep -c '^### Pending ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| DB13 | `Cargo.toml` against the design document | n/a — drift | The design document assigns two dependency edges and the manifest declares four, with nothing reconciling them and no test that would fail if one were removed. |
| DB14 | Pending 1 | **misleading doc** | The deferred widening is framed as adding a new capability to a ring end, and `ring_spsc` already offers exactly that accessor on both of its own ends. |
| DB15 | `Watch::observe` recovery | n/a — coverage | The transition this decision is entirely about is named in the lifecycle document and walked by none of the 22 tests. |
| DB16 | Pending 2 Consequences | n/a — observation | The deferral's cost is stated as cheap by the one measure that can be counted, and the cost that cannot be counted is the one a caller pays. |
