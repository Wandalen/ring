# Integration: A Re-Export That Carries the Record but Not Its Vocabulary

### Scope

**Purpose:** Record what `ring_factory`'s `pub use` of `RingConfig` does to the
dependency graph and to a caller's imports.

**Responsibility:** The one re-export, the one crate that arrives through it, the
edge that is therefore undeclared, and the three field types that do not travel
with it.

**In Scope:** `ring_factory/src/lib.rs:104-105`, `:140-143`;
`ring_bench/src/lib.rs:105`; `ring_bench/Cargo.toml:36-41`.

**Out of Scope:** The declaring crates and their manifest sections are
[`integration/001`](001_eleven_declarers_and_four_that_build_on_it.md). What
`ring_bench` does with the record once it has it is
[`data_structure/002`](../data_structure/002_four_times_a_reference_and_nobody_pays_it.md).

---

## The Re-Export, and What Travels With It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- everything ring_factory re-exports --'
command grep 'pub use' ring_factory/src/lib.rs
echo '  -- the one crate that reaches RingConfig through it --'
command grep '^use ring_factory' ring_bench/src/lib.rs
command grep '^ring_config' ring_bench/Cargo.toml || echo '  (ring_bench declares no ring_config edge)'
echo '  -- and what the re-export does not carry: the factory own doctest reaching past it --'
command grep -m1 -A4 -F '  /// use ring_factory::{ BuildError, Factory, RingConfig };' ring_factory/src/lib.rs
```

Live output:

```
  -- everything ring_factory re-exports --
pub use ring_config::RingConfig;
pub use ring_registry::Registry;
  -- the one crate that reaches RingConfig through it --
use ring_factory::{ BuildError, Factory, RingConfig };
  (ring_bench declares no ring_config edge)
  -- and what the re-export does not carry: the factory own doctest reaching past it --
  /// use ring_factory::{ BuildError, Factory, RingConfig };
  /// use ring_types::{ OverflowPolicy, RingError };
  ///
  /// let evicting = RingConfig::new( 8 ).unwrap().with_overflow( OverflowPolicy::DropOldest );
  /// assert_eq!
```

---

### RC19 — `ring_bench` Carries the Type in Three Signature Positions and Declares No Edge to This Crate

`ring_factory` re-exports `RingConfig`, and `ring_bench` imports it from there:
`use ring_factory::{ BuildError, Factory, RingConfig };`. Its manifest has no
`ring_config` line. The type appears in a struct field at `:173` and a
constructor parameter at `:186`, and its module documentation opens with
`use ring_factory::RingConfig;`.

**Finding.** The re-export is the right call for the factory — a caller building
a ring needs the record and should not have to name a second crate to get it — and
it produces two effects on this crate that nothing records.

The first is documentary. `ring_bench` teaches `use ring_factory::RingConfig` to
every reader of its module comment and its doctests, so the type's apparent home
is a tier-11 crate rather than the tier-1 crate that defines it. A reader tracing
`RingConfig` from `ring_bench` will find `ring_factory` first, and `ring_factory`
re-exports it in one line with no note saying where it came from.

The second is structural. The coupling exists — a breaking change to `RingConfig`
breaks `ring_bench` — and the dependency graph does not show it. Nothing is wrong
with the build; `cargo` resolves it correctly through `ring_factory`. But the edge
that would explain the breakage to whoever has to fix it is not written down
anywhere, in either manifest.

---

### RC20 — Two of the Four Setters Cannot Be Called Through the Re-Export Alone

`ring_factory` re-exports exactly two names: `RingConfig` and
`ring_registry::Registry`. It does not re-export `Capacity`, `WaitKind` or
`OverflowPolicy`, so a caller reaching the record through the factory can call
`new`, `with_producers` and `with_batch` — whose arguments are `usize` — and
cannot call `with_wait` or `with_overflow` without naming `ring_types` as well.

`ring_factory`'s own documentation demonstrates it. The doctest at `:140` imports
`RingConfig` from itself and `OverflowPolicy` from `ring_types` on the very next
line, because the line after that is
`RingConfig::new( 8 ).unwrap().with_overflow( OverflowPolicy::DropOldest )`.

**Finding.** So the re-export carries the record and not the vocabulary needed to
fill it, and half the builder is unreachable through the surface it appears on.
`ring_bench` pays this directly: its manifest declares `ring_types` with a comment
giving the reason — "`RingStats::record_drop` takes an `OverflowPolicy`, so the
per-run counters feature 185 supplies cannot be recorded without naming the policy
vocabulary."

That comment names a different reason for the same import, which is the part worth
recording. `ring_bench` would have needed `ring_types` regardless, so the gap
costs it nothing and is invisible from there. A future crate that wants a
configured ring and nothing else would meet it directly: it would add
`ring_factory` for the builder, discover two setters it cannot call, and add
`ring_types` for two enum names — at which point the re-export has saved it
nothing and the record's actual home is still not in its manifest.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/001`](001_eleven_declarers_and_four_that_build_on_it.md) | The eleven declared edges this one is missing from |
| [`api/001`](../api/001_twelve_functions_eleven_of_them_const.md) | The four setters, two of which need `ring_types` in scope |
| [`data_structure/002`](../data_structure/002_four_times_a_reference_and_nobody_pays_it.md) | The by-value convention that arrives with the re-export |
| [`type/002`](../type/002_two_counts_that_are_usize_and_three_fields_that_are_not.md) | Why two setters take a plain `usize` and two do not |

### Sources

| Fact | Where |
|------|-------|
| The two re-exports, and only two | `ring_factory/src/lib.rs:104-105` |
| `ring_bench` importing through the factory | `ring_bench/src/lib.rs:105` |
| `ring_bench` declaring no `ring_config` edge | `ring_bench/Cargo.toml`, census above |
| The factory's own doctest reaching past its surface | `ring_factory/src/lib.rs:140-143` |
| `ring_bench`'s stated reason for declaring `ring_types` | `ring_bench/Cargo.toml:36-38` |

### Tests

| Test | Covers |
|------|--------|
| `every_named_field_is_carried` | The record the re-export hands on |
| `each_setter_is_independent` | The four setters, two reachable through the factory alone |
| `the_record_is_copy_and_compares_by_value` | What the by-value re-export convention costs |
