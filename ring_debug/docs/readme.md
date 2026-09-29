# docs

Design documentation for `ring_debug`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | How D1 and D2 are evaluated without perturbing the ring |
| `api/` | The three entry points, their guarantees and preconditions |
| `data_structure/` | A watch as three scalars with no identity, and four variants that drop their newtype |
| `decisions/` | Closed and open trade-offs, each with what settled or would settle it |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | What this crate can be pointed at, and the reachability boundary it cannot cross |
| `invariant/` | The cursor properties the family assumes and never verifies |
| `item/` | Three nouns and five verbs, and the enum two of the three doors cannot reach |
| `non_functional_requirement/` | The constraints keeping this crate off the hot path, with measurements |
| `pattern/` | The guard that makes the next line legal, and why nothing commits until every check passes |
| `pitfall/` | The measurement that gives the crate its reason to exist |
| `lifecycle/` | `Watch`'s states — the stateful half, and what it still cannot see |
| `type/` | `Violation` and `Cursor` — the reported value |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

Scope of this crate: runtime invariant checks over a live ring.

## Where to start

**[`pitfall/001`](pitfall/001_saturating_arithmetic_reports_health.md)** — it is
the measurement the rest is downstream of. Everything else answers a question it
raises.

Then [`invariant/001`](invariant/001_cursor_invariants_over_a_live_ring.md) for
what is being checked, [`api/001`](api/001_the_check_surface.md) for how to call
it, and
**[`integration/001`](integration/001_reaching_the_cursors_of_a_live_ring.md)**
before relying on it — that one records the limitation a reader is otherwise
likely to discover the hard way.

## What was not written, and now is

`data_structure/`, `item/` and `pattern/` were listed here as having no
instances, along with "the rest of the family's definition set". All three now
carry two instances each, as does every other directory in the table above —
and this crate's own [`definition/readme.md`](definition/readme.md) already
registers every one of them, with per-instance rows and finding counts. The
Module Index was updated when they landed; this paragraph was not.

`item/`'s absence was recorded as **a family-grain deferral rather than a
readiness one** — nothing about this crate was blocking it, so nothing about
this crate would signal when it ended. That is precisely how the sentence
outlived its subject: the deferral was discharged across the whole family at
once, and a claim whose truth turned on a family-wide decision had no local
event to be corrected by.

This file is still the account of what exists rather than of what is planned.
That account is now the table above, with nothing left over for this section to
hold apart:

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'directories under docs/:             %s\n' \
  "$( ls -d ring_debug/docs/*/ | wc -l )"
printf 'of them carrying numbered instances: %s\n' \
  "$( for d in ring_debug/docs/*/; do
        ls "$d"[0-9][0-9][0-9]_*.md >/dev/null 2>&1 && echo "$d"
      done | wc -l )"
printf 'family crates carrying docs/item:    %s of %s\n' \
  "$( ls -d ring_*/docs/item 2>/dev/null | wc -l )" "$( ls -d ring_*/ | wc -l )"
```

Live output:

```
directories under docs/:             14
of them carrying numbered instances: 13
family crates carrying docs/item:    33 of 33
```

The one directory without numbered instances is `definition/`, which holds the
Module Index and by construction has none of its own.
