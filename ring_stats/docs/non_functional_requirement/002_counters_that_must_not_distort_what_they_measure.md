# Non-Functional Requirement: Counters That Must Not Distort What They Measure

### Scope

**Purpose:** Record the requirement `ring_stats` does not state — that a counter must
not change the thing it counts — and what the crate gives a caller who has to hold it.

**Responsibility:** The unqualified cheapness claim, the absence of any opt-out, and
the one consumer that had to reason its way past both.

**In Scope:** `ring_stats/Cargo.toml`; the module comment in
`ring_stats/src/lib.rs`;
`ring_bench/docs/pitfall/002_a_counter_inside_the_timed_region_measures_itself.md`.

**Out of Scope:** What the counters actually cost is
[`non_functional_requirement/001`](001_cheap_enough_to_leave_on.md). `ring_bench`'s
own mitigation and its five failure modes belong to that crate and are not restated
here.

---

## What a Caller Who Cannot Afford Them Is Offered

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what the crate offers a caller who cannot afford the counters --'
printf '    [features] in the manifest: %s   cfg(feature) in src: %s   no-op or disabled form: %s\n' \
  "$( command grep -c '^\[features\]' ring_stats/Cargo.toml || true )" \
  "$( command grep -c 'cfg( feature\|cfg(feature' ring_stats/src/lib.rs || true )" \
  "$( command grep -ci 'noop\|no_op\|disabled\|enabled' ring_stats/src/lib.rs || true )"
echo '  -- and what the one consumer that faced the question had to write --'
command grep -m1 -A6 -F '**"Cheap enough to leave on" is a claim about a production write path, and this' ring_bench/docs/pitfall/002_a_counter_inside_the_timed_region_measures_itself.md
echo '  -- the sharded mitigation, and why it was rejected --'
# anchored on the row's opening clause only: the rest of the cell is that
# crate's own reasoning and is revised as its findings are dispositioned
command grep -m1 -F '| Give each producer its own counter and sum afterwards' ring_bench/docs/pitfall/002_a_counter_inside_the_timed_region_measures_itself.md
```

Live output:

```
  -- what the crate offers a caller who cannot afford the counters --
    [features] in the manifest: 0   cfg(feature) in src: 0   no-op or disabled form: 0
  -- and what the one consumer that faced the question had to write --
**"Cheap enough to leave on" is a claim about a production write path, and this
crate is not one.** In production the counter's cost is amortised against work
that dwarfs it and, more importantly, is paid *identically* by whatever the
alternative would have been. In a comparison, the same cost is paid differently
by each candidate and is attributed to the candidate rather than to the counter.
The feature's sentence is true and the inference from it — leave them on here
too — is not.
  -- the sharded mitigation, and why it was rejected --
| Give each producer its own counter and sum afterwards, *incrementing it once per record* | Removes contention, but still keeps a `fetch_add` per record on every path — the cost this mitigation exists to remove. A single harness-owned atomic incremented once *per thread* at close, not per record, is a different shape and is what the two threaded runners actually do (`reported` in `run_direct_mpsc`/`run_mutex_queue`, → BN43) |
```

---

### ST35 — The Cheapness Claim Is Unqualified, and Its First Real Consumer Had to Publish the Qualification

`ring_stats` says the counters are cheap enough to leave on, twice — once restating
its own original requirement in the module comment's opening paragraph, once in its own voice in the
ordering paragraph below it. It
does not say when they are not, and it names no condition under which a caller should
decline them.

There is one such condition and it is not exotic. If the work being counted is also
the work being timed, the counter's cost is inside the measurement. Worse, that cost is
not uniform across whatever is being compared: it is an uncontended `fetch_add` for a
single-producer path and a contended one for a multi-producer path, so the
instrumentation is most expensive on exactly the candidates whose advantage is
multi-producer throughput.

`ring_bench` is the crate that hit this, and it had to state the qualification itself:
"The feature's sentence is true and the inference from it — leave them on here too —
is not."

**Finding.** That sentence is the qualification missing from this crate's own module
documentation. `ring_bench` reached the right answer, wrote a full pitfall instance to
record how, and paid for it in what it gave up — no intra-run distribution, no drops
per second, no burst profile, because every counter is now written once from totals
after the clock stops.

The next consumer facing the same question gets none of that. What `ring_stats` tells
it is that the counters are cheap enough to leave on, in a module comment that argues
carefully about a fence cost measuring roughly nothing
([`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md)
§ ST14) and says nothing about a cache-line cost measuring 2.6×–3.3×
([`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md) § ST10).
The one sentence that would generalise `ring_bench`'s finding across the family sits
in `ring_bench`.

---

### ST36 — There Is No Way to Turn Them Off and No Sharded Form to Turn To

A caller who concludes the counters cost too much has three options in principle:
compile them out, shard them so producers stop contending, or stop calling the
recorders.

The manifest has no `[features]` section. The source has no `cfg( feature = … )`, no
no-op variant, no enabled flag — zero occurrences of any of it. The first option does
not exist. `RingStats` is one struct with seven `AtomicU64` fields and five recorders
that unconditionally execute a `fetch_add`, and the only way not to pay is not to
call.

The second option is the one that would actually raise the ceiling — per-producer
counters summed on read remove the shared line entirely — and it appears in
`ring_bench`'s *rejected* table, one of its stated reasons being that it "adds
per-thread state the candidates do not otherwise have". The state is per-thread
precisely because `ring_stats` does not offer it; had the crate shipped a sharded
recorder, the bookkeeping would have belonged to the counters rather than to the
harness, and the objection would not apply in that form.

**Finding.** So the third option is the only one available, and it is the one
`ring_bench` took: call the recorders four times per run instead of once per record.
That works, and it converts a per-event instrument into a per-run summary — the
counters stop being counters and become a report.

The requirement in this instance's title is one `ring_stats` never states and cannot
currently help with. A crate whose entire purpose is instrumentation ships no
mechanism for controlling the instrument: no feature gate, no sharded form, no
documented guidance on where the recorders do not belong. Every consumer that needs
any of that has to invent it, and so far exactly one has had to, and did.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/001`](001_cheap_enough_to_leave_on.md) | What the counters actually cost, and the ceiling neither lever moves |
| [`decisions/001`](../decisions/001_relaxed_with_a_reason_that_covers_one_load.md) | The cost the module comment argues about instead |
| [`integration/002`](../integration/002_the_read_path_and_the_removed_edge.md) | `ring_bench` as the crate that reads and writes these counters |
| [`data_structure/001`](../data_structure/001_seven_counters_on_one_line.md) | The shared line that makes sharding the fix and packing the problem |

### Sources

| Fact | Where |
|------|-------|
| No features, no cfg, no no-op form | Census above |
| The unqualified claim, twice | The module comment's opening and ordering paragraphs |
| The qualification, written by the consumer | `ring_bench/docs/pitfall/002_a_counter_inside_the_timed_region_measures_itself.md:28-29` |
| The sharded mitigation and its rejection | Same file, line 93 |
| What the accepted mitigation gives up | Same file, failure C4 |

### Tests

| Test | Covers |
|------|--------|
| `batched_and_single_recording_agree` | The property that makes per-run totals equivalent to per-event calls |
| `recording_zero_changes_nothing` | The degenerate call a disabled path would make, if one existed |
| *(to create)* | Nothing exercises the crate with recording disabled, because there is no way to disable it |
