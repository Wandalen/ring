# Integration: Eleven Declarers, and Four That Build On It

### Scope

**Purpose:** Record the crate's real fan-out — who declares the dependency, in
which manifest section, and which of them actually carry the type in a signature.

**Responsibility:** The eleven declaring manifests, the four/seven split between
`[dependencies]` and `[dev-dependencies]`, and the two crates that name the type
without declaring it.

**In Scope:** the eleven `ring_*/Cargo.toml` declarations;
`ring_wait/src/lib.rs:11`; `ring_bench/src/lib.rs:105`.

**Out of Scope:** The re-export `ring_bench` arrives through is
[`integration/002`](002_a_re_export_that_carries_the_record_but_not_its_vocabulary.md).
Which accessors these dependents call is
[`api/002`](../api/002_three_readers_used_and_four_with_no_caller.md).

---

## Who Declares It, and Who Builds On It

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every crate declaring ring_config, and the section it declares it in --'
for c in ring_*/Cargo.toml; do awk -v n="$( basename "$( dirname "$c" )" )" '/^\[/ { s = $0 } /^ring_config/ { printf "  %-13s %s\n", n, s }' "$c"; done
echo '  -- every crate naming RingConfig in a src/ signature or field --'
command grep -r ': *&\?RingConfig' --include=*.rs */src | command grep -v '//\|use ring_config' | sed 's|^ring/\([a-z_]*\)/.*|  \1|' | sort -u
echo '  -- and the two that name it without declaring it --'
command grep 'RingConfig' ring_wait/src/lib.rs
command grep '^use ring_factory' ring_bench/src/lib.rs
```

Live output:

```
  -- every crate declaring ring_config, and the section it declares it in --
  ring_core     [dependencies]
  ring_debug    [dev-dependencies]
  ring_factory  [dependencies]
  ring_flush    [dev-dependencies]
  ring_handle   [dev-dependencies]
  ring_mpsc     [dependencies]
  ring_poll     [dev-dependencies]
  ring_registry [dev-dependencies]
  ring_shutdown [dev-dependencies]
  ring_spsc     [dependencies]
  ring_testkit  [dev-dependencies]
  -- every crate naming RingConfig in a src/ signature or field --
  ring_bench
  ring_core
  ring_factory
  ring_mpsc
  ring_spsc
  -- and the two that name it without declaring it --
//! configuration value that travels through a `RingConfig` and into a struct
use ring_factory::{ BuildError, Factory, RingConfig };
```

---

### RC17 — Every Manifest Section Is Right, Which Is Not the Family's Habit

Four crates declare `ring_config` under `[dependencies]` — `ring_core`,
`ring_factory`, `ring_mpsc`, `ring_spsc` — and seven under `[dev-dependencies]`:
`ring_debug`, `ring_flush`, `ring_handle`, `ring_poll`, `ring_registry`,
`ring_shutdown`, `ring_testkit`.

The split matches the usage exactly. The same four that declare a real dependency
are the four whose `src/` carries a `RingConfig` in a signature or a field, and
none of the seven does. There is no crate declaring an edge it does not use, and
none using one it has not declared — except `ring_bench`, which is a different
case and is deliberate.

**Finding.** Recording a pass, because in this family it is not the default.
`ring_atomic` declares `ring_seqno` and never uses it; `ring_align` declares
`ring_types` and never uses it. Against that background, eleven manifests that all
place this dependency in the section matching its actual use is a fact worth
knowing rather than assuming — particularly since seven of the eleven use the
crate only to build fixtures, which is exactly the case that tends to drift into
`[dependencies]` by accident.

---

### RC18 — Seven of the Eleven Depend On It Only to Build Test Fixtures

Eleven declaring crates makes `ring_config` look like a widely-coupled tier-1
type. The signature census says the real number is four: only `ring_core`,
`ring_factory`, `ring_mpsc` and `ring_spsc` accept or store a `RingConfig` in
production code, plus `ring_bench` through the factory's re-export.

The other seven construct one, hand it to something else, and never mention the
type in their own surface — the configuration is an argument they pass through on
the way to a ring they need for a test.

**Finding.** So the coupling this crate creates is roughly a third of what the
manifest count suggests, and the distinction is invisible from any single file.
`ring_shutdown` and `ring_registry` reading as dependents of the configuration
record is an artifact of needing a ring to shut down or register.

Two further crates blur the count in opposite directions. `ring_bench` uses the
type in three signature positions and declares no edge to this crate at all,
reaching it through `ring_factory`'s `pub use`. And `ring_wait` names `RingConfig`
in its module comment — "a configuration value that travels through a
`RingConfig` and into a struct" — while declaring only `ring_types` and
`ring_cursor`, so it describes a type it cannot name in code. Neither is wrong;
both mean the dependency graph and the prose disagree about who knows about this
crate.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/002`](002_a_re_export_that_carries_the_record_but_not_its_vocabulary.md) | The edge `ring_bench` uses instead of declaring one |
| [`api/002`](../api/002_three_readers_used_and_four_with_no_caller.md) | What the four real dependents actually read |
| [`data_structure/002`](../data_structure/002_four_times_a_reference_and_nobody_pays_it.md) | How each of them takes it — by value or by reference |
| [`lifecycle/001`](../lifecycle/001_built_once_copied_never_mutated.md) | What happens to the record once it crosses an edge |

### Sources

| Fact | Where |
|------|-------|
| Four `[dependencies]` declarations | `ring_core/Cargo.toml`, `ring_factory/Cargo.toml`, `ring_mpsc/Cargo.toml`, `ring_spsc/Cargo.toml` |
| Seven `[dev-dependencies]` declarations | `ring_debug/Cargo.toml:23` and six siblings |
| Five crates carrying the type in a `src/` signature | Census above |
| `ring_bench` reaching it through the factory | `ring_bench/src/lib.rs:105` |
| `ring_wait` naming it in prose with no edge | `ring_wait/src/lib.rs:11`; `ring_wait/Cargo.toml:9-10` |

### Tests

| Test | Covers |
|------|--------|
| `every_named_field_is_carried` | The contract every dependent relies on |
| `the_record_is_copy_and_compares_by_value` | What crossing an edge costs |
| `defaults_are_the_documented_ones` | What the seven fixture-building dependents get without asking |
