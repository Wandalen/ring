# Item: Six Constructors for Two Types

### Scope

**Purpose:** Record every way a cell can be built, why there are six of them, and
what the capability they were split for is used for.

**Responsibility:** `AtomicSeq::new` (both), `CountingSeq::new` (both), both
`Default` impls, and the `cfg( loom )` split across all four `new`s.

**In Scope:** `ring_atomic/src/lib.rs:168-180`, `:184-209`, `:327-376`;
`ring_cursor/src/lib.rs:148-171`.

**Out of Scope:** The `loom` seam itself, and the manifest coupling it carries, is
[`workaround/001`](../workaround/001_the_loom_seam_and_the_manifest_above_it.md).
The `must_use` these four carry and the trait methods do not is
[`api/001`](../api/001_the_return_value_that_is_a_claim.md).

---

## Every Way to Build a Cell

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- six constructors for two types --'
command grep -E 'pub (const )?fn new|fn default' ring_atomic/src/lib.rs
echo '  -- the split, and the Default written out beside it --'
command grep -m1 -A2 -F '  /// `const` in an ordinary build. Not under `--cfg loom`, whose atomics carry' ring_atomic/src/lib.rs
command grep -m1 -A3 -F '  /// Written out rather than derived so that it does not depend on whichever' ring_atomic/src/lib.rs
echo '  -- which one the crate itself reaches for --'
for n in AtomicSeq::new AtomicSeq::default CountingSeq::new CountingSeq::default
do printf '    %-22s %s\n' "$n" "$( command grep -c "$n" ring_atomic/tests/atomic_test.rs || true )"; done
echo '  -- and every const or static in 33 crates holding a cell --'
printf '    %s\n' "$( command grep -rhE '^(static|const) +[A-Z_]+ *:' --include=*.rs */ | command grep -cE 'AtomicSeq|CountingSeq|PaddedCursor' || true )"
```

Live output:

```
  -- six constructors for two types --
  fn default() -> Self
  pub const fn new( value : Seq ) -> Self
  pub fn new( value : Seq ) -> Self
  fn default() -> Self
  pub const fn new( value : Seq ) -> Self
  pub fn new( value : Seq ) -> Self
  -- the split, and the Default written out beside it --
  /// `const` in an ordinary build. Not under `--cfg loom`, whose atomics carry
  /// per-execution model state and have no `const` constructor — see the module
  /// documentation on the seam.
  /// Written out rather than derived so that it does not depend on whichever
  /// `AtomicU64` is in scope having its own `Default` — one of the two comes
  /// from `loom` and its trait impls are its own business, not something this
  /// crate should be pinned to.
  -- which one the crate itself reaches for --
    AtomicSeq::new         7
    AtomicSeq::default     5
    CountingSeq::new       4
    CountingSeq::default   10
  -- and every const or static in 33 crates holding a cell --
    0
```

---

### AT25 — Both Decisions Behind the Six Are Written Down, Which Makes Them the Best-Documented Items in the Crate

Two types, six constructors, and each duplication has a reason on the line above
it. The `new` pair is split because `loom`'s atomics carry per-execution model
state and have no `const` constructor. The `Default` impls are hand-written rather
than derived so the crate does not depend on "whichever `AtomicU64` is in scope
having its own `Default` — one of the two comes from `loom` and its trait impls are
its own business."

**Finding.** That second reason is subtle, correct, and the kind of thing normally
discovered by a broken `--cfg loom` build six months later. It is also, together
with the trait-versus-struct paragraph
([`decisions/002`](../decisions/002_a_trait_because_the_criteria_needed_two.md)),
one of only three explained decisions in the crate — against the nine undocumented
ordering literals, the undocumented packing, and the two absent declaration-level
attributes. The crate documents constructors thoroughly and contracts not at all.

The usage split is worth noting alongside it: the crate's own suite calls
`default()` thirteen times and `new()` seven, so the shape that exists identically
in both builds is also the one most reached for. The `Default` doc's caution about
`loom` was earned.

---

### AT26 — The `const` the Split Exists For Is Used by Nothing, in Any of the 33 Crates

`const fn new` is the entire reason the pair exists: without wanting `const`, one
`fn new` would compile in both builds and no `cfg` would be needed anywhere. Across
all 33 crates there is not one `const` or `static` item holding an `AtomicSeq`, a
`CountingSeq`, or a `PaddedCursor` — zero contexts where `const`-ness is required
and zero where it is used.

The split has also been copied upward. `ring_cursor::PaddedCursor::new` carries the
same two declarations for the same reason, so four duplicated constructors across
two crates are maintained for a capability nothing exercises.

**Finding.** The cost is not the duplication, which is six lines. It is that these
four declarations are where the `loom` build's compile-time surface lives, and that
surface does not stand on its own: a copy of this crate built outside the workspace
fails with six `unexpected cfg condition name: loom` errors under `-D warnings`,
because the `check-cfg` entry that makes `cfg( loom )` legal is in the *root*
manifest's `[workspace.lints.rust]` and nowhere in the crate
([`workaround/001`](../workaround/001_the_loom_seam_and_the_manifest_above_it.md)).

Keeping `const` is defensible — a future `static PRODUCER : PaddedCursor` is
plausible, and the cost is genuinely small. What is not recorded anywhere is that
the capability is currently unused, so nobody weighing the seam against its
maintenance has the one number that would decide it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](002_counts_the_method_that_is_not_a_snapshot.md) | The two methods that exist off the trait, and what that costs |
| [`workaround/001`](../workaround/001_the_loom_seam_and_the_manifest_above_it.md) | The seam these four declarations implement, and its coupling upward |
| [`api/001`](../api/001_the_return_value_that_is_a_claim.md) | The four `must_use` here, and the three trait methods without |
| [`lifecycle/001`](../lifecycle/001_born_at_zero_climbing_until_dropped.md) | What happens to a cell after one of these six builds it |

### Sources

| Fact | Where |
|------|-------|
| The six constructors | `ring_atomic/src/lib.rs:176`, `:197`, `:205`, `:331`, `:351`, `:366` |
| The `const` split's reason | `ring_atomic/src/lib.rs:186-188` |
| The hand-written `Default`'s reason | `ring_atomic/src/lib.rs:172-175` |
| The same split one tier up | `ring_cursor/src/lib.rs:148-171` |
| Zero `const`/`static` cells in 33 crates | Census above |

### Tests

| Test | Covers |
|------|--------|
| `a_fresh_cell_reads_zero` | `Default` for both types |
| `a_cell_reads_back_what_it_was_built_with` | `new` for both types |
| `a_counting_cell_starts_at_zero_of_everything` | That `CountingSeq::default` zeroes the counters as well as the cell |
| *(to create)* | Nothing builds a cell in a `const` context, which is the only thing the split exists for |
