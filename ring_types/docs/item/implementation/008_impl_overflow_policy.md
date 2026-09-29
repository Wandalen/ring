# impl OverflowPolicy

## Representation

The inherent implementation block on
[`OverflowPolicy`](../enum/003_overflow_policy.md), holding the
[`ALL`](../associated_constant/003_overflow_policy_all.md) roster and two
classifiers —
[`reports_failure`](../associated_function/012_overflow_policy_reports_failure.md)
and
[`drops_silently`](../associated_function/013_overflow_policy_drops_silently.md).

**The two classifiers partition the three variants exactly**, which is the one
place in the crate where a pair of predicates does. `Fail` reports; `DropNewest`
and `DropOldest` drop silently; there is no third case and no variant that is
neither. Contrast [`impl RingError`](002_impl_ring_error.md), whose two
classifiers leave three of nine variants uncovered.

That completeness is asserted, but only from one side.
`overflow_policies_partition_by_reporting` filters `ALL` by `reports_failure` and
checks the count is 1. Nothing asserts that the two predicates are complementary
across the set — a fourth variant that neither reports nor drops would pass every
test in the workspace.

## Kind

Implementation (§ Item Kind Taxonomy : Stable Item Kinds #12)

## Definition

`ring_types/src/policy.rs:116`

```rust
impl OverflowPolicy
{
  pub const ALL : [ Self; 3 ] = [ Self::DropNewest, Self::DropOldest, Self::Fail ];   // :131
  pub const fn reports_failure( self ) -> bool                                         // :153
  pub const fn drops_silently( self ) -> bool                                          // :178
}
```

Three members against `impl WaitKind`'s two. The asymmetry is not arbitrary: a
wait strategy is either usable on the tick path or not, one question; an overflow
policy has two independent ones — whether the caller is told, and whether an item
is lost.

Both classifiers were single `matches!` expressions until
`Fix(overflow_policy_classification_not_exhaustive)` (`policy.rs:140`, `:170`)
converted each to an exhaustive `match` naming both the true arm and the false
arm by variant — a `matches!` over one or two named variants answered every
variant it was not told about with the same default, so a fourth `OverflowPolicy`
variant would have compiled clean while silently misclassifying on whichever side
the macro's implicit arm landed. The two-versus-three partition described below
is unchanged; only the construct enforcing it against a new variant is.

## File Usage

| File | Line(s) | Context |
|------|---------|---------|
| `ring_types/src/policy.rs` | 116, 131, 153, 178, 186 | **Block header (116)**; `ALL` (131); `reports_failure` (153); `drops_silently` (178); closing brace (186) |

Test-only references: `ring_types` — three of the nineteen tests
(`overflow_policy_has_no_overwrite_variant`,
`overflow_policies_partition_by_reporting`,
`overflow_policy_defaults_to_drop_newest`), plus 12 consumer suites.

**One of those three used to be the crate's test gap — now closed.**
`overflow_policy_has_no_overwrite_variant` asserted `ALL.len() == 3` and ran a
wildcard-free `match`, but omitted the per-variant `contains` loop its `WaitKind`
counterpart has — so a duplicated entry in `ALL` passed every test in the
workspace. The fix was three lines, transposed from the sibling
(→ [`../../non_functional_requirement/002`](../../non_functional_requirement/002_the_enum_sets_are_closed_and_asserted.md), T6, closed).

## Crate Usage

| Crate | Via File | Purpose |
|-------|----------|---------|
| `ring_types` | `src/policy.rs` | Declares the block |
| `ring_stats` | `src/lib.rs` | **The only production caller** — `OverflowPolicy::ALL.iter().map( \| p \| self.dropped( *p ) ).fold( 0, u64::saturating_add )` at `:378`, summing per-policy drop counters into a total |

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r 'OverflowPolicy::ALL\|reports_failure\|drops_silently' ring_*/src \
  | command grep -v ring_types/
```

Live output:

```
ring_overflow/src/lib.rs:  /// Mirrors `OverflowPolicy::ALL`, and exists for the same reason: a fourth
ring_stats/src/lib.rs:  /// the array by line. `ring_types` publishes `OverflowPolicy::ALL` and asserts
ring_stats/src/lib.rs:  /// for p in OverflowPolicy::ALL { s.record_drop( p, 1 ); }
ring_stats/src/lib.rs:    OverflowPolicy::ALL.iter().map( | p | self.dropped( *p ) ).fold( 0, u64::saturating_add )
```

returns four lines across two crates, not the two lines in one crate this catalog
once reported. Only the last `ring_stats` line is production code — the call
already named in Crate Usage above. The other three are doc comments: the middle
`ring_stats` line is the same iteration written as a doc example, the first
`ring_stats` line is a sentence about the convention, and the `ring_overflow` line
is a sibling constant's doc comment noting that it "mirrors `OverflowPolicy::ALL`"
without calling it.

**Neither classifier has a single caller in the family.** `reports_failure` and
`drops_silently` name the two properties the overflow design is about, and every
crate that acts on a policy — `ring_core`, `ring_overflow`, `ring_poll`,
`ring_shutdown` — matches the variant directly instead. So the predicates are
documentation with a test suite, and the behaviour they describe is re-derived at
four separate sites.

That is the same shape as [`impl WaitKind`](007_impl_wait_kind.md)'s finding and
the opposite outcome: there, `ring_config` has a written manual check that it
keeps delegating to the predicate rather than re-deriving it. Nothing equivalent
exists here.
