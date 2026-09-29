# docs

Design documentation for `ring_stats`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | Every atomic site, and the three methods that issue more than one |
| `api/` | Sixteen signatures, and what the surface will not let a caller ask |
| `data_structure/` | The seven fields as a layout, and the three behind an enum |
| `decisions/` | The ordering ruling, and what this crate was originally asked to count against what shipped |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Both dependency edges, and the callers that are named but absent |
| `invariant/` | What holds per counter, and the relation across counters that nothing enforces |
| `item/` | Declaration-level reference for each of the crate's public methods |
| `lifecycle/` | The counter set at runtime, and the crate against its feature record |
| `non_functional_requirement/` | The cost the crate claims, and the distortion it does not discuss |
| `pattern/` | The two reusable shapes this crate instantiates, one exactly and one opaquely |
| `pitfall/` | The two readings that fail under the traffic that prompts them |
| `type/` | `RingStats` as a contract — what its fields foreclose, and what one width hides |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

Scope of this crate: ring counters.

**13 definitions, 26 instances, 52 findings** — the full index, with what each
instance carries and every finding in one table, is
[definition/readme.md](definition/readme.md). Regenerate the counts:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_stats/docs
find . -mindepth 1 -maxdepth 1 -type d ! -name definition | wc -l   # 13
find . -name '[0-9][0-9][0-9]_*.md' | wc -l                          # 26
command grep -rho '^### ST[0-9]*' . | wc -l                          # 52
```

### Where to Start

The crate is 484 lines of relaxed `fetch_add` and relaxed `load` — 259 when these
findings were written. Ten of its sixteen methods are one instruction and do exactly
what their names say; the documentation is almost entirely about the other six.

- **To understand what the crate is**, read
  [pattern/001](pattern/001_the_record_and_read_pair.md) then
  [type/002](type/002_seven_counters_and_one_width.md) — the record-and-read
  mirror that generates the whole surface, then the one width it carries
  everything in.
- **Before trusting a number this crate returns**, read
  [pitfall/001](pitfall/001_the_leak_in_flight_cannot_see.md) and
  [pitfall/002](pitfall/002_the_total_that_counts_a_refusal_as_a_loss.md).
  They cover `in_flight` and `dropped_total`, which are the two readings the
  crate puts forward and the two that are wrong exactly when consulted.
- **Before wiring a producer to it**, read
  [integration/001](integration/001_the_write_path_and_two_callers_that_are_not_there.md).
  Two of the crate's counters have no production writer today, and one of its
  doc comments names a caller that could not compile.
- **Before deciding whether the counters are cheap enough**, read
  [non_functional_requirement/001](non_functional_requirement/001_cheap_enough_to_leave_on.md).
  The per-call answer is yes; the per-system answer is a fixed ceiling with no
  lever, and the crate states only the first.
- **To find what is wrong today**, read the *Findings* and *Severity* tables in
  [definition/readme.md](definition/readme.md) — 21 of the 52 are reachable.
