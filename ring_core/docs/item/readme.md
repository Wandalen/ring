# Items

### Scope

- **Purpose**: Inventory every item `ring_core` declares, so the twelve behavioural definitions can point at a signature rather than restate it, and so the crate's real size is a number rather than an impression.
- **Responsibility**: The census by taxonomy kind, the public/private split, and the reach of each public name across the eight dependent crates.
- **In Scope**: Items whose Defining Crate is `ring_core` — 1 module, 4 use declarations, 4 structs, 5 enums, 5 implementations, 17 associated functions.
- **Out of Scope**: Items this crate merely *uses* — `RingConfig`, `TypedSlot`, `Capacity`, `OverflowPolicy`, `RingError`, `would_resolve`, `Resolution` — which belong to their defining crates; what each item *does*, which is the twelve behavioural definitions' subject.

### The Census

| Kind | Public | Private | Total |
|------|-------:|--------:|------:|
| Module | 1 | 0 | 1 |
| Use declaration | 0 | 4 | 4 |
| Struct | 4 | 0 | 4 |
| Enum | 1 | 4 | 5 |
| Implementation | 5 | 0 | 5 |
| Associated function | 16 | 1 | 17 |
| Associated constant | 0 | 0 | 0 |
| **Total** | **27** | **9** | **36** |

**Eleven of the taxonomy's eighteen kinds are absent**, and the absences are the
shape of the crate: no trait, because the module documentation's opening argument
is that no shared method set exists to define one over; no free function, no
static, no macro, no type alias, no constant. Everything here is reached by
naming one of five types.

**The four private enums are the crate's actual mechanism.** `Storage`,
`EndsInner`, `ProducerInner` and `ConsumerInner` are one backend choice
expressed four times, once per lifetime and mutability shape the surface needs
(→ [`../data_structure/002`](../data_structure/002_four_parallel_enums_over_one_backend_choice.md)).
A reader counting only public items sees five types and misses that the crate is
mostly a four-way parallel dispatch.

### Where the Counts Come From

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
awk '/^use /{u++} /^pub struct /{ps++} /^pub enum /{pe++} /^enum /{xe++}
     /^impl/{im++} /^  pub (const )?fn /{pf++} /^  fn /{xf++}
     END{ printf "use %d  pub struct %d  pub enum %d  priv enum %d  impl %d  pub fn %d  priv fn %d  total %d\n",
          u,ps,pe,xe,im,pf,xf, 1+u+ps+pe+xe+im+pf+xf }' src/lib.rs
# use 4  pub struct 4  pub enum 1  priv enum 4  impl 5  pub fn 16  priv fn 1  total 36
```

Anchored at line start throughout: an unanchored `pub fn` count would include the
four doc examples, and an unanchored `impl` count would include the word in
prose. The instances under this definition read the set; this table is what they
read.

### Instances

| File | What it establishes |
|------|---------------------|
| [001_thirty_six_items_and_the_four_names_the_family_imports.md](001_thirty_six_items_and_the_four_names_the_family_imports.md) | Which of the 27 public items the eight dependents actually reach |
| [002_the_backend_discrimination_surface.md](002_the_backend_discrimination_surface.md) | The three items that exist so a caller can tell the backends apart, and who calls them |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_backend.md](../type/001_backend.md) | The one public enum, read as a domain type |
| [../type/002_producer_cardinality.md](../type/002_producer_cardinality.md) | `try_clone`'s return as the cardinality answer |



### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/item
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### CO[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| CO[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  6
# rows in the table below:  6
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CO27 | the census | n/a — observation | No trait, free function, static, macro, type alias or constant — everything here is reached by naming one of five types. |
| CO28 | `capacity`, `len`, `is_empty` | n/a — diagnostics | `Ring::capacity`, `Consumer::len` and `Consumer::is_empty` collide with `Vec`, `str` and `RingConfig` methods, so a name-based reach count is meaningless for them. |
| CO29 | the measurement | n/a — diagnostics | `Ring::new` reads as 18 call sites unfiltered and 2 with doc examples removed. |
| CO30 | the discrimination surface | n/a — unadopted | The module documentation names `try_clone` as the machine-checkable way to tell the backends apart, and no library calls it. |
| CO31 | `ring_factory` | n/a — coverage | The only `Backend` import in the family is a test import, asserting that the factory selects correctly. |
| CO32 | `ring_handle` | n/a — observation | `ring_handle/tests/handle_test.rs:382` reads the backend into a binding and takes no decision from it. |
