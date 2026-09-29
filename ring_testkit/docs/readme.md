# docs

Design documentation for `ring_testkit`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | How a `Step` becomes a field of the `Outcome`, and the four passes that check it |
| `api/` | The scripting surface, the two loom helpers, and the surface no crate has taken |
| `data_structure/` | A flat step list with no nesting, and ten fields one of which means two things |
| `decisions/` | Closed and open trade-offs, each with what settled or would settle it |
| `definition/` | Module Index — every definition, instance and finding in this crate, in one place |
| `integration/` | Three declared edges, the crate that is missing, and the edge only a cfg creates |
| `invariant/` | Every minted record is somewhere, and the shape a delivered list must have |
| `item/` | Two components that meet in no line of `src/`, and what the crate declines to declare |
| `lifecycle/` | Open and Closed, the transition that passes through a state it did not intend to, and the object with no end |
| `non_functional_requirement/` | Determinism: what ten equal runs are and are not evidence of, and what a fixture owes consumers |
| `pattern/` | The script as data with the run as its interpreter, and returning a verdict instead of making one |
| `pitfall/` | Three measured surprises: the reading that misses a drop, the reopen that closes, the flush with no ring |
| `type/` | `Outcome`'s ten fields, `Anomaly`'s four arms, and the three paths a value arrives by |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

Scope of this crate: determinism-test fixtures driving scripted claim and drain
sequences, plus the bridge that lets those sequences run under `loom`.

## Where to start

**[`pitfall/001`](pitfall/001_neither_the_count_nor_the_list_alone.md)** — a
measurement that contradicted its own prediction in both halves. Two rings
differing only in overflow policy deliver **identical records** and **different
counts**, which is the inverse of what a draft of `src/lib.rs` claimed. A fixture
reading either one alone cannot see that a record was destroyed;
`Outcome::vanished` is the two together and exists because of that run.

Then [`invariant/001`](invariant/001_every_minted_record_is_somewhere.md) for the
accounting law and why it deliberately passes on a destroyed record,
[`api/001`](api/001_the_script_surface.md) for the surface, and
[`non_functional_requirement/001`](non_functional_requirement/001_two_runs_compare_equal.md)
for the limit: ten equal single-threaded runs establish that the fixture adds no
variability of its own, and nothing about concurrency. That limit is why there is
a `loom` half, and [`tests/manual/readme.md`](../tests/manual/readme.md) M2 is
the negative control proving the loom half explores rather than trivially
passing.

For the whole corpus at once — 13 definitions, 27 instances, 54 findings ordered
by ID — [`definition/readme.md`](definition/readme.md) is the index.

## What the findings are for

Every instance ends with numbered `### TKn` sections, and each one is a
measurement rather than an opinion: a recipe in that same file produces the
numbers quoted, and the `Live output:` block below it is that recipe's actual
output. Re-running a recipe is the way to check whether a finding has gone stale.

Twenty of the fifty-four carry a bold tier, meaning something is actually wrong
rather than merely worth knowing. Nine are **latent hazard** — a way the code can
be used that will not fail loudly — and eleven are **misleading doc**, marking a
place where a document in this crate says something the code does not support.
The other thirty-four are `n/a` tiers: observations, gaps and coverage notes that
record a fact without asserting a defect.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/definition
printf 'findings total:   %s\n' "$( command grep -cE '^\| TK[0-9]+ \|' readme.md )"
printf 'bold-tier:        %s\n' "$( awk -F' \\| ' '/^\| TK[0-9]+ \|/ && $4 ~ /^\*\*/ { c++ } END{ print c+0 }' readme.md )"
printf 'latent hazard:    %s\n' "$( awk -F' \\| ' '/^\| TK[0-9]+ \|/ && $4 ~ /latent hazard/ { sub( /^\| /, "", $1 ); printf "%s ", $1 } END{ print "" }' readme.md )"
printf 'misleading doc:   %s\n' "$( awk -F' \\| ' '/^\| TK[0-9]+ \|/ && $4 ~ /misleading doc/ { sub( /^\| /, "", $1 ); printf "%s ", $1 } END{ print "" }' readme.md )"
printf 'distinct n/a tiers: %s\n' "$( awk -F' \\| ' '/^\| TK[0-9]+ \|/ && $4 ~ /n.a/ { print $4 }' readme.md | sort -u | wc -l )"
```

Live output:

```
findings total:   54
bold-tier:        20
latent hazard:    TK3 TK10 TK11 TK16 TK26 TK27 TK36 TK41 TK42 
misleading doc:   TK1 TK4 TK9 TK17 TK18 TK22 TK29 TK31 TK46 TK47 TK54 
distinct n/a tiers: 9
```

