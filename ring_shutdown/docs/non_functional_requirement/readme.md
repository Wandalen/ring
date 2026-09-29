# Non-Functional Requirement Doc Definition

### Scope

- **Purpose**: State what this crate's operations cost, on both the publish side and the teardown side, and record which of those costs has any measurement behind it — the answer is none of them.
- **Responsibility**: The per-record and per-batch cost of the guard; the two drain shapes and which one `reset` takes; the allocation the surface promises not to make and the one it pushes onto the caller unsized.
- **In Scope**: `Guarded::try_push`, `try_push_batch`, `is_blocked`; `Stopped::drain_all`, `discard_all`; `reset`.
- **Out of Scope**: Whether the operations are *correct* — orderings (→ [`../data_structure/001`](../data_structure/001_the_close_flag_and_its_orderings.md)), termination (→ [`../invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md)); what each item promises (→ [`../api/readme.md`](../api/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [What a Guarded Push Costs](001_what_a_guarded_push_costs.md) | One flag read per publish, once per batch, and the accessor that doubles it for nothing | 🔄 |
| 002 | [The Teardown Path Takes the Slow One](002_the_teardown_path_takes_the_slow_one.md) | Two drains, one record-at-a-time, and the one `reset` routes through | 🔄 |

**The split is the hot path against the cold one**, which is the split that
decides how much a cost matters. `001` is about work paid per published record,
where a wasted atomic load is a real tax; `002` is about work paid once per
teardown, where it is not, and the finding has to earn its place on other
grounds — which it does, by comparing the chosen shape against the one sitting
three lines away in the same `impl`.

The four findings pair up across that split rather than within it. **Two are
about a measurement that does not exist**: the crate's only recorded numbers
are coverage ratios, and the family has no `benches/` and no `criterion`
anywhere (SD33), while the one cost claim the surface does make — *"nothing on
this surface allocates"* — has no check behind it despite a sibling crate having
already built and documented the strongest safe check available (SD36). **Two
are about a cheaper path declined**: `is_blocked` invites a pre-flight idiom
that costs more and promises less than simply pushing (SD34), and `reset`
routes the crate's headline teardown through the record-at-a-time drain rather
than the batched one, paying for destructor semantics no type in the family's
reach uses (SD35).

Read together they describe a crate whose correctness is argued in detail and
whose cost is asserted without evidence — and the gap is not local, because
there is nowhere in the family for evidence to go.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/non_functional_requirement
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'benches in the ring family:   %s\n' "$( ls -d ../../../ring_*/benches 2>/dev/null | wc -l )"
printf 'time figures in this crate:   %s\n' "$( command grep -rhoE '[0-9]+ (ns|us|ms|cycles)' .. ../../src ../../tests 2>/dev/null | sort -u | wc -l )"
printf 'cost claims on the surface:   %s\n' "$( command grep -c 'Nothing on this surface' ../api/001_shutdown_surface.md || true )"
printf 'family tests on allocation:   %s\n' "$( command grep -rl 'GlobalAlloc' ../../../ring_*/tests 2>/dev/null | sed 's|\.\./||g' | tr '\n' ' ' )"
printf 'such a test in this crate:    %s\n' "$( command grep -rl 'GlobalAlloc' ../../tests 2>/dev/null | wc -l )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
benches in the ring family:   0
time figures in this crate:   0
cost claims on the surface:   1
family tests on allocation:   ring_barrier/tests/allocation_test.rs ring_claim/tests/allocation_test.rs ring_consume/tests/allocation_test.rs ring_cursor/tests/allocation_test.rs ring_flush/tests/append_cost_test.rs 
such a test in this crate:    0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD33 | the crate's only recorded numbers are coverage ratios | **measured cost** | Every figure this crate has written down is a coverage ratio — `72/73`, `73/73`, `4/4` from the `llvm-cov` investigation and the manual probe tally — and there is not one duration, cycle count or throughput figure in its source, tests or docs; that is structural rather than local, because the ring family has zero `benches/` directories and zero manifests naming `criterion`, so the guard's central design argument (that consulting a flag is cheap enough to put on every publish) is a performance claim in a family with no machinery to check one, and "almost certainly cheap" is the strongest statement available for the property the whole design rests on. |
| SD34 | the pre-flight accessor costs more and promises less than just pushing | n/a — observation | `Guarded::is_blocked` reads `self.shutdown.is_closed() \|\| self.producer.is_full()` and its name invites `if !g.is_blocked() { g.try_push( r ) }`, which pays two flag reads and an occupancy check per record instead of one flag read and buys nothing — the doc comment itself says *"It does not predict a refusal"*, since under the default `OverflowPolicy::DropNewest` a blocked-by-occupancy push still returns `Ok` having discarded the record — while `try_push` already returns the whole answer atomically with the operation it guards, so the obvious use of the accessor is strictly worse on both axes and no measurement anywhere in the family would let a reader weigh the difference. |
| SD35 | the headline teardown call routes through the record-at-a-time loop | n/a — observation | `drain_all` empties a ring with `consumer.try_recv_batch( out )` and `discard_all` with `consumer.try_recv()` once per record, and `reset` — documented as *"the whole teardown in one call"* and *"the operation a test harness or a world recycle wants"*, so the one called repeatedly between rounds — routes through `discard_all`; the justification is `discard_all`'s own *"so a `T` with a `Drop` impl still runs it"*, which is correct in principle and currently protects nothing, since this crate declares no destructor, its suite's only `Drop` occurrences are the `DropNewest` overflow policy, and exactly two crates in the whole family impl `Drop`. |
| SD36 | the surface promises it does not allocate, and the family already built the check it does not use | n/a — unenforced | [`api/001`](../api/001_shutdown_surface.md)'s fourth surface claim is *"Nothing on this surface allocates or parks"*, tied in the same sentence to what makes the surface reachable from inside a tick, and it holds — outside doctests the crate has zero allocation sites — but nothing enforces it: the only crate-level attribute is `#![ deny( missing_docs ) ]`, there is no `no_std`, no allocator assertion and no test, while `ring_flush/tests/append_cost_test.rs` already faces the identical requirement under the same `unsafe_code = "deny"` constraint and lands on a transferable safe proxy (assert the buffer's capacity is unchanged across N operations); separately, `drain_all` never calls `reserve` on the caller's `Vec` though `ring_core::Consumer::len` is one public call away, so the caller pays the growth schedule the crate could size away in a line. |
