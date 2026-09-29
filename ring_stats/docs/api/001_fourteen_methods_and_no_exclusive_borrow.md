# API: Fourteen Methods, and No Way to Borrow the Type Exclusively

### Scope

**Purpose:** Record the shape of the public surface — what each method takes, what
it gives back, and which attributes it carries — and what that shape forecloses.

**Responsibility:** The `pub fn` signatures — fourteen when this was written, sixteen
now — the `#[ must_use ]` marks, and the crate-level lint that produced the doc on
every one of them.

**In Scope:** the `impl RingStats` and `impl StatsCounts` blocks and the crate-level
lints in `ring_stats/src/lib.rs`.

**Out of Scope:** That there is no way to read more than one counter at a time is
[`api/002`](002_seven_readers_and_no_way_to_read_the_set.md). What the three
composing methods execute is
[`algorithm/001`](../algorithm/001_eleven_operations_and_three_compositions.md).

---

## What the Surface Takes and Gives

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every public method, by what it takes and gives --'
command grep '^  pub fn \|^  pub const fn ' ring_stats/src/lib.rs
printf '    on RingStats %-4s on StatsCounts %s\n' \
  "$( awk '/^impl RingStats$/ { on = 1; next } on && /^}$/ { on = 0 } on && /^  pub (const )?fn /' ring_stats/src/lib.rs | wc -l )" \
  "$( awk '/^impl StatsCounts$/ { on = 1; next } on && /^}$/ { on = 0 } on && /^  pub (const )?fn /' ring_stats/src/lib.rs | wc -l )"
printf '    take &self %-4s take &mut self %-4s take nothing %s\n' \
  "$( command grep -c '^  pub \(const \)\?fn [a-z_]*( &self' ring_stats/src/lib.rs || true )" \
  "$( command grep -c '&mut self' ring_stats/src/lib.rs || true )" \
  "$( command grep -c '^  pub const fn new() -> Self' ring_stats/src/lib.rs || true )"
printf '    return a value %-4s return ()  %s\n' \
  "$( command grep '^  pub fn \|^  pub const fn ' ring_stats/src/lib.rs | command grep -c ' -> ' )" \
  "$( command grep '^  pub fn \|^  pub const fn ' ring_stats/src/lib.rs | command grep -vc ' -> ' )"
echo '  -- must_use, against the methods that return anything --'
printf '    must_use %s\n' "$( command grep -c '#\[ must_use \]' ring_stats/src/lib.rs || true )"
echo '  -- and the lint that forced a doc onto every one --'
command grep -m1 -F '#![ deny( missing_docs ) ]' ring_stats/src/lib.rs
```

Live output:

```
  -- every public method, by what it takes and gives --
  pub const fn checked_in_flight( &self ) -> Option< u64 >
  pub const fn new() -> Self
  pub fn record_claim( &self, n : u64 )
  pub fn record_publish( &self, n : u64 )
  pub fn record_consume( &self, n : u64 )
  pub fn record_drop( &self, policy : OverflowPolicy, n : u64 )
  pub fn record_wait( &self, nanos : u64 )
  pub fn claimed( &self ) -> u64
  pub fn published( &self ) -> u64
  pub fn consumed( &self ) -> u64
  pub fn dropped( &self, policy : OverflowPolicy ) -> u64
  pub fn dropped_total( &self ) -> u64
  pub fn wait_nanos( &self ) -> u64
  pub fn in_flight( &self ) -> u64
  pub fn snapshot( &self ) -> StatsCounts
  pub fn reset( &self )
    on RingStats 15   on StatsCounts 1
    take &self 15   take &mut self 0    take nothing 1
    return a value 10   return ()  6
  -- must_use, against the methods that return anything --
    must_use 10
  -- and the lint that forced a doc onto every one --
#![ deny( missing_docs ) ]
```

---

### ST5 — `reset` Takes a Shared Reference, Which Is What Makes It Seven Stores

Every method on `RingStats` but one takes `&self`; that one takes nothing and returns
`Self`. `&mut self` appears zero times in the file — it was thirteen of fourteen when
this was written and is fourteen of fifteen now, with the ratio unchanged and the
absence still total. That is the correct and deliberate shape for the type — the doc
comment on `RingStats` says so directly, "Every method takes `&self`, so a single
instance is shared by every producer and consumer of its ring" — and it is what lets
one `RingStats` sit behind an `Arc` with no lock anywhere in the family.

All of them but `reset` genuinely want it. A recorder is called from whichever
producer thread is publishing; a reader is called from whatever is monitoring. None
of them needs exclusive access and giving them exclusive access would defeat the
type.

`reset` is the exception, and it is different in kind. It is the one operation whose
meaning is about the set rather than about a member: "Reset every counter to zero",
called by `ring_shutdown` so that a recycled ring does not carry the previous
world's numbers. An operation over the whole set is exactly what an exclusive borrow
is for — and taking `&self` means the implementation has no choice but seven
independent stores
([`algorithm/001`](../algorithm/001_eleven_operations_and_three_compositions.md)
§ ST2), with an externally observable window in which some are cleared and others
are not.

**Finding.** The signature is not wrong so much as load-bearing in a way nothing
records. `&self` on `reset` is the reason the operation cannot be atomic, and it is
also the reason `ring_shutdown` can call it without restructuring how the stats are
held. Both halves of that trade are real; neither is written down. A caller reading
`pub fn reset( &self )` beside `/// Reset every counter to zero` has no way to learn
that the second sentence describes an outcome rather than an instant.

Nothing here needs `&mut self`. What the surface lacks is the sentence naming what
the shared reference costs — one line, on the one method whose contract is about all
seven counters at once.

---

### ST6 — Every Value the Crate Returns Is Marked, and None of Them Needs to Be

A `#[ must_use ]` mark sits on exactly the methods that return a value: `new` and the
seven readers when this was written, `snapshot` and `StatsCounts::checked_in_flight`
since — eight marks against eight returns, then ten against ten. The six methods
returning `()` carry none, correctly. Coverage is total, mechanical, and easy to check,
and stayed that way through the two additions.

It is also uniformly spent on values whose loss costs nothing. Discarding
`stats.claimed()` wastes one relaxed load. Discarding `RingStats::new()` wastes one
stack allocation of seven zeroed words. Discarding `stats.snapshot()` wastes seven
loads and a 72-byte move. There is no method in this crate whose return value is a
resource, a token, or a record of something irreversible — every one of the ten can be
obtained again by calling again, at the same price.

**Finding.** This is the family's habit at its most complete. `ring_atomic` spends
all five of its marks the same way and leaves `SeqCell::fetch_add` — whose return is
a caller's only record of an irreversible claim — unmarked
([`ring_atomic` § AT46](../../../ring_atomic/docs/type/001_what_the_trait_promises.md));
`ring_types` spends eleven on pure functions over a `Copy` type. `ring_stats` added
eight to the same pile, and then two more.

The observation is not that these marks are wrong. On a `#[ deny( missing_docs ) ]`
crate whose readers are all pure, marking them is consistent and costs nothing. It is
that the attribute is being applied by position — *does this return a value* — rather
than by consequence, and a family that applies it that way has no attention left to
spend on the one place where dropping a return actually loses something. The two
additions confirm the habit rather than break it: both were marked because they return
something, not because dropping either costs anything a second call cannot recover.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/002`](002_seven_readers_and_no_way_to_read_the_set.md) | What the surface does not offer: any reading of more than one counter |
| [`item/002`](../item/002_the_two_methods_that_are_not_one_operation.md) | `reset` and `in_flight` read as contracts |
| [`lifecycle/001`](../lifecycle/001_zero_to_reset_to_zero.md) | The reset window, measured from outside the call |
| [`type/001`](../type/001_a_type_that_cannot_be_copied_compared_or_cloned.md) | The type behind these methods, and the traits it declines |

### Sources

| Fact | Where |
|------|-------|
| 15 `&self`, 0 `&mut self`, 1 associated fn | Census above |
| 10 `must_use` against 10 value returns | Census above |
| 15 methods on `RingStats`, 1 on `StatsCounts` | Census above |
| The shared-reference rationale | `RingStats`'s own doc comment |
| `reset`'s contract and its caller | `RingStats::reset`'s doc comment |
| `#![ deny( missing_docs ) ]` | Census above |

### Tests

| Test | Covers |
|------|--------|
| `recording_needs_only_a_shared_reference` | That the surface is usable through `&self` alone |
| `reset_returns_every_counter_to_the_fresh_state` | `reset`'s outcome, on one thread |
| `a_reset_set_counts_again` | That a reset set is not poisoned |
| `a_reader_beside_a_reset_sees_only_values_the_writer_wrote` | `reset` observed from a second thread, which is the only way its window appears |
