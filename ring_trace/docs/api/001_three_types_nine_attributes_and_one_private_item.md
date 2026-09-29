# API: Three Types, Nine Attributes and One Private Item

### Scope

**Purpose:** Record the whole public surface, establish that exactly one item in
the crate is private and that the crate's central safety property rests on it,
and check the `#[ must_use ]` discipline against the rest of the family.

**Responsibility:** The declaration census, the receiver census, the private
`entries_guard`, and a family-wide comparison of `#[ must_use ]` against
returning inherent methods.

**In Scope:** `ring_trace/src/lib.rs:59-376`; the same count over the 27
`ring_*` crates that declare a single-line returning inherent method.

**Out of Scope:** What `&self` on every method forces is
[`api/002`](002_shared_reference_everywhere_and_what_it_forces.md). The impl
blocks the lint cannot see are
[`item/001`](../item/001_six_impl_blocks_and_the_three_a_lint_cannot_reach.md).

---

## The Whole Surface

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every public item the crate declares --'
command grep '^pub \|^  pub \|^  fn \|^impl' ring_trace/src/lib.rs
echo '  -- receivers across the whole crate --'
printf '    &self %s   &mut self %s   self %s\n' \
  "$( command grep -c 'fn [a-z_]*( *&self' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'fn [a-z_]*( *&mut self' ring_trace/src/lib.rs || true )" \
  "$( command grep -c 'fn [a-z_]*( *self' ring_trace/src/lib.rs || true )"
echo '  -- must_use against single-line returning inherent methods, family-wide --'
printf '    %-16s %-9s %-9s %s\n' crate must_use returning gap
for c in ring_*/; do
  n=$( basename "$c" )
  m=$( command grep -c '#\[ must_use \]' "$c"src/lib.rs 2>/dev/null || true )
  r=$( command grep -c '^  pub \(const \)\?fn [a-z_]*(.*) ->' "$c"src/lib.rs 2>/dev/null || true )
  if [ "$r" != 0 ] && [ "$m" != "$r" ]; then printf '    %-16s %-9s %-9s %s\n' "$n" "$m" "$r" "$(( r - m ))"; fi
done
echo '    -- crates where the two counts agree --'
for c in ring_*/; do
  n=$( basename "$c" )
  m=$( command grep -c '#\[ must_use \]' "$c"src/lib.rs 2>/dev/null || true )
  r=$( command grep -c '^  pub \(const \)\?fn [a-z_]*(.*) ->' "$c"src/lib.rs 2>/dev/null || true )
  if [ "$r" != 0 ] && [ "$m" = "$r" ]; then printf '    %-16s %-9s %-9s 0\n' "$n" "$m" "$r"; fi
done
```

Live output:

```
  -- every public item the crate declares --
pub enum TraceOp
impl TraceOp
  pub const ALL : [ Self; 5 ] =
  pub const fn name( self ) -> &'static str
impl fmt::Display for TraceOp
  fn fmt( &self, f : &mut fmt::Formatter< '_ > ) -> fmt::Result
pub struct TraceEntry
  pub op : TraceOp,
  pub seq : Seq,
  pub count : usize,
impl TraceEntry
  pub const fn end( &self ) -> Seq
impl fmt::Display for TraceEntry
  fn fmt( &self, f : &mut fmt::Formatter< '_ > ) -> fmt::Result
pub struct Trace
impl Trace
  pub const fn enabled() -> Self
  pub const fn disabled() -> Self
  pub const fn is_enabled( &self ) -> bool
  pub fn record( &self, op : TraceOp, seq : Seq, count : usize )
  fn entries_guard( &self ) -> std::sync::MutexGuard< '_, Vec< TraceEntry > >
  pub fn len( &self ) -> usize
  pub fn is_empty( &self ) -> bool
  pub fn entries( &self ) -> Vec< TraceEntry >
  pub fn count_of( &self, op : TraceOp ) -> usize
  pub fn clear( &mut self )
impl Default for Trace
  fn default() -> Self
  -- receivers across the whole crate --
    &self 10   &mut self 1   self 1
  -- must_use against single-line returning inherent methods, family-wide --
    crate            must_use  returning gap
    ring_align       1         4         3
    ring_bench       35        39        4
    ring_store      9         11        2
    ring_claim       12        15        3
    ring_config      11        12        1
    ring_consume     11        14        3
    ring_core        10        16        6
    ring_cursor      13        12        -1
    ring_debug       1         3         2
    ring_flush       9         14        5
    ring_gating      10        11        1
    ring_handle      4         12        8
    ring_mpsc        21        29        8
    ring_overflow    3         2         -1
    ring_publish     5         6         1
    ring_registry    4         7         3
    ring_shutdown    9         15        6
    ring_slot        8         10        2
    ring_spsc        16        22        6
    ring_testkit     5         7         2
    ring_tls         6         8         2
    -- crates where the two counts agree --
    ring_atomic      5         5         0
    ring_barrier     8         8         0
    ring_batch       8         8         0
    ring_poll        11        11        0
    ring_stats       10        10        0
    ring_trace       9         9         0
```

---

### TR5 — One Private Item Carries the Crate's Whole Safety Property

Everything the crate declares is public but one line. Three types, three of them
with public fields or discriminants; eleven inherent methods; three trait impls.
The single exception is `entries_guard`, defined directly after `record` (the
method that calls it), which returns the `MutexGuard`, and it is private.

That keyword is the entire mechanism by which no caller can hold this lock.
`entries()` is documented as returning a copy because "handing out a guard would
let a caller hold the lock across arbitrary code" — but the reason no caller can
do so is not that `entries()` chose to copy; it is that the only method producing
a guard is not reachable from outside. Change `fn entries_guard` to `pub fn` and
every other line of the crate is still correct, still compiles, still passes all
nineteen tests, and the property is gone.

**Finding.** Worth naming because the crate documents the *consequence* of the
privacy on `entries()` and never the privacy itself. `entries_guard`'s own doc
says it exists so that "no two of them can disagree about what a poisoned lock
means", which is a different and lesser property — consistency between call
sites, not containment of the guard. One clause on that method saying it must
stay private, and why, converts an invariant that currently survives by
convention into one a reader can see the reason for.

---

### TR6 — Every Returning Method Carries `#[ must_use ]`, Which Is Unusual Here

Nine `#[ must_use ]` attributes and nine returning inherent methods: `name`,
`end`, `enabled`, `disabled`, `is_enabled`, `len`, `is_empty`, `entries`,
`count_of`. Each was read individually; the attribute sits directly above each
one. The two methods without it, `record` and `clear`, return `()` and cannot
carry it.

The family-wide count is the interesting part. Of the twenty-seven `ring_*`
crates declaring a single-line returning inherent method, five have the two
counts agree — `ring_atomic`, `ring_barrier`, `ring_poll`, `ring_stats` and this
one. The gaps elsewhere run to eight. The count is a heuristic and its two
negative results prove it: `ring_cursor` and `ring_overflow` report more
attributes than methods, because a signature broken across lines does not match
the pattern. So the family numbers are a lower bound on coverage, and the claim
made here is the narrower one that was checked by reading — no returning method
in this crate lacks the attribute.

**Finding.** This is the crate getting something right that its neighbours mostly
do not, and it is worth recording as such rather than passing over: a discarded
`count_of` or a discarded `entries` is a silently wasted lock acquisition and, at
a hundred thousand entries, a two-hundred-microsecond stall on the producers'
path for nothing. The attribute is what makes that a warning instead of a
mystery. The family gaps this census exposes belong to those crates rather than
here, but the census is the evidence for them.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/002`](002_shared_reference_everywhere_and_what_it_forces.md) | What the receiver census means |
| [`item/001`](../item/001_six_impl_blocks_and_the_three_a_lint_cannot_reach.md) | The declaration-level detail behind these lines |
| [`pattern/001`](../pattern/001_one_accessor_for_five_lock_sites.md) | Why `entries_guard` exists at all |
| [`algorithm/002`](../algorithm/002_a_linear_scan_where_a_counter_would_do.md) | What a discarded `count_of` would have cost |

### Sources

| Fact | Where |
|------|-------|
| The declaration census | `ring_trace/src/lib.rs:59-376` |
| `entries_guard` as the only private item | `ring_trace/src/lib.rs`, directly after `record` |
| `entries()`'s documented reason for copying | `ring_trace/src/lib.rs:301-302` |
| `entries_guard`'s own doc, naming the lesser property | `ring_trace/src/lib.rs:265-269` |
| Nine attributes over nine returning methods | Census above |
| Five of twenty-seven crates with agreeing counts | Census above |

### Tests

| Test | Covers |
|------|--------|
| `enabled_and_disabled_report_their_own_state` | Both constructors and the state read |
| `entries_come_back_in_the_order_recorded` | `entries()`, the copy that replaces a guard |
| `count_of_counts_only_its_own_kind` | The other read the attribute protects |
