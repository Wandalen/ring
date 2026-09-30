# Items

### Scope

- **Purpose**: Inventory every item `ring_mpsc` declares, so the twelve behavioural definitions can point at a signature rather than restate it, and so a 1,214-line crate's real surface is a number.
- **Responsibility**: The census by taxonomy kind, the public/private split, and the reach of each public name across the two crates that depend on this one.
- **In Scope**: Items whose Defining Crate is `ring_mpsc` — 1 module, 12 use declarations, 6 structs, 16 implementations, 40 associated functions, 5 constants.
- **Out of Scope**: Items this crate uses — `Capacity`, `Seq`, `RingError`, `Buffer`, `AtomicSeq`, `Claimer`, `GatingSet` — which belong to their defining crates; the `unsafe` those items make possible (→ [`../workaround/001`](../workaround/001_the_unsafe_code_opt_out_and_its_obligations.md)).

### The Census

| Kind | Public | Private | Total |
|------|-------:|--------:|------:|
| Module | 1 | 0 | 1 |
| Use declaration | 0 | 12 | 12 |
| Struct | 6 | 0 | 6 |
| Implementation | 16 | 0 | 16 |
| Associated function | 29 | 11 | 40 |
| Constant | 5 | 0 | 5 |
| **Total** | **57** | **23** | **80** |

**No enum and no trait.** The crate models states as cursor arithmetic rather
than as variants — a slot's state is the relation between its stamp and the
sequence addressing it, not a discriminant — which is why `lifecycle/003` can
describe four slot states with no type to point at.

**Six structs, and five of them are borrowed views.** Only `Ring` owns anything;
`Ends`, `Producer`, `Reserved`, `Consumer` and `Batch` are each a lifetime-bound
handle onto it. That ratio is the crate's shape: one allocation, five ways to
hold it.

### Where the Counts Come From

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
awk '/^use /{u++} /^pub struct /{ps++} /^impl|^unsafe impl/{im++}
     /^    pub (const )?fn /{pf++} /^    fn |^    unsafe fn /{xf++} /^pub const /{pc++}
     END{ printf "use %d  struct %d  impl %d  pub fn %d  priv fn %d  const %d  total %d\n",
          u,ps,im,pf,xf,pc, 1+u+ps+im+pf+xf+pc }' src/lib.rs
```

Live output:

```
use 12  struct 6  impl 16  pub fn 29  priv fn 11  const 5  total 80
```

### Instances

| File | What it establishes |
|------|---------------------|
| [001_eighty_items_and_the_four_names_that_leave_the_crate.md](001_eighty_items_and_the_four_names_that_leave_the_crate.md) | Which of the 57 public items the two dependents reach |
| [002_five_public_ordering_constants.md](002_five_public_ordering_constants.md) | The memory-ordering vocabulary this crate exports, and who reads it |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_publication_ordering.md](../invariant/002_publication_ordering.md) | The pairs the five constants name |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_unsafe_code_opt_out_and_its_obligations.md](../workaround/001_the_unsafe_code_opt_out_and_its_obligations.md) | The ten `unsafe` lines among these items |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/item
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### MP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  5
# rows in the table below:  5
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| MP26 | the public surface | n/a — observation | The two dependent crates name `Ring`, `Ends`, `Producer` and `Consumer`, and nothing else. |
| MP27 | the census | n/a — observation | Slot state is cursor arithmetic rather than a discriminant, so the crate has no state type to point at. |
| MP28 | the census | n/a — observation | The impl count exceeds the struct count because several types carry separate `impl` blocks per bound. |
| MP29 | the five constants | n/a — coverage | Every one of the five carries a rustdoc example asserting its value, so the vocabulary is checked by `cargo test --doc`. |
| MP30 | the five constants | n/a — observation | `PUBLISH`/`OBSERVE`/`COMMIT`/`OWN` name what the access is for; two of the four have the same `Ordering` value. |
