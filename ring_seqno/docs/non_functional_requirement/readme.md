# non_functional_requirement

How the crate must behave beyond returning right answers, and how each
requirement is checked.

### Overview Table

| ID | Name | Requirement | Checked by |
|----|------|-------------|------------|
| 001 | [Every Reading Is Allocation-Free](001_every_reading_is_allocation_free.md) | No function here allocates, and none may start | Inspection — five bodies, all visible |
| 002 | [The Arithmetic Must Survive a Narrow `usize`](002_the_arithmetic_must_survive_a_narrow_usize.md) | The three capacity readings must agree on every target | **Nothing** — held by construction instead |

### The Gap Between Them

001 is a requirement the crate meets easily and will keep meeting, because the
bodies are three lines each and any allocation would be visible in review. It was
documented mainly to locate the allocation the family *used to* pay, one tier up
and not this crate's to remove — though this crate's signature was what forced
it. Commit `b7e075ca` removed it by removing the caller rather than the cost:
`ring_cursor::slowest` folds the loads in place now and no longer delegates here
at all.

002 was a requirement the crate **did not meet**, on a target nobody has built
for. It now meets it, and the two documents fail in opposite directions on the
same axis: 001's guarantee is checked by nothing and needs nothing, because an
allocation would be visible on sight; 002's is checked by nothing and *would*
have needed something, because the disagreement it described was invisible on a
64-bit host — the two types coincide there, so the suite passed identically
before and after. What closed it was structure, not a test: `free_slots` now
narrows only its already-`capacity`-bounded result, so the three readings agree
on every target by construction. There is still no test, no lint and no CI job
behind the requirement, and none can be written that would distinguish the two
bodies where the suite runs.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SQ35 | `slowest`'s parameter | **measured cost** | `slowest`'s `&[ Seq ]` parameter forced `ring_cursor` to heap-allocate in the condition of a lock-free retry loop until `b7e075ca`, which removed the allocation by removing the delegation — leaving this function with no caller anywhere outside its own tests |
| SQ36 | The division's cost | n/a — observation | `laps_between` divides by a runtime value the compiler cannot prove is a power of two, so the family's only lap count is also its only integer division — and nothing calls it |
| SQ37 | Narrow `usize` | **latent hazard** | `free_slots` truncated on any target where `usize` is under 64 bits while `may_claim` and `laps_between` did not, and `free_slots` is the one two other crates gate writes with. It now widens, saturates, then narrows a result already bounded by `capacity`, so the loss is gone on every target width |
| SQ38 | Time to reach it | **latent hazard** | The truncation was reachable in roughly seventy minutes at a modest publication rate, unlike the `Seq` overflow it superficially resembles — which is what made SQ37 worth fixing rather than documenting. The same fix closed it: the seventy-minute clock still runs, and has no truncation left to reach at the end of it |

### Regenerate

The inline hint the family declines and the workspace uses:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rc '#\[ *inline' ring_*/src/*.rs | awk -F: '{s+=$2} END {print "ring family: "s}'
command grep -rc '#\[ *inline' */src/*.rs      | awk -F: '{s+=$2} END {print "workspace:   "s}'
```

Live output:

```
ring family: 2
workspace:   120
```

