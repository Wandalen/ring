# ring_publish

Publication of claimed slots to consumers.

Depends on [`ring_types`](../ring_types/readme.md), [`ring_cursor`](../ring_cursor/readme.md).

One of the 33 `ring_*` crates that make up this family's concurrency
write-path — a 33-crate dependency forest rooted at `ring_types`, acyclic
by construction. Build order follows [`../Cargo.toml`](../Cargo.toml)'s
member list; the family as a whole is described in
[`../readme.md`](../readme.md).

Implemented, delivering the publish half of the four-operation handshake — a producer advances the
published cursor only when it has *finished writing* the range it claimed, and
only when its predecessor has already published. Behaviour is asserted by
[`tests/publish_test.rs`](tests/publish_test.rs) and read by hand against
[`tests/manual/readme.md`](tests/manual/readme.md); every line is covered.

The handshake's reached-test lives here too, in
[`tests/handshake_test.rs`](tests/handshake_test.rs): the whole
claim → publish → available → commit handshake across four crates, in two
harnesses. Real threads carry 20,000 items through a 16-slot ring; under
`--cfg loom` the same file becomes an exhaustive model of one claim against one
drain, checking every interleaving. Publication is where the reached-test
belongs because it is the moment the other three operations become observable
together — before it a claim is invisible, after it the consumer's whole
contract is decided.

The published cursor is deliberately *not* `ring_claim`'s cursor. Between a
producer taking a range and finishing it, the slot is claimed and unwritten, and
the handshake's central requirement is that no consumer sees it. Conflating the
two publishes uninitialised memory, passes every single-threaded test, and fails
only under load — which is exactly the bug the loom model exists to catch, and
does: weakening the publish ordering to `Relaxed` still passes all eight
threaded tests on x86, and fails the model immediately.

The crate is 222 lines: one `struct` with one field, six public methods, no
`unsafe`, no `std::` path, and one cast — `len as u64`, in the widening
direction. Its executable content is a compare-exchange and a loop around it.
What it holds instead is a boundary: the single moment a consumer is allowed to
see a slot, and the guarantee that it sees nothing earlier. The corpus under
`docs/` is about that boundary, what it costs, and why nothing yet depends on it.

| File | Responsibility |
|------|-----------------|
| `docs/` | 13 doc definitions, 26 instances, 46 findings — see [docs/readme.md](docs/readme.md) |
| `src/lib.rs` | Crate root — the crate's whole public surface |
| `tests/` | `publish_test.rs`, the handshake's `handshake_test.rs`, and the manual plan under `manual/` |

### Reading Order

| Read | For |
|------|-----|
| [`docs/definition/readme.md`](docs/definition/readme.md) | The Module Index — every definition, every instance, and all 46 findings with severity |
| [`docs/algorithm/001`](docs/algorithm/001_the_compare_exchange_that_refuses.md) | The one operation: three positions a `start` can be in, and what each means |
| [`docs/invariant/002`](docs/invariant/002_is_published_is_exclusive_of_the_frontier.md) | Why the boundary is exclusive, asserted 320 times |
| [`docs/lifecycle/002`](docs/lifecycle/002_the_four_operation_handshake.md) | The handshake's three clauses, and which assertion pins each |
| [`docs/pitfall/001`](docs/pitfall/001_publishing_a_range_you_never_claimed.md) | The one thing to get right before calling `publish` |
| [`docs/algorithm/002`](docs/algorithm/002_a_loop_with_no_budget.md) | Why the family's only unbounded spin is allowed to be one |
| [`docs/integration/001`](docs/integration/001_ten_crates_name_it_and_none_depends_on_it.md) | Every edge in and out, and why there are no edges in |
