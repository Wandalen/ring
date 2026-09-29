# docs

Design documentation for `ring_tls`, as typed doc definitions.

| Directory | Responsibility |
|------|-----------------|
| `algorithm/` | The append and reset procedures — steps, branches, and the one growth allocation |
| `api/` | The two caller surfaces — a synchronization-free writer, and a consolidator reading three primitives rather than one fused call |
| `data_structure/` | The per-thread append log at its decided grain — identity and operations |
| `decisions/` | Architecture Decision Records for open trade-offs this crate could not close unilaterally |
| `definition/` | Module Index — every definition and every instance in this crate, in one place |
| `integration/` | Two dependencies, four conspicuous absences, and the export boundary this crate sits *on* rather than behind |
| `invariant/` | The single-writer epoch discipline that makes zero-lock appends sound, and the allocation count that keeps it lock-free |
| `item/` | The three crates that reach these items, and the ordering constant this crate does not publish |
| `lifecycle/` | A buffer's life bounded by its thread's, and the epoch and registration states it holds between consolidations |
| `non_functional_requirement/` | The measured adoption gate, and the POD/alignment preconditions zero-copy needs |
| `pattern/` | The ordering rule that closes the silent-loss window, and the two-stage composition this crate is half of |
| `pitfall/` | Traps this crate's own vocabulary invites — readings that compile and mislead |
| `type/` | The values the correctness arguments are written in — the tag that makes a region walkable, and the epoch that dates a snapshot |
| `workaround/` | External constraints this crate absorbs, with costs and deletion conditions |

The instances sit at three grains. `invariant/` and
`non_functional_requirement/` document the **contract grain** that
this crate's own design decided — per-thread bump-allocated append,
single-writer epochs, payload agnosticism, adoption gated on a measured
verdict. `algorithm/`, `data_structure/`, `pattern/`, and `pitfall/` document
the **mechanism grain** that follows from this crate being a per-thread
append log. `api/`, `lifecycle/`, `type/`, and
`integration/` document a **surface grain** — what a caller touches, when, and
across which crate boundary.

**The surface grain carries more weight here than it would for an internal
crate.** `ring_tls` is one of the family's five exported crates, so its
signatures are a public contract with no absorbing indirection above them;
questions the `api/` instances leave open are correspondingly more expensive
to leave open than the equivalent ones in a crate hidden behind `ring_handle`
(→ [`integration/001`](integration/001_family_dependency_seam.md)). Two of the
five surface instances also turned out to hold this crate's sharpest unsolved
problems rather than merely describing its edges: the double-buffer-versus-
barrier sealing choice in
[`lifecycle/003`](lifecycle/003_buffer_epoch_cycle.md), and the
thread-death loss window in
[`lifecycle/001`](lifecycle/001_thread_registration_and_teardown.md).

No `format/` directory exists: the concrete buffer layout and growth policy
are undecided, pending a future benchmark verdict, and each instance marks
explicitly which of its details that verdict still owns. `algorithm/001`
specifies the append *procedure* — the step sequence and its cost profile,
which this crate's own contract grain does fix — without specifying the
region layout those steps write into, which it does not.

**`item/` exists, and the condition this paragraph set for it is exactly the
one that came true.** The absence used to be an ordering constraint:
`src/lib.rs` declared no items, so an `item/` instance would have been
invention rather than documentation, and the directory was promised for "when
the implementation does". The implementation arrived and that promise came due.

The answer recorded next was that it should still not be discharged.
`item_des.rulebook.md`'s required sections — Representation, Kind, Definition
location, File Usage Table, Crate Usage Table, and a Caller or Callee Tree per
function — are all derivable from the source, and rustdoc already renders them
from the same declarations, on demand and without drifting. Eleven
hand-maintained instances restating what `cargo doc` computes would be
duplicated knowledge, which `principles_general.rulebook.md` forbids for
exactly the reason it would bite here: two records of one fact, one of which is
always the stale one.

That answer wrote its own escape clause, and the escape clause is what fired: a
reader needing something the source does not carry — a per-item usage story
across crates, once the family has consumers to have one. Two instances were
written rather than eleven, and both are that story rather than a restatement
of the declarations.
[`item/001`](item/001_twenty_two_items_and_the_three_crates_that_reach_them.md)
names the crates that reach these items, which no declaration here records, and
[`item/002`](item/002_no_published_constant_and_a_caller_supplied_ordering.md)
documents an *absence* — no published ordering constant — which rustdoc cannot
render at all, because there is no declaration for it to attach to.

Measured:

```sh
cd "$(git rev-parse --show-toplevel)"
# Was 0 while this crate was a skeleton; the released constraint is the change.
printf 'declarations in src/lib.rs: %s\n' \
  "$( command grep -cE '^\s*(pub )?(fn|struct|enum|trait|type|const) ' ring_tls/src/lib.rs )"
printf 'instances written:          %s\n' \
  "$( ls ring_tls/docs/item/[0-9][0-9][0-9]_*.md | wc -l )"
```

Live output:

```
declarations in src/lib.rs: 15
instances written:          2
```

### Related Crates

Not yet a dependency of any crate — adoption is future work. Doc references
run in both directions across a crate boundary once real. This crate declares
two dependencies of its own, and was factored out as a shared, family-neutral
mechanism rather than being built into one specific consumer.
[`integration/001`](integration/001_family_dependency_seam.md) accounts for
the four family crates conspicuously *absent* from the dependency list, and
[`integration/002`](integration/002_prospective_consumer_adoption.md) for what
adoption would cost a prospective consumer.

| Crate | Relationship |
|-------|--------------|
| [`ring_types/readme.md`](../../ring_types/readme.md) | Dependency — the shared vocabulary tags are drawn from, which must supply tag *representation* without supplying tag *meanings* |
| [`ring_atomic/readme.md`](../../ring_atomic/readme.md) | Dependency — the ordering primitives the seal/publish handshake is built from |
| [`ring_batch/readme.md`](../../ring_batch/readme.md) | Dependency — the contiguous multi-slot claim that lets a drained region land as one claim rather than N, which is this crate's central throughput property |
| [`ring_flush/readme.md`](../../ring_flush/readme.md) | **Dependent, not dependency** — it declares this crate and owns the consolidation trigger. The edge points inward only, so nothing here can reach or name it |
| [`ring_slot/readme.md`](../../ring_slot/readme.md) | **Dev-dependency only** — slot addressing, demoted from `[dependencies]` during implementation. This crate's own [`integration/001`](integration/001_family_dependency_seam.md) had recorded the fit as questionable; it belongs to testing this crate against a ring, not to the mechanism |
| [`ring_mpsc/docs/readme.md`](../../ring_mpsc/docs/readme.md) | Not a competitor but the merge half of the same composition — the reading [`pattern/002`](pattern/002_staging_then_merge.md) exists to correct |
