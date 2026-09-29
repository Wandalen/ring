# Items

### Scope

- **Purpose**: Inventory every item `ring_tls` declares, so the crate's real surface is a number rather than an impression — which matters more here than elsewhere in the family, because twelve of this crate's doc instances describe items it does not declare.
- **Responsibility**: The census by taxonomy kind, the public/private split, and which crates reach each public name.
- **In Scope**: Items whose Defining Crate is `ring_tls` — 4 use declarations, 2 structs, 4 implementations, 12 associated functions.
- **Out of Scope**: Items this crate uses — `RingError`, `Seq`, `SeqCell`, `BatchClaim`, `claim`, `Ordering` — which belong to their defining crates; the eight names the corpus specifies and the crate never declared (→ [`../decisions/001`](../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md)).

### The Census

| Kind | Public | Private | Total |
|------|-------:|--------:|------:|
| Use declaration | 0 | 4 | 4 |
| Struct | 2 | 0 | 2 |
| Implementation | 4 | 0 | 4 |
| Associated function | 10 | 2 | 12 |
| Constant | 0 | 0 | 0 |
| **Total** | **16** | **6** | **22** |

**Twenty-two items, and the two counted private are not private.** They are
`Iterator::next` and `Iterator::size_hint` on `Flush` — trait methods, reachable
by every caller that has the trait in scope, spelled `fn` rather than `pub fn`
because a trait impl carries its visibility from the trait. A census keyed on
the `pub` token puts them in the wrong column, and this table says so rather
than quietly reporting sixteen public items where callers can reach eighteen.

**No public constant.** `ring_spsc` publishes two orderings and `ring_mpsc`
four; this crate publishes none and takes an `Ordering` parameter instead
(→ [`002`](002_no_published_constant_and_a_caller_supplied_ordering.md)).

### Where the Counts Come From

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
grep -vE '^\s*(//|///|//!)' src/lib.rs \
| awk '/^use /{u++} /^pub struct /{ps++} /^impl|^unsafe impl/{im++}
       /^  pub (const )?fn /{pf++} /^  fn |^  unsafe fn /{xf++} /^pub const /{pc++}
       END{ printf "use %d  struct %d  impl %d  pub fn %d  priv fn %d  const %d  total %d\n",
            u,ps,im,pf,xf,pc, u+ps+im+pf+xf+pc }'
```

Live output:

```
use 4  struct 2  impl 4  pub fn 10  priv fn 2  const 0  total 22
```

### Instances

| ID | Name | Records |
|----|------|---------|
| [001](001_twenty_two_items_and_the_three_crates_that_reach_them.md) | Twenty-Two Items, and the Three Crates That Reach Them | The census by kind and the measured reach of each public name |
| [002](002_no_published_constant_and_a_caller_supplied_ordering.md) | No Published Constant, and a Caller-Supplied Ordering | Why the ordering vocabulary is a parameter here and a constant in both composed cores |

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | The declaration site of every item counted here |
| `../../../ring_flush/src/lib.rs` | The consumer that drove one of the twelve functions into existence |




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/item
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TL[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  5
# rows in the table below:  5
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TL32 | the census | n/a — observation | `Iterator::next` and `size_hint` on `Flush` are reachable by every caller and spelled `fn`, so a census keyed on the `pub` token miscounts them. |
| TL33 | the reach | n/a — observation | `TlsBuffer` and `with_capacity` are named by all three consumers; `Flush` is never spelled anywhere outside this crate. |
| TL34 | the reach scan | n/a — diagnostics | The scan filters TOML comments with `^\s*#` as well as Rust comments, because one manifest names this crate only to record its removal. |
| TL35 | the ordering surface | n/a — inconsistency | `ring_spsc` publishes two ordering constants and `ring_mpsc` four; this crate publishes none and takes an `Ordering` argument. |
| TL36 | `order` | **latent hazard** | `flush_into` accepts any `Ordering`; a `Relaxed` claim allocates correct sequences and publishes nothing, failing in a consumer rather than here. |
