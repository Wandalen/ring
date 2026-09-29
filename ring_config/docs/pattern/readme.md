# pattern

The crate applies two patterns and nothing else. A consuming builder — four
`with_*` methods that take `self` by value and hand back a replacement — and a
derived reading, where a question the record can answer is computed from a field
rather than stored beside it. Both are applied uniformly, correctly, and without
comment.

Both instances here are about what the surrounding choices do to those patterns.
The consuming builder's safety property is the move, and `Copy` deletes it, so the
four `#[ must_use ]` attributes are standing in for a compile error rather than
tidying up after one. The derived reading's safety property is that the answer
cannot lag the field, and the two crates downstream that copy instead of deriving
show both what that buys and what its absence costs.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_consuming_builder_over_a_copy_record.md) | A Consuming Builder Over a Copy Record | The four setters, the `must_use` that replaces a move, and the workspace census |
| [002](002_a_derived_reading_instead_of_a_stored_field.md) | A Derived Reading Instead of a Stored Field | The two computed readings, and the family's three answers to the same question |

## The Guarantee `Copy` Removes

On an ordinary consuming builder, forgetting to bind a setter's result does not
compile — the receiver was moved, so the next use of it is an error. That is the
pattern's whole safety property, and it is free.

`RingConfig` is `Copy`, so the move is a copy and the discard compiles. What
remains is `#[ must_use ]`, which turns the same mistake into a warn-level lint —
an error only under `RUSTFLAGS="-D warnings"`, which the family's verification
levels set and a caller's own build need not. The attribute and the derive that
makes it necessary sit thirty lines apart in one file and neither mentions the
other.

## Derived, Copied, and Copied Wrong

Three crates hold the same two fields and answer differently. `RingConfig`
derives: `is_multi_producer` is `self.producers > 1`, computed on the spot, unable
to disagree with its source. `ring_core::Ring` copies `overflow` out of the
record and is safe anyway, because the record is dropped and no second reader
survives to disagree. `ring_bench::Workload` copies `producers` and `batch` while
keeping the record that holds them, and its constructor writes the copies with
different literals, so its two public getters are out of step from line one.

Only the last of the three explains itself in a comment.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the two patterns: consuming setters, then derived readings --'
command grep -c 'pub const fn with_' ring_config/src/lib.rs || true
command grep -c 'pub const fn is_' ring_config/src/lib.rs || true
echo '  -- must_use attributes immediately above a consuming setter --'
command grep -B 1 'pub const fn with_' ring_config/src/lib.rs | command grep -c 'must_use' || true
echo '  -- files anywhere in the workspace declaring a consuming builder --'
command grep -rl 'fn [a-z_]*( mut self' --include=*.rs */src | sort 
echo '  -- every field of type RingConfig or OverflowPolicy in the family --'
command grep -rn '^  [a-z_]* : \(RingConfig\|OverflowPolicy\),' --include=*.rs */src 
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RC37 | `ring_config` | n/a — doc gap | The consuming builder's safety property is that the receiver is moved, so a discarded setter result is a compile error — and `Copy` removes it, leaving `#[ must_use ]` to stand in with a warn-level lint that is an error only under `RUSTFLAGS="-D warnings"`; a probe confirms both halves on the same line of code, the four attributes are therefore load-bearing rather than hygiene, nothing in the crate says so, and no test can notice their removal because every test binds every result as correct code does |
| RC38 | `ring_config` | n/a — observation | Eleven consuming builders across four receiver types exist in this family and exactly two receivers are `Copy` — `RingConfig` and `ring_bench::Workload`, both records that configure a ring — where four of `Workload`'s five setters return `Result` and carry no attribute, safe anyway because `Result` is `#[ must_use ]` in the standard library, while the fifth, `with_semantics`, returns `Self` and carries the attribute by hand exactly as this crate's four do, so the guarantee is bought by hand wherever the return is infallible and comes free from the fallibility `decisions/002` records this crate rejecting, and the two answer the same zero with `Err( ZeroProducers )` and `1` respectively |
| RC39 | `ring_config` | n/a — doc gap | Neither derived reading is stored: `is_multi_producer` is `self.producers > 1` and `is_tick_safe` is `self.wait.is_non_blocking()`, each a total function of one field, so the pattern costs nothing and buys the guarantee that no reading can lag its source at any point in any chain — storing them instead would add two `bool`s and a second write inside two setters — and nothing in the record, the readings' docs or the crate states that the values are computed deliberately, leaving a third reading with no convention to follow |
| RC40 | `ring_config` | n/a — observation | The same two fields get three treatments in one family — derived and safe by construction here, copied into `ring_core::Ring` and safe only because the source record is dropped, copied into `ring_bench::Workload` beside the record that still holds them and written with different literals by its own constructor — and only the riskiest of the three carries a written rationale, at `ring_bench:266-271`; storing the whole record in `Ring`, which `non_functional_requirement/002` records as the cheap close for the record's own surviving symptom, would also move `Ring` from the middle treatment to the first |
