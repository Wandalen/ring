# type

This crate declares no type of its own. What it has is a set of borrowed ones it
uses in a particular arrangement, and the arrangement is worth recording: two
integer domains that never mix, three return shapes over six functions, and two
of `RingError`'s nine variants — which turn out to be exactly the two the family
calls retryable.

Both instances are about what the types do *not* prevent. `spins = 0` and
`count = 0` both pass the type checker; `pause`'s guarantee-carrying `bool` can
be discarded without a warning. Each of those is one attribute or one newtype
away from being impossible, and each is recorded here with the reason it was not
done.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A `usize` Budget and a `u64` Count](001_a_usize_budget_and_a_u64_count.md) | The two integer domains, where each width was decided, the seam at `for_data`, and the two values the plain types admit |
| 002 | [One Return Type and the One `#[ must_use ]`](002_one_return_type_and_the_one_must_use.md) | WT8 and WT15 — three shapes, two of nine variants, `#[ non_exhaustive ]`, and the annotation density against the family |

### Every Type in the Signature

| Type | Owner | Appears as |
|------|-------|------------|
| `usize` | — | `DEFAULT_SPINS`, `spins`, `attempt`, `Ok`'s payload |
| `u64` | via `Seq( pub u64 )`, `ring_types` | `count` |
| `bool` | — | `pause`'s answer |
| `WaitKind` | `ring_types::policy` | every function's first parameter |
| `RingError` | `ring_types::error` | every `Err` |
| `CursorPair` | `ring_cursor` | the two wrappers' first parameter |
| `Option< WaitKind >` | — | `escalation_hint`'s answer |
| `Result< usize, RingError >` | — | the four waits |

The crate's whole set of named types is its two `use` lines:

```rust
// ring_wait/src/lib.rs:50-51
use ring_cursor::CursorPair;
use ring_types::{ RingError, WaitKind };
```

Three named types, two from `ring_types` and one from `ring_cursor`, none from
here ([`data_structure/001`](../data_structure/001_a_crate_with_no_type_of_its_own.md)).
Everything else in the surface is a primitive or a `std` wrapper around one.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# every public signature, and every named type behind it
grep -E "^pub (const )?fn|^pub const|^-> " ring_wait/src/lib.rs
grep -E "^use " ring_wait/src/lib.rs

# the crate's whole error surface
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -oE "RingError::[A-Za-z]+" | sort -u

# any conversion between the two integer domains
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E " as (u64|usize)|try_into|from\("

# annotation density against the neighbours
for c in ring_wait ring_cursor ring_seqno ring_types ring_config ring_gating ring_barrier; do
  printf '%-14s %2s pub fn  %2s must_use\n' "$c" \
    "$( grep -hcE '^[[:space:]]*pub (const )?fn' ring/$c/src/*.rs | paste -sd+ | bc )" \
    "$( grep -hc 'must_use' ring/$c/src/*.rs | paste -sd+ | bc )"
done
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
-> Result< usize, RingError >
use ring_cursor::CursorPair;
use ring_types::{ RingError, WaitKind };
RingError::Empty
RingError::Full
ring_wait       6 pub fn   1 must_use
ring_cursor    13 pub fn  13 must_use
ring_seqno        5 pub fn   5 must_use
ring_types     12 pub fn  11 must_use
ring_config    12 pub fn  11 must_use
ring_gating    11 pub fn  10 must_use
ring_barrier    9 pub fn   8 must_use
```

| | Value |
|--|------:|
| Types declared by this crate | 0 |
| Named types imported, and so nameable in a signature | 3 |
| Of those, owned by `ring_types` | 2 |
| Of those, owned by `ring_cursor` | 1 |
| Public functions | 6 |
| Distinct return shapes | 3 |
| Functions returning `Result< usize, RingError >` | 4 |
| Integer domains | 2 — `usize` attempts, `u64` items |
| Casts between them | **0** |
| `RingError` variants total | 9 |
| Variants this crate can produce | 2 |
| Variants for which `is_transient()` is true | 2 — the same two |
| `#[ must_use ]` in this crate | 1 |
| `#[ must_use ]` per `pub fn`, this crate | 0.17 |
| `#[ must_use ]` per `pub fn`, six neighbours | 0.89–1.00 |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT8 | `ring_wait` | n/a — unenforced | The crate's single `#[ must_use ]` is on `escalation_hint`, which has no caller, while `pause`'s `bool` carries the tick-path guarantee and is silently discardable; six neighbours annotate at 0.89–1.00 per `pub fn` against this crate's 0.17 |
| WT15 | family | n/a — observation | The two `RingError` variants this crate can produce are exactly the set `RingError::is_transient` matches on — two definitions written independently that landed on the same pair |
| WT49 | `ring_wait` | n/a — observation | `spins` and the returned attempt count are different quantities sharing one type, so feeding a result back as the next budget typechecks and means nothing; `ring_poll` avoids the same collision by construction with `Budget` and `Progress` newtypes |
| WT50 | `ring_wait` | n/a — observation | The crate's one type parameter is `F : FnMut() -> bool`, so its thirteen-line loop is monomorphised per predicate — the right trade against a vtable call inside a spin loop measured at two `Acquire` loads per look, and the reason the crate has one return *type* and no single return instance |
