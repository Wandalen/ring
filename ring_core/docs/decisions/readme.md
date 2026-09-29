# Decisions

### Scope

- **Purpose**: Record architecture decisions for `ring_core` that passed the Decision Gate — genuine open trade-offs with real switching cost — rather than being fixed in place directly.
- **Responsibility**: Index this crate's Architecture Decision Records (ADRs).
- **In Scope**: Proposed, accepted, and superseded ADRs for this crate's design.
- **Out of Scope**: Decisions already settled and documented where they take effect — enum-over-trait dispatch (→ [`algorithm/001`](../algorithm/001_backend_dispatch_and_the_refusal_seam.md)), two constructors rather than a backend config field (→ [`lifecycle/001`](../lifecycle/001_construction_and_backend_selection.md)), and no `Cardinality` type (→ [`type/002`](../type/002_producer_cardinality.md)); the choice of `crossbeam-queue` itself, which is a workaround with a deletion condition (→ [`workaround/001`](../workaround/001_crossbeam_queue_as_interim_backend.md)).

ADRs here use the format at `doc_des.rulebook.md § Architecture Documentation :
Architecture Decision Records`. They are indexed both below and in
[`definition/readme.md`](../definition/readme.md), which counts every instance
under every definition and would otherwise report a total that does not match
the directory.

### Index

| ID | Decision | Status | Turns on |
|----|----------|--------|----------|
| [001](001_the_backend_discrimination_surface_has_no_caller.md) | The backend discrimination surface has no caller | **Open** | Whether a surface with zero production callers is unnecessary, unadopted, or answering a need nobody had |
| [002](002_free_capacity_keeps_one_signature_over_two_contracts.md) | `free_capacity` keeps one signature over two contracts | **Open** | Whether an SPSC guarantee nobody exploits is worth a documented hazard |

**Both were raised by the item census rather than by a design discussion**, which
is why neither was filed earlier: the question only becomes visible once reach is
measured across the family (→ [`item/002`](../item/002_the_backend_discrimination_surface.md)).
They are also one question seen twice — 002 is the asymmetry, 001 is the remedy
for it, and each is open for a reason the other supplies.

The three design choices a reader might otherwise expect here remain settled
rather than open — enum-over-trait dispatch, two constructors, and no
`Cardinality` type — each documented at the point it takes effect, per the Out of
Scope list above.

The crate's other open question is **not** crate-local: whether `ring_handle`'s
specified `&self` receivers or this crate's `&mut self` ones are correct is a
decision shared between two layers, and it is recorded as an obligation with a
named owner at
[`integration/002`](../integration/002_handle_surface_divergence.md) rather than
decided unilaterally here.



### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/decisions
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
| CO13 | the discrimination surface | n/a — unadopted | Across thirty-two sibling crates, the surface is exercised by `ring_factory` and `ring_handle` tests and named by three doc comments explaining non-use. |
| CO14 | `try_clone` | n/a — observation | `try_clone` has no production caller and is the only way to obtain a second producer, so "unused" and "removable" are different questions. |
| CO15 | `ring_handle` | n/a — inconsistency | `ring_handle` and `ring_bench` each spend prose explaining that they do not forward `try_clone`, which no dependent was reaching for. |
| CO16 | `free_capacity` | n/a — observation | No dependent branches on backend before reading occupancy, so the decision's cost has not yet been paid by anyone. |
| CO17 | `is_full` | **latent hazard** | `is_full` is `free_capacity() == 0`, so it races at MPSC while `ring_shutdown` branches on it; the asymmetry is stated on `is_full` itself now. |
| CO18 | this decision | n/a — doc gap | Both instances under `decisions/` state a status and neither states what evidence would change it. |
