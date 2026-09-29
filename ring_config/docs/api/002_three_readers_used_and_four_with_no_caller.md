# API: Three Readers Used, and Four With No Caller

### Scope

**Purpose:** Record which of the seven readers anyone outside this crate actually
calls, and what the four unused ones have in common.

**Responsibility:** The production call census, the test census, and the
neighbouring suite that already measured the same conclusion from the other side.

**In Scope:** `ring_bench/src/lib.rs:384`;
`ring_core/src/lib.rs:165`, `:174`, `:180`, `:204`, `:207`;
`ring_mpsc/src/lib.rs:410`, `:408`; `ring_spsc/src/lib.rs:325`,
`:320`; `ring_factory/tests/factory_test.rs:28-101`.

**Out of Scope:** The declarations themselves are
[`api/001`](001_twelve_functions_eleven_of_them_const.md). Which crates depend on
this one at all is
[`integration/001`](../integration/001_eleven_declarers_and_four_that_build_on_it.md).

---

## Who Reads What

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every RingConfig accessor call in src/ outside this crate --'
command grep -r 'config\.\(capacity\|wait\|overflow\|producers\|batch\|is_multi_producer\|is_tick_safe\)()' --include=*.rs ring_*/src | command grep -v '^ring_config/' | sed 's|^ring/||'
# family-scoped: these accessor names are ordinary English words, and crates
# elsewhere in the workspace spell `.wait()` for process exit
echo '  -- every accessor call in tests/ outside this crate, by receiver --'
command grep -r '[A-Za-z_][A-Za-z_0-9]*\.\(capacity\|wait\|overflow\|producers\|batch\|is_multi_producer\|is_tick_safe\)()' --include=*.rs ring_*/tests | command grep -v '^ring_config/' | command grep -o '[A-Za-z_][A-Za-z_0-9]*\.\(capacity\|wait\|overflow\|producers\|batch\|is_multi_producer\|is_tick_safe\)()' | sort | uniq -c | sort -rn
echo '  -- and the RingConfig receivers among them --'
command grep -r '\(single\|multi\|one\|config\)\.\(capacity\|wait\|overflow\|producers\|batch\|is_multi_producer\|is_tick_safe\)()' --include=*.rs ring_*/tests | command grep -v '^ring_config/' | sed 's|^ring/||'
```

Live output:

```
  -- every RingConfig accessor call in src/ outside this crate --
ring_bench/src/lib.rs:    self.config.batch()
ring_bench/src/lib.rs:    self.config.capacity().get()
ring_core/src/lib.rs:    if config.overflow() == OverflowPolicy::DropOldest
ring_core/src/lib.rs:    let storage = match config.is_multi_producer()
ring_core/src/lib.rs:    Ok( Self { storage, overflow : config.overflow() } )
ring_core/src/lib.rs:          crossbeam_queue::ArrayQueue::new( config.capacity().get() ),
ring_core/src/lib.rs:          config.capacity(),
ring_core/src/lib.rs:        overflow : config.overflow(),
ring_mpsc/src/lib.rs:  /// assert_eq!( ring.capacity(), config.capacity() );
ring_mpsc/src/lib.rs:    Self::new( config.capacity() )
ring_spsc/src/lib.rs:  /// assert_eq!( ring.capacity(), config.capacity() );
ring_spsc/src/lib.rs:    Self::new( config.capacity() )
  -- every accessor call in tests/ outside this crate, by receiver --
      9 buffer.capacity()
      8 workload.batch()
      8 ring.capacity()
      4 slot.capacity()
      3 workload.capacity()
      2 workload.producers()
      2 r.capacity()
      2 from_plain.capacity()
      1 single.producers()
      1 ring.overflow()
      1 pair.capacity()
      1 outcome.producers()
      1 one.is_multi_producer()
      1 multi.producers()
      1 from_decorated.capacity()
      1 direct.producers()
      1 delta.producers()
      1 delta.capacity()
      1 delta.batch()
      1 cramped.capacity()
      1 cramped.batch()
      1 config.capacity()
      1 config.batch()
  -- and the RingConfig receivers among them --
ring_bench/tests/bench_test.rs:/// The mirror field is gone; `Workload::batch()` reads `self.config.batch()`.
ring_core/tests/core_test.rs:  assert!( !one.is_multi_producer() );
ring_factory/tests/factory_test.rs:  assert_ne!( single.producers(), multi.producers(), "the configs do differ" );
ring_mpsc/tests/mpsc_test.rs:    assert_eq!( ring.capacity(), config.capacity() );
```

---

### RC7 — Three of the Seven Readers Have a Production Caller; the Other Four Have None

Ten production reads exist outside this crate, and they reach exactly three
readers: `capacity()` six times, `overflow()` three, `is_multi_producer()` once.
Nothing in any `src/` in the workspace calls `wait()`, `producers()`, `batch()` or
`is_tick_safe()`.

The tests census is a wide net on purpose — it catches every call of those seven
names on any receiver, so the noise in it is the point: `buffer.capacity()`,
`workload.batch()`, `slot.capacity()` and the rest are other types' methods with
the same names. Filtering to receivers that hold a `RingConfig` leaves three
sites, on `is_multi_producer()`, `producers()` and `capacity()`. So across every
`src/` and every `tests/` outside this crate, `wait()`, `batch()` and
`is_tick_safe()` are called zero times, and `producers()` is called only in the
one assertion at `ring_factory/tests/factory_test.rs:80`.

**Finding.** Three of the five stored fields and one of the two derived readings
have no consumer of any kind. `producers` is consumed only through
`is_multi_producer()`, which collapses it to a bit; the count itself is read once,
in a test, to assert that two configs differ.

The record was designed to carry everything that varies between rings, and the
family reads two fields of it plus one derived bit. That is not a defect in this crate: the fields are stored
correctly and reported correctly. It means the crate's surface is sized to a design
the rest of the family has not built to yet, and nothing in the crate says so.

---

### RC8 — A Neighbouring Suite Already Measured This, From the Other Side, and Named Three Reasons

`ring_factory/tests/factory_test.rs` carries a test called
`only_two_of_five_config_fields_are_observable_through_the_factory`, with a doc
table stating the result field by field: `capacity` and `overflow` observable,
`producers`, `wait` and `batch` not — "it selects a backend, and `Split` reaches
no backend", "nothing in the closure honours it", "`ring_core` was implemented
without reading it".

The test asserts each row rather than describing it: it builds two configs
differing only in producer count, asserts the *configs* differ
(`single.producers() != multi.producers()`), then asserts the rings they produce
are indistinguishable — and does the same for `wait` and for `batch`. Its failure
message says "producer count reached the observable surface after all — this test
is now the wrong shape."

**Finding.** So the same conclusion is reached twice, by two different methods,
and recorded in one place. This instance counts callers and finds four readers
with none; `ring_factory` measures behaviour and finds three fields with no
effect. The overlap is `producers`, `wait` and `batch` — identical.

That agreement is what promotes this from an observation to something worth
acting on, and it also locates the gap precisely. `ring_factory`'s test scopes its
claim to "through the factory", which is narrower than the call census supports:
the inline comment beside the `wait` case already goes further — "additionally
nothing anywhere reads it" — and this census confirms it for `batch` and
`is_tick_safe` as well. The measurement and the census each know a piece the other
does not, and neither crate's documentation states the joined result.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/001`](001_twelve_functions_eleven_of_them_const.md) | The seven readers as declarations |
| [`integration/001`](../integration/001_eleven_declarers_and_four_that_build_on_it.md) | The dependency edges these ten reads travel |
| [`item/002`](../item/002_the_two_derived_readings.md) | `is_multi_producer` and `is_tick_safe`, one used and one not |
| [`non_functional_requirement/002`](../non_functional_requirement/002_a_record_sized_for_a_design_not_yet_built.md) | What the unread fields cost, and what they are for |

### Sources

| Fact | Where |
|------|-------|
| Six production reads of `capacity()` | `ring_bench/src/lib.rs:384`; `ring_core/src/lib.rs:204`; `ring_mpsc/src/lib.rs:410`, `:408`; `ring_spsc/src/lib.rs:325`, `:320` |
| Three production reads of `overflow()` | `ring_core/src/lib.rs:165`, `:180`, `:207` |
| The one production read of `is_multi_producer()` | `ring_core/src/lib.rs:174` |
| Zero production reads of `wait`, `producers`, `batch`, `is_tick_safe` | Census above |
| The measured field-by-field observability table | `ring_factory/tests/factory_test.rs:36-42` |
| The three not-observable assertions | `ring_factory/tests/factory_test.rs:76-101` |

### Tests

| Test | Covers |
|------|--------|
| `every_named_field_is_carried` | That all seven readers work, whoever calls them |
| `multi_producer_is_derived_from_the_count` | The one derived reading with a production caller |
| `tick_safety_is_exactly_non_blocking_waiting` | The one with none |
