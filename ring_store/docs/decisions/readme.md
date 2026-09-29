# decisions

This crate argues for itself more than most. Two choices that a reader would
otherwise take for granted — panicking on a bad index, and keeping a `capacity`
field beside a slice that already knows its length — are stated in the source
with the reasoning attached. Both arguments are worth reading and one of them
rests on a premise the type system does not hold.

The first instance follows the panic decision to its stated ground: *a
`SlotIndex` reaching this crate came from `ring_index::of`*. That is a claim
about provenance, not about the type, and `SlotIndex`'s field is public because
`ring_index` lives in another crate and Rust offers no narrower visibility — so
the permission the argument assumes away is load-bearing elsewhere, and this
crate's own suite builds twenty-five indices by hand against three folded ones.
The second takes the redundant word and finds it honest: eight bytes per ring
buys an externally checkable allocation claim, and the `is_empty` that follows
from keeping `len` is documented, in its first line, as always false.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Panic Rather Than Option](001_panic_rather_than_option.md) | BF6, BF7 — a decision argued from provenance, and a premise the type does not hold |
| 002 | [A Length Kept to Be Checked Against Itself](002_a_length_kept_to_be_checked_against_itself.md) | BF8, BF9 — a redundant word that guards one line, and a function that is a compile-time constant |

### The Two Decisions

| Decision | Stated reason | Rests on |
|----------|---------------|----------|
| Panic, not `Option`, on an out-of-range index | An out-of-range index means two rings' capacities were mixed — a defect, not a condition | That every `SlotIndex` came from `ring_index::of` |
| Keep `capacity` beside `slots.len()` | So a test can assert the two agree; an over-allocating buffer would still report the request | That nothing writes either value after `new` |

Both are the right call. The panic keeps an `.unwrap()` off the ring's hot path
for a case only a defect produces; the field costs eight bytes per ring and
protects the reached-test's first clause. The findings are about the reasoning, not
the outcomes.

### Where the Provenance Argument Breaks

`SlotIndex( pub usize )` is public because `ring_index::of` is in a different
crate and no Rust visibility admits a sibling crate alone. Across the family's
executable code the tuple constructor appears exactly twice — the definition and
that one fold. Across *this crate's* doctests and tests it appears twenty-five
times, and `ring_index::of` three. The out-of-range index the panic documentation
describes as a mixed-capacity defect is also just something you can type.

### The Constant Function

`is_empty` returns `false` for every capacity a `Capacity` permits, measured
across all sixteen powers of two up to 32768, because `Capacity::new` rejects
zero. The crate opens the function's documentation with "Always false" and
explains the chain: `len` is kept for the reason above, a `len` without an
`is_empty` is a lint, and a hand-written `is_empty` that could drift from `len`
would be worse than one that provably cannot. What it costs is the name — the
question a caller actually has is answered by `all_empty`, three characters away.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the panic decision as argued
command grep -m1 -A13 -F '  /// Borrow the slot at `index`.' ring_store/src/lib.rs

# the type the argument rests on, prose and permission together
command grep -m1 -A11 -F '/// A position within a ring'"'"'s storage, always in `0..capacity`.' ring_types/src/id.rs

# the only two places the family's executable code names the constructor
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn 'SlotIndex( ' ring_*/src/*.rs | grep -vE ':[[:space:]]*(///|//!|//)' | sed 's|^/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||' | sort

# and how this crate builds its own
printf '  by hand: %s   folded: %s\n' \
  "$( grep -rho 'SlotIndex( [0-9a-z]' ring_store/src/lib.rs ring_store/tests/buffer_test.rs | wc -l )" \
  "$( grep -rho 'of( seq\|of( Seq' ring_store/src/lib.rs ring_store/tests/buffer_test.rs | wc -l )"

# the two fields, and the reasons given for the two readings
awk '/^#\[ derive\( Debug \) \]$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 5 { print } /^  \/\/\/ Slots allocated — always equal to `capacity\(\)\.get\(\)`, never more\.$/{ n2 = NR } n2 && NR >= n2 + 1 && NR <= n2 + 5 { print } /^  \/\/\/ Always false — a `Capacity` cannot be zero, so a buffer always has slots\.$/{ n3 = NR } n3 && NR >= n3 && NR <= n3 + 5 { print }' ring_store/src/lib.rs
```

`size_of` figures and the sixteen-capacity `is_empty` sweep come from a release
probe; they are quoted in
[`decisions/002`](002_a_length_kept_to_be_checked_against_itself.md).

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF6 | `ring_store` | n/a — observation | The panic decision is argued from where a `SlotIndex` came from rather than from what the type permits, so it inherits whatever strength that provenance claim has |
| BF7 | `ring_store` | **misleading doc** | `# Panics` tells a caller an out-of-range index means two rings were mixed; the field is public, no validation exists, and this crate's own suite hand-builds twenty-five indices against three folded ones |
| BF8 | `ring_store` | **measured cost** | The `capacity` field is eight bytes per buffer that `slots.len()` already knows; it is kept so `new`'s allocation claim is checkable from outside, and it guards exactly that one line |
| BF9 | `ring_store` | n/a — observation | `is_empty` is false for all sixteen legal capacities and its documentation opens by saying so; it exists because `len` does, and a lint requires the pair |
