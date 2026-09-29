# docs

Design documentation for `ring_align`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The two rules the crate computes with |
| `api/` | The surface, split by whether it grants a capability |
| `data_structure/` | The wrapper as a layout, and the pair it is built into |
| `decisions/` | The two choices with a live alternative |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Both dependency edges, and the argument for a separate crate |
| `invariant/` | The two standing restrictions, one enforced and one not |
| `item/` | Declaration-level reference for each exported item |
| `lifecycle/` | The two things here with a life longer than a call |
| `non_functional_requirement/` | Verifiability, and cost |
| `pattern/` | The two reusable shapes this crate instantiates |
| `pitfall/` | The two failures that leave every test green |
| `type/` | The constant and the wrapper as contracts |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

Scope of this crate: cache-line padding constants and alignment wrappers.

**13 definitions, 26 instances** — the full index, with what each instance
carries, is [definition/readme.md](definition/readme.md). Regenerate the
counts:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_align/docs
find . -mindepth 1 -maxdepth 1 -type d ! -name definition | wc -l
find . -name '[0-9][0-9][0-9]_*.md' | wc -l
```

### Where to Start

The crate is three items and one attribute, so the documentation is mostly
about consequences rather than about mechanics.

- **To understand what the crate does**, read [type/002](type/002_cache_aligned.md)
  then [data_structure/002](data_structure/002_a_padded_pair_in_one_struct.md) —
  the wrapper, then the arrangement it exists to make possible.
- **To understand why it is a crate at all**, read
  [pattern/002](pattern/002_one_owner_for_a_magic_number.md) and
  [integration/002](integration/002_why_the_constant_lives_here.md), which
  reach different conclusions about how well the separation worked and are
  meant to be read together.
- **To find what is wrong today**, read
  [pitfall/001](pitfall/001_a_constant_too_small_buys_nothing.md) — its
  § The Duplicate Already Exists is the crate's most consequential finding — and
  the *Findings Recorded, Not Fixed* table in
  [definition/readme.md](definition/readme.md).
- **Before porting to a machine with 128-byte lines**, read
  [lifecycle/002](lifecycle/002_the_constant_across_a_platform_port.md) first.
  Four of its eleven sites have no test that will prompt you.
