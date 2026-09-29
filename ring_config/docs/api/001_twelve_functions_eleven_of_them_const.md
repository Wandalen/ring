# API: Twelve Functions, Eleven of Them `const`

### Scope

**Purpose:** Record the crate's whole public surface as declarations — one type
and twelve functions — with every attribute that appears on them and the reason
the one bare function is bare.

**Responsibility:** The `const` and `#[ must_use ]` split, the one exception, and
the return-type asymmetry among the readers.

**In Scope:** `ring_config/src/lib.rs:41-42`, `:65`, `:88-89`, `:103-104`,
`:121-122`, `:139-140`, `:153-154`, `:166-167`, `:179-180`, `:191-192`,
`:203-204`, `:220-221`, `:237-238`.

**Out of Scope:** Which of these readers anyone calls is
[`api/002`](002_three_readers_used_and_four_with_no_caller.md). Why the one
exception cannot join the other eleven, and what it would take, is
[`workaround/001`](../workaround/001_the_question_mark_forecloses_const.md).

---

## The Whole Surface, With Its Attributes

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- every public declaration, with the attribute above it --'
command grep -B1 'pub struct \|pub fn \|pub const fn ' ring_config/src/lib.rs | command grep -v '^--$'
echo '  -- totals: functions, const fns, must_use marks --'
command grep -c 'pub fn \|pub const fn ' ring_config/src/lib.rs || true
command grep -c 'pub const fn ' ring_config/src/lib.rs || true
command grep -c 'must_use' ring_config/src/lib.rs || true
```

Live output:

```
  -- every public declaration, with the attribute above it --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub struct RingConfig
  /// ```
  pub fn new( slots : usize ) -> Result< Self, RingError >
  #[ must_use ]
  pub const fn with_wait( mut self, wait : WaitKind ) -> Self
  #[ must_use ]
  pub const fn with_overflow( mut self, overflow : OverflowPolicy ) -> Self
  #[ must_use ]
  pub const fn with_producers( mut self, producers : usize ) -> Self
  #[ must_use ]
  pub const fn with_batch( mut self, batch : usize ) -> Self
  #[ must_use ]
  pub const fn capacity( &self ) -> Capacity
  #[ must_use ]
  pub const fn wait( &self ) -> WaitKind
  #[ must_use ]
  pub const fn overflow( &self ) -> OverflowPolicy
  #[ must_use ]
  pub const fn producers( &self ) -> usize
  #[ must_use ]
  pub const fn batch( &self ) -> usize
  #[ must_use ]
  pub const fn is_multi_producer( &self ) -> bool
  #[ must_use ]
  pub const fn is_tick_safe( &self ) -> bool
  -- totals: functions, const fns, must_use marks --
12
11
11
```

---

### RC5 — Eleven of Twelve Are `const fn` and Carry `#[ must_use ]`; the Twelfth Is the Constructor

The counts line up exactly: twelve functions, eleven `const`, eleven `must_use`,
and it is the same eleven both times. `new` is the one function that is neither.

Both absences are correct, and for unrelated reasons. `new` returns
`Result< Self, RingError >`, which `core` already marks `#[ must_use ]`, so an
explicit mark would be redundant. And `new` contains the crate's only `?`, which
is not permitted in a const context — so `const` is foreclosed by the language
rather than declined.

**Finding.** The attribute set is therefore exactly right and reads as though it
were inconsistent. A reader scanning the file sees eleven identical two-line
prefixes and one function with neither, and the difference that matters — a `?`
one line into the body — is not visible at the declaration.

This is the same shape `ring_overflow` shows, where three of four functions carry
marks the fourth does not need
([`ring_overflow` § OV5](../../../ring_overflow/docs/api/001_four_declarations_three_of_them_const.md)).
The difference is that `ring_overflow`'s exception is irreducible — its blocker is
an atomic `fetch_add` — and this crate's is not: a `match` in place of the `?`
lifts it, which is why the exception is worth a sentence at the declaration here
and only a sentence there.

---

### RC6 — One Reader Returns a Wrapper, Four Return Raw Values, and Half Its Callers Unwrap It Immediately

Seven readers, all `&self`, all returning by value. Five report a stored field and
two compute a `bool`. Four of the five return the field's own type — `WaitKind`,
`OverflowPolicy`, `usize`, `usize` — and `capacity()` returns `Capacity`, a
validated newtype rather than the `usize` inside it.

**Finding.** The wrapper is doing real work: it is the only field whose validity
is a property rather than a type, and returning it whole is what stops a caller
reconstructing an unvalidated one. But it lands unevenly on the call sites. Of the
six production reads of `capacity()` outside this crate, two immediately call
`.get()` on it — `ring_bench/src/lib.rs:384` and `ring_core/src/lib.rs:204` — and
the other four pass the `Capacity` on intact.

Counting only the four that are not doctests, the split is exactly even: two
unwrap, two pass it on. So the surface asks every caller to decide, and half of
them decide to discard it one expression later, which is the least useful outcome
a newtype can have — it is neither consistently carried nor consistently
unwrapped. There is no `capacity_slots() -> usize` for the callers that
want the number, and no note saying the wrapper is deliberately not unwrapped
here. The cost is small and entirely in ergonomics; it is recorded because it is
the only place the reader surface is not uniform, and because the two shapes are
split down the middle rather than one being clearly rare.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/002`](002_three_readers_used_and_four_with_no_caller.md) | Which of these seven readers anyone calls |
| [`workaround/001`](../workaround/001_the_question_mark_forecloses_const.md) | What it would take for the twelfth to be `const` |
| [`type/001`](../type/001_five_derives_and_the_one_that_is_free_and_absent.md) | The derive list on line `:41` |
| [`pattern/001`](../pattern/001_a_consuming_builder_over_a_copy_record.md) | Why the setters take `mut self` and the readers take `&self` |

### Sources

| Fact | Where |
|------|-------|
| The derive list and the type | `ring_config/src/lib.rs:41-42` |
| The one function with neither attribute | `ring_config/src/lib.rs:65` |
| The four setters, `const` + `must_use` | `ring_config/src/lib.rs:88-89`, `:103-104`, `:121-122`, `:139-140` |
| The five field readers | `ring_config/src/lib.rs:153-154`, `:166-167`, `:179-180`, `:191-192`, `:203-204` |
| The two derived readings | `ring_config/src/lib.rs:220-221`, `:237-238` |
| The two callers that unwrap `Capacity` at once | `ring_bench/src/lib.rs:384`, `ring_core/src/lib.rs:204` |

### Tests

| Test | Covers |
|------|--------|
| `every_named_field_is_carried` | That all five readers report what was stored |
| `the_record_is_copy_and_compares_by_value` | What the derive list on `:41` buys a caller |
| `capacity_is_validated_at_construction` | The property the `Capacity` wrapper exists to hold |
