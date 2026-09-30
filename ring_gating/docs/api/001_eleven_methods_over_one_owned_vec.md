# API: Eleven Methods Over One Owned `Vec`

### Scope

- **Purpose**: Give `GatingSet`'s whole public surface, and draw out the three properties that are visible only when the eleven signatures are read together.
- **Responsibility**: List every method with its receiver, its `const`-ness and its `#[ must_use ]`, then establish what the uniform `&self` does and does not imply.
- **In Scope**: The signatures, and what they permit.
- **Out of Scope**: What the gating methods compute — see [`algorithm/001`](../algorithm/001_headroom_in_two_delegations.md) and [`algorithm/002`](../algorithm/002_check_orders_its_two_refusals.md).

### The Whole Surface

| Line | Method | Receiver | `const` | `#[ must_use ]` | Returns |
|-----:|--------|----------|:-------:|:---------------:|---------|
| 93 | `new( capacity, consumers )` | — | | ✅ | `Self` |
| 108 | `len()` | `&self` | | ✅ | `usize` |
| 121 | `is_empty()` | `&self` | | ✅ | `bool` |
| 142 | `cursor( index )` | `&self` | | ✅ | `Option< &PaddedCursor >` |
| 155 | `cursors()` | `&self` | | ✅ | `&[ PaddedCursor ]` |
| 168 | `capacity()` | `&self` | ✅ | ✅ | `Capacity` |
| 197 | `slowest()` | `&self` | | ✅ | `Option< Seq >` |
| 222 | `headroom( producer )` | `&self` | | ✅ | `usize` |
| 242 | `admits( producer, count )` | `&self` | | ✅ | `bool` |
| 283 | `check( producer, count )` | `&self` | | | `Result< (), RingError >` |
| 321 | `limit()` | `&self` | | ✅ | `Option< Seq >` |

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*pub (const )?fn ' ring_gating/src/lib.rs   # the 11 rows above
grep -c '#\[ must_use \]'       ring_gating/src/lib.rs   # 10
grep -c '&mut self'             ring_gating/src/lib.rs || true   # 0
```

Live output:

```
  pub fn new( capacity : Capacity, consumers : usize ) -> Self
  pub fn len( &self ) -> usize
  pub fn is_empty( &self ) -> bool
  pub fn cursor( &self, index : usize ) -> Option< &PaddedCursor >
  pub fn cursors( &self ) -> &[ PaddedCursor ]
  pub const fn capacity( &self ) -> Capacity
  pub fn slowest( &self ) -> Option< Seq >
  pub fn headroom( &self, producer : Seq ) -> usize
  pub fn admits( &self, producer : Seq, count : usize ) -> bool
  pub fn check( &self, producer : Seq, count : usize ) -> Result< (), RingError >
  pub fn limit( &self ) -> Option< Seq >
10
0
```

Every method is documented and every method carries a doctest — twelve run,
eleven of them here and one on the module:

```sh
cd "$(git rev-parse --show-toplevel)"
# the result line only, with its duration stripped — the surrounding lines carry
# wall-clock timings, which would put this recipe permanently at odds with its
# own quoted output
cargo test -p ring_gating --doc 2>&1 | grep '^test result:' | sed 's/; finished in [0-9.]*s//'
```

Live output:

```
test result: ok. 12 passed; 0 failed; 0 ignored; 0 measured; 0 filtered out
```

### G1 — There Is No `&mut self`, and the Set Is Still Not Immutable

Zero methods take `&mut self`. So after `new` returns, **nothing in the public
surface can add a consumer, remove one, or change the capacity.** The set's shape
is decided by two constructor arguments and then frozen for the value's whole
life — argued in [`data_structure/001`](../data_structure/001_the_set_that_cannot_grow.md).

The tempting second conclusion is wrong. `GatingSet` is *not* an immutable value,
because the thing that actually changes during a ring's operation — consumer
position — is reached through a shared borrow:

```rust
// ring_gating/src/lib.rs:141-145
#[ must_use ]
pub fn cursor( &self, index : usize ) -> Option< &PaddedCursor >
{
  self.cursors.get( index )
}
```

`PaddedCursor` wraps an atomic, so `&PaddedCursor` is enough to store through it.
The crate's own tests do exactly that:

```rust
// tests/gating_test.rs:411 — inside std::thread::scope
set.cursor( 0 ).unwrap().store( Seq( position ), Ordering::Release );
```

`set` is a `&GatingSet` there, in a spawned closure, while the main thread reads
`headroom` from the same value. **The `&self`-only surface is not a statement that
the value does not change; it is the precondition for the value changing from two
threads at once.** Interior mutability is what makes the two compatible, and it
lives one crate down in `ring_align`/`ring_cursor` rather than here.

So the surface splits cleanly along a line that is not the usual one:

| What is fixed at construction | What changes through `&self` |
|-------------------------------|------------------------------|
| The number of cursors | Every cursor's stored `Seq` |
| The capacity | — |

`len`, `is_empty`, `cursors` and `capacity` read only the first column and are
therefore *constant for the value's whole life*. `slowest`, `headroom`, `admits`,
`check` and `limit` read the second and are snapshots — see
[`002`](002_the_reading_that_returns_a_position.md).

### G5 — One `const fn`, and Only Two of the Others Are Blocked

`capacity()` is the crate's only `const fn`. Reading the table, four other methods
look like plausible candidates — `len`, `is_empty`, `cursor`, `cursors` — and they
divide in half. Compiled on this toolchain (`rustc 1.97.1`) against a two-field
stand-in with the same bodies:

| Method body | `const fn` compiles? |
|-------------|:--------------------:|
| `self.v.len()` | ✅ |
| `self.v.is_empty()` | ✅ |
| `self.v.get( i )` | ❌ |
| `&self.v` | ❌ |

Both failures are the same failure. `Vec` has no `get`, and is not a slice — both
bodies deref-coerce to `[ T ]` first, and `Deref` is not yet a const trait:

```
error: `Deref` is not yet stable as a const trait
error[E0658]: cannot perform conditionally-const deref coercion on `Vec<u32>` in constant functions
  = note: see issue #143874 <https://github.com/rust-lang/rust/issues/143874>
```

So `cursor` and `cursors` are blocked by the language, and `len` and `is_empty`
are not — **they could carry `const` today and do not.** Nothing depends on it;
recorded because the asymmetry reads as deliberate and is not. The remaining five
gating methods are out of reach for a reason that will not expire: every one of
them descends to an atomic load in `ring_cursor::slowest`, and an atomic load can
never be a compile-time operation.

### G6 — `check` Is the One Method Without `#[ must_use ]`

Ten of eleven carry the attribute. `check` does not, and it is the only one that
does not need it:

```rust
pub fn check( &self, producer : Seq, count : usize ) -> Result< (), RingError >
```

`Result` is annotated `#[ must_use ]` in the standard library, so
`set.check( p, n );` warns on its own. Adding the attribute to the function would
be redundant rather than wrong — and `admits`, which returns a bare `bool`, needs
it precisely because `bool` is not.

The consistency question is worth asking anyway: a reader scanning for the
attribute sees ten and one gap, and the gap looks like an oversight. Two ways to
close it, neither taken:

| Option | Cost |
|--------|------|
| Add `#[ must_use ]` to `check` | Redundant, and clippy's `double_must_use` lint objects |
| Note the exception in `check`'s doc | One line, no lint interaction |

Neither is done today, and nothing forces the choice: the workspace's clippy
table does not name `double_must_use`.

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/\[workspace.lints.clippy\]/,/^\[[a-z]/p' Cargo.toml
```

Live output:

```
[workspace.lints.clippy]
# Unsafe: one operation per block, each with its own `// SAFETY:`.
undocumented_unsafe_blocks = "deny"
multiple_unsafe_ops_per_block = "deny"
unnecessary_safety_comment = "warn"
unnecessary_safety_doc = "warn"
cast_ptr_alignment = "warn"
ptr_as_ptr = "warn"
ptr_cast_constness = "warn"
mem_forget = "warn"
# Concurrency: locks are what this family exists to avoid; refcount bumps stay visible.
mutex_atomic = "warn"
mutex_integer = "warn"
rc_mutex = "warn"
clone_on_ref_ptr = "warn"
# Determinism: hash iteration order would leak into the delivery order.
iter_over_hash_type = "warn"
# Sequence and index arithmetic.
cast_sign_loss = "warn"
precedence_bits = "warn"
# Hygiene.
dbg_macro = "warn"
exit = "warn"
infinite_loop = "warn"
large_stack_frames = "warn"
todo = "warn"
unimplemented = "warn"
unused_result_ok = "warn"

[workspace.lints.rustdoc]
```

So the redundant-attribute option would cost nothing today and would start
warning the moment anyone enables the `pedantic` group. That is an argument for
the doc-comment option, and it is why the gap is recorded rather than closed
here.

### What the Surface Does Not Offer

| Absent | Consequence |
|--------|-------------|
| Any `&mut self` method | Membership is a constructor argument, permanently |
| Any wait or block | A caller that must wait spins; `ring_barrier` owns waiting |
| Any way to read one consumer's headroom | The bound is the minimum, and only the minimum |
| Any `Iterator` over positions | Callers get `cursors()` and load them |

The third is the one that matches the invariant exactly: there is no
`headroom_for( index )`, because a per-consumer answer would be a number no
producer may act on. See
[`invariant/001`](../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md).

### GT6 — Fixed Membership Is Not Immutability

```
&mut self in ring_gating/src/lib.rs : 0
137:  pub fn cursor( &self, index : usize ) -> Option< &PaddedCursor >
150:  pub fn cursors( &self ) -> &[ PaddedCursor ]
```

A type with no `&mut self` method reads as immutable. This one is not: the
cursors it lends are atomics, and an atomic behind a shared reference is
writable.

**Finding.** No method takes `&mut self`, so the set's membership is fixed at construction; the type is nonetheless not immutable, because `cursor()` hands out interior mutability through a shared borrow

The doc comment on `cursor()` now states this inline, rather than leaving it
only in this file's G1 prose:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F "/// One consumer" ring_gating/src/lib.rs
```

Live output:

```
  /// One consumer's cursor, for that consumer to advance.
  ///
  /// The `&self` receiver does not make the return value read-only: a
  /// `PaddedCursor` wraps an atomic, so `&PaddedCursor` is enough to store
  /// through it. Membership is fixed once the set is constructed; a cursor's
  /// stored position is not.
```

**Disposition:** applied — `cursor()`'s own doc comment in `src/lib.rs` now
states the interior-mutability-through-shared-borrow fact inline instead of
leaving it only in this file's surrounding prose, and the crate's 23 unit
tests plus 12 doctests re-verified passing (`cargo test -p ring_gating
--all-features`, 2026-09-04). Now prints:
`Membership is fixed once the set is constructed; a cursor's`

---

### GT7 — One `const fn`, Three Reasons for the Other Ten

```
163:  pub const fn capacity( &self ) -> usize      <- the only one
137:  cursor      -> Vec indexing
192:  slowest     -> atomic loads
217:  headroom    -> delegation into non-const callees
```

`capacity` is `const` because it reads a field that is neither behind an
allocation nor behind an atomic. Every other method fails one of those three
tests, and which one it fails is worth knowing separately.

**Finding.** The crate's only `const fn`, and the other ten cannot be, for three different reasons — `Vec` indexing, atomic loads, and delegation into non-`const` callees

---

### GT8 — The Missing Attribute That Is Not Missing

```
must_use in ring_gating/src/lib.rs : 10
public methods                     : 11
265:  pub fn check( ... ) -> Result< (), RingError >   <- the eleventh
```

Counting attributes across the surface turns up a gap. Reading the one method
that has the gap explains why it is not one.

**Finding.** The one method of eleven without `#[ must_use ]`, and the one method that does not need it — `Result` carries the attribute already, so the convention is complete without being uniform and a reader counting attributes finds an apparent gap that is not one

---


### APIs

| File | Relationship |
|------|--------------|
| [002_the_reading_that_returns_a_position.md](002_the_reading_that_returns_a_position.md) | The five snapshot readings, and their two return shapes |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_set_that_cannot_grow.md](../data_structure/001_the_set_that_cannot_grow.md) | G1 in full — what a fixed membership costs |
| [../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md](../data_structure/002_owning_the_cursors_rather_than_borrowing_them.md) | The `Vec` these eleven methods sit over |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md](../invariant/001_the_bound_is_the_minimum_and_only_the_minimum.md) | Why no per-consumer reading is offered |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_send_and_sync_without_unsafe.md](../type/002_send_and_sync_without_unsafe.md) | Why `&self` is enough to cross a thread boundary |

### Sources

| File | Relationship |
|------|--------------|
| `ring_gating/src/lib.rs:92-324` | All eleven methods |
| `ring_gating/src/lib.rs:141-145` | `cursor` — the shared borrow that permits stores |
| `Cargo.toml` § `[workspace.lints.clippy]` | One line, `undocumented_unsafe_blocks` — `double_must_use` unenforced |
| rust-lang/rust#143874 | `Deref` const-stability, which blocks two of the four candidates |

### Tests

| File | Relationship |
|------|--------------|
| `tests/gating_test.rs:315-327` | `len`, `is_empty`, `cursors` and `cursor`'s bounds, swept over 0–4 consumers |
| `tests/gating_test.rs:355-359` | `capacity` round-trips the constructor argument |
| `tests/gating_test.rs:394-428` | Stores through `&self` while another thread reads |
