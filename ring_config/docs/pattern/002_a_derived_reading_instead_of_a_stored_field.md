# Pattern: A Derived Reading Instead of a Stored Field

### Scope

**Purpose:** Record the derive-don't-store pattern as this crate applies it, and
place it against the two crates downstream that answered the same question by
storing a copy.

**Responsibility:** The two derived readings, the absence of any derived field in
the record, and the three treatments the same family gives to the same question —
derived, stored-with-the-source-dropped, and stored-with-the-source-retained.

**In Scope:** `ring_config/src/lib.rs:42-49`, `:221-223`, `:238-240`;
`ring_core/src/lib.rs:120-124`, `:180`;
`ring_bench/src/lib.rs:206-212`, `:226-229`.

**Out of Scope:** What the two readings mean and who calls them is
[`item/002`](../item/002_the_two_derived_readings.md). The measured divergence in
the harness's copies is
[`item/001`](../item/001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md).

---

## Three Answers to the Same Question in One Family

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the two derived readings, computed rather than stored --'
sed -n '/^  pub const fn is_multi_producer( &self ) -> bool$/,/^    self\.producers > 1$/p;/^  pub const fn is_tick_safe( &self ) -> bool$/,/^    self\.wait\.is_non_blocking()$/p' ring_config/src/lib.rs
echo '  -- and every field of the record, for comparison: none of them is derived state --'
command grep -m1 -A7 -F 'pub struct RingConfig' ring_config/src/lib.rs
echo '  -- the same question answered by storing: what a built ring keeps --'
command grep -m1 -A4 -F 'pub struct Ring< T >' ring_core/src/lib.rs
command grep -m1 -F '    Ok( Self { storage, overflow : config.overflow() } )' ring_core/src/lib.rs
echo '  -- and what the harness keeps, alongside the record that already holds it --'
command grep -m1 -A6 -F 'pub struct Workload' ring_bench/src/lib.rs
command grep -m1 -A3 -F '  pub const fn new( config : RingConfig ) -> Self' ring_bench/src/lib.rs
```

Live output:

```
  -- the two derived readings, computed rather than stored --
  pub const fn is_multi_producer( &self ) -> bool
  {
    self.producers > 1
  pub const fn is_tick_safe( &self ) -> bool
  {
    self.wait.is_non_blocking()
  -- and every field of the record, for comparison: none of them is derived state --
pub struct RingConfig
{
  capacity : Capacity,
  wait : WaitKind,
  overflow : OverflowPolicy,
  producers : usize,
  batch : usize,
}
  -- the same question answered by storing: what a built ring keeps --
pub struct Ring< T >
{
  storage : Storage< T >,
  overflow : OverflowPolicy,
}
    Ok( Self { storage, overflow : config.overflow() } )
  -- and what the harness keeps, alongside the record that already holds it --
pub struct Workload
{
  config : RingConfig,
  producers : usize,
  records_per_producer : usize,
  cells : usize,
  semantics : AccumulatorSemantics,
  pub const fn new( config : RingConfig ) -> Self
  {
    Self { config, producers : 1, records_per_producer : 1024, cells : 1, semantics : AccumulatorSemantics::Set }
  }
```

---

### RC39 — Both Derived Readings Are Total Functions of One Field, Which Is Why the Pattern Costs Nothing Here

The record has five fields and none of them is derived state. Everything a caller
can ask that is not simply a field is computed on the spot: `is_multi_producer`
is `self.producers > 1` and `is_tick_safe` is `self.wait.is_non_blocking()`. Both
are one line, both `const`, both reading exactly one field.

That is what makes the pattern free rather than a tradeoff. Each reading is a
total function of a single field, so there is no argument to thread and no case
where the answer is unavailable. Storing them instead would mean two `bool`s in
the struct and a second write inside `with_producers` and `with_wait` to keep
them current — four bytes and two maintenance obligations bought in exchange for
saving a comparison that the optimiser would fold anyway.

**Finding.** The consequence worth recording is not the saved bytes; it is that
the two readings cannot disagree with the fields they report on, at any point in
any chain, including partway through one. A caller that writes
`cfg.with_producers( 4 ).is_multi_producer()` gets `true` because there is no
intermediate state in which the derived answer has not caught up yet.

Nothing says this, and the shape it protects against is not hypothetical — it is
what the crate one level down does with the same two fields. Neither reading's
documentation, nor the struct's, records that the values are computed
deliberately rather than incidentally, so a later maintainer adding a third
reading has no stated convention to follow and no reason to prefer computing it.

---

### RC40 — The Family Answers the Same Question Three Ways and Only the Riskiest One Is Documented

Two crates downstream face the identical question — is a value that came from a
`RingConfig` recomputed from the record or copied out of it — and each answers
differently.

`ring_core::Ring` copies. Its two fields are `storage` and
`overflow : config.overflow()`, so the policy lives in the ring as a duplicate of
what the record said. That copy cannot drift, but the reason is incidental:
`Ring::new` takes the record by value and drops it, so after construction there
is no second reader left to disagree with. Nothing about `Ring` enforces this. It
holds because the record is gone.

`ring_bench::Workload` copies and keeps the source. Its four fields include a
whole `RingConfig` and separate `producers` and `batch` beside it, and
`Workload::new` writes `producers : 1` and `batch : 32` into the copies while the
record it was handed carries whatever it carries — `RingConfig::new` defaults
both to `1`. The two are therefore out of step from the constructor onward, and
both are publicly readable, which is the divergence measured in
[`item/001`](../item/001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md).

**Finding.** Three points, one family, same two fields: derived and safe by
construction, copied and safe by accident, copied and wrong on line one. The only
one carrying a written rationale is `Workload`'s, whose `with_batch` explains at
`:266-271` what its own copy is for — the riskiest of the three is the one that
argued for itself, and the two safer ones are silent.

The practical hook is that this ordering is also the fix path for something
already recorded. `non_functional_requirement/002` notes that the record's own
third promised symptom — reading what a given ring was built with — survives
the implementation, and that the cheap close is to store the whole thirty-two-byte
record in `Ring` instead of one byte of it. Doing that would move `Ring` from the
middle point to the first: `overflow()` would become a derived reading over a
stored record rather than a stored copy of one field, which is the same pattern
this crate applies, applied one level up.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/001`](001_a_consuming_builder_over_a_copy_record.md) | The crate's other pattern, and the guarantee `Copy` removes from it |
| [`item/002`](../item/002_the_two_derived_readings.md) | What the two readings mean and who calls them |
| [`item/001`](../item/001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md) | The measured divergence in the copies this finding contrasts |
| [`non_functional_requirement/002`](../non_functional_requirement/002_a_record_sized_for_a_design_not_yet_built.md) | The fix that would move `Ring` onto this pattern |
| [`lifecycle/002`](../lifecycle/002_nothing_can_ask_a_built_ring_what_it_was_configured_with.md) | What survives construction, and in what shape |

### Sources

| Fact | Where |
|------|-------|
| The two derived readings and their one-line bodies | `ring_config/src/lib.rs:221-223`, `:238-240` |
| Five fields, none of them derived state | `ring_config/src/lib.rs:42-49` |
| `Ring` storing a copy of one field | `ring_core/src/lib.rs:120-124`, `:180` |
| `Workload` storing copies beside the record | `ring_bench/src/lib.rs:206-212` |
| The constructor that puts the copies out of step | `ring_bench/src/lib.rs:226-229` |
| The only written rationale of the three | `ring_bench/src/lib.rs:266-271` |

### Tests

| Test | Covers |
|------|--------|
| `multi_producer_is_derived_from_the_count` | That the reading tracks the field rather than a stored flag |
| `tick_safety_is_exactly_non_blocking_waiting` | The same for the second reading |
| `each_setter_is_independent` | That no setter maintains derived state on the side |
| `setters_commute` | That there is no intermediate state for a reading to lag behind |
