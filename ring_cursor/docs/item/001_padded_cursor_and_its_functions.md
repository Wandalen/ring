# Item: `PaddedCursor` and Its Functions

### Scope

- **Purpose**: Inventory every function callable on a `PaddedCursor`, with its signature, its `const`ness, and what covers it.
- **Responsibility**: Separate the two inherent functions from the four arriving by trait impl, and explain the one that could be `const` and is not.
- **In Scope**: `PaddedCursor::new`, `::addr`, and the four `SeqCell` methods.
- **Out of Scope**: `CursorPair`'s eight, which is [`item/002`](002_cursor_pair_and_its_readings.md).

### The Inventory

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*pub (const )?fn ' ring_cursor/src/lib.rs | sed -n '1,4p'
sed -n '/impl SeqCell for PaddedCursor/,/^}/p' ring_cursor/src/lib.rs
```

Live output:

```
pub fn slowest( cursors : &[ PaddedCursor ] ) -> Option< Seq >
  pub const fn new( value : Seq ) -> Self
  pub fn new( value : Seq ) -> Self
  pub fn addr( &self ) -> usize
impl SeqCell for PaddedCursor
{
  fn load( &self, order : Ordering ) -> Seq
  {
    self.0.get().load( order )
  }

  fn store( &self, value : Seq, order : Ordering )
  {
    self.0.get().store( value, order );
  }

  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq
  {
    self.0.get().fetch_add( n, order )
  }

  fn compare_exchange( &self, current : Seq, new : Seq, success : Ordering, failure : Ordering )
  -> Result< Seq, Seq >
  {
    self.0.get().compare_exchange( current, new, success, failure )
  }
}
```

| Function | Signature | Origin | `const`? | Line |
|----------|-----------|--------|:--------:|-----:|
| `new` | `( Seq ) -> Self` | inherent | ✅ *(not under `loom`)* | 160 / 168 |
| `addr` | `( &self ) -> usize` | inherent | ❌ | 187 |
| `load` | `( &self, Ordering ) -> Seq` | `SeqCell` | ❌ | 201 |
| `store` | `( &self, Seq, Ordering )` | `SeqCell` | ❌ | 206 |
| `fetch_add` | `( &self, u64, Ordering ) -> Seq` | `SeqCell` | ❌ | 211 |
| `compare_exchange` | `( &self, Seq, Seq, Ordering, Ordering ) -> Result< Seq, Seq >` | `SeqCell` | ❌ | 216 |

Six functions. **Two are this crate's, four are `ring_atomic`'s trait, forwarded
one line each.**

### The Four Forwarded Ones Add Nothing, Deliberately

```rust
impl SeqCell for PaddedCursor
{
  fn load( &self, order : Ordering ) -> Seq { self.0.get().load( order ) }
  // …three more, each one line
}
```

Every body is `self.0.get().<same method>( <same args> )`. The test that pins
this is `padding_does_not_change_what_the_cell_does`, and its comment states the
requirement:

> Every `SeqCell` method must behave exactly as the unpadded cell does, or the
> padding has stopped being free.

It exercises all four, including the failure path that is easy to get wrong:

```rust
assert_eq!(
  cursor.compare_exchange( Seq( 3 ), Seq( 9 ), Ordering::AcqRel, Ordering::Acquire ),
  Err( Seq( 4 ) ),
  "a failed exchange reports what it actually found — the retry's input"
);
```

**That assertion is the valuable one.** A `compare_exchange` that returned the
*expected* value on failure instead of the actual one would compile, would pass a
success-path test, and would turn `ring_claim`'s retry loop into an infinite
loop — it re-reads `current` from exactly this `Err`.

### `addr` Is the Only One That Could Be `const` and Is Not

```rust
#[ must_use ]
pub fn addr( &self ) -> usize
{
  core::ptr::from_ref( self ) as usize
}
```

The cast is what blocks it: **pointer-to-integer casts are not permitted in a
`const fn`**, because an address is not known at compile time. So `addr` cannot
be `const`, and neither can anything calling it — which is why
`CursorPair::on_distinct_lines` is not `const` even though
`ring_align::on_distinct_lines`, its callee, *is*.

That is a chain worth noticing: a `const` predicate made non-`const` by its
argument, not by itself.

### `addr` Exists for One Reason and Has Two Users

Its doc comment names the reason:

> The input to [`CursorPair::on_distinct_lines`], and the only way to check the
> padding against reality rather than against `size_of`.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -r '\.addr()' --include=*.rs | command grep -vE ':[[:space:]]*(///|//!)' \
  | sed -E 's/:/: /' | LC_ALL=C sort
```

Live output:

```
ring_cursor/src/lib.rs:         on_distinct_lines(self.producer.addr(), self.consumer.addr())
ring_cursor/tests/cursor_test.rs:         let gap = window[1].addr() - window[0].addr();
ring_cursor/tests/cursor_test.rs:     for (name, addr) in [("producer", pair.producer().addr()), ("consumer", pair.consumer().addr())]
ring_cursor/tests/cursor_test.rs:     let gap = pair.producer().addr().abs_diff(pair.consumer().addr());
ring_gating/tests/gating_test.rs:         assert_eq!(window[1].addr() - window[0].addr(), 64);
ring_mpsc/src/lib.rs:         let claim = self.claimer.cursor().addr();
ring_mpsc/src/lib.rs:         let consume = self.ring.consumer_cursor().addr();
```

| Caller | Use |
|--------|-----|
| `ring_cursor/src/lib.rs:440` | `CursorPair::on_distinct_lines` — the stated reason |
| `ring_mpsc/src/lib.rs:862-863` | Its own line predicate, over two loose cursors |
| `ring_cursor/tests/cursor_test.rs:78, 107, 123` | Three of the five layout tests |
| `ring_gating/tests/gating_test.rs:351` | Asserts the `Vec`'s stride is 64 |

**`addr` is what makes the layout claim checkable at all.** Without it, every
layout test would be a `size_of` assertion, and `size_of` cannot see whether two
*actual* fields are separated — see
[`invariant/001`](../invariant/001_one_cursor_one_line.md) on why clause 3 is a
different kind of statement from clauses 1 and 2.

It is also the item that let `ring_mpsc` build its forked predicate. A public
`addr` is what makes both the check and the fork possible; the crate could not
have had one without the other.

### Coverage

| Function | Unit test | Doctest | Cross-crate |
|----------|-----------|:-------:|-------------|
| `new` | `a_cursor_holds_the_sequence_it_was_built_with` | ✅ | Everywhere |
| `addr` | 3 layout tests | ✅ | `ring_mpsc`, `ring_gating` tests |
| `load` | `padding_does_not_change_…` | ✅ | Every consumer |
| `store` | same | ✅ | Every consumer |
| `fetch_add` | same, plus `two_threads_advancing_…`, `many_producers_…` | — | **one** — `ring_claim`, on purpose; see below |
| `compare_exchange` | same, both arms | — | `Claimer::claim` and `Claimer::claim_up_to` — the two retry loops |

**`fetch_add` and `compare_exchange` have no doctest of their own here**, because
they are documented on the trait in `ring_atomic` rather than on the impl. That
is correct placement — the impl adds nothing to document — and it means this
crate's 12 doc tests do not exercise two of its six functions. The unit test
covers both.

### `fetch_add` Has One Caller Outside This Crate, and It Is There on Purpose

```sh
cd "$(git rev-parse --show-toplevel)"
# Scoped two ways this recipe previously was not, each of which hid the answer.
#
# By crate: only crates declaring `ring_cursor` can hold a `PaddedCursor` at
# all.  The earlier form scanned every crate's src/ in the workspace, so its
# output moved whenever any unrelated crate gained any `fetch_add` on any
# counter, and the one line that mattered sat in a wall of `AtomicUsize` noise.
#
# By directory: it scanned src/ only.  The sole cross-crate caller is in a
# tests/ tree, so a recipe restricted to src/ could not have found it however
# wide it searched.  That caller postdates the finding below rather than having
# been missed by it -- but the recipe would not have caught it either way, so
# the finding would have gone on reading "none" for as long as it was left in
# a form that cannot see a tests/ tree.
#
# Line numbers stay dropped and the result sorted, as before: the claim is which
# crates call the method, never where in a file.
deps=$( command grep -l '^ring_cursor' */Cargo.toml | cut -d/ -f1 | sort )
echo '  -- crates a PaddedCursor can reach (they declare ring_cursor) --'
printf '%s\n' "$deps" | tr '\n' ' '; echo
echo '  -- every .fetch_add( call site in those crates, src/ and tests/ alike --'
for c in $deps ring_cursor; do
  command grep -r '\.fetch_add(' $c/src $c/tests --include='*.rs' 2>/dev/null \
    | command grep -vE ':[[:space:]]*(///|//!|//)' | sed 's|^ring/||'
done | LC_ALL=C sort
```

Live output:

```
  -- crates a PaddedCursor can reach (they declare ring_cursor) --
ring_barrier ring_claim ring_consume ring_debug ring_gating ring_mpsc ring_publish ring_shutdown ring_spsc ring_wait 
  -- every .fetch_add( call site in those crates, src/ and tests/ alike --
ring_barrier/tests/allocation_test.rs:        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
ring_barrier/tests/allocation_test.rs:        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
ring_claim/tests/allocation_test.rs:        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
ring_claim/tests/allocation_test.rs:        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
ring_claim/tests/claim_test.rs:    let stolen = claimer.cursor().fetch_add(4, Ordering::AcqRel);
ring_consume/tests/allocation_test.rs:        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
ring_consume/tests/allocation_test.rs:        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
ring_cursor/src/lib.rs:        self.0.get().fetch_add(n, order)
ring_cursor/tests/allocation_test.rs:        ALLOCATIONS.fetch_add(1, Ordering::Relaxed);
ring_cursor/tests/allocation_test.rs:        BYTES.fetch_add(layout.size(), Ordering::Relaxed);
ring_cursor/tests/cursor_test.rs:                    let _ = cursor.fetch_add(1, Ordering::AcqRel);
ring_cursor/tests/cursor_test.rs:                let _ = pair.consumer().fetch_add(1, Ordering::AcqRel);
ring_cursor/tests/cursor_test.rs:                let _ = pair.producer().fetch_add(1, Ordering::AcqRel);
ring_cursor/tests/cursor_test.rs:    assert_eq!(cursor.fetch_add(5, Ordering::AcqRel), Seq(10), "returns the pre-advance value");
ring_mpsc/tests/mpsc_test.rs:                  full_retries.fetch_add(1, Ordering::Relaxed);
ring_mpsc/tests/mpsc_test.rs:            DROPPED.fetch_add(1, Ordering::Relaxed);
ring_spsc/tests/spsc_test.rs:            DROPPED[TAG].fetch_add(1, Ordering::Relaxed);
```

Every line above is on an unrelated `AtomicUsize` — an allocation probe's
counter, a dropped-message tally, a retry counter — except three: this crate's
own impl body, the four in its own test suite, and one in `ring_claim`'s.

There is also a generic call site the scoped output cannot show, because the
crate holding it does not declare `ring_cursor` at all:

```rust
// ring_batch/src/lib.rs, the body of `pub fn claim< C : SeqCell >`
BatchClaim::new( cursor.fetch_add( count as u64, order ), count )
```

`ring_batch`'s only consumer is `ring_tls`, which declares `ring_atomic` and not
`ring_cursor` — so it passes an `AtomicSeq`. **No `PaddedCursor` reaches that
call site**, and its absence from the census above is that same fact read off
the dependency graph rather than off a grep.

| | |
|---|---|
| Callers of `PaddedCursor::fetch_add` in any `src/` | **0** |
| Callers in `ring_cursor/tests/` | 4 |
| Callers in another crate's `tests/` | **1** — `ring_claim`, deliberately |
| Why it exists anyway | `SeqCell` requires it — the impl cannot omit a method |

The one external caller is not an accident and not a misuse to fix. It is
`ring_claim`'s `writing_through_the_cursor_accessor_defeats_the_gate`, which
calls `claimer.cursor().fetch_add( 4, .. )` in safe code from outside that
crate, and asserts that the claim gate is thereby defeated. Its own doc comment
says it "asserts the hazard is *real*, not that the crate is broken, so it is
expected to keep passing", and traces it to CL33 in `ring_claim`'s
`docs/lifecycle/002_the_claimer_over_a_rings_life.md`: `cursor()` returns a
`&PaddedCursor`, `SeqCell` is public, every `SeqCell` method takes `&self`, so
`store` and `fetch_add` are reachable by any holder of the accessor's result.

So `ring_claim` does advance *its own* cursor with `compare_exchange` rather
than `fetch_add`, because it needs the retry loop — the crate's design is as
this document described it. What changed is that the same crate separately
proves `fetch_add` is reachable across the boundary, and that proof is the thing
this method's caller count is really about. The proof was committed 2026-09-04,
five days after CU25 recorded that no such caller existed — so CU25 was correct
when written and was overtaken, not wrong. The recipe beneath it could not have
reported the overtaking on its own, because the new caller landed in a `tests/`
tree that the recipe's `src/`-only scope did not read. A reader
tracing "how does a producer advance a cursor" will still find `fetch_add` first
and it is still the wrong answer for every producer in the family; the
difference is that reaching for it anyway is demonstrably possible from outside,
not merely undocumented.

### CU25 — `fetch_add`'s One Named Caller Is a Test Asserting It Should Not Be Callable

`fetch_add` is one of the four `SeqCell` methods `PaddedCursor` implements. No
*production* code anywhere calls it on a `PaddedCursor`: the family's generic
call site is in `ring_batch`, which is generic over `SeqCell` and does not
depend on this crate, so it never receives one.

Exactly one caller names the receiver's type. It is
`ring_claim`'s `writing_through_the_cursor_accessor_defeats_the_gate`, reaching
`claimer.cursor().fetch_add( .. )` in safe code from outside `ring_claim`, and
asserting that doing so hands out sequences the claim gate had just refused.

**Finding.** The only named use of this method is a demonstration that it is
reachable where it should not be — `ring_claim`'s CL33 hazard, written as an
executable assertion that is *expected to keep passing*. So the method is
exercised from outside, but by a test whose purpose is to pin a leak in another
crate's encapsulation rather than to use the forward for anything.

That leaves the forward itself covered only from inside: `padding_does_not_…`
asserts it returns the pre-advance value, and the two concurrency tests drive it
under contention. A change to `PaddedCursor`'s forward — a different ordering, a
wrapper — is caught by those three and by nothing downstream, because no
downstream caller exists that would want it to work. Narrowing `ring_claim`'s
`cursor()` accessor, which is what that crate records as the fix, would remove
the one external call site without removing any external capability.

**What this finding said before, and the two different ways it failed.** As
recorded on 2026-08-30 it read: *"No crate calls it on a `PaddedCursor` by name
… The method's only exercise arrives through a caller that cannot see its
receiver's type, so a change to `PaddedCursor`'s forward … would be caught by
`ring_batch`'s tests or by nothing. Nothing in this crate calls it either; its
own coverage is the doctest."*

The first sentence was true when written and has since been overtaken:
`ring_claim`'s test landed 2026-09-04, five days later. Every sentence after it
was false on arrival, and each clause is refuted by something already in the
repository on the day it was written:

| Clause | Refuted by | Present since |
|---|---|---|
| the only exercise arrives via a caller that cannot see the receiver's type | the four in-crate callers, each naming `PaddedCursor` or `CursorPair` concretely | 2026-08-28 |
| a change would be caught by `ring_batch`'s tests or by nothing | no `PaddedCursor` ever reaches `ring_batch` — its only consumer `ring_tls` passes an `AtomicSeq` | the dependency graph, then and now |
| nothing in this crate calls it either | those same four in-crate callers | 2026-08-28 |
| its own coverage is the doctest | this file's own Coverage table, twenty lines up: *"`fetch_add` and `compare_exchange` have no doctest of their own here"* | 2026-08-29 |

Only the first kind of failure is the passage of time, and only that kind is
what re-running a census repairs — the recipe above was rescoped to catch it,
and will now report the next such caller the day it appears. The second kind was
visible to anyone who read this document's own table on the day the finding was
written. No recipe catches that, and a census that had been correct all along
would not have made it any less wrong.

---

### CU26 — `addr` Takes the Address of the Wrapper, Not of the Atomic

```rust
core::ptr::from_ref( self ) as usize
```

`self` is the `PaddedCursor`. The `AtomicSeq` is inside a `CacheAligned` inside
it. The two addresses coincide only because `CacheAligned` is a single-field
wrapper whose payload sits at offset zero.

**Finding.** That offset is a property of `ring_align`, and it is asserted in
neither crate. `on_distinct_lines` compares two of these addresses, so the whole
third clause of [`invariant/001`](../invariant/001_one_cursor_one_line.md) rests on a layout
fact that no test states — true today, and true by convention rather than by
contract.

---

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_surface_that_forwards.md](../api/001_the_surface_that_forwards.md) | The same six functions as a surface rather than an inventory |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_padded_cursor.md](../data_structure/001_the_padded_cursor.md) | The layout these functions operate on |

### Items

| File | Relationship |
|------|--------------|
| [002_cursor_pair_and_its_readings.md](002_cursor_pair_and_its_readings.md) | The eight functions on the composite type |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_cursor_from_new_to_shared.md](../lifecycle/001_a_cursor_from_new_to_shared.md) | Where each function sits in the type's six stages |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_the_forwarding_newtype.md](../pattern/001_the_forwarding_newtype.md) | Why the four forwarded functions must exist |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_the_loom_constructor_cannot_be_const.md](../workaround/001_the_loom_constructor_cannot_be_const.md) | Why `new` appears twice |

### Sources

| File | Relationship |
|------|--------------|
| `ring_cursor/src/lib.rs:146-191` | Both inherent functions |
| `ring_cursor/src/lib.rs:199-221` | The four forwarded ones |
| `ring_atomic/src/lib.rs:108-151` | The trait they come from |
| `ring_claim/src/lib.rs:440, 491` | The two retry loops that depend on `compare_exchange`'s `Err` |
| `ring_batch/src/lib.rs:223` | The family's only other `SeqCell::fetch_add`, generic and never reached by a `PaddedCursor` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/cursor_test.rs:137-161` | All four forwarded functions, both `compare_exchange` arms |
| `tests/cursor_test.rs:130-136` | `new` and `default` |
| `tests/cursor_test.rs:74-129` | `addr`, three times |
| `ring_gating/tests/gating_test.rs:351` | `addr` across a `Vec` |
