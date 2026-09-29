# Pitfall Doc Definition

### Scope

- **Purpose**: Record the traps that arrive bundled with the ring pattern itself, so a consumer adopting the mechanism does not silently adopt a cost its own workload never needed.
- **Responsibility**: Document `ring_mpsc`'s own confirmed traps — those a person hits by taking the pattern's canonical shape as the pattern's requirement.
- **In Scope**: Assumptions about what the ring supplies that it does not, starting with a triggering event it has none of.
- **Out of Scope**: External constraints this crate absorbs, which carry a deletion condition a trap never has (→ [`../workaround/`](../workaround/readme.md)); the correctness traps that live in the ordering contract rather than in usage (→ [Publication Ordering](../invariant/002_publication_ordering.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Spinning Consumer Owns a Core](001_spinning_consumer_owns_a_core.md) | The ring has no triggering event, so its canonical `while(true)` consumer burns ~99% of a core at a ~1% duty cycle on a tick-bounded workload — and is correct on a continuous one | 🔄 |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc/docs/pitfall
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### MP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| MP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  4
# rows in the table below:  4
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| MP44 | `spinning` | n/a — observation | `try_recv`-shaped drain with no blocking variant means a consumer that wants to wait must write the loop itself. |
| MP45 | `spinning` | n/a — coverage | The pitfall names a cost — one core — and no benchmark in the family produces the number. |
| MP46 | the stale-stamp trap | **latent hazard** | `stamp != UNSTAMPED` and `stamp == seq` agree on every ring that never wraps, so a suite that fills a ring once cannot distinguish them. |
| MP47 | `stamps` | **latent hazard** | The accessor exposes `&[ AtomicSeq ]`, so an external reader must reimplement the publication test — and the wrong version is the intuitive one. |
