# Decision: Four Edges, Not Two

**Status:** accepted. The edges are in the manifest and the design document
that assigned two has not been amended.

### Scope

- **Purpose**: Record why this crate depends on four family crates when its design assigned it two, and what the two extra edges are evidence of.
- **Responsibility**: The context, the options, the decision, and the consequences of the edge count.
- **In Scope**: The four `[dependencies]` entries and what each is needed for.
- **Out of Scope**: What is done with the cursors once reached (→ [`algorithm/001`](../algorithm/001_checking_a_pair_without_touching_it.md)); the door that is missing (→ [`workaround/001`](../workaround/001_the_door_ring_core_does_not_open.md)).

### Context

The initial design assigned `ring_debug → ring_core, ring_cursor`. Two edges
is the natural count for a crate that checks a cursor pair belonging to a ring:
one for the pair, one for the ring.

It is also what you get if you assume the *derived* readings — `len`,
`free_capacity` — are enough to check with. They are not, and
[`pitfall/001`](../pitfall/001_saturating_arithmetic_reports_health.md) is the
measurement of why: a D1-corrupted ring reports `free_slots = capacity`,
`pending = 0`, `may_claim = true`, which is indistinguishable from a fresh empty
one. Checking against derived readings means checking against exactly the numbers
that hide the defect.

### Options

| Option | Edges | Consequence |
|---|---|---|
| Check derived readings only | 2 | D1 undetectable — the corrupted ring reads as healthy |
| Reach the raw cursors | 4 | `ring_atomic` for `SeqCell::load`, `ring_types` for `Seq`/`Capacity` in every signature |
| Have `ring_cursor` re-export what is needed | 2 | Widens a Contract crate's surface to narrow a consumer's manifest |

### Decision

**Four.** `PaddedCursor::load` and `store` are trait methods, so `ring_atomic`
must be in scope to call them at all; `Seq` and `Capacity` appear in every public
signature this crate declares, so `ring_types` is a public-API dependency rather
than an implementation detail.

The third option was rejected on the family's own terms: re-exporting through
`ring_cursor` moves the edge rather than removing it, and pays for a shorter
manifest with a wider Contract surface — the trade the family declines everywhere
else.

### Consequences

The manifest is the shortest true statement of what this crate reads. Two of its
four edges exist *because* of a finding, which makes the edge count itself a piece
of evidence rather than bookkeeping — and it is the reason the design document's
two-edge assignment was not careless but simply written before the measurement
existed.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '-- the declared edges --'
sed -n '/^\[dependencies\]/,/^\[/p' ring_debug/Cargo.toml | command grep -E '^ring_'
echo '-- what each is used for, in the source --'
S=ring_debug/src/lib.rs
command grep -E '^use ring_' $S
echo '-- and whether ring_core alone would have sufficed: does it expose a cursor? --'
printf 'ring_core fns named position: %s\n' "$( command grep -c 'fn position' ring_core/src/lib.rs || true )"
printf 'ring_spsc fns named position: %s\n' "$( command grep -c 'fn position' ring_spsc/src/lib.rs || true )"
```

Live output:

```
-- the declared edges --
ring_core = { path = "../ring_core" }
ring_cursor = { path = "../ring_cursor" }
ring_types = { path = "../ring_types" }
ring_atomic = { path = "../ring_atomic" }
-- what each is used for, in the source --
use ring_atomic::SeqCell;
use ring_core::{ Consumer, Producer };
use ring_cursor::CursorPair;
use ring_types::{ Capacity, Seq };
-- and whether ring_core alone would have sufficed: does it expose a cursor? --
ring_core fns named position: 0
ring_spsc fns named position: 2
```

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_saturating_arithmetic_reports_health.md](../pitfall/001_saturating_arithmetic_reports_health.md) | The measurement that turned two edges into four |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_reaching_the_cursors_of_a_live_ring.md](../integration/001_reaching_the_cursors_of_a_live_ring.md) | What the four edges do and do not reach |

### Sources

| File | Relationship |
|------|--------------|
| [`Cargo.toml`](../../Cargo.toml) | The four edges |
| [`src/lib.rs`](../../src/lib.rs) | The four `use` statements they support |

### Tests

| Test | Relationship |
|------|--------------|
| `the_arithmetic_reports_an_empty_ring_for_a_consumer_ahead_cursor` | The measurement that rejected the two-edge option, asserted directly |
| `the_arithmetic_reports_a_lapped_ring_as_full_and_unclaimable` | The same, for D2 |

### DB13 — the manifest is the only place the edge correction is recorded

The design document assigns two edges. The manifest declares four. Nothing
reconciles them: there is no amendment, no reconciling note, and no test
that would fail if an edge were removed for the wrong reason.

The correction survives only as the difference between two documents that are
never compared — and this crate's whole subject is readings that must be compared
to mean anything. **A dependency added because of a finding is indistinguishable,
in the manifest, from one added by habit**, and only the ADR above records which
these are.

The cost is bounded and worth stating precisely: nothing breaks today, and the
exposure is that a future reader tidying "unused-looking" edges finds
`ring_atomic` — which appears in no public signature, only in trait-method
resolution — and has no mechanical signal that removing it costs D1 detection.

### DB14 — the deferred widening has a working precedent one crate away

Pending 1 asks whether `ring_core` should expose `position()` on its ends, and
defers on the grounds that it widens an exported crate's surface for a
non-exported consumer.

`ring_spsc` already has exactly that method, twice — once on each end, at lines
499 and 776. So the question is not whether the accessor is a reasonable thing for
a ring end to offer; the family has already answered that affirmatively, one level
down, in the crate `ring_core` wraps.

**What is actually being deferred is narrower than the Pending entry states**:
not "should a ring end expose its position" but "should the wrapper forward what
the wrapped type already exposes". Recorded because the broader framing makes the
deferral look more conservative than it is, and a reader weighing the trade should
weigh the real one.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -A4 -F '### Pending 1' ring_debug/docs/decisions/readme.md
```

Live output:

```
### Pending 1 — Should `ring_core` expose `position()` on its ends?

**What is undecided:** not whether a ring end should expose its position at
all — `ring_spsc`'s already do, on both ends — but the narrower question of
whether the wrapper should forward what the wrapped type already exposes.
```

**Disposition:** applied — `decisions/readme.md`'s Pending 1 entry no longer
frames the deferral as whether a ring end should expose its position at all;
its "What is undecided" line now states the narrower question this instance
identifies — whether the wrapper should forward what the wrapped type already
exposes. Now prints: `the wrapper should forward what the wrapped type already exposes`

