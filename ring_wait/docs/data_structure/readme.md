# data_structure

No `struct`, no `enum`, no `trait`, no field, no state. Both instances are about
what stands in for a data structure in a crate that declares none: three borrowed
types, and two integers.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A Crate With No Type of Its Own](001_a_crate_with_no_type_of_its_own.md) | The family-wide census, WT14 — three type-free crates and why this is the impure one — and the three types it borrows instead |
| 002 | [The Budget and the Attempt Index](002_the_budget_and_the_attempt_index.md) | `spins` and `attempt`: where each comes from, where each goes, and what a caller can learn from the one it gets back |

### What the Crate Declares

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE '^\s*(///|//!|//)' ring_wait/src/lib.rs \
  | grep -E 'pub (struct|enum|trait|union|type) ' \
  || echo '(no public type declared)'
# control: the identical expression over ring_poll, which declares three
grep -vE '^\s*(///|//!|//)' ring_poll/src/lib.rs \
  | grep -cE 'pub (struct|enum|trait|union|type) '
```

Live output:

```
(no public type declared)
3
```

| | Count | |
|--|------:|--|
| Public types declared | **0** | no `struct`, `enum`, `trait`, or `type` alias |
| Types imported | 3 | `WaitKind`, `RingError`, `CursorPair` |
| Fields anywhere | 0 | there is nothing to have a field |
| Integers threaded | 2 | `spins` in, `attempt` out and sideways |
| Other family crates declaring no type | 2 | `ring_index`, `ring_seqno` — both pure |
| Declared `const fn` | 1 of 6 | the other five are barred, not declined |

### Regenerate the Census

```sh
cd "$(git rev-parse --show-toplevel)"

# every crate in the family that declares no struct, enum, or trait
for d in ring_*/; do
  code=$( cat "$d"src/*.rs | grep -vE '^\s*(///|//!|//)' )
  s=$( printf '%s' "$code" | grep -cE '^\s*pub struct ' )
  e=$( printf '%s' "$code" | grep -cE '^\s*pub enum ' )
  t=$( printf '%s' "$code" | grep -cE '^\s*pub trait ' )
  [ "$s$e$t" = "000" ] && echo "$( basename "$d" )"
done

# and the one const fn, against the six public functions
grep -E '^\s*pub (const )?fn ' ring_wait/src/lib.rs
```

Live output:

```
ring_index
ring_seqno
ring_wait
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
pub fn pause( kind : WaitKind, attempt : usize ) -> bool
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT14 | family | n/a — observation | Three of the family's 33 crates declare no `struct`, `enum`, or `trait`: `ring_index`, `ring_seqno`, and `ring_wait`; the first two are pure arithmetic and this one reads the scheduler, a clock, and memory it was never handed |
| WT27 | `ring_wait` | n/a — observation | The `ring_cursor` dependency exists for `for_space` and `for_data` alone, neither of which is called outside this crate; the manual plan's W6 asks whether every dependency is *used*, which is a question a manifest can answer and reach is not |
| WT28 | family | n/a — observation | Eight crates name `WaitKind` in their source and two take a dependency on `ring_wait`, so the vocabulary travels four times further than the code — which is what makes the parking ban enforceable at all |
| WT29 | `ring_wait` | n/a — coverage | `for_data`'s `count` is bounded by nothing, so a request above capacity can never be satisfied and burns the whole budget; the only test of the failing case uses a budget of 1, where an unsatisfiable request and an unsatisfied one cost the same |
