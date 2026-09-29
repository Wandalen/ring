# Data Structure: The Configuration Record as Input

### Scope

- **Purpose**: Describe the only thing `build` receives — a 32-byte `Copy` record of five fields — from this crate's point of view, which is the point of view of a consumer that may only read it.
- **Responsibility**: State the abstract, the structure, and the operations available on it.
- **In Scope**: The five fields, their layout, what this crate may do with them, and what has already happened to them.
- **Out of Scope**: The record's own construction and clamping rules, which are `ring_config`'s (→ [`ring_config`](../../../ring_config/readme.md)); what each field means for a running ring.

### Abstract

**`RingConfig` is the entire input surface of this crate**, and by the time it
arrives it is immutable, validated, `Copy`, and partly not what the caller
wrote.

Two of those four matter structurally. `Copy` is why
[`api/001`](../api/001_the_build_surface.md)'s B1 can take it by value at no
cost. `PartialEq` is why
[`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)
can be stated as an equation rather than a wish.

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^pub struct RingConfig/,/^}/p' ring_config/src/lib.rs
```

Live output:

```
pub struct RingConfig
{
  capacity : Capacity,
  wait : WaitKind,
  overflow : OverflowPolicy,
  producers : usize,
  batch : usize,
}
```

**All five fields are private.** This crate reaches every one of them through
an accessor, which is what makes `is_multi_producer()` — a *derived* reading
with no field behind it — indistinguishable from a stored one at the call site
(→ [`algorithm/001`](../algorithm/001_selecting_a_backend_from_one_boolean.md)).

### Structure

**Thirty-two bytes, eight-byte aligned, no padding worth reclaiming.** Measure
rather than infer — the two enums' sizes are not obvious from their variant
counts:

```sh
cd "$(git rev-parse --show-toplevel)"
# every type in the record is spliced out of real source rather than retyped.
# doc comments and `#[ default ]` are already valid Rust as they stand, so the
# two enums need no editing at all — which means a variant added upstream
# re-measures this block instead of silently invalidating it
{
  echo '#[ derive( Clone, Copy ) ]'
  command grep -m1 '^pub struct Capacity' ring_types/src/capacity.rs
  echo '#[ derive( Clone, Copy, Default ) ]'
  sed -n '/^pub enum WaitKind/,/^}/p' ring_types/src/policy.rs
  echo '#[ derive( Clone, Copy, Default ) ]'
  sed -n '/^pub enum OverflowPolicy/,/^}/p' ring_types/src/policy.rs
  echo '#[ derive( Clone, Copy ) ]'
  sed -n '/^pub struct RingConfig/,/^}/p' ring_config/src/lib.rs
cat <<'MAIN'
fn main()
{
  macro_rules! row { ( $t : ty ) => {
    println!( "    {:15} size {:2}  align {}", stringify!( $t ),
      std::mem::size_of::< $t >(), std::mem::align_of::< $t >() ); } }
  row!( Capacity ); row!( WaitKind ); row!( OverflowPolicy ); row!( usize );
  println!( "    {:15} size {:2}  align {}", "RingConfig",
    std::mem::size_of::< RingConfig >(), std::mem::align_of::< RingConfig >() );
  println!( "    the five fields sum to {}, the record is {}",
    std::mem::size_of::< Capacity >() + std::mem::size_of::< WaitKind >()
      + std::mem::size_of::< OverflowPolicy >() + 2 * std::mem::size_of::< usize >(),
    std::mem::size_of::< RingConfig >() );
}
MAIN
} > /tmp/-rf01.rs

# bare `rustc` defaults to edition 2015, where `size_of` is not in the prelude —
# hence the `std::mem::` spelling — and `--crate-name` is required because the
# hyphen-prefixed output path is not a legal crate name
rustc -O -A dead_code --crate-name rf01 -o /tmp/-rf01 /tmp/-rf01.rs 2>&1 | command grep '^error' || /tmp/-rf01
```

Live output:

```
    Capacity        size  8  align 8
    WaitKind        size  1  align 1
    OverflowPolicy  size  1  align 1
    usize           size  8  align 8
    RingConfig      size 32  align 8
    the five fields sum to 26, the record is 32
```

| Field | Type | Bytes | Validated | Clamped |
|-------|------|-------|-----------|---------|
| `capacity` | `Capacity` (newtype over `usize`) | 8 | **Yes** — `RingConfig::new` rejects zero and non-powers-of-two | No |
| `producers` | `usize` | 8 | No | **Yes** — zero becomes 1 |
| `batch` | `usize` | 8 | No | **Yes** — zero becomes 1; above capacity becomes capacity |
| `wait` | `WaitKind` | 1 | Total — four variants, all legal | No |
| `overflow` | `OverflowPolicy` | 1 | Total — three variants, all legal | No |

Every number in the Bytes column is a `size_of` reading from the block above,
not an inference from the type's shape: three eight-byte fields plus two
one-byte enums is 26, against a record of 32, and the remaining six bytes are
alignment padding. **Reordering to reclaim it would save nothing** — the record
is passed in registers or copied whole, never stored in bulk, and 32 is already
two cache-line-friendly words short of a line.

**None of the three types carries a `repr` attribute**, so all of this is
`repr(Rust)` and the compiler is free to order the fields however it likes. That
is the second reason to measure rather than infer, and it is the stronger one:
the sum-versus-total gap above is a property of one toolchain's chosen layout,
not a guarantee, and the only honest way to state it is to have just read it.

**The validated/clamped columns are the structure's most important property and
they are not visible in the type.** Reading the struct definition, all five
fields look alike. Three of them behave in three different ways at their
setter, and this crate — which sees only the result — cannot tell which
happened (→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)).

**There is no `name` field.** Naming is a separate argument to a separate
operation (→ [`api/002`](../api/002_the_named_build_surface.md)), which keeps
`RingConfig` comparable: two rings with the same shape and different names have
equal configs, and a sweep enumerating configs does not enumerate names.

**There is no `FlushPolicy` field either**, and this is worth stating because
`ring_flush` is on the same export Contract and takes a policy of its own. A
consumer configuring a ring and a flush policy configures two things through
two surfaces. Recorded in [`decisions/`](../decisions/readme.md) as one of the
three questions **deliberately not opened**: merging them would make
`RingConfig` depend on `ring_flush` and give every ring a staging decision it
may not use, and nothing has yet pushed against the two-door arrangement.

### Operations

**This crate may only read.** The record arrives by value, so the operations
available are the accessors and the derived readings:

| # | Operation | Kind | Used by |
|---|-----------|------|---------|
| U1 | `capacity() -> Capacity` | Stored | [`algorithm/002`](../algorithm/002_assembling_a_ring_from_a_validated_record.md) step 2 |
| U2 | `producers() -> usize` | Stored | Nothing directly. **The count above 1 does not reach the ring** |
| U3 | `is_multi_producer() -> bool` | **Derived** — `producers > 1` | [`algorithm/001`](../algorithm/001_selecting_a_backend_from_one_boolean.md), the whole branch |
| U4 | `batch() -> usize` | Stored | Step 4 |
| U5 | `overflow() -> OverflowPolicy` | Stored | Step 3 |
| U6 | `wait() -> WaitKind` | Stored | **Nothing.** No waiter can be built (→ [`pitfall/002`](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md)) |
| U7 | `is_tick_safe() -> bool` | **Derived** — `wait.is_non_blocking()` | Nothing, anywhere in the workspace |
| U8 | `==` | Derived | [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)'s statement |

**The setters are not available here and their absence is structural, not
incidental.** `with_batch`, `with_producers` and `with_wait` all take `self` and
return `Self`, so calling one inside `build` would produce a *different*
config — a second input by construction, and the clearest possible violation of
`invariant/001`. The rule for this crate is that the record is read-only in
fact as well as in intent.

**U2 and U6 are read-and-discard; U7 is never read at all.** Three of eight
operations lead nowhere, which is a fair summary of this crate's current
position: it has a complete input and an incomplete set of things to do with it.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_selecting_a_backend_from_one_boolean.md](../algorithm/001_selecting_a_backend_from_one_boolean.md) | U3, the only operation whose answer changes what is built |
| [../algorithm/002_assembling_a_ring_from_a_validated_record.md](../algorithm/002_assembling_a_ring_from_a_validated_record.md) | U1, U4, U5 in consumption order |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | B1 — why 32 `Copy` bytes are taken by value, and the Contract problem the type creates |

### Data Structures

| File | Relationship |
|------|--------------|
| [002_the_handle_pair_as_output.md](002_the_handle_pair_as_output.md) | The other side of the same call |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_configuration_fully_determines_the_ring.md](../invariant/001_configuration_fully_determines_the_ring.md) | U8, and why the setters must not be reachable from `build` |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_configuration_as_data.md](../pattern/001_configuration_as_data.md) | Why a record at all, and what `Copy` plus `PartialEq` buy |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_criterion_grades_the_clamped_value.md](../pitfall/001_the_criterion_grades_the_clamped_value.md) | The Clamped column, and why this crate cannot see it |
| [../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md) | U6 |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/003_config_state_through_a_build.md](../lifecycle/003_config_state_through_a_build.md) | The record's states either side of this crate's boundary |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_build_error.md](../type/002_build_error.md) | U7 — the derived reading a refusal would be built on |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | The struct and all eight operations |
| [`ring_types/src/policy.rs`](../../../ring_types/src/policy.rs) | `WaitKind` and `OverflowPolicy`, the two one-byte fields |
| [`ring_types/src/capacity.rs`](../../../ring_types/src/capacity.rs) | `Capacity`, the one validated field |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | The instruction *not to present all five fields as equivalent* is discharged by `only_two_of_five_config_fields_are_observable_through_the_factory`, whose name is the finding. The helper it uses carries a `capacity < 8` guard, because at the first attempt the profile offered 8 records to an 8-slot ring, never overflowed, and made three negative assertions pass vacuously — → `tests/manual/readme.md` F2 |

### FC9 — Three Fields Carry Their Constraint in the Type and Two Carry It in a Setter Body

The five fields are not five of a kind. Three are types that cannot hold an
illegal value; two are bare `usize` whose legality is maintained by the setter
that writes them:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the five fields, as declared --'
sed -n '/^pub struct RingConfig/,/^}/p' ring_config/src/lib.rs | command grep -E '^  [a-z_]+ :'
echo '  -- and where the two usize constraints actually live --'
command grep 'self.producers = \|self.batch = ' ring_config/src/lib.rs
```

Live output:

```
  -- the five fields, as declared --
  capacity : Capacity,
  wait : WaitKind,
  overflow : OverflowPolicy,
  producers : usize,
  batch : usize,
  -- and where the two usize constraints actually live --
    self.producers = if producers == 0 { 1 } else { producers };
    self.batch = if capped == 0 { 1 } else { capped };
```

`Capacity` refuses a non-power-of-two at construction and `WaitKind` and
`OverflowPolicy` are closed enums, so a wrong value of those three is a compile
error or an `Err` the caller already saw. `producers` and `batch` accept
`usize::MAX` from the type's point of view and are corrected by an `if` inside
`with_producers` and `with_batch`.

The split is not arbitrary and the coincidence is the finding: the two
setter-constrained fields are exactly the two nothing on the build arc reads
(→ [`algorithm/002`](../algorithm/002_assembling_a_ring_from_a_validated_record.md)
FC4), and exactly the two whose correction is silent rather than reported
(→ [`pitfall/001`](../pitfall/001_the_criterion_grades_the_clamped_value.md)).
Three fields got a type, and they are the three that reach a ring. Nothing
states that as a rule, so a sixth field arriving as a `usize` looks like it is
following two precedents when it is following the wrong two.

### FC10 — Config Equality Compares Five Fields and the Ring It Builds Depends on Two

`RingConfig` derives `PartialEq` and `Eq`, which is what makes a benchmark sweep
able to deduplicate its own parameter grid. [`invariant/001`](../invariant/001_configuration_fully_determines_the_ring.md)
states the forward direction: equal configs build indistinguishable rings. The
sweep needs the converse, and the converse is false:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what equality compares --'
command grep -B1 '^pub struct RingConfig' ring_config/src/lib.rs | command grep 'derive'
echo '  -- what the factory surface can observe --'
command grep 'fn only_two_of_five_config_fields_are_observable_through_the_factory' \
  ring_factory/tests/factory_test.rs
```

Live output:

```
  -- what equality compares --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
  -- what the factory surface can observe --
fn only_two_of_five_config_fields_are_observable_through_the_factory()
```

Two configs differing only in `batch`, or only in `wait`, or only in `producers`
compare **unequal** and produce rings this crate's own test asserts are
indistinguishable. So a sweep keyed on config equality enumerates points that
collapse onto the same measurement, and reports them as distinct rows.

The consequence is a measurement error rather than a correctness one, which is
what makes it worth recording here: the family exists to produce a comparison
table, and its parameter type over-discriminates by exactly the three fields the
build arc drops. Deduplicating on `PartialEq` is the obvious thing to do and it
is not enough. The honest predicate is "equal in `capacity` and `overflow`", and
it exists nowhere — not as a method, not as a second type, not as a note on the
derive.

**Disposition:** declined — the derive this finding is about
(`#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]`) sits on `RingConfig` in
`ring_config/src/lib.rs`, a crate outside this pass's assigned scope
(`ring_event`, `ring_factory`, `ring_flush`); "equal in `capacity` and
`overflow`" as a method or second type would have to live there too, since
`RingConfig`'s fields are private to `ring_config` and `ring_factory` only
sees them through its own accessors. Adding such a predicate to
`ring_factory` itself would be speculative: there is no sweep or dedup code
anywhere in the workspace today that would call it (confirmed —
`command grep -rn 'dedup\|HashSet.*RingConfig\|sweep' ring_bench/src`
returns nothing), so it would be a public method with no caller.
