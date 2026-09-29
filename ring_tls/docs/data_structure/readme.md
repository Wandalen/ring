# Data Structure Doc Definition

### Scope

- **Purpose**: Define the structure this crate exists to hold, at the grain this crate's own design decided, so the open layout questions are named rather than papered over.
- **Responsibility**: Document `ring_tls`'s own data structures — identity, decided shape, and operations.
- **In Scope**: The per-thread append log's structural identity and its operation set.
- **Out of Scope**: The exact buffer layout and growth policy, still undecided; the operation vocabulary consumers encode into it, which stays entirely theirs.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Thread-Local Append Log](001_thread_local_append_log.md) | One bump-allocated byte region per thread — append with no atomics, read after quiesce | 🔄 |




### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs/data_structure
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TL[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  6
# rows in the table below:  6
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TL11 | `TlsBuffer` | **wrong doc** | This instance specifies a byte region with a bump pointer; `TlsBuffer` declares `items : Vec< T >` and `limit : usize`. |
| TL12 | `TlsBuffer` | n/a — observation | `TlsBuffer` is the one identifier common to the specified design and the built one, and it names a thread-local mechanism neither has. |
| TL13 | `limit` | n/a — observation | `limit` is stored beside the `Vec` rather than read from `Vec::capacity`, so refusal cannot drift with the allocator's rounding. |
| TL14 | the growth path | **latent hazard** | Nothing but the `>=` in `push` keeps `Vec` from reallocating; a second insertion path added later would silently restore the growth the invariant forbids. |
| TL15 | `Vec< T >` | n/a — observation | A typed `Vec< T >` serves all three current consumers and cannot serve the walkable byte stream the specified design existed for. |
| TL16 | `readme.md` | n/a — inconsistency | `readme.md` opens by calling the crate a bump-allocated log with no atomics, then states four paragraphs later that `TlsBuffer< T >` is a `Vec< T >`. |
