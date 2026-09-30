# Decision: A Slice Rather Than an Aggregate

### Scope

- **Purpose**: Record why `Barrier::over` takes `&[ PaddedCursor ]` rather than the aggregate the producer side already has, and name the concrete thing that would break otherwise.
- **Responsibility**: Give the alternative, the failure it produced, the four-cursor wiring that forced the change, and what the slice costs in exchange.
- **In Scope**: The parameter type of `over`.
- **Out of Scope**: The provenances the slice admits — see [`data_structure/002`](../data_structure/002_the_slices_three_provenances.md).

### The Decision

```rust
// ring_barrier/src/lib.rs:104
pub const fn over( dependencies : &'a [ PaddedCursor ] ) -> Self
```

**Dependencies are a borrowed slice, not a `GatingSet` and not an owned `Vec`.**

The obvious alternative had a real argument behind it: `ring_barrier` and
`ring_gating` are two halves of one thing, `ring_gating` already owns a set of cursors, and
`Barrier::over( &GatingSet )` would have made the pairing visible in the type
system. It was tried and it does not work.

### What Broke

> Taking the aggregate instead would have meant a publisher's cursor could never
> be depended on at all, since there is no way to move an existing cursor into a
> set that owns its own. That is not a hypothetical: it is what made the
> four-operation handshake in `ring_publish/tests/handshake_test.rs` unwireable
> until this signature changed.
>
> — `ring_barrier/src/lib.rs:40-44`

The handshake test's own header explains why its wiring is load-bearing: four
crates, four cursors, and only two of them shared.

| Cursor | Owner | Read by |
|--------|-------|---------|
| Claimed | `ring_claim::Claimer`, private | nobody |
| **Published** | **`ring_publish::Publisher`** | **the consumer's barrier** |
| Consumer position | the producer's `GatingSet` | the producer, to stop rather than lap |
| Capacity | that same `GatingSet` | the producer |

The second row is the problem. `Publisher` owns its published cursor and hands
out `&PaddedCursor`; a `GatingSet` allocates its own cursors in a `Vec` it owns.
There is no operation that moves an existing `PaddedCursor` into somebody else's
`Vec` — and there should not be, because the publisher is still writing to it.
So an aggregate-taking `Barrier::over` makes *the single most common shape in
the family* — one consumer reading behind one producer — unexpressible.

With a slice it is one `const fn` call and no allocation:

```rust
// ring_publish/tests/handshake_test.rs:119 (and nine more)
let barrier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
```

### The Alternatives, With What Each Costs

| Parameter | Publisher's cursor | `GatingSet`'s cursors | A bare array | Allocates |
|-----------|:------------------:|:---------------------:|:------------:|:---------:|
| `&'a [ PaddedCursor ]` | ✔ `from_ref` | ✔ `.cursors()` | ✔ | no |
| `&'a GatingSet` | ✘ **unwireable** | ✔ | ✘ | no |
| `Vec< PaddedCursor >` | ✘ cannot move it | ✘ cannot move it | ✔ by copy | yes |
| `&'a [ &'a PaddedCursor ]` | ✔ | ✘ needs a rebuild | ✔ | at the caller |

The fourth row is the near miss. A slice of *references* accepts the publisher's
cursor and a hand-assembled list, but a `GatingSet` stores `PaddedCursor` values
contiguously, so feeding it in requires building a `Vec< &PaddedCursor >` — an
allocation the caller pays, per barrier, to describe storage that already
exists. `a_barrier_over_a_gating_set_reads_that_set_and_not_a_copy` would then
be asserting something about a copy of the layout rather than the layout.

### What the Slice Costs

| Cost | Detail |
|------|--------|
| The pairing is invisible to the compiler | Nothing in the type says these cursors are `ring_gating`'s other half; `ring_gating` is a dev-dependency, named only by tests — [`workaround/002`](../workaround/002_ring_gating_as_a_dev_dependency.md) |
| Contiguity is required | Dependencies scattered across three owners cannot be one barrier; they need one barrier each, or a caller-built array |
| A wrong slice compiles | A barrier over a private cursor nobody advances is well-typed and waits forever — the exact failure `the_consumer_position_the_producer_gates_on_is_the_one_commit_moves` exists to catch, one crate over |

The third is the sharpest, and it is not fixable at this boundary. Any signature
that accepts a cursor accepts the *wrong* cursor, because "the cursor the
producer actually publishes to" is a fact about the wiring, not about the type.
The family's answer is to assert the wiring itself in
`ring_publish/tests/handshake_test.rs` rather than to encode it — which is why
that test's header says the wiring is the test as much as the assertions are.

### Why This Is Not Reversible Later

The slice is on the crate's only constructor, so narrowing it later breaks every
one of the 63 construction sites and, more importantly, deletes the ten
`from_ref` sites outright — there is no mechanical rewrite from a publisher's
cursor to an aggregate that owns its own. Widening it (to
`impl AsRef< [ PaddedCursor ] >`, say) is possible at any time and buys nothing:
every current caller already has a slice or a one-line way to make one.

### BR31 — The Family Has Two Functions Named `slowest` Over Different Element Types

`ring_seqno::slowest` folds a `&[ Seq ]`. `ring_cursor::slowest` folds a
`&[ PaddedCursor ]`. Same name, same return type, same empty-set convention,
different input — one takes values already read, the other performs the reads.

The split is correct: `ring_seqno` must stay free of atomics, so the fold that
loads cannot live there. But the two are one concept at two levels of a stack
that has no way to say so, and a reader who greps the family for `slowest` finds
both with nothing distinguishing them but a type.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r "^pub fn slowest" ring_seqno/src/lib.rs ring_cursor/src/lib.rs
# every crate that folds one, and which of the two it reaches for
command grep -r "ring_seqno::slowest\|ring_cursor::slowest" --include=*.rs ring_*/src/ | sed 's|ring/||'
```

Live output:

```
ring_seqno/src/lib.rs:pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
ring_cursor/src/lib.rs:pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
ring_barrier/src/lib.rs://! [`ring_cursor::slowest`] and lives in neither of them.
ring_barrier/src/lib.rs:    ring_cursor::slowest( self.dependencies )
ring_cursor/src/lib.rs:/// assert_eq!( ring_cursor::slowest( &[] ), None );
ring_cursor/src/lib.rs:/// assert_eq!( ring_cursor::slowest( &cursors ), Some( Seq( 4 ) ) );
ring_cursor/src/lib.rs:/// assert_eq!( ring_cursor::slowest( &cursors ), Some( Seq( 9 ) ) );
ring_gating/src/lib.rs://! `ring_cursor::slowest` returns `None` for an empty set rather than `Seq::ZERO`,
ring_gating/src/lib.rs:  /// The fold itself is [`ring_cursor::slowest`], shared with `ring_barrier`,
ring_gating/src/lib.rs:    ring_cursor::slowest( &self.cursors )
ring_seqno/src/lib.rs:/// use ring_seqno::slowest;
```

### BR32 — `ring_gating`'s Empty-Set Argument Names the Function Its Code Does Not Call

`ring_gating`'s module documentation opens its empty-set section with
*"`ring_seqno::slowest` returns `None` for an empty set rather than `Seq::ZERO`,
and this crate carries that distinction through"*. `GatingSet::slowest` calls
`ring_cursor::slowest`.

Both return `None` for an empty input, so the argument survives its own citation
being wrong — which is exactly why it has gone unnoticed. The two functions are
distinguishable only by element type (BR31), and the sentence is making a claim
about a convention rather than about a call, so the substitution reads fine.
This crate inherits the same convention from the same place and states it in its
own words instead, which is why the discrepancy is visible from here at all.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A1 -F '//! `ring_seqno::slowest` returns `None` for an empty set rather than `Seq::ZERO`,' ring_gating/src/lib.rs
# what GatingSet::slowest actually calls
grep -A3 "pub fn slowest" ring_gating/src/lib.rs | grep "ring_"
```

Live output:

```
        ring_cursor::slowest(&self.cursors)
```

**Disposition:** declined — the wrong-function citation is `ring_gating`'s own
module doc at `ring_gating/src/lib.rs:24`; this instance's own text
already names `ring_gating` as the crate whose doc says `ring_seqno::slowest`
when its code calls `ring_cursor::slowest` — not this crate's doc to change in
a corpus disposition pass.

**Correction (2026-09-28):** the finding above, and the Disposition declining
to fix it from here, were accurate when written but are no longer current.
`ring_gating/src/lib.rs:24` now reads *"`ring_cursor::slowest` returns
`None` for an empty set rather than `Seq::ZERO`,"* — matching what
`GatingSet::slowest` actually calls. The wrong-function citation this finding
named is gone; nothing here still needs the fix the Disposition placed out of
this crate's scope.

### Decisions

| File | Relationship |
|------|--------------|
| [001_zero_for_a_barrier_over_nothing.md](001_zero_for_a_barrier_over_nothing.md) | The other decision this crate makes on its own |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_nine_methods_over_one_borrowed_slice.md](../api/001_nine_methods_over_one_borrowed_slice.md) | `over` among the nine |
| [../api/002_the_borrow_is_the_whole_type.md](../api/002_the_borrow_is_the_whole_type.md) | What the `&'a` propagates into |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_field_and_a_sixteen_byte_view.md](../data_structure/001_one_field_and_a_sixteen_byte_view.md) | The field the slice lands in |
| [../data_structure/002_the_slices_three_provenances.md](../data_structure/002_the_slices_three_provenances.md) | The three origins this signature admits |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_three_dependencies_and_one_dependent.md](../integration/001_three_dependencies_and_one_dependent.md) | Why `ring_publish` is a caller and not a dependency |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_a_consumer_draining_behind_a_producer.md](../lifecycle/002_a_consumer_draining_behind_a_producer.md) | The handshake in sequence |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_borrowed_view_and_the_owned_set.md](../pattern/001_the_borrowed_view_and_the_owned_set.md) | Borrow-versus-own as a family shape |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/002_ring_gating_as_a_dev_dependency.md](../workaround/002_ring_gating_as_a_dev_dependency.md) | The edge this decision removed from the manifest |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:30-44` | The module's own argument |
| `ring_barrier/src/lib.rs:88-107` | `over`, and the `from_ref` note in its doc |
| `ring_publish/src/lib.rs:117` | The cursor that cannot be moved |
| `ring_gating/src/lib.rs:155` | The aggregate's own slice accessor |

### Tests

| File | Relationship |
|------|--------------|
| `ring_publish/tests/handshake_test.rs:1-51` | The four-cursor wiring, and why it is the test |
| `ring_publish/tests/handshake_test.rs:119` | The `from_ref` call this signature exists for |
| `tests/barrier_test.rs:436-449` | The aggregate provenance, still available |
