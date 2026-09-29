# api

Seven public items — six functions and one constant — over 71 lines of code. Both
instances are about reach rather than shape: which of the seven anything calls,
and what the one that is called takes as its third parameter.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Seven Items and the One With a Caller](001_seven_items_and_the_one_with_a_caller.md) | The whole surface in one table, the three tiers, and WT1/WT2/WT18 — one item reached, no `use` anywhere, and twenty `wait(` calls that are somebody else's |
| 002 | [The Predicate Is the Parameter](002_the_predicate_is_the_parameter.md) | `F : FnMut() -> bool`, the three real closures, and where the non-blocking guarantee stops |

### The Surface, In Full

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*pub (const |fn |const fn )' ring_wait/src/lib.rs
```

Live output:

```
pub const DEFAULT_SPINS : usize = 1024;
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
pub fn pause( kind : WaitKind, attempt : usize ) -> bool
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
```

| | Count | |
|--|------:|--|
| Public items | 7 | 6 functions, 1 constant |
| `const fn` | 1 | `escalation_hint` — the only pure one |
| `#[ must_use ]` | 1 | also `escalation_hint`, at `:82` |
| Doctests | 7 | one per item, 14 fence lines |
| Generic functions | 2 | `wait_until`, `wait` |
| Functions naming `ring_cursor` | 2 | `for_space`, `for_data` |
| Items with a caller in any other crate's `src/` | **1** | `wait_until`, 3 call sites |

### Regenerate the Caller Census

```sh
cd "$(git rev-parse --show-toplevel)"

# every call into this crate, family-wide — three lines, all `wait_until`
grep -r --include=*.rs "ring_wait::" . \
  | grep -vE ':\s*(///|//!|//)' | grep -v '^ring_wait/'

# nobody imports the name — no output
grep -r --include=*.rs "use ring_wait" . | grep -v '^ring_wait/'

# and the name collision that makes a naive census wrong
grep -r --include=*.rs -F "wait(" . \
  | grep -vE ':\s*(///|//!|//)' | grep -v '^ring_wait/' | wc -l
```

Live output:

```
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
24
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT1 | `ring_wait` | n/a — coverage | Of seven public items, only `wait_until` is called from another crate's `src/`; the other six are reached solely from `tests/wait_test.rs` and their own doctests |
| WT2 | family | n/a — observation | No crate anywhere writes `use ring_wait`; all three call sites spell the path in full, which is what a single-call dependency looks like |
| WT18 | family | n/a — inconsistency | Every `wait(` call site outside this crate — twenty-four of them, twenty when this was written — is `RingConfig::with_wait`, `RingConfig::wait` or `RingStats::record_wait`, and none is `ring_wait::wait`, so a bare-name census returns nothing but false positives and misses the one real caller |
| WT26 | `ring_wait` | n/a — observation | Every public item carries exactly one executable example, so documentation effort is allocated by position on the surface rather than by reach — six of the seven demonstrate items no other crate calls |
