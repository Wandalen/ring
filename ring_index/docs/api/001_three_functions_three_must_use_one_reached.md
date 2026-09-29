# API: Three Functions, Three `must_use`, One Reached

### Scope

**Purpose:** Record the entire public surface — three free functions, no types,
no traits, no constants — the uniform `#[ must_use ]` across it, and the fact
that two of the three have no caller anywhere in the family.

**Responsibility:** The surface as a contract: what is exported, what each
signature commits to, and which parts of it anything actually uses.

**In Scope:** `ring_index/src/lib.rs:39-119`; every `src/` and `tests/`
file in `ring_*`.

**Out of Scope:** Each function read on its own is [`item/001`](../item/001_the_fold_itself.md)
and [`item/002`](../item/002_the_two_that_nothing_calls.md). Why `run` returns a
`Vec` is [`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md).

---

## The Whole Surface

```sh
cd "$(git rev-parse --show-toplevel)"
command grep '^pub \|^#\[ must_use' ring_index/src/lib.rs
```

Live output:

```
#[ must_use ]
pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex
#[ must_use ]
pub fn aliases( a : Seq, b : Seq, capacity : Capacity ) -> bool
#[ must_use ]
pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >
```

Three functions. No `pub struct`, no `pub enum`, no `pub trait`, no `pub const`,
no `pub mod`. Every one of the three is `#[ must_use ]`, and every one takes a
`Capacity` as its last argument.

---

### IX5 — Uniform `#[ must_use ]`, Because Every Function Is Pure

There is nothing to decide here and the crate decided it consistently: all three
functions compute a value and have no effect, so discarding a result is always a
mistake, so all three carry the attribute. The count is 3 of 3.

**Finding.** This is worth recording only as the contrast it provides. The family
is not uniform on this — `ring_store` publishes twelve functions of which seven
are `#[ must_use ]`, and the five that are not include `get_mut` and `at_mut`,
whose discarded results are silent no-ops (that crate's BF40). A crate with no
`&mut self` and no interior mutability cannot make that mistake, and this one
does not.

The reason is structural rather than careful: `ring_index` has no state to
mutate, so there is no method whose value is incidental to an effect. Uniformity
here is a consequence of the crate's shape, not a policy that could be copied.

---

### IX6 — Two of the Three Functions Have No Caller in the Family

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# every import or path-call of this crate, anywhere but this crate
command grep -r 'use ring_index\|ring_index::' ring_*/src/*.rs ring_*/tests/*.rs \
  | command grep -v '^ring_index/' | sed 's|ring/||'

echo '  --- files that name it at all, importing or not ---'
command grep -rl 'ring_index' ring_*/src/*.rs ring_*/tests/*.rs \
  | command grep -v '^ring_index/' | sed 's|ring/||'
```

Live output:

```
ring_batch/src/lib.rs:use ring_index::of;
ring_store/src/lib.rs:use ring_index::of;
ring_store/src/lib.rs:  /// any other way than through `ring_index::of` for this same capacity is
ring_batch/tests/batch_test.rs:    assert_eq!( slot, ring_index::of( seq, capacity ), "the fold must be ring_index's" );
ring_store/tests/buffer_test.rs:use ring_index::of;
  --- files that name it at all, importing or not ---
ring_batch/src/lib.rs
ring_store/src/lib.rs
ring_seqno/src/lib.rs
ring_types/src/capacity.rs
ring_types/src/id.rs
ring_batch/tests/batch_test.rs
ring_store/tests/buffer_test.rs
ring_seqno/tests/seq_test.rs
ring_types/tests/types_test.rs
```

**Finding.** Every one of the five imports and path-calls names `of`. Nothing
anywhere — no source file, no test, in any of the 33 crates — names `aliases` or
`run`. Two thirds of the surface is reachable only from this crate's own test
file.

The second census is the more interesting number, because it is larger than the
first and in the other direction. Nine files across four crates name
`ring_index`; only three of those files import it. `ring_seqno` and `ring_types`
are *upstream* of this crate and cannot import it, yet both name it in prose to
say what it is not their job to do — `ring_seqno:17` says "The folding itself is
`ring_index`'s job, deliberately in another crate", and `ring_types/id.rs:82`
says a `SlotIndex` is "Derived from a [`Seq`] by `ring_index`, never constructed
by counting."

So the crate is cited as an owner five times more widely than it is called. That
is what a one-owner boundary looks like when it is working: the neighbours point
at it instead of duplicating it. One crate does duplicate it anyway, and that is
[`pitfall/002`](../pitfall/002_the_second_fold_nobody_noticed.md) IX19.

`aliases` has a documented reason to exist without callers: its doc says it
"reports the aliasing so a gate can be tested against it; it does not prevent
anything itself", i.e. it is a testing affordance. No test outside this crate
uses it for that. `run` has no such note.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/002`](../item/002_the_two_that_nothing_calls.md) | The two unreached functions, one at a time |
| [`integration/001`](../integration/001_two_dependents_and_a_third_that_did_it_again.md) | Which crates depend on this one, and which reimplemented it |
| [`api/002`](002_the_three_signatures_and_the_const_they_are_not.md) | The same signatures read for what they give up — `const`, and an inverse |
| [`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md) | `run`'s return type against the alternative |

### Sources

| Fact | Where |
|------|-------|
| The three signatures and their attributes | `ring_index/src/lib.rs:39-119` |
| Every cross-crate mention | Census above |
| `aliases`' stated purpose | `ring_index/src/lib.rs:54-62` |
| `ring_store`'s seven-of-twelve `must_use` | `ring_store` — [`api/001`](../../../ring_store/docs/api/001_twelve_functions_three_const_seven_must_use.md) |

### Tests

| Test | Covers |
|------|--------|
| `mask_equals_modulo_over_four_laps_of_every_capacity` | `of` |
| `aliasing_is_exactly_whole_laps`, `a_sequence_aliases_itself` | `aliases` |
| `a_run_wraps_at_most_once_within_one_capacity` and three siblings | `run` |
| *(to create)* | Nothing — the surface is fully covered here; what is missing is a *caller*, which no test can supply |
