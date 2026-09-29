# Item: Five Fields, and the One Another Crate Keeps a Copy Of

### Scope

**Purpose:** Read the record as a list of five named fields — what each one is,
which of them anything outside the crate actually reads, and where the two that
nothing reads went instead.

**Responsibility:** The five declarations, the per-field read census, and
`ring_bench::Workload`'s shadow copies of `producers` and `batch`.

**In Scope:** `ring_config/src/lib.rs:42-49`;
`ring_bench/src/lib.rs:206-212`, `:226-229`, `:244`, `:266-271`, `:283`,
`:248-271`.

**Out of Scope:** The two derived readings computed from these fields are
[`item/002`](002_the_two_derived_readings.md). The declarations as a public
surface, with their attributes, are
[`api/001`](../api/001_twelve_functions_eleven_of_them_const.md).

---

## Five Names, and Where Two of Them Are Kept Instead

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the five fields --'
command grep -m1 -A7 -F 'pub struct RingConfig' ring_config/src/lib.rs
echo '  -- and the struct in another crate that holds one and re-declares two of them --'
command grep -m1 -A6 -F 'pub struct Workload' ring_bench/src/lib.rs
echo '  -- its constructor, which fills the copies and leaves the record it was handed alone --'
command grep -m1 -A3 -F '  pub const fn new( config : RingConfig ) -> Self' ring_bench/src/lib.rs
echo '  -- the two setters that do write through --'
command grep 'self\.config = self\.config' ring_bench/src/lib.rs
echo '  -- every non-doctest read through a RingConfig receiver outside this crate --'
command grep -rE '(config\(\)|config|cfg)\.(capacity|wait|overflow|producers|batch|is_multi_producer|is_tick_safe)\(\)' --include=*.rs */src | command grep -v '^ring_config/' | command grep -v '///\|//!' | command grep -oE '(config\(\)|config|cfg)\.(capacity|wait|overflow|producers|batch|is_multi_producer|is_tick_safe)\(\)' | sort | uniq -c
echo '  -- and the two shadowed names read off the copy instead --'
command grep -rE 'workload\.(producers|batch)\(\)' --include=*.rs */src | command grep -v '///\|//!' | command grep -oE 'workload\.(producers|batch)\(\)' | sort | uniq -c
```

Live output:

```
  -- the five fields --
pub struct RingConfig
{
  capacity : Capacity,
  wait : WaitKind,
  overflow : OverflowPolicy,
  producers : usize,
  batch : usize,
}
  -- and the struct in another crate that holds one and re-declares two of them --
pub struct Workload
{
  config : RingConfig,
  producers : usize,
  records_per_producer : usize,
  cells : usize,
  semantics : AccumulatorSemantics,
  -- its constructor, which fills the copies and leaves the record it was handed alone --
  pub const fn new( config : RingConfig ) -> Self
  {
    Self { config, producers : 1, records_per_producer : 1024, cells : 1, semantics : AccumulatorSemantics::Set }
  }
  -- the two setters that do write through --
    self.config = self.config.with_producers( producers );
    self.config = self.config.with_batch( batch );
  -- every non-doctest read through a RingConfig receiver outside this crate --
      1 config.batch()
      5 config.capacity()
      1 config.is_multi_producer()
      2 config().overflow()
      3 config.overflow()
  -- and the two shadowed names read off the copy instead --
      7 workload.batch()
      9 workload.producers()
```

---

### RC25 — Two Fields Are Read as Themselves, One as a Single Bit, and Two Not at All

Read the census by field rather than by call site and the record separates into
three groups.

`capacity` and `overflow` are read as themselves: four reads of `config.capacity()`
and five of the overflow — three direct, two through `workload.config()`. Both are
consumed as values, and the whole value matters.

`producers` is read once, and not as a number. `config.is_multi_producer()` is the
single production read anywhere in the workspace that reaches the field, and it
returns `self.producers > 1`. So the record stores a `usize` that could be `2` or
`64`, and every consumer that exists asks it one yes-or-no question.

`wait` and `batch` are read zero times. Not once, in any `src/` file in
thirty-three crates, outside doc comments.

**Finding.** Three of the five fields never reach a consumer in the form the
record carries them. Two are unread and the third is downcast to a bit at its only
call site, which puts four bytes of the eight `producers` occupies — and all eight
of `batch`'s, and `wait`'s one — beyond anything's reach.

`ring_factory`'s own suite reached the behavioural half of this independently and
gave a separate reason per field: `producers` "selects a backend, and `Split`
reaches no backend"; `wait` because "nothing in the closure honours it"; `batch`
because "`ring_core` was implemented without reading it". Three fields, three
unrelated causes, one shape — which is why this is recorded per field here rather
than as a single count.

---

### RC26 — `ring_bench::Workload` Holds the Record and Two Copies of Its Fields, and Its Constructor Leaves One Disagreeing

`Workload` carries a `RingConfig` and re-declares two of that record's five
fields beside it:

```rust
pub struct Workload
{
  config : RingConfig,
  producers : usize,
  records_per_producer : usize,
  batch : usize,
}
```

Both setters keep the copy and the record in step — `:204` writes
`self.config.with_producers( producers )`, `:242` writes
`self.config.with_batch( batch )`. The constructor does not. `Workload::new`
writes `producers : 1, batch : 32` into the copies and stores the `RingConfig` it
was handed untouched, and a `RingConfig` fresh from `new` carries `batch : 1`.

A probe against both crates:

```
RingConfig::new( 1024 )      batch 1   producers 1
Workload::new( cfg )         workload.batch()  32   config().batch()   1   workload.producers() 1   config().producers() 1
  .with_batch( 32 )          workload.batch()  32   config().batch()  32   workload.producers() 1   config().producers() 1
  .with_producers( 4 )       workload.batch()  32   config().batch()   1   workload.producers() 4   config().producers() 4
  both setters               workload.batch()  32   config().batch()  32   workload.producers() 4   config().producers() 4
```

```rust
let cfg = RingConfig::new( 1024 ).unwrap();
let fresh = Workload::new( cfg );
// two public getters, one struct, thirty-two apart
println!( "{} {}", fresh.batch(), fresh.config().batch() );
```

**Finding.** A default `Workload` reports a batch of `32` through one public
getter and `1` through another, and both are reachable from ordinary compiling
code. Setting the producer count does not repair it — line four of the probe shows
`producers` reconciled to `4` while `batch` is still `32` against `1`. Only calling
`with_batch` explicitly brings the two into agreement, and a caller has no reason
to call a setter for a value they are happy to take as the default.

Nothing misbehaves today, for the reason RC25 records: no ring backend reads
`RingConfig::batch`, so the stale `1` never reaches anything. That is also the only
thing holding it. `Workload::with_batch`'s own doc states the stake precisely —
"Every candidate honours this: the mutex queue takes its lock once per batch, the
ring candidates publish a batch at a time, and the staged candidate binds it as its
`FlushPolicy::OnBatch` trigger. A batch size that only one arm observed would
measure batching against nothing." A backend added tomorrow that reads its batch
from the configuration, as `ring_core` reads its overflow, would be the arm that
observed a different number — and it would not fail, it would quietly measure
batches of one against batches of thirty-two.

The fix is one line in `Workload::new`: build the stored config from the same two
literals the copies get, rather than from the caller's record alone.

**Disposition:** declined — the one-line fix belongs in `ring_bench::Workload::new`
(`ring_bench/src/lib.rs:226-229`), a different crate from this
dispositioning pass's three assigned crates (`ring_config`, `ring_consume`,
`ring_cursor`); not this crate's own `src/` to change here.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](002_the_two_derived_readings.md) | The two readings computed from these fields |
| [`api/002`](../api/002_three_readers_used_and_four_with_no_caller.md) | The same census taken per accessor rather than per field |
| [`type/002`](../type/002_two_counts_that_are_usize_and_three_fields_that_are_not.md) | Why `producers` and `batch` are bare `usize` while the other three are types |
| [`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md) | The clamp that guards the field nothing reads |

### Sources

| Fact | Where |
|------|-------|
| The five fields | `ring_config/src/lib.rs:42-49` |
| `Workload` holding the record and two copies | `ring_bench/src/lib.rs:206-212` |
| The constructor writing `batch : 32` past an untouched config | `ring_bench/src/lib.rs:226-229` |
| Both setters writing through | `ring_bench/src/lib.rs:244`, `:283` |
| `with_batch`'s stated stake | `ring_bench/src/lib.rs:266-271` |
| Per-field read counts | Census above |
| The 32-against-1 divergence | Probe above, source inlined |
| `ring_factory`'s per-field reasons | `ring_factory/tests/factory_test.rs:36-42` |

### Tests

| Test | Covers |
|------|--------|
| `every_named_field_is_carried` | That all five survive construction |
| `defaults_are_the_documented_ones` | The `batch : 1` a fresh record carries |
| `multi_producer_is_derived_from_the_count` | The one bit anything reads off `producers` |
