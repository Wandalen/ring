# pattern

The two reusable shapes this crate instantiates, and what each one costs.

### Overview Table

| ID | Name | Shape | Instantiated by |
|----|------|-------|-----------------|
| 001 | [The Forwarding Newtype](001_the_forwarding_newtype.md) | A newtype that adds no capability and exists so a trait impl is legal | `PaddedCursor` |
| 002 | [One Fold, Two Questions](002_one_fold_two_questions.md) | One function serving callers that mean opposite things by its answer | `slowest`, and its `None` |

**Both are about not duplicating a decision**, and they fail differently. 001 is
enforced by the compiler — the shape is not merely preferred, it is the only
legal arrangement. 002 is enforced by nothing, and holds because the two callers
were written by someone who had read both.

The family-wide version of the same concern — one constant, many crates — is
[`decisions/001`](../decisions/001_gating_is_fixed_not_a_parameter.md), and it is
the one that did not hold.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the newtype, and the four forwards it is made of --'
command grep -E '^pub struct PaddedCursor|^    self\.0\.get\(\)' ring_cursor/src/lib.rs
echo '  -- the one fold, and who calls it outside doc comments --'
command grep -r 'ring_cursor::slowest' --include=*.rs */src/ | command grep -v '///' | sed 's|ring/||'
```

Live output:

```
  -- the newtype, and the four forwards it is made of --
pub struct PaddedCursor( CacheAligned< AtomicSeq > );
    self.0.get().load( order )
    self.0.get().store( value, order );
    self.0.get().fetch_add( n, order )
    self.0.get().compare_exchange( current, new, success, failure )
  -- the one fold, and who calls it outside doc comments --
ring_barrier/src/lib.rs://! [`ring_cursor::slowest`] and lives in neither of them.
ring_barrier/src/lib.rs:    ring_cursor::slowest( self.dependencies )
ring_gating/src/lib.rs://! `ring_cursor::slowest` returns `None` for an empty set rather than `Seq::ZERO`,
ring_gating/src/lib.rs:    ring_cursor::slowest( &self.cursors )
```

**Four bodies, each one line, each the same shape.** The second arm now surfaces
four hits, not the three it printed when this prose was last written —
`ring_gating`'s module doc gained a `//!` line reattributing `slowest` from
`ring_seqno` to `ring_cursor` (a wording correction, not a new caller). The fold's
entire production user base is still the two calls: `ring_barrier` bounding a
consumer and `ring_gating` bounding a producer — which is what makes sharing it
a pattern rather than a coincidence of two callers.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU37 | The four forwards | n/a — observation | Every method body is `self.0.get().<same name>( … )` — one line each, no transformation, no reordering of arguments. The newtype's entire behaviour is the layout it inherits plus the vocabulary it re-exports, which is what makes it a pattern rather than a wrapper with opinions |
| CU38 | `CacheAligned::get` | **latent hazard** | The pattern holds only while that method exists and stays free. It is in another crate, it is not `const`, and this crate asserts nothing about it — so a change that made `get` do work would add that work to all four forwards and to all three readings, invisibly to every test here |
| CU39 | The shared fold | n/a — observation | The two crates that ask the fold opposite questions are also its only two callers, so the pattern's justification and its entire user base are the same two lines. That is not an argument against sharing it — it is what makes the third caller the one that will test the pattern |
| CU40 | The `None` decision | n/a — duplication | Three tests across three crates assert the empty case independently — this crate's `slowest` doctest, `ring_gating`'s pair in `gating_test.rs`, and `ring_barrier`'s `an_empty_barrier_and_an_empty_gating_set_answer_oppositely`. Nothing links this crate's `None` to the two opposite readings that depend on it |
