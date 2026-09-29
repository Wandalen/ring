# workaround

External constraints `ring_spsc` absorbs.

### Scope

- **Purpose**: Record every external constraint this crate compensates for, so each one carries a cost and a deletion condition.
- **Responsibility**: Document this crate's workarounds, or record explicitly that it has none.
- **In Scope**: Constraints originating outside this repository — for a crate whose only dependencies are workspace siblings, that means the language, the toolchain, and the targets. Also the `unsafe` opt-out's justification, which gate `G6` requires to live here.
- **Out of Scope**: This crate's own design decisions, which are not workarounds however unusual they look; constraints compensated in shared tooling elsewhere in the family.

### Overview

**One: the language cannot express a two-thread disjoint split of one
allocation, so this crate opts out of the workspace `unsafe-code = "deny"`.**

| Field | Value |
|-------|-------|
| Constraint | Rust has exactly one way to mutate through a shared reference — `UnsafeCell` — and no safe primitive for "two threads, one array, provably disjoint index sets" |
| Where | `src/lib.rs`'s `#![ allow( unsafe_code ) ]`, `Ring`'s `slots` field, `Ring::slot`, `Ring::slot_mut`, and the `unsafe impl Sync for Ring` |
| Cost | Three `unsafe` sites and one `unsafe impl`, each carrying a `SAFETY` comment the workspace's `undocumented_unsafe_blocks = "deny"` enforces |
| Ruled by | This crate's own design: the unsafe is sited here rather than in `ring_store` or `ring_slot` |
| Deletion condition | A safe API below this crate that encapsulates the *whole* invariant — none exists, because the invariant is stated in terms of cursors that a storage-only crate does not hold |

**Why it is a workaround and not a design choice.** The disjointness is a real,
checkable property of the program: the producer writes only the slot at its own
cursor, the consumer reads only slots strictly below it, and the two ranges
cannot intersect. What is missing is a way to *tell the compiler* — and that
absence is a property of the language, not of this design. A `slice::split_at_mut`
generalized to "split by a predicate that holds at runtime" would delete the
opt-out unchanged; no such thing exists.

**What bounds it.** The `unsafe` is three functions wide, and every one of them
is private. Nothing a caller can write reaches an `unsafe` operation, because
the only paths in are `Reservation`'s `Deref`/`DerefMut` and `Batch::get`, each
of which can only be obtained from a `claim` or a `drain` that has already
established the precondition. Five `compile_fail` doc tests in the module
documentation assert the negatives the argument rests on — no `Clone`, no
`Sync` on either end, no second live split, no batch outliving its commit.

Verify the extent:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
command grep -c "unsafe" src/lib.rs         # the sites, plus their SAFETY comments
command grep "unsafe fn\|unsafe impl" src/lib.rs
```

Live output:

```
16
unsafe impl< S : Send > Sync for Ring< S > {}
  unsafe fn slot( &self, seq : Seq ) -> &S
  unsafe fn slot_mut( &self, seq : Seq ) -> &mut S
```

**Expected:** exactly two `unsafe fn` — `slot` and `slot_mut` — and exactly one
`unsafe impl`, for `Sync`. A third `unsafe fn` means the surface grew and its
argument was not written down here.

---

**Two near-misses, neither of which is a workaround.**

The cursor padding this crate depends on through
[`ring_cursor`](../../../ring_cursor/readme.md) — `align_of == 64`,
`size_of == 64` — compensates for a hardware property (cache-line granularity
of coherence traffic), not for a defect in an external dependency. It is a
design decision documented at
[`data_structure/001`](../data_structure/001_two_cursor_ring.md) and it has no
deletion condition, because the hardware behaviour it addresses is not going
to be fixed upstream.

The absence of `ring_gating`, `ring_claim`, `ring_publish` and `ring_consume`
(→ [`integration/001`](../integration/001_family_dependency_seam.md)) is not
compensation for a shortcoming in any of them either — it is the observation
that each answers a question that only has an answer worth computing when
producers can overtake one another. A future author reading the dependency list
as "routes around `ring_gating`" would file it here; that reading is wrong, and
the entry it would produce would have no cost and no deletion condition, which
is the test that catches it.

Verify the dependency surface:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
cargo tree --depth 1
```

Live output:

```
ring_spsc v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_spsc)
├── ring_store v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_store)
├── ring_config v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_config)
├── ring_cursor v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_cursor)
├── ring_slot v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_slot)
└── ring_types v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_types)
```

**Expected:** five path dependencies — `ring_store`, `ring_config`,
`ring_cursor`, `ring_slot`, `ring_types` — and no published crate at all, so
nothing outside this repository imposes a constraint beyond the language one
above.

### Workarounds

| File | Relationship |
|------|-----------------|
| [`001_the_unsafe_code_opt_out_and_what_bounds_it.md`](001_the_unsafe_code_opt_out_and_what_bounds_it.md) | This crate's own two workaround instances are the only ones that apply; none from shared tooling reach this crate, which has no rendering or dev-server path |

### Integrations

| File | Relationship |
|------|-----------------|
| [`../integration/001_family_dependency_seam.md`](../integration/001_family_dependency_seam.md) | The five-dependency surface this finding examines, and the absences that are not workarounds |

### Sources

| File | Relationship |
|------|-----------------|
| `Cargo.toml` | The dependency surface examined for this finding — five path dependencies, no published crates |
| `src/lib.rs` | The `#![ allow( unsafe_code ) ]` this file justifies, and the three sites it covers |


### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc/docs/workaround
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### SP[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| SP[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  3
# rows in the table below:  3
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SP50 | the unsafe census | n/a — observation | Both composed cores carry exactly ten, and no other crate in the family carries any. |
| SP51 | `compile_fail` | n/a — observation | No `Clone`, no `Sync`, no second live split, no batch outliving its commit — none is assertable by a test that runs. |
| SP52 | `Producer` | **latent hazard** | `ring_spsc::Producer` must not be `Sync`; `ring_mpsc::Producer` must be — and `ring_core` holds both behind one enum. |
