# workaround

External constraints `ring_types` absorbs on behalf of its consumers, each with the
cost it imposes and the condition under which it can be deleted.

### Scope

- **Purpose**: Record every external constraint this crate compensates for, so each one carries a stated cost and a checkable deletion condition rather than becoming permanent by default.
- **Responsibility**: Document this crate's workarounds, and keep each one's Removal trigger falsifiable at a toolchain or dependency bump.
- **In Scope**: Constraints originating outside this repository. For a crate with an empty `[dependencies]` table that means one source only — **the Rust language and standard library itself**.
- **Out of Scope**: This crate's own design decisions, which are not workarounds however unusual they look (→ [`decisions/`](../decisions/readme.md)); constraints compensated in shared tooling outside this crate (none apply here — see Workarounds below).

### Overview Table

| ID | Status | Name | Constraint Source | Removal Trigger |
|----|--------|------|-------------------|-----------------|
| 001 | 🔄 | [Hand-Written ALL Arrays Stand In for Variant Enumeration](001_hand_written_all_arrays_stand_in_for_variant_enumeration.md) | Rust language — no stable variant enumeration; `core::mem::variant_count` unstable, [#73662](https://github.com/rust-lang/rust/issues/73662) | A variant-enumeration derive is adopted at tier 0, **or** Rust stabilises enumeration (not merely the count) |

### Why a Crate With No Dependencies Still Has One

**The tier-0 rule that keeps this crate's dependency table empty is also what
makes a workaround necessary.** Every other crate in the family could reach for
`strum::EnumIter` and delete the compensation; `ring_types` cannot, because
[`../invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md) forbids
the dependency that would fix it. So the one crate with nothing external to be
constrained *by* is the one whose constraint has no cheap escape.

That inverts the usual reading of an empty `[dependencies]` table. Verify it is
still empty:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types
cargo tree --depth 1
```

Live output:

```
ring_types v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_types)
```

Expect `ring_types v0.1.0` and nothing beneath it — one of only two crates in the
family of which that is true, the other being `ring_align`.

### What Was Examined and Found Clean

The `**None.**` convention exists so an empty definition means *"someone looked"*
rather than *"nobody looked"*. This crate has one instance rather than none, so
the same obligation applies to everything the instance does **not** cover:

| Surface | Finding |
|---------|---------|
| Third-party crates | None to be constrained by — `[dependencies]` is empty and asserted by [`../invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md) |
| Platform / target | Nothing target-conditional in `src/` — no `#[ cfg( target_os ) ]`, no `#[ cfg( target_arch ) ]`, no `#[ cfg( target_pointer_width ) ]` |
| `std` availability | The crate is `core`-only by construction and stays compilable without `std`; that is a met requirement, not a compensation (→ [`../non_functional_requirement/003`](../non_functional_requirement/003_the_crate_compiles_without_std.md)) |
| Toolchain features | One gap found, and it is instance 001. `#[ must_use ]`-on-constants is a second language limitation but costs nothing here (→ [`../item/associated_constant/001_seq_zero.md`](../item/associated_constant/001_seq_zero.md)) |
| Const evaluation | `Default::default()` is not callable in a `const fn`, which is why `Seq::ZERO` exists — recorded as a design justification rather than a workaround, since nothing is given up |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'cfg( *target' ring_types/src/ || true
```

Live output:

```
```

Exits 1.

**The last two rows are the judgement calls**, and both went the same way: a
language limitation that costs nothing is not a workaround, because this doc
definition's own Quality Checklist rules out a costless one. `Seq::ZERO` compensates for a `const`
limitation and the compensation is *better* than what it replaces — more
readable at every call site. Filing that as debt would attach a deletion
condition to something nobody should delete.

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/002_tier_zero_depends_on_nothing.md`](../invariant/002_tier_zero_depends_on_nothing.md) | The rule that forbids the dependency which would retire instance 001 |

### Workarounds

| File | Relationship |
|------|-----------------|
| *(none)* | No repo-wide workaround catalog is reachable from a standalone `ring` checkout; regardless, this crate has no rendering, wasm, or dev-server path for one to compensate |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_types/docs/workaround
printf 'instances:               '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### TY[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| TY[0-9]+ ' readme.md
# instances:               2
# finding headings inside:  2
# rows in the table below:  2
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TY17 | ring family | **measured cost** | `Capacity::new` cannot be `TryFrom< usize >` and stay `const fn`, so zero `try_into()` uses and zero `TryFrom` bounds exist across 30 dependents |
| TY18 | `ring_types` | n/a — observation | No `From` and no `TryFrom` on any of its four types — the same absence that leaves the six error enums unable to compose |
