# Items

### Scope

- **Purpose**: Inventory every item `ring_spsc` declares, so the twelve behavioural definitions can point at a signature rather than restate it, and so the crate's real surface is a number rather than an impression.
- **Responsibility**: The census by taxonomy kind, the public/private split, and the reach of each public name across the crates that depend on this one.
- **In Scope**: Items whose Defining Crate is `ring_spsc` — 9 use declarations, 5 structs, 13 implementations, 31 associated functions, 2 constants.
- **Out of Scope**: Items this crate uses — `Capacity`, `Seq`, `RingError`, `Buffer`, `CursorPair`, `SeqCell`, `GATING`, `Slot`, `TypedSlot` — which belong to their defining crates; the `unsafe` those items make possible (→ [`../workaround/001`](../workaround/001_the_unsafe_code_opt_out_and_what_bounds_it.md)).

### The Census

| Kind | Public | Private | Total |
|------|-------:|--------:|------:|
| Use declaration | 0 | 9 | 9 |
| Struct | 5 | 0 | 5 |
| Implementation | 13 | 0 | 13 |
| Associated function | 23 | 8 | 31 |
| Constant | 2 | 0 | 2 |
| **Total** | **43** | **17** | **60** |

**Two of the eight private functions are `unsafe fn`** — `slot` and `slot_mut`,
the whole of this crate's unsafe surface
(→ [`../workaround/001`](../workaround/001_the_unsafe_code_opt_out_and_what_bounds_it.md)).
They are the two the first draft of this table lost: a census that spells the
private case as `fn` and the crate's most consequential pair as `unsafe fn`
counts six where there are eight, and reports a plausible total either way.

**No enum, no trait, and no module declaration.** The crate is one flat file of
five structs and the impls over them. Where its sibling `ring_mpsc` models a
slot's state as a stamp compared against a sequence, this crate does not model
slot state at all: with one producer and one consumer the two cursors already
determine it, which is why `lifecycle/003` is titled *without holes* rather than
*across one lap*.

**Five structs, and four of them are borrowed views.** Only `Ring` owns
anything; `Producer`, `Reservation`, `Consumer` and `Batch` are each a
lifetime-bound handle onto it. One allocation, four ways to hold it — the same
shape as `ring_mpsc` with one fewer handle, because there is no `Ends`
intermediate to hand out.

### Where the Counts Come From

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
grep -vE '^\s*(//|///|//!)' src/lib.rs \
| awk '/^use /{u++} /^pub struct /{ps++} /^impl|^unsafe impl/{im++}
       /^    pub (const )?fn /{pf++} /^    fn |^    unsafe fn /{xf++} /^pub const /{pc++}
       END{ printf "use %d  struct %d  impl %d  pub fn %d  priv fn %d  const %d  total %d\n",
            u,ps,im,pf,xf,pc, u+ps+im+pf+xf+pc }'
```

Live output:

```
use 9  struct 5  impl 13  pub fn 23  priv fn 8  const 2  total 60
```

The comment filter is not decoration. This crate's module documentation runs to
163 lines before the first `use`, and it names most of its own items in prose —
counting the file unfiltered inflates every row.

### Instances

| ID | Name | Records |
|----|------|---------|
| [001](001_sixty_items_and_what_actually_reaches_them.md) | Sixty Items, and What Actually Reaches Them | The census by kind and the measured reach of each public name |
| [002](002_two_ordering_constants_where_the_sibling_has_five.md) | Two Ordering Constants Where the Sibling Has Five | Why the published ordering vocabulary is smaller here, and what that omits |

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | The declaration site of every item counted here |
| `../../../ring_core/src/lib.rs` | Five of the seven code references to this crate in the family |
| `../../../ring_bench/src/lib.rs` | The other two |
| `../../../ring_handle/tests/ui/producer_shared_across_threads.stderr` | Reaches a name without a code reference at all |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs/item
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### SP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| SP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  5
# rows in the table below:  5
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SP26 | the public surface | n/a — observation | `Ring`, `Producer` and `Consumer` leave the crate; `Reservation`, `Batch` and both constants never do. |
| SP27 | the census | n/a — observation | Both `ring_spsc` and `ring_mpsc` are flat files of structs and impls with no variant type anywhere. |
| SP28 | the census | n/a — observation | The distribution is uneven on purpose — `Ring` carries five, `Batch` carries six, `Reservation` carries one. |
| SP29 | `OWN` and `HANDOFF` | n/a — observation | The sibling asserts each of its five constants in a rustdoc example; this crate asserts both of its two in one integration test. |
| SP30 | the ordering vocabulary | n/a — inconsistency | `GATING` is used on every cross-end load and is not among this crate's published constants, so the vocabulary is incomplete where it is read. |
