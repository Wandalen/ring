# integration

Four manifests name this crate and two of them mean it. `ring_mpsc` and
`ring_spsc` are built out of `Buffer< S >`; `ring_event` and `ring_tls` take it
as a dev-dependency so their own tests run over real storage rather than a
stand-in. Both real consumers sit at Tier 6, four tiers above this one, and every
tier in between builds a write-path protocol without knowing storage exists.

The two instances here find the same seam from two sides. The first counts the
edges and then counts what they actually use: three functions out of twelve.
The second follows those three across the boundary and finds that the crates
holding them are precisely the family's two `#![ allow( unsafe_code ) ]` crates —
that all twelve `unsafe` blocks in the family are, directly or one call away, an
access to a `Buffer` slot, and that this crate's own freedom from `unsafe` is a
local property that reaches no consumer.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Four Dependents, Two That Build On It](001_four_dependents_two_that_build_on_it.md) | BF1, BF2, BF3 — two kinds of edge under one name, a four-tier gap, and nine functions with no caller |
| 002 | [Every Unsafe Block in the Family Reaches Into a Buffer](002_every_unsafe_block_in_the_family.md) | BF4, BF5 — the opt-out set equals the consumer set, and where the soundness argument actually lives |

### Two Kinds of Edge

| Crate | Section | Tier | Names `Buffer` in |
|-------|---------|:----:|-------------------|
| `ring_mpsc` | `[dependencies]` | 6 | `src/lib.rs` — a field of `Ring` |
| `ring_spsc` | `[dependencies]` | 6 | `src/lib.rs` — a field of `Ring` |
| `ring_event` | `[dev-dependencies]` | 2 | `tests/event_test.rs` only |
| `ring_tls` | `[dev-dependencies]` | 2 | `tests/tls_test.rs` only |

The dev-dependency pair is the family's no-mocking rule showing up in the build
graph: `ring_event`'s translators fill a slot, so its tests fill a real one.

### What Crosses the Boundary

Of twelve public functions, the two real consumers call three — `new`, `at`,
`at_mut` — each once per consumer, and each through the same two-function
`unsafe` wrapper. The remaining nine (`clear`, `all_empty`, `capacity`, `len`,
`is_empty`, `get`, `get_mut`, `iter`, `iter_mut`) have no caller outside this
crate's own tests and doctests.

### Where the Safety Argument Lives

`ring_store` offers `at_mut( &mut self ) -> &mut S`, an ordinary exclusive
borrow. Both consumers turn it into `slot_mut( &self ) -> &mut S` behind an
`UnsafeCell` and a `clippy::mut_from_ref` waiver, justified by a SAFETY comment
about claim exclusivity that this crate never sees. The concentration is a good
outcome — one container and four call sites to audit — and it is stated entirely
on the consumer side.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# who names it, and in which manifest section
# sorted: the crate list is sorted first, so the awk output follows it
for c in $( ls -d ring_*/ | sed 's|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||;s|/||' | sort ); do
  awk -v c="$c" '/^\[dependencies\]/{s="dependencies"} /^\[dev-dependencies\]/{s="dev-dependencies"} /ring_store =/{printf "  %-10s %s\n", c, s}' ring/$c/Cargo.toml
done

# where each of them imports the type
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn 'use ring_store::' ring_event/tests/*.rs ring_tls/tests/*.rs \
  ring_mpsc/src/lib.rs ring_spsc/src/lib.rs | sed 's|^/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||' | sort

# how much of the surface the two real consumers reach
for m in new clear all_empty capacity len is_empty get get_mut at at_mut iter iter_mut; do
  printf '  %-10s %s\n' "$m" "$( grep -rho "slots\.get() )\.$m(\|Buffer::$m(" ring_mpsc/src/lib.rs ring_spsc/src/lib.rs | wc -l )"
done

# the workspace denial, and the only two crates that opt out
grep -n 'unsafe-code' Cargo.toml
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn 'allow( unsafe_code )' ring_*/src/lib.rs | sed 's|^/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||' | sort

# every unsafe block in the family, and the four that dereference the cell
printf '  total: %s  direct: %s  via slot/slot_mut: %s\n' \
  "$( grep -rho 'unsafe {' ring_*/src/*.rs | wc -l )" \
  "$( grep -rho 'unsafe { ( \*self\.slots\.get() )' ring_*/src/*.rs | wc -l )" \
  "$( grep -rho 'unsafe { self\.ring\.slot' ring_*/src/*.rs | wc -l )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF1 | `ring_store` | n/a — observation | Four manifests name the crate and two of them are dev-dependencies; the apparent dependent count is double the build-graph one, and nothing distinguishes the two kinds |
| BF2 | family | n/a — observation | Both real consumers sit at Tier 6 and the four intervening tiers never touch storage — so no protocol crate can break on a `Buffer` change, and none exercises it either |
| BF3 | `ring_store` | n/a — coverage | Of twelve public functions the real consumers call three; the other nine are covered by this crate's own suite and by nothing downstream |
| BF4 | family | n/a — observation | The two crates that opt out of the workspace `unsafe` denial are exactly this crate's two ordinary dependents — a ring cannot express its borrow pattern through `&mut Buffer` |
| BF5 | `ring_store` | **latent hazard** | All twelve `unsafe` blocks in the family reach a `Buffer` slot, and the soundness argument for them lives entirely on the consumer side — a change here would be assessed against a contract both callers have reinterpreted |
