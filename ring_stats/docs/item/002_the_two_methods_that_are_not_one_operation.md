# Item: The Two Methods That Are Not One Operation

### Scope

**Purpose:** Read `in_flight` and `reset` as named items in a module rather than as
code — what their names and signatures place them among, and what that placement
hides.

**Responsibility:** `in_flight` against the six readers it sits with, and `reset`
against `new`, the other route to an all-zero set.

**In Scope:** `RingStats::new`, the seven readers, `RingStats::in_flight` and
`RingStats::reset` in `ring_stats/src/lib.rs`.

**Out of Scope:** What each executes is
[`algorithm/001`](../algorithm/001_eleven_operations_and_three_compositions.md) and
[`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md). The write-side
family is [`item/001`](001_five_recorders_and_the_one_nothing_calls.md).

---

## Seven Readers, Two Constructors of Zero

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every reader, and the one whose name is not a past participle --'
command grep -o 'pub fn \(claimed\|published\|consumed\|dropped\|dropped_total\|wait_nanos\|in_flight\)' ring_stats/src/lib.rs
echo '  -- the two routes to an all-zero set, and what each promises --'
awk '/^  \/\/\/ A fresh set of counters, all zero\.$/{ print } /^  \/\/\/ assert_eq!\( RingStats::new\(\)\.claimed\(\), 0 \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 4 { print }' ring_stats/src/lib.rs
sed -n '/^  \/\/\/ Reset every counter to zero\.$/p;/^  pub fn reset( &self )$/,/^  {$/p' ring_stats/src/lib.rs
echo '  -- one builds a value, the other walks seven counters --'
printf '    new   : atomic ops in its body : %s\n' \
  "$( command grep -m1 -B1 -A12 -F '    Self' ring_stats/src/lib.rs | command grep -c 'fetch_add(\|\.store(\|\.load(' || true )"
printf '    reset : counters its loop walks : %s\n' \
  "$( command grep -m1 -A4 -F '    [' ring_stats/src/lib.rs | command grep -o '&self\.[a-z_]*' | wc -l )"
```

Live output:

```
  -- every reader, and the one whose name is not a past participle --
pub fn claimed
pub fn published
pub fn consumed
pub fn dropped
pub fn dropped_total
pub fn wait_nanos
pub fn in_flight
  -- the two routes to an all-zero set, and what each promises --
  /// A fresh set of counters, all zero.
  #[ must_use ]
  pub const fn new() -> Self
  {
  /// Reset every counter to zero.
  pub fn reset( &self )
  {
  -- one builds a value, the other walks seven counters --
    new   : atomic ops in its body : 0
    reset : counters its loop walks : 7
```

---

### ST27 — Six Readers Answer "How Much Has Happened"; the Seventh Answers "What Is True Now"

Read the seven reader names as a list. `claimed`, `published`, `consumed`, `dropped`,
`dropped_total`, `wait_nanos` — every one is a past participle or a cumulative total.
Each asks *how much has happened since the counters were zeroed*, and each is
answered by one monotone number that only grows.

`in_flight` is a prepositional phrase in the present tense. It asks *how many slots
are open right now*, which is not a total of anything. It rises and falls. It has no
history. It is a gauge sitting in a list of counters, with the same `-> u64`, the
same `#[ must_use ]`, and a doc comment the same length as its neighbours'.

**Finding.** The distinction is real and consequential — a gauge assembled from two
monotone counters is the one reading in this crate that can be wrong about the
present without any counter being wrong about the past
([`algorithm/002`](../algorithm/002_in_flight_subtracts_two_moments.md) § ST3) — and
the API presents the two kinds identically. Nothing in the naming, the signature, the
attribute or the module layout separates the six that accumulate from the one that
observes.

A reader scanning the item list sees seven counters. Six of them are. The one that is
not is the one the crate's own doc holds up as its interesting reading, and the one
whose zero means three different things
([`invariant/002`](../invariant/002_claimed_never_trails_published.md) § ST24).

---

### ST28 — `new` and `reset` Produce the Same State With Opposite Guarantees

There are three ways to obtain an all-zero `RingStats` — `new`, `reset`, and the
derived `Default` — and the two that carry docs carry them one clause apart: "A fresh
set of counters, all zero" and "Reset every counter to zero". The third
([`type/001`](../type/001_a_type_that_cannot_be_copied_compared_or_cloned.md) § ST45)
has no doc of its own and is not `const`.

`new` is a `const fn` returning `Self`. Its body contains no atomic operation at all —
it constructs seven `AtomicU64::new( 0 )` fields as a value. There is no moment at
which a caller can observe it half-built, because until it is returned no other thread
has a reference to it.

`reset` takes `&self` and walks seven counters, storing zero into each
([`api/001`](../api/001_fourteen_methods_and_no_exclusive_borrow.md) § ST5). Every
other thread holding the same `Arc` can read throughout, and what it reads partway
through is a set in which some counters have been cleared and others have not — a
state no sequence of recordings could ever produce.

**Finding.** Same postcondition, opposite atomicity, and at the time no note on
either. The difference is not arbitrary: it follows from `new` owning the value and
`reset` sharing it, which is exactly the property `api/001` records as unstated. But
the two sit in the same `impl` block behind parallel one-line summaries — "A fresh set
of counters, all zero" against "Reset every counter to zero" — and a caller choosing
between them, replacing the `RingStats` behind the `Arc` or calling `reset` on it, is
choosing between an atomic transition and a torn one.

Half of that has since been closed, and only half. `reset`'s doc now carries the
window in writing — the combination a reader can catch mid-call, and how often it was
measured to appear ([`lifecycle/001`](../lifecycle/001_zero_to_reset_to_zero.md)
§ ST29). `new`'s doc still says nothing, so the pair still reads as two routes to the
same state, and the one that is safe is the one that does not mention safety.

The suite has moved the same distance. `reset_returns_every_counter_to_the_fresh_state`
and `a_reset_set_counts_again` still call `reset` on one thread and assert afterwards,
which is the condition in which the two methods are indistinguishable;
`a_reader_beside_a_reset_sees_only_values_the_writer_wrote` reads `claimed`,
`wait_nanos` and `snapshot` from a second thread while a writer resets beneath it,
which is the only condition in which they are not.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/001`](001_five_recorders_and_the_one_nothing_calls.md) | The write-side family, and the recorder with no caller |
| [`api/001`](../api/001_fourteen_methods_and_no_exclusive_borrow.md) | Why `reset` has no choice but seven stores |
| [`lifecycle/001`](../lifecycle/001_zero_to_reset_to_zero.md) | The reset window, observed from a second thread |
| [`invariant/002`](../invariant/002_claimed_never_trails_published.md) | What `in_flight`'s zero does and does not establish |

### Sources

| Fact | Where |
|------|-------|
| The seven reader names | Census above |
| `new` is a `const fn` with no atomic operation | `RingStats::new` |
| `reset` walks seven counters | `RingStats::reset` |
| The two one-line summaries | `RingStats::new`'s and `RingStats::reset`'s doc comments |
| `in_flight` as a subtraction of two counters | `RingStats::in_flight` |

### Tests

| Test | Covers |
|------|--------|
| `a_fresh_set_is_all_zero` | `new`'s postcondition |
| `reset_returns_every_counter_to_the_fresh_state` | `reset`'s postcondition, single-threaded |
| `a_reset_set_counts_again` | That the two routes leave an equally usable set |
| `in_flight_is_claimed_minus_published` | The gauge, at rest |
| `a_reader_beside_a_reset_sees_only_values_the_writer_wrote` | The one difference between `new` and `reset`, read from the second thread that is the only place it appears |
