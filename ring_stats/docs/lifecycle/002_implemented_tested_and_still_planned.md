# Lifecycle: Implemented, Tested, and Still Planned

### Scope

**Purpose:** Record where `ring_stats` sits in the project's own lifecycle record —
what the feature docs say its state is, against what the crate is — and what that
record can and cannot report.

**Responsibility:** This crate's own tracked `Status` field, the same field across all other
tracked records, and the one downstream record that is the only documented consumer of these counters.

**In Scope:** This crate's own tracked feature-record status entries;
`ring_stats/src/lib.rs`; `ring_stats/tests/stats_test.rs`.

**Out of Scope:** The lifecycle of a counter set at runtime is
[`lifecycle/001`](001_zero_to_reset_to_zero.md). What the feature asked for against
what shipped is
[`decisions/002`](../decisions/002_per_policy_drop_counters_stated_and_delivered.md).
Whether the counters are in fact cheap enough to leave on is
[`non_functional_requirement/001`](../non_functional_requirement/001_cheap_enough_to_leave_on.md).

---

## The Record, the Crate, and the Consumer

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the feature record says ring_stats is --'
command grep '^\- \*\*Status:\*\*' docs/feature/185_ring_stats.md
echo '  -- what the crate is --'
printf '    src lines %s   public methods %s   tests %s\n' \
  "$( wc -l < ring_stats/src/lib.rs )" \
  "$( command grep -c 'pub \(const \)\?fn ' ring_stats/src/lib.rs || true )" \
  "$( command grep -c '^fn ' ring_stats/tests/stats_test.rs || true )"
# Both censuses below print shape rather than population. Other workstreams flip
# their own blocks between runs, so a raw tally or a full run listing goes stale
# without anything about this crate changing -- which is itself the finding.
echo '  -- and what Status says across every feature record --'
command grep -h '^\- \*\*Status:\*\*' docs/feature/*.md | sort | uniq -c | sort -rn \
  | awk 'NR==1{ printf "    majority: %s\n", $NF } NR==2{ printf "    minority: %s\n", $NF } END{ printf "    distinct values: %d\n", NR }'
echo '  -- present is not scattered: it is whole workstreams, marked in blocks --'
command grep -l '^\- \*\*Status:\*\* present' docs/feature/*.md \
  | sed -E 's#.*/([0-9]+)_.*#\1#' | sort -n \
  | awk 'NR==1{s=$1;p=$1;next} $1==p+1{p=$1;next} {n++; if(p-s+1>m)m=p-s+1; s=$1; p=$1} END{n++; if(p-s+1>m)m=p-s+1; printf "    contiguous runs: %d\n    widest run:      %d features\n", n, m}'
echo '  -- and the run that contains this crate feature --'
command grep -l '^\- \*\*Status:\*\* present' docs/feature/1[678]*.md \
  | sed -E 's#.*/([0-9]+)_.*#\1#' | sort -n | awk 'NR==1{f=$1} END{ printf "    %s-%s (%d features)\n", f, $1, NR }'
echo '  -- the consumer feature, and the counters it names --'
command grep '^\- \*\*Status:\*\*\|ring counters' docs/feature/353_spatial_and_ring_stats_overlay.md
```

Live output:

```
  -- what the feature record says ring_stats is --
- **Status:** present
  -- what the crate is --
    src lines 518   public methods 16   tests 22
  -- and what Status says across every feature record --
    majority: planned
    minority: present
    distinct values: 2
  -- present is not scattered: it is whole workstreams, marked in blocks --
    contiguous runs: 8
    widest run:      22 features
  -- and the run that contains this crate feature --
    167-188 (22 features)
  -- the consumer feature, and the counters it names --
- **Status:** planned
An overlay reporting the spatial index's own counters (cells touched per query) alongside the concurrency substrate's ring counters (drops), read while the world runs. The source marks it optional: it is the diagnostic layer above the committed core surface, included when the cost of the counters is already being paid and dropped when it is not.
```

---

### ST31 — The Status Field Tracks Workstreams, So It Reports Nothing About This Crate

This crate's own feature record is marked `present`, and the crate it specifies is implemented source
behind sixteen public methods, with twenty-one tests that pass — 259 lines, fourteen
methods and seventeen tests when this was written, 484 lines now.
The record and the code now agree.

**Correction (2026-09-28):** this paragraph read "twenty-one tests that pass" and
"484 lines now". The census above now reads 518 lines and twenty-two tests; method
count is unchanged at sixteen. Growth is ordinary maintenance rather than a new
finding — the paragraph's own point, that the field and the code happen to agree,
is unaffected.

**They agree by coincidence.** This instance was filed when this crate's own record read `planned`,
on the argument that `planned` was not a claim about `ring_stats` at all — it was
the value every record was created with, on a field maintained exactly once, for
one contiguous orbital-mechanics run (`104`-`125`, plus `419`). Of 426 records at
the time, 402 said `planned` and 23 said `present`.

The field has since been maintained again, and the shape of that maintenance is
the finding. `planned` still leads and `present` still trails — the census above
names which is which rather than counting either, because both move — and
`present` is not scattered across individual features: it falls in eight
contiguous runs, the widest 22 features wide. That widest run is `167`-`188`,
this family's own feature-tracking range, this crate's included. This crate's own feature record did not
flip because anyone checked `ring_stats`. It flipped because the block containing
it was marked.

**The census printed a population until this round, and the population is what
kept going stale.** A different workstream's run extended by one feature between
two runs of this document's own gate — nothing about `ring_stats` changed, and
nothing about the feature that joined was checked against this crate either. The
recipe above now prints run count and widest run instead of a tally, because the
tally was measuring other people's workstreams. That a field can go stale here
without anything here changing is not a defect in the recipe; it is the same
claim the finding makes, arriving as a maintenance cost.

**Finding.** So the original reading survives its own correction, and is
sharpened by it. The `Status` field is a per-workstream marker rendered once per
feature, and a per-workstream marker cannot report a per-feature fact. `present`
is exactly as uninformative about this crate as `planned` was, for exactly the
same reason — and it is now uninformative in the more dangerous direction, because
a reader who sees `planned` against working code goes looking for the
discrepancy, while a reader who sees `present` against working code stops.

The two structural gaps below are what a per-feature field would have shown, and
neither flipping the block nor leaving it would have surfaced either. A lifecycle field
that distinguished specified from implemented would be the natural place for this
crate's two structural gaps to surface: `record_wait` has no producer anywhere in the
workspace ([`item/001`](../item/001_five_recorders_and_the_one_nothing_calls.md)
§ ST26), and `reset`'s doc names `ring_shutdown` as its caller, a crate with no
dependency edge to this one
([`integration/001`](../integration/001_the_write_path_and_two_callers_that_are_not_there.md)
§ ST18). Both are exactly the kind of half-delivery a maintained status would show.

Neither is visible from the record, before the block flip or after it. The crate
reads as complete from the code — every method exists, every method is tested —
and now reads as complete from the docs too, and neither reading is the true one,
which is that two of this crate's four originally-named counters ship with no live way to
become nonzero. The flip moved the record from wrong-and-conspicuous to
wrong-and-quiet without touching the fact underneath.

This instance keeps the name it was filed under. "Still planned" describes the
state that produced the finding, not the state today, and the finding it produced
outlived the condition — renaming it would cost every citation and buy a filename
that agrees with a field the finding's own conclusion says means nothing.

**Disposition:** declined — the fix this finding actually points at is the
project's own per-feature `Status` field convention and the identical
per-workstream-block maintenance applied across all its tracked records,
not anything in this crate's own `src/`, `docs/`, or
`Cargo.toml`; the finding's own closing paragraph explicitly declines to rename
or otherwise alter this instance, so there is no in-scope edit here to make.

---

### ST32 — The Only Documented Consumer Wants One Counter and Is Explicitly Droppable

The sole external record that reads
these counters rather than produces them states: "an overlay reporting the spatial index's
own counters (cells touched per query) alongside the concurrency substrate's ring
counters (drops), read while the world runs."

It names one counter. `drops` — which in this crate is three counters behind an enum
plus a fourth that sums them
([`data_structure/002`](../data_structure/002_three_drop_counters_behind_one_enum.md)),
and whose only non-benchmark producer sits in a function no `src/` imports. Nothing
downstream asks for `claimed`, `published`, `consumed`, `wait_nanos` or `in_flight`.

And it carries its own escape clause, quoted from the source: the overlay is
"included when the cost of the counters is already being paid and dropped when it is
not."

**Finding.** The condition on which the crate's only documented consumer would drop it
is a cost — and that cost is the one property of this crate nobody has measured.
This crate's own stated requirement asserts the counters are "cheap enough to leave on" as a bare claim; the
crate's own module comment argues at length about an ordering cost that measures
within ten percent of nothing, while the packed seven-counter layout it ships measures
2.6×–3.3× slower than a padded one under contention
([`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md) § ST10,
[`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md)
§ ST14).

So the lifecycle reads: requested by name in this crate's own original design record, specified as four counters
in this crate's own feature record, implemented as seven with sixteen methods and twenty-one tests,
recorded as `planned` on a field that says `planned` about almost everything, and
awaited by one optional consumer that wants a seventh of what shipped and reserves the
right to drop it on a cost that has never been taken.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`lifecycle/001`](001_zero_to_reset_to_zero.md) | The runtime lifecycle of one counter set, rather than the crate's |
| [`decisions/002`](../decisions/002_per_policy_drop_counters_stated_and_delivered.md) | Four counters asked for, seven delivered |
| [`item/001`](../item/001_five_recorders_and_the_one_nothing_calls.md) | The requested counter with no producer |
| [`non_functional_requirement/001`](../non_functional_requirement/001_cheap_enough_to_leave_on.md) | The cost claim the consumer's escape clause turns on |

### Sources

| Fact | Where |
|------|-------|
| Its "cheap enough to leave on" claim | [`non_functional_requirement/001`](../non_functional_requirement/001_cheap_enough_to_leave_on.md)'s own census |
| 402 of 426 records say `planned` | Census above |
| The 23 `present` records are 104–125 plus 419 | `docs/feature/`, by filename |
| The overlay names `drops` and marks itself optional | Census above |
| The overlay is itself `planned` | Census above |

### Tests

| Test | Covers |
|------|--------|
| `each_recorder_moves_exactly_one_counter` | That all five recorders, including the one with no producer, work |
| `a_fresh_set_is_all_zero` | The initial state the record says is unbuilt |
| `counts_are_exact_under_contention` | That the implementation the record calls `planned` is correct |
| *(to create)* | Nothing relates a feature record to the crate implementing it, in either direction |
