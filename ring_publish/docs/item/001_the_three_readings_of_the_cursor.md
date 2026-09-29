# Item: The Three Readings of the Cursor

### Scope

- **Purpose**: Take the three methods that read the published cursor — `cursor`, `published`, `is_published` — one at a time, and record what each returns, who calls it, and why all three exist when one is derivable from another.
- **Responsibility**: Give each reading its contract, its call sites counted from the tests, and its place in the derivability chain; then record that one of the three is not a reading at all.
- **In Scope**: `Publisher::cursor` (`src/lib.rs:105-120`), `Publisher::published` (`:122-136`), `Publisher::is_published` (`:212-232`).
- **Out of Scope**: The two that move the cursor — see [`item/002`](002_the_two_publications.md).

### The Three, Side by Side

| | `cursor` | `published` | `is_published` |
|--|----------|-------------|----------------|
| Signature | `( &self ) -> &PaddedCursor` | `( &self ) -> Seq` | `( &self, seq : Seq ) -> bool` |
| `const` | **yes** | no | no |
| Body | `&self.cursor` | `self.cursor.load( GATING )` | `seq < self.published()` |
| Atomic operations | **zero** | one `Acquire` load | one `Acquire` load |
| Answers | *where is the cursor* | *how far has publication reached* | *is this sequence readable* |
| Call sites in tests | 13 | 15 | 8 |
| Call sites in doctests | 1 | 6 | 3 |
| Derivable from the others | no | no | **yes**, from `published` |

The last row is the whole reason this file exists. `is_published( seq )` is
literally `seq < published()` — a caller with `published` needs nothing else. The
converse fails: no finite number of `is_published` answers pins the frontier,
because the frontier can move between them.

So the surface carries one redundant method, deliberately, and two that are not
redundant with anything.

### `cursor` — The One That Is Not a Reading

`src/lib.rs:105` calls it *"the published cursor, for a consumer's barrier to be
built over"*, and that is exactly what every non-trivial call site does.

```rust
#[ must_use ]
pub const fn cursor( &self ) -> &PaddedCursor
{
  &self.cursor
}
```

It performs no atomic operation. It is the crate's only `const fn`, and — as
[`data_structure/001`](../data_structure/001_one_padded_cursor_and_nothing_else.md)
§ PB14 records — being `const` is possible here only because returning a
reference to a field does not touch the loom-conditional inside `PaddedCursor`.

### PB21 — Ten of Thirteen Call Sites Are the Same Expression, to the Character

```sh
cd "$(git rev-parse --show-toplevel)"/ring_publish
grep -hoE '.{0,45}publisher\.cursor\(\).{0,12}' tests/*.rs | sort | uniq -c
```

Live output:

```
      1   assert!( core::ptr::eq( cursor, publisher.cursor() ), "and it 
      1   assert_eq!( publisher.cursor().load( Order
      1   let cursor = publisher.cursor();
     10 rier = Barrier::over( core::slice::from_ref( publisher.cursor() ) );
```

| Count | Expression |
|------:|------------|
| **10** | `Barrier::over( core::slice::from_ref( publisher.cursor() ) );` |
| 1 | `let cursor = publisher.cursor();` |
| 1 | `assert!( core::ptr::eq( cursor, publisher.cursor() ), …` |
| 1 | `assert_eq!( publisher.cursor().load( Ordering::Acquire ), …` |

The bottom three are `tests/publish_test.rs:32,52,57` — the accessor testing
itself. The other ten are every use in `tests/handshake_test.rs`, and they are
not merely similar: they are one expression repeated verbatim in ten test
bodies, at `:119`, `:157`, `:186`, `:300`, `:339`, `:374`, `:390`, `:468`,
`:536`, `:577`.

That uniformity is worth recording because it says the accessor has exactly one
real use and the crate never abstracted it. A helper — `fn barrier_over(
&Publisher ) -> Barrier` — would collapse ten identical lines to one, and it does
not exist. The reason is the dependency direction: `ring_barrier` is a
dev-dependency, so any such helper would have to live in the test file, and
`tests/handshake_test.rs:248-271`'s three helpers (`cap`, `slots`, `payload`)
stop short of it. Nothing forced the eleventh.

`core::slice::from_ref` appearing in all ten is the visible mark of
`ring_barrier::wait_for` taking a bare `&[ PaddedCursor ]` rather than a
`GatingSet`, which `ring_barrier/src/lib.rs:40-44` records as a signature
this crate's handshake test forced.

### `published` — The Frontier Itself

```rust
#[ must_use ]
pub fn published( &self ) -> Seq
{
  self.cursor.load( GATING )
}
```

One `Acquire` load, and the summary at `:122` carries the crate's most
load-bearing four words: *"one past the last readable sequence"*. Every
off-by-one argument in
[`invariant/002`](../invariant/002_is_published_is_exclusive_of_the_frontier.md)
resolves against that phrase.

It is the most-called of the three — 15 sites in tests, 6 in doctests — and its
distribution says why: 11 of the 15 are in `tests/publish_test.rs`, asserting
the frontier's exact value after an operation, and 4 are in
`tests/handshake_test.rs` (`:155`, `:431`, `:494`, `:559`) checking a whole run
ended where it should. It is the crate's observation point, not part of any
production path.

`tests/publish_test.rs:42` calls it twice in one line, which is the entire body
of `new_and_default_agree`:

```rust
assert_eq!( Publisher::new().published(), Publisher::default().published() );
```

Two constructors compared through the only state either produces.

### `is_published` — The Redundant One, and Why It Is Kept

```rust
#[ must_use ]
pub fn is_published( &self, seq : Seq ) -> bool
{
  seq < self.published()
}
```

Derivable, kept anyway, and `src/lib.rs:212-215` says what it is for:

> The consumer-facing question feature 170 is graded on: a slot claimed but not
> published must answer `false`.

Three arguments for keeping it, in decreasing strength:

1. **It names the requirement.** The feature is graded on a question, and the
   crate answers that question in the shape it is asked. `seq < publisher
   .published()` at a call site is the same computation with the requirement
   removed from the text.
2. **It moves the boundary convention inside the crate.** A caller writing
   `seq < published()` has to know the frontier is exclusive; a caller writing
   `is_published( seq )` does not. Exactly one comparison operator in the family
   encodes that convention, and this method is where it lives — so an off-by-one
   is a one-line fix in one place rather than a sweep of call sites.
3. **It gives the exhaustive test something to compare against.**
   `tests/publish_test.rs:214-238` asserts `is_published( c ) == ( c < frontier )`
   at 320 points. That test is only meaningful because the method and the
   predicate are separately written; if `is_published` were removed the test
   would be asserting a tautology.

Its 8 call sites are all in `tests/publish_test.rs` — `:33`, `:196-200`, `:210`,
`:230` — and every one is inside an assertion. Nothing computes with the result;
the method exists to be checked.

### PB22 — Six Cursor Accessors in the Family, and Not One of Them Is Read-Only

The three readings are advertised as readings, but `cursor()` hands out
`&PaddedCursor`, and `PaddedCursor` implements `SeqCell` publicly — `load`,
`store`, `fetch_add`, `compare_exchange`. So the "read triple" is two reads and
one full read-write handle.

```sh
cd "$(git rev-parse --show-toplevel)"
# every accessor handing out a cursor. `-n` is deliberately absent: the table
# below names symbols, never lines, and six line numbers across five crates
# go stale on the next edit to any one of them
command grep -rE 'fn [a-z_]+\([^)]*\) *-> *(&|Option< *&) *PaddedCursor' ring_*/src/*.rs
# and the read-only counterpart, which does not exist — so this prints nothing
# and exits nonzero, and the empty result is the evidence, not a failure
command grep -rE 'pub (struct|trait) .*(Reader|ReadOnly|View)' ring_*/src/*.rs \
  || echo '(nothing — no read-only cursor type exists in the family)'
```

Live output:

```
ring_claim/src/lib.rs:  pub const fn cursor( &self ) -> &PaddedCursor
ring_cursor/src/lib.rs:  pub const fn producer( &self ) -> &PaddedCursor
ring_cursor/src/lib.rs:  pub const fn consumer( &self ) -> &PaddedCursor
ring_gating/src/lib.rs:  pub fn cursor( &self, index : usize ) -> Option< &PaddedCursor >
ring_mpsc/src/lib.rs:  fn consumer_cursor( &self ) -> &PaddedCursor
ring_publish/src/lib.rs:  pub const fn cursor( &self ) -> &PaddedCursor
(nothing — no read-only cursor type exists in the family)
```

| Accessor | Crate | `const` | Visibility | Hands out |
|----------|-------|:-------:|-----------|-----------|
| `Publisher::cursor` | `ring_publish` | ✔ | `pub` | `&PaddedCursor` |
| `Claimer::cursor` | `ring_claim` | ✔ | `pub` | `&PaddedCursor` |
| `CursorPair::producer` | `ring_cursor` | ✔ | `pub` | `&PaddedCursor` |
| `CursorPair::consumer` | `ring_cursor` | ✔ | `pub` | `&PaddedCursor` |
| `GatingSet::cursor` | `ring_gating` | — | `pub` | `Option< &PaddedCursor >` |
| `Ring::consumer_cursor` | `ring_mpsc` | — | private | `&PaddedCursor` |

Six accessors across five crates, all handing out the same mutable-through-shared
handle. The second command matches nothing at all: **there is no read-only cursor
type anywhere in the family** — no `CursorView`, no `SeqReader`, no sealed read
trait.

So the hole
[`invariant/001`](../invariant/001_the_frontier_moves_only_by_compare_exchange.md)
names in this crate is not local to it. Every crate that shares a cursor shares
a writable one, and every one of them relies on the same thing this crate relies
on: nobody writes through a borrowed cursor. Nothing checks that, in any crate.

Adding the missing type is cheap in isolation and expensive in aggregate — it
would land in `ring_cursor` (Tier 2) and change the signature of five public
accessors across four crates, plus `Barrier::over`'s parameter, plus
`GatingSet`'s. That is a Tier 2 change rippling to Tier 5, for a hazard that has
never fired. Recording it is the proportionate response; the alternative belongs
to a family-level decision this crate cannot make alone.

### The Derivability Chain, Stated Once

```
cursor()  ──load(GATING)──▶  published()  ──` < `──▶  is_published( seq )
```

Left to right is total: each step is one operation on the value before it, and
the crate implements exactly those two operations. Right to left is impossible
at every step — a `bool` does not determine a `Seq`, and a `Seq` does not
determine an address.

That is why the surface has three rather than one. The leftmost is the only one
that can be shared, the rightmost is the only one that states the requirement,
and the middle is the only one a test can assert an exact value against.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_six_methods_and_no_caller.md](../api/001_six_methods_and_no_caller.md) | All six methods, and the call-site census these three came from |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_one_padded_cursor_and_nothing_else.md](../data_structure/001_one_padded_cursor_and_nothing_else.md) | The field all three read, and why `cursor` is the crate's only `const fn` |
| [../data_structure/002_the_four_cursors_of_the_handshake.md](../data_structure/002_the_four_cursors_of_the_handshake.md) | What the ten identical `Barrier::over` lines wire together |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_frontier_moves_only_by_compare_exchange.md](../invariant/001_the_frontier_moves_only_by_compare_exchange.md) | The hole `cursor` opens, here shown to be family-wide |
| [../invariant/002_is_published_is_exclusive_of_the_frontier.md](../invariant/002_is_published_is_exclusive_of_the_frontier.md) | The convention `is_published` exists to keep in one place |

### Items

| File | Relationship |
|------|--------------|
| [002_the_two_publications.md](002_the_two_publications.md) | The two methods that move what these three read |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_what_a_publication_costs.md](../non_functional_requirement/001_what_a_publication_costs.md) | Zero, one, one — the atomic cost of each reading |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_seq_a_usize_and_the_one_cast.md](../type/001_a_seq_a_usize_and_the_one_cast.md) | `Seq`, `bool`, `&PaddedCursor` — the three return types |

### Sources

| File | Relationship |
|------|--------------|
| `ring_publish/src/lib.rs:105-136,212-232` | The three methods, their doctests, and their summaries |
| `ring_cursor/src/lib.rs:89,311,333` | `GATING`, and two more accessors handing out the same handle |
| `ring_atomic/src/lib.rs:108-151` | The four `SeqCell` methods a borrowed cursor exposes |
| `ring_barrier/src/lib.rs:40-44` | Why the ten identical lines take `core::slice::from_ref` |

### Tests

| File | Relationship |
|------|--------------|
| `tests/publish_test.rs:26-34` | All three read on a fresh publisher, in one test |
| `tests/publish_test.rs:45-58` | `cursor` returns the same address twice, checked by `core::ptr::eq` |
| `tests/publish_test.rs:36-43` | `published` as the only observable both constructors produce |
| `tests/publish_test.rs:214-238` | `is_published` against its own predicate, 320 times |
| `tests/handshake_test.rs:119,157,186,300,339,374,390,468,536,577` | The ten identical `Barrier::over` lines |
