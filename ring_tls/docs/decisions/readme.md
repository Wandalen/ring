# Decisions

### Scope

- **Purpose**: Record architecture decisions for `ring_tls` that passed the Decision Gate — genuine open trade-offs with real switching cost — rather than being fixed in place directly.
- **Responsibility**: Index this crate's Architecture Decision Records (ADRs).
- **In Scope**: Proposed, accepted, and superseded ADRs for this crate's design.
- **Out of Scope**: The crate's existence and shared, family-neutral shape, which this crate's own design already decided, not a local ADR; the concrete buffer layout, which is a future benchmark's open question, not this crate's to decide unilaterally either.

ADRs here use the format at `doc_des.rulebook.md § Architecture Documentation :
Architecture Decision Records`. They are indexed both below and in
[`definition/readme.md`](../definition/readme.md), which counts every instance
under every definition and would otherwise report a total that does not match
the directory.

The earlier text here said they are "indexed only in this file, not in
`definition/readme.md`". That reading is what left this directory outside the
Module Index while it held no instances and no one had to reconcile the two.

### Index

| ID | Decision | Status | Turns on |
|----|----------|--------|----------|
| [001](001_the_corpus_specifies_an_api_the_crate_did_not_build.md) | The corpus specifies an API the crate did not build | **Open** | Whether the nineteen pre-implementation instances are rewritten, superseded in place, or refiled under the consumer that still needs them |
| [002](002_a_refused_push_destroys_the_item_its_doc_promises_to_return.md) | A refused `push` destroys the item its doc promises to return | **Open** | Whether `push` adopts the family's payload-carrying error shape, or its documentation is corrected to match what it does |

### Why the Buffer Layout Is Still Not One of Them

This crate's one *pre-implementation* open trade-off — the concrete buffer
layout — is tracked at benchmark grain, not as a crate-local ADR, because the
answer is shared with a prospective consumer's own append-path replacement
and isn't this crate's alone to decide.

That is still true, and it is no longer the whole picture: the layout question
was answered here unilaterally, in code, by building `Vec< T >`
(→ [`../data_structure/002`](../data_structure/002_a_vec_and_a_limit.md)). What
001 files is not the layout question re-opened — it is the fact that the corpus
was never told, and that a consumer crate's module comment is where the
divergence is recorded instead.

**Both ADRs here are Open, and neither is open because the answer is unclear.**
They are open because applying either changes something outside this crate: 001
moves design material into a crate in another family, and 002 breaks an
exported signature. The Decision Gate exists to catch exactly that — a fix that
looks local and is not.




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/decisions
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TL[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  7
# rows in the table below:  7
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TL17 | the corpus | **wrong doc** | The pre-implementation corpus and `src/lib.rs` share no function name at all. |
| TL18 | the corpus checkers | n/a — coverage | `recipes.py` compares an instance against reality only where the instance publishes a `sh` block, and not one of the nineteen bodies does. |
| TL19 | `ring_flush` | n/a — doc gap | `ring_flush/src/lib.rs` compares both designs and explains why the built one failed its requirement; `ring_tls`'s own corpus says nothing. |
| TL20 | `drain` | n/a — doc gap | `TlsBuffer::drain` exists because `ring_flush` could not use `flush_into`, and only `ring_flush` records why. |
| TL21 | `push` | **wrong doc** | `push` states that a refused item "is returned to the caller by never being taken", and `item : T` is moved in and dropped on the refusal path. |
| TL22 | the refusal path | **latent hazard** | All three consumers stage `Copy` types, so the drop is invisible today; a `T` owning a resource loses one per refused push. |
| TL23 | `ring_registry` | n/a — unadopted | `ring_registry::register` returns `Result< (), ( RegistryError, Split< T > ) >`, handing the payload back in the error tuple. |
