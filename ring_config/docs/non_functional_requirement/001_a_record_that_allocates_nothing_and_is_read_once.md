# Non-Functional Requirement: A Record That Allocates Nothing and Is Read Once

### Scope

**Purpose:** Record the crate's non-functional properties — no allocation, no
`unsafe`, no `std` path, bounded read count — and which of them anything actually
enforces.

**Responsibility:** The allocation and `std` census, the read count on the whole
construction path, and the gap between the properties the crate has and the ones
the build checks.

**In Scope:** `ring_config/src/lib.rs:19`, `:21`;
`ring_config/Cargo.toml:11-12`; `Cargo.toml:229-234`;
`ring_core/src/lib.rs:165`, `:174`, `:180`, `:204`, `:207`;
`ring_mpsc/src/lib.rs:410`; `ring_spsc/src/lib.rs:325`.

**Out of Scope:** What the record's size costs per hand-off is
[`data_structure/002`](../data_structure/002_four_times_a_reference_and_nobody_pays_it.md).
Why the record carries fields nothing reads is
[`non_functional_requirement/002`](002_a_record_sized_for_a_design_not_yet_built.md).

---

## What the Crate Is, and What the Build Checks

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- lines naming an allocation, a std path or unsafe in this crate --'
command grep -c 'std::\|alloc::\|Vec<\|String\|Box<\|unsafe' ring_config/src/lib.rs || true
echo '  -- every read of the record on the whole path from factory to backend --'
command grep -rE '(config|cfg)\.(capacity|wait|overflow|producers|batch|is_multi_producer|is_tick_safe)\(\)' --include=*.rs ring_factory/src ring_core/src ring_mpsc/src ring_spsc/src | command grep -v '///\|//!' | sed 's|^ring/||'
echo '  -- what the 33 crates declare at crate level, by attribute --'
command grep -rh '^#!\[' ring_*/src/lib.rs | command grep -o '#!\[ *[a-z_]*( *[a-z_]*' | sort | uniq -c | sort -rn
echo '  -- what the workspace table enforces centrally --'
# anchored on the section header, not a line number: the members list above it
# grows with every crate added to the workspace, and a fixed offset then quotes
# whatever moved into its place
sed -n '/^\[workspace.lints.rust\]/,/^$/p' Cargo.toml
echo '  -- and how many of the 33 declare no_std --'
command grep -rl 'no_std' ring_*/src/lib.rs | wc -l
```

Live output:

```
  -- lines naming an allocation, a std path or unsafe in this crate --
0
  -- every read of the record on the whole path from factory to backend --
ring_core/src/lib.rs:    if config.overflow() == OverflowPolicy::DropOldest
ring_core/src/lib.rs:    let storage = match config.is_multi_producer()
ring_core/src/lib.rs:    Ok( Self { storage, overflow : config.overflow() } )
ring_core/src/lib.rs:          crossbeam_queue::ArrayQueue::new( config.capacity().get() ),
ring_core/src/lib.rs:          config.capacity(),
ring_core/src/lib.rs:        overflow : config.overflow(),
ring_mpsc/src/lib.rs:    Self::new( config.capacity() )
ring_spsc/src/lib.rs:    Self::new( config.capacity() )
  -- what the 33 crates declare at crate level, by attribute --
     33 #![ deny( missing_docs
      2 #![ allow( unsafe_code
  -- what the workspace table enforces centrally --
[workspace.lints.rust]
rust_2018_idioms = { level = "warn", priority = -1 }
future_incompatible = { level = "warn", priority = -1 }
missing_docs = "warn"
missing_debug_implementations = "warn"
unsafe-code = "deny"
# `loom` is set by RUSTFLAGS, not by any feature, so rustc has no other way to
# learn it is a real cfg. Declared once here rather than per crate: the lints
# table is inherited workspace-wide, and a crate cannot both inherit it and add
# its own. Only ring_atomic, ring_cursor and ring_publish read the cfg — see
# ring_atomic's module documentation on the seam.
unexpected_cfgs = { level = "warn", check-cfg = [ 'cfg(loom)' ] }

  -- and how many of the 33 declare no_std --
3
```

---

### RC33 — Four Reads Per Ring, and Nothing on Any Hot Path

`ring_factory` never touches a field. It takes the record by value and hands it
on. Every read on the whole construction path happens further down, and there are
four of them per ring built: `config.overflow()` at `ring_core:165` to reject
`DropOldest`, `config.is_multi_producer()` at `:174` to pick a backend,
`config.overflow()` again at `:180` to store it, and `config.capacity()` in
whichever backend was chosen — `ring_mpsc:410` or `ring_spsc:325`, identical
lines. The crossbeam constructor is a separate two-read path at `:204`, `:207`.

Four reads, once, at construction. Nothing reads a `RingConfig` inside a publish,
a claim, a drain, or any loop anywhere in the family.

**Finding.** That is the whole reason the record's cost never matters. A
by-value hand-off of thirty-two bytes measured 3.7× to 4.0× a reference in
[`data_structure/002`](../data_structure/002_four_times_a_reference_and_nobody_pays_it.md),
and at four reads per ring the difference is unmeasurable against the allocation
of the ring itself.

So the non-functional requirement here is satisfied by traffic volume rather than
by design, and nothing states the bound. A future field that a producer consults
per publish would move the record onto the hot path, and there is no comment, no
test and no lint that would notice — the crate reads exactly like one designed for
a hot path, being `const` and `Copy` throughout, and is not on one.

---

### RC34 — Every Non-Functional Property It Has Is a Convention, Except the One That Is Checked

The crate names no `std::` path, no `alloc::` path, no `Vec`, `String` or `Box`,
and no `unsafe` — zero lines match any of them. Its single import is
`ring_types`, which is `core`-only itself. It compiles, in principle, without an
operating system.

Nothing checks that here. Three of the thirty-three `ring_*` crates declare
`#![ no_std ]` — `ring_overflow`, `ring_stats` and `ring_types` — and
`ring_config` is not among them, though `ring_types` in its own closure is.
Twenty-five of the thirty-three never name `std::` or `alloc::` in `src/` at
all, so a `std` dependency added tomorrow would compile clean in the twenty-two
that need none today and declare nothing.

**Correction (2026-09-20):** this paragraph read "Zero of the thirty-three
`ring_*` crates declare `#![ no_std ]`" while the census quoted at the top of
this same file — the one the Sources table below points at — has printed `3`
for the `no_std` count ever since it was retargeted at `ring/`. The prose
contradicted its own evidence, and this crate's
[`definition/readme.md`](../definition/readme.md) RC34 row already carried the
corrected count. The finding below is unchanged in substance: `ring_config`
declares nothing either way. What changed is that the family is no longer
uniform, so the unenforced gap is twenty-two crates, not twenty-five.

**Finding.** The contrast with `unsafe` is the point. `unsafe-code = "deny"` sits
in the workspace lints table at `Cargo.toml:234`, `ring_config` inherits it with
`[lints] workspace = true`, and exactly two crates in the family opt out with an
explicit `#![ allow( unsafe_code ) ]` — a property that is stated centrally,
enforced by the compiler, and whose exceptions are visible as two lines.

Core-only-ness gets most of that on three crates and none of it on the other
twenty-two. It is a property twenty-five crates have and three declare, so on the
remaining twenty-two — `ring_config` among them — it cannot be relied on, cannot
be exempted from deliberately, and would be lost silently. Adding `#![ no_std ]`
to this crate costs one line and converts a habit into a check; finishing the job
family-wide would need each of the twenty-two checked individually, since "never
names `std::`" is necessary and not sufficient — `Vec` and `String` reach the
prelude without a path.

The same table shows a second, smaller shape: `missing_docs = "warn"` centrally,
escalated to `deny` by hand in all thirty-three crates. One word in the table
would delete thirty-three lines.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/002`](002_a_record_sized_for_a_design_not_yet_built.md) | The fields carried for a consumer that does not exist |
| [`data_structure/002`](../data_structure/002_four_times_a_reference_and_nobody_pays_it.md) | The per-hand-off cost these four reads never pay |
| [`api/001`](../api/001_twelve_functions_eleven_of_them_const.md) | The `const` surface that suggests a hot path |
| [`lifecycle/001`](../lifecycle/001_built_once_copied_never_mutated.md) | Where each of the four reads happens |

### Sources

| Fact | Where |
|------|-------|
| No allocation, `std` path or `unsafe` | Census above, zero matches |
| The crate's one import | `ring_config/src/lib.rs:21` |
| The one crate-level attribute it declares | `ring_config/src/lib.rs:19` |
| Inheriting the workspace lints | `ring_config/Cargo.toml:11-12` |
| `unsafe-code = "deny"` and `missing_docs = "warn"` | `Cargo.toml:232`, `:234` |
| Two crates opting out of the `unsafe` deny | Census above |
| Four reads per ring built | `ring_core/src/lib.rs:165`, `:174`, `:180`; `ring_mpsc/src/lib.rs:410`; `ring_spsc/src/lib.rs:325` |
| Three `no_std` declarations in thirty-three crates | Census above |

### Tests

| Test | Covers |
|------|--------|
| `defaults_are_the_documented_ones` | The record a ring is built from without any setter |
| `capacity_is_validated_at_construction` | The one check that happens before any of the four reads |
| `the_record_is_copy_and_compares_by_value` | The hand-off whose cost the read count makes irrelevant |
