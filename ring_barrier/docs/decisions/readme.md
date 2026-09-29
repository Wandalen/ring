# decisions

Two choices with live alternatives, and both are about the same thing: what a
barrier is allowed to assume about cursors it did not create. One resolves the
empty case; the other decides what shape the non-empty case arrives in.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Zero for a Barrier Over Nothing](001_zero_for_a_barrier_over_nothing.md) | Why an empty barrier reads nothing where an empty gating set writes freely, and why the shared fold refuses to choose |
| 002 | [A Slice Rather Than an Aggregate](002_a_slice_rather_than_an_aggregate.md) | Why `over` takes `&[ PaddedCursor ]`, and the four-cursor handshake the alternative made unwireable |

### The Two at a Glance

| | 001 | 002 |
|--|-----|-----|
| Decides | The `map_or` default | The parameter of `over` |
| Alternative | `capacity`, `u64::MAX`, `unwrap_or( Seq::ZERO )` | `&GatingSet`, `Vec< PaddedCursor >`, `&[ &PaddedCursor ]` |
| Forced by | The `Option` `ring_cursor::slowest` returns | `ring_publish::Publisher` owning its own cursor |
| Reversible | Yes — one line, one test | No — it deletes ten call sites with no mechanical rewrite |
| Asserted by | `an_empty_barrier_and_an_empty_gating_set_answer_oppositely` | The ten `from_ref` sites in `handshake_test.rs` |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -n 'map_or' ring_barrier/src/lib.rs                       # 1 — the decision in 001
grep -c 'from_ref' ring_publish/tests/handshake_test.rs        # 10 — the callers 002 protects
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR30 | `ring_barrier` | n/a — inconsistency | The empty barrier gets three different shapes of answer inside one crate — `None` from `frontier`, `0` from `available` indistinguishable from *caught up*, and `Err( Empty )` from `wait_for` indistinguishable from a spent budget |
| BR31 | family | n/a — duplication | `ring_seqno::slowest` and `ring_cursor::slowest` are one concept at two levels — same name, same return, same empty-set convention, different element type — and nothing in the family distinguishes them but the argument |
| BR32 | `ring_gating` | **wrong doc** | `ring_gating`'s empty-set argument opens by citing `ring_seqno::slowest`; `GatingSet::slowest` calls `ring_cursor::slowest`. Both return `None` for an empty input, so the argument survives its own citation being wrong, which is why it has gone unnoticed |
