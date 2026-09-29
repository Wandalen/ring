# Workaround: Hand-Written ALL Arrays Stand In for Variant Enumeration

### Scope

- **Purpose**: Record that Rust offers no stable way to enumerate an enum's variants, that this crate absorbs the gap with two hand-maintained `[ Self; N ]` constants, and what that costs its consumers.
- **Responsibility**: What is absorbed, where the compensating code lives, why absorbing it here rather than in each consumer is the cheaper site, what it costs, and the condition under which it can be deleted.
- **In Scope**: [`WaitKind::ALL`](../item/associated_constant/002_wait_kind_all.md) and [`OverflowPolicy::ALL`](../item/associated_constant/003_overflow_policy_all.md); the language features that would replace them; the assertion burden they transfer to tests.
- **Out of Scope**: Whether the two arrays' contents are currently correct and what would catch it if they were not (→ [`../non_functional_requirement/002`](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md)); `RingError`, which has no `ALL` and does not want one.

### Constraint

**Rust cannot enumerate an enum's variants on stable.** There is no built-in
iterator, no `Self::VARIANTS`, and no derive in the standard library. The nearest
named API is `core::mem::variant_count`, which is unstable:

```sh
cat > /tmp/probe_variant_count.rs <<'EOF'
enum E { A, B, C }
fn main() { println!( "{}", core::mem::variant_count::< E >() ); }
EOF
rustc --edition 2021 /tmp/probe_variant_count.rs
```

Live output:

```
error[E0658]: use of unstable library feature `variant_count`
 --> /tmp/probe_variant_count.rs:2:29
  |
2 | fn main() { println!( "{}", core::mem::variant_count::< E >() ); }
  |                             ^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^^
  |
  = note: see issue #73662 <https://github.com/rust-lang/rust/issues/73662> for more information

error: aborting due to 1 previous error

For more information about this error, try `rustc --explain E0658`.
```

Toolchain: rustc 1.97.1, the workspace's active stable. A
`nightly-aarch64-unknown-linux-gnu` toolchain *is* installed, so the constraint
is the project's commitment to building on stable rather than an absolute
unavailability — which matters, because it means no upstream event is required
to lift it, only a decision nobody wants to make.

**And `variant_count` would not close the gap even if it stabilised.** It returns
a `usize`, not the variants. It would let a test assert
`ALL.len() == variant_count::< WaitKind >()`, replacing a magic `4` with a
compiler-derived number — a real improvement — and it would leave the array's
*contents* hand-written.

So two things are missing and they have different fixes. The count is a library
feature waiting on stabilisation. The enumeration is a derive macro, and the
standard library has never offered one.

### Workaround

Two hand-maintained constants, both in
[`src/policy.rs`](../../src/policy.rs), one per enum:

| Symbol | Line | Value |
|--------|-----:|-------|
| `WaitKind::ALL` | 58 | `[ Self::Spin, Self::Yield, Self::Park, Self::None ]` |
| `OverflowPolicy::ALL` | 131 | `[ Self::DropNewest, Self::DropOldest, Self::Fail ]` |

`WaitKind::ALL`'s doc comment is explicit about what the array's own assertion
does *not* catch: *"a fifth variant compiles clean in this file, leaves `ALL`
at four, and passes the assertion below — measured, not assumed"*
(`policy.rs:41-43`). What actually refuses a fifth variant is a wildcard-free
`match` — `ring_wait`'s `escalation_hint` and `pause` in production source,
plus `wait_kind_has_exactly_four_variants` in `tests/types_test.rs`
(`policy.rs:44-46`).

**The compensation is centralised here rather than repeated in each consumer**,
which is the substance of the choice:

| | Each consumer writes its own list | `ring_types` exports `ALL` |
|---|---|---|
| Lists to update when a variant is added | One per sweeping crate — currently 4 (`ring_wait`, `ring_config`, `ring_overflow`, `ring_stats`) | One, here |
| What a stale list does | That crate's suite silently tests a subset and passes | Every sweeping suite tests the same subset, so at least the corruption is uniform |
| Where the roster can be asserted | Nowhere central | Here, next to the enum, with the variants named |
| Cost of a new sweeping consumer | Copy the list, and now it can drift | `use ring_types::WaitKind` |

**The third row is the argument.** A per-consumer list cannot be checked against
anything, because there is nothing to check it against — the enum has no
enumerable form. A shared roster can at least be checked against a hand-written
expectation once, in the crate that declares both. Both enums now take that
offer — `wait_kind_has_exactly_four_variants` and
`overflow_policy_has_no_overwrite_variant` carry the identical three mechanisms
(→ [`../non_functional_requirement/002`](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md) T6, closed).

### Alternatives Rejected

**A variant-enumeration derive — `strum::EnumIter` or equivalent.** It generates
the iterator from the enum itself, making a stale roster impossible rather than
merely detectable, and deletes both constants. Rejected by the family's own
dependency rule: `ring_types` has an empty `[dependencies]` table and an
invariant saying so
(→ [`../invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md)), and a
proc-macro dependency at tier 0 propagates a build-time cost into all **30**
crates that declare this one — every member of the family except
`ring_registry` and `ring_align` (which dropped the dependency as unused).
**The trade is explicit: two hand-maintained arrays against one proc-macro crate
in almost every downstream build graph.**

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_*/; do
  command grep -qE '^ring_types(\.workspace)? *=' "$c/Cargo.toml" && basename "$c"
done | wc -l
```

Live output:

```
30
```

**A build script generating the arrays from the source.** Removes the dependency
but adds a `build.rs` to the one crate the family relies on compiling trivially,
and re-derives an enum parser that would itself need testing. Strictly worse than
the derive it imitates.

**`#[ non_exhaustive ]` on both enums, forcing consumers to handle an unknown
variant.** Addresses a different problem — it protects consumers from *new*
variants; it does nothing about a roster that is missing an *existing* one, which
is the failure mode here. `RingError` carries the attribute for that other reason
(→ [`../item/enum/002_ring_error.md`](../item/enum/002_ring_error.md)).

### Cost

| Cost | Borne by | Measure |
|------|----------|---------|
| Two constants to keep in sync with two enums | This crate | 2 lines, `policy.rs:58` and `:131` |
| A test asserting each roster's contents | This crate's suite | Present for both — `WaitKind` and `OverflowPolicy` each carry the identical 3 mechanisms |
| A silent-wrong-answer failure mode | `ring_stats` | `dropped_total` sums over `OverflowPolicy::ALL` (`ring_stats/src/lib.rs:378`) — a corrupted roster is a wrong total, not an error |
| Sweeping suites that inherit rather than detect corruption | `ring_wait` (8 refs), `ring_overflow` (9), `ring_stats` (13), `ring_config` (1) | **30 of 31** consumer references sweep the array; one validates it |
| Public API that exists only for tests | Consumers | `WaitKind::ALL` has **zero** production references family-wide |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -rn 'OverflowPolicy::ALL\|WaitKind::ALL' ring_*/tests \
  | command grep -v '^ring_types/' \
  | sed 's|ring/\([a-z_]*\)/.*|\1|' | sort | uniq -c
```

Live output:

```
      1 ring_config
      9 ring_overflow
     13 ring_stats
      8 ring_wait
```

**The one exception is worth naming**, because it runs against the direction of
the whole table: `ring_wait/tests/wait_test.rs:56-65` (`there_are_exactly_four_wait_kinds`)
asserts `WaitKind::ALL` equal to a spelled-out four-element array at `:60-64`,
pinning contents *and* order. A consumer crate is the only place in the
workspace where either roster's *order* is pinned against a literal it did not
derive from the roster itself — both in-crate tests now check membership via a
`contains` loop, which accepts any permutation
(→ [`../item/associated_constant/002_wait_kind_all.md`](../item/associated_constant/002_wait_kind_all.md)).

**The cost is not the arrays.** Two lines is nothing. The cost is that a roster's
correctness rests on whoever happened to care — and for `WaitKind`, the crate
that cared is not the crate that declares it.

### Removal

**Delete both constants, and every `ALL` reference in the four sweeping crates,
when either of these lands:**

1. **A variant-enumeration derive is adopted at tier 0.** Requires reversing
   [`../invariant/002`](../invariant/002_tier_zero_depends_on_nothing.md), which
   is a decision for the family rather than for this crate. Checkable at every
   dependency review: *has tier 0's empty dependency table been given up for
   something else anyway?* If it has, this workaround's remaining justification
   is gone with it.
2. **Rust stabilises variant enumeration** — not merely `variant_count`.
   Tracking issue #73662 covers the count only; an enumerable form would have to
   be a separate feature, and none is proposed. Checkable at every toolchain
   bump.

**What gets deleted:** `policy.rs:39-58` and `:118-131` (both constants with their
doc blocks), the two roster tests in `tests/types_test.rs`
(`wait_kind_has_exactly_four_variants`, `overflow_policy_has_no_overwrite_variant`),
and the `ALL` mention in [`../api/001`](../api/001_the_vocabulary_surface.md)'s
export inventory. The 30 consumer references become iterator calls rather than
disappearing.

**Neither trigger is close, and the one interim step this document used to
recommend has already landed:** `OverflowPolicy`'s suite now carries the same
per-variant `contains` loop `WaitKind`'s does
(→ [`../non_functional_requirement/002`](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md) T6, closed). What remains open is only the
two triggers above — a derive at tier 0, or stabilised variant enumeration —
neither of which is close.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_vocabulary_surface.md](../api/001_the_vocabulary_surface.md) | The export Contract these two constants sit in despite having no production consumer |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_tier_zero_depends_on_nothing.md](../invariant/002_tier_zero_depends_on_nothing.md) | The rule that rules out the derive-macro fix, and trigger 1's checkable condition |

### Items

| File | Relationship |
|------|--------------|
| [../item/associated_constant/002_wait_kind_all.md](../item/associated_constant/002_wait_kind_all.md) | Equally defended with `OverflowPolicy::ALL` now; the `ring_wait` assertion that additionally pins its order |
| [../item/associated_constant/003_overflow_policy_all.md](../item/associated_constant/003_overflow_policy_all.md) | The roster with a production consumer, now carrying the same contents assertion as `WaitKind::ALL` |
| [../item/associated_constant/readme.md](../item/associated_constant/readme.md) | Both rosters' measured reference counts |
| [../item/enum/002_ring_error.md](../item/enum/002_ring_error.md) | The crate's one `#[ non_exhaustive ]` enum — carries the attribute for a different reason than this workaround would need |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md](../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md) | T4–T6 — the thresholds this workaround transfers onto the test suite; all three now met |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_discriminants_here_handlers_elsewhere.md](../pattern/001_discriminants_here_handlers_elsewhere.md) | Why the handler crates sweep a roster they cannot validate |

### Sources

| File | Relationship |
|------|--------------|
| [`src/policy.rs`](../../src/policy.rs) | Lines 58 and 131 — both constants; 39-52, the doc comment stating what the length assertion does and does not catch |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/types_test.rs`](../../tests/types_test.rs) | ✅ `wait_kind_has_exactly_four_variants` — length assert, per-variant `contains` loop, exhaustive match. ✅ `overflow_policy_has_no_overwrite_variant` — now carries the identical three mechanisms |
