# item

One public type and eleven methods, catalogued. The split is 3 / 8: three
readings answer *may I claim*, and eight describe the set they read.

### Overview Table

| ID | Name | Catalogues |
|----|------|-----------|
| 001 | [The Three Gating Readings](001_the_three_gating_readings.md) | `headroom`, `admits`, `check` — the producer-facing surface |
| 002 | [The Four Accessors and the Limit](002_the_four_accessors_and_the_limit.md) | The remaining eight: four accessors over the `Vec`, two positions, and the two that fix the shape |

### Every Item

| Item | Kind | Instance |
|------|------|:--------:|
| `GatingSet` | struct | both |
| `GatingSet::new` | associated function | 002 |
| `GatingSet::len` | method | 002 |
| `GatingSet::is_empty` | method | 002 |
| `GatingSet::cursor` | method | 002 |
| `GatingSet::cursors` | method | 002 |
| `GatingSet::capacity` | method | 002 |
| `GatingSet::slowest` | method | 002 |
| `GatingSet::headroom` | method | 001 |
| `GatingSet::admits` | method | 001 |
| `GatingSet::check` | method | 001 |
| `GatingSet::limit` | method | 002 |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -nE '^\s*pub (struct|const fn|fn) ' ring_gating/src/lib.rs
```

Twelve items, one of them the type. There is no trait, no free function, no
re-export and no `mod` — the crate is one `impl` block and its struct.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| GT27 | The doctests | n/a — observation | They are *scenarios* rather than smoke checks — four of the twelve drive a cursor with `store` before asserting, and the module-level one does it twice, for five `store` calls in all |
| GT28 | The three readings | n/a — duplication | `admits` calls `headroom`; `check` calls `headroom`. There is one reading of the state and three entry points to it, and the two derived ones differ from each other only in what they do with an identical comparison |
| GT29 | `new` | n/a — coverage | It accepts any `consumers` count including one larger than the ring's capacity, which is constructible, harmless, and untested — all sixteen `set_at` calls in the suite build fewer consumers than the capacity, none at or above it |
| GT30 | `cursor` and `cursors` | **latent hazard** | One hands out a single `&PaddedCursor`, the other the whole slice, and both go through `&self`. Neither signature distinguishes a consumer advancing its own position from a producer advancing someone else's: the set knows which index belongs to which consumer and never uses that knowledge to restrict access |
