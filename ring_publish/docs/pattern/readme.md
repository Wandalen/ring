# pattern

Two conventions, both instantiated by the same nine lines of code. One is about
shape — a fallible primitive with a blocking wrapper over it — and one is about
vocabulary: every memory ordering bound to a named constant that states its role
and names its pairing partner.

They are recorded together because each turns out to be a family-wide convention
this crate relates to unusually: it is the *only* legitimate instance of the
first, and it is one of eleven instances of the second, four of which share a name
with a crate they have no dependency edge to.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Try-and-Loop Over Compare-Exchange](001_try_and_loop_over_compare_exchange.md) | PB33 — the pattern in three parts, the family's thirteen `try_*` methods of which twelve have no twin, the two ways the pattern is normally wrong, and the loop census |
| 002 | [The Named Ordering Constant](002_the_named_ordering_constant.md) | PB34 — all eleven constants with visibility and value, four names shared across crates with no edge between them, and the four things the pattern does not buy |

### Pattern 1 — Try-and-Loop

| Part | Here |
|------|------|
| A fallible primitive attempting one exchange | `try_publish` |
| A blocking wrapper looping until it succeeds | `publish` |
| A hint, not a yield, between attempts | `core::hint::spin_loop()` |

Two properties make the conversion from *"not yet"* to *"eventually"* legitimate,
and both are unusual:

- **The retry target is fixed.** `start` and `len` never change between attempts.
  `ring_claim`'s superficially identical loop feeds the failure value back as the
  next attempt's input — that is a race for a moving target, this is a wait for a
  turn.
- **Success is guaranteed, not merely likely.** The predecessor is committed, so
  *loop until it works* is a termination argument rather than optimism.

The family's rule is *the caller decides what to do when it cannot proceed*, and
this crate is the single documented exception — defensible for exactly the reason
the rule exists. Everywhere else "cannot proceed" means the ring is full or there
is no data, states a peer may never leave. Here it means it is not your turn yet,
and turns always arrive.

### Pattern 2 — Named Orderings

| | Declared here | Imported |
|--|---------------|----------|
| Name | `PUBLISH` | `GATING` |
| Value | `Release` | `Acquire` |
| From | `src/lib.rs:67` | `ring_cursor:89` |
| Applied to | the exchange's success path | the exchange's failure path, and `published()`'s load |
| Documented with | role, partner, and the architecture where weakening is invisible | its own crate |

Both are passed to one call. The asymmetry is deliberate — a failed exchange
published nothing, so it needs no release, but it did read the cursor. Seven lines
of documentation for one line of code, and `§ P3` keeps it that way: four expected
lines, no inline `Ordering::` anywhere.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# every try_* method in the family, and every blocking-sounding twin
grep -rnE '^\s*pub (const )?fn try_[a-z_]+' ring_*/src/*.rs
grep -rnE '^\s*pub (const )?fn (push|push_batch|recv|recv_batch|clone|publish)\b' ring_*/src/*.rs

# every bare loop, and what its exit condition depends on
for f in ring_*/src/*.rs; do
  n=$( grep -vE '^[[:space:]]*//' "$f" \
       | grep -cE '^[[:space:]]*loop[[:space:]]*\{?[[:space:]]*$' )
  [ "$n" -gt 0 ] && printf "%-40s %s\n" "$f" "$n"
done

# every named ordering constant in the family
grep -rnE '^\s*(pub )?const [A-Z_]+ *: *(core::sync::atomic::)?Ordering' */src/*.rs 

# this crate's own four lines — expect exactly four
grep -vE "^[[:space:]]*//" ring_publish/src/lib.rs \
  | grep -nE "Ordering::|GATING|PUBLISH"
```

| | Value |
|--|------:|
| `try_*` methods in the family | 13, across 5 crates |
| …with a blocking twin of the same name | **1** |
| Blocking-sounding `push` methods that exist | 3 |
| …that actually block | **0** |
| Bare loops in `ring_*` libraries | 6, in 2 crates |
| …whose exit condition is another thread's action | **1** |
| Named ordering constants family-wide | 11, in 7 crates |
| Distinct names among them | 7 |
| Names appearing twice | 4 — `PUBLISH`, `OBSERVE`, `COMMIT`, `OWN` |
| …with a dependency edge between the two crates | **0** |
| Distinct ordering *values* among the eleven | 4 |
| Constants `ring_mpsc` holds | 4 |
| Constants this crate declares / imports | 1 / 1 |
| Lines `§ P3` expects | exactly 4 |
| Doc-comment lines on `PUBLISH` | 7, for 1 line of code |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB33 | family | n/a — observation | Thirteen `try_*` methods exist across five crates and twelve are the only form their operation has; the three blocking-sounding `push` methods belong to types with no `try_` twin and none of them loops |
| PB34 | family | n/a — observation | Eleven named ordering constants exist in seven crates; four names appear twice and no dependency edge connects either member of any pair, so the shared vocabulary is convention held in prose, not in code — nothing imports another crate's `PUBLISH`, asserts they are equal, or would notice if one changed |
| PB52 | family | n/a — observation | All three genuine compare-exchange sites pair a crate-local success constant with `ring_cursor`'s imported `GATING`; no site in the family names both orderings from one place, and `GATING`'s single declaration is why the failure half cannot drift |
