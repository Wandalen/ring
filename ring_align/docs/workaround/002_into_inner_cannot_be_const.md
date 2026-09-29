# Workaround: `into_inner` Cannot Be `const`

### Scope

- **Purpose**: Record why one of the wrapper's four associated functions breaks the crate's otherwise-uniform `const fn` surface, with the cost and the deletion condition.
- **Responsibility**: State the constraint, prove it, give what the crate does instead, price it, and name what would let the workaround go.
- **In Scope**: `CacheAligned::into_inner`'s missing `const`.
- **Out of Scope**: What unwrapping costs semantically, which is [`lifecycle/001`](../lifecycle/001_the_wrapped_values_arc.md); the other three functions, which are [`item/001`](../item/001_cache_aligned_and_its_associated_functions.md).

### The Constraint

A `const fn` that moves a field out of `self` cannot compile, because the
remainder of `self` must be dropped and const-eval will not run a destructor.
Verified on `rustc 1.97.1 (8bab26f4f 2026-07-14)`:

```sh
cat > ./-probe.rs <<'EOF'
#[ repr( align( 64 ) ) ]
pub struct CacheAligned< T >( T );

impl< T > CacheAligned< T >
{
  pub const fn into_inner( self ) -> T { self.0 }
}

fn main() {}
EOF
rustc --crate-name probe --edition 2021 -o /dev/null ./-probe.rs
rm -f ./-probe.rs
```

Live output:

```
error[E0493]: destructor of `CacheAligned<T>` cannot be evaluated at compile-time
 --> ./-probe.rs:6:28
  |
6 |   pub const fn into_inner( self ) -> T { self.0 }
  |                            ^^^^                 - value is dropped here
  |                            |
  |                            the destructor for this type cannot be evaluated in constant functions

error: aborting due to 1 previous error

For more information about this error, try `rustc --explain E0493`.
```

**The wrapper has no destructor of its own** — it is a tuple struct over a
generic `T` with no `Drop` impl. The error is about the *unbounded* `T`: for
some `T` there would be a destructor, and a `const fn` must compile for every
instantiation. Adding a `T : Copy` bound would satisfy const-eval and would
narrow the type for every caller, which is a worse trade than losing `const` on
one function.

### The Workaround

```rust
// ring_align/src/lib.rs:115
pub fn into_inner( self ) -> T
{
  self.0
}
```

Drop the `const`. Three of the four associated functions keep it — `new`,
`get`, `get_mut` — and this one does not
(→ [`item/001`](../item/001_cache_aligned_and_its_associated_functions.md)).

### The Cost

| # | Cost | Severity |
|---|------|----------|
| X1 | A caller cannot unwrap in a `const` context | **Zero today.** No caller in the workspace unwraps at all — the one production consumer holds the wrapper for the value's whole life (→ [`item/001`](../item/001_cache_aligned_and_its_associated_functions.md) § Callers) |
| X2 | The surface is not uniformly `const`, so "everything here is compile-time" needs a qualifier | Documentation. The qualifier is in [`non_functional_requirement/002`](../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md)'s table rather than being quietly omitted |
| X3 | The friction is *useful* here, incidentally | Not a cost. `into_inner` is the one operation that discards the crate's guarantee (→ [`lifecycle/001`](../lifecycle/001_the_wrapped_values_arc.md) § The Exit Is Silent), so the one place it is harder to reach is the right place |

**X1 being zero is why this is a footnote rather than a problem**, and it is
worth checking rather than assuming:

```sh
cd "$(git rev-parse --show-toplevel)"
# The question is not "who calls some `into_inner`" — `Mutex` and `PoisonError`
# both have one, and every crate added to the workspace may bring another. It is
# "who reaches this crate's type at all", which only `CacheAligned` answers.
echo '  -- every mention of the wrapper outside the crate that defines it --'
command grep -r 'CacheAligned' --include=*.rs . | command grep -v '^ring_align/'
echo '  -- and every into_inner in the family, none of them this one --'
command grep -r 'into_inner' --include=*.rs ring_*/ | command grep -v '^ring_align/' | sed 's|^ring/||'
```

Live output:

```
  -- every mention of the wrapper outside the crate that defines it --
ring_cursor/src/lib.rs://! [`ring_align::CacheAligned`] and nothing else — no field of its own, no
ring_cursor/src/lib.rs:use ring_align::{ on_distinct_lines, CacheAligned };
ring_cursor/src/lib.rs:pub struct PaddedCursor( CacheAligned< AtomicSeq > );
ring_cursor/src/lib.rs:    Self( CacheAligned::new( AtomicSeq::new( value ) ) )
ring_cursor/src/lib.rs:    Self( CacheAligned::new( AtomicSeq::new( value ) ) )
ring_cursor/src/lib.rs:// Each forward below assumes `CacheAligned::get` stays a free, no-op
  -- and every into_inner in the family, none of them this one --
ring_bench/tests/bench_test.rs:/// `.unwrap_or_else( std::sync::PoisonError::into_inner )`, recovering the
ring_bench/tests/bench_test.rs:  // The fix: `.unwrap_or_else( PoisonError::into_inner )`, `commit_batch`'s
ring_bench/tests/bench_test.rs:  let recovered = queue.lock().unwrap_or_else( std::sync::PoisonError::into_inner );
ring_bench/src/lib.rs:  let mut guard = queue.lock().unwrap_or_else( std::sync::PoisonError::into_inner );
ring_bench/src/lib.rs:    queue.into_inner().expect( "no producer panics while holding the lock" ).drain( .. ).collect();
ring_bench/src/lib.rs:  ( reported.into_inner(), drained, write_nanos )
ring_bench/src/lib.rs:  ( reported.into_inner(), drained, write_nanos )
ring_mpsc/tests/mpsc_test.rs:          granted.lock().unwrap_or_else( std::sync::PoisonError::into_inner ).extend( mine );
ring_mpsc/tests/mpsc_test.rs:        received.lock().unwrap_or_else( std::sync::PoisonError::into_inner ).extend( drained );
ring_mpsc/tests/mpsc_test.rs:    // Fix(mpsc_test_granted_received_lock_poison_recovery): `into_inner` can
ring_mpsc/tests/mpsc_test.rs:    let granted = granted.into_inner().unwrap_or_else( std::sync::PoisonError::into_inner );
ring_mpsc/tests/mpsc_test.rs:    let received = received.into_inner().unwrap_or_else( std::sync::PoisonError::into_inner );
ring_mpsc/tests/mpsc_test.rs:          granted.lock().unwrap_or_else( std::sync::PoisonError::into_inner ).extend( mine );
ring_mpsc/tests/mpsc_test.rs:        received.lock().unwrap_or_else( std::sync::PoisonError::into_inner ).extend( drained );
ring_mpsc/tests/mpsc_test.rs:    let granted = granted.into_inner().unwrap_or_else( std::sync::PoisonError::into_inner );
ring_mpsc/tests/mpsc_test.rs:    let received = received.into_inner().unwrap_or_else( std::sync::PoisonError::into_inner );
ring_mpsc/tests/mpsc_test.rs:  /// `.unwrap_or_else( std::sync::PoisonError::into_inner )` instead of
ring_mpsc/tests/mpsc_test.rs:    // The fix: `.unwrap_or_else( PoisonError::into_inner )`, the real test's
ring_mpsc/tests/mpsc_test.rs:    let recovered = granted.lock().unwrap_or_else( std::sync::PoisonError::into_inner );
ring_shutdown/tests/shutdown_test.rs:/// This asserts the crate's limitation on purpose. `into_inner` hands back a
ring_shutdown/tests/shutdown_test.rs:  let mut raw = guarded.into_inner();
ring_shutdown/src/lib.rs:  /// [`Guarded::into_inner`] is the documented one — it consumes the guard and
ring_shutdown/src/lib.rs:  pub fn into_inner( self ) -> Producer< 'a, T >
ring_trace/src/lib.rs:    self.entries.lock().unwrap_or_else( std::sync::PoisonError::into_inner )
```

Thirty hits across the family, and **the name is shared**: most are
`Mutex::into_inner`, `PoisonError::into_inner`, or `ring_shutdown`'s own
method. Exactly four concern this function, all of them inside `ring_align` —
the declaration (`src/lib.rs:115`), its doctest (`:113`), and two assertions
in `tests/align_test.rs` (`:139`, `:141`). **No crate outside this one unwraps a
`CacheAligned` at all.**

**Correction (2026-09-28):** this paragraph read "Thirteen hits". `ring_bench`
gained `tests/bench_test.rs` and a lock-poison-recovery path in its own
`src/lib.rs`, and `ring_mpsc/tests/mpsc_test.rs` gained a
`mpsc_test_granted_received_lock_poison_recovery` fix — both unrelated to
this crate or to `ring_align → ring_types`; the shared method name is what
pulls them into this census (→ "the name is shared", above). The count is
thirty now, and the paragraph's own two claims about *this* crate's function
specifically — four internal hits, zero external unwraps — are unaffected by
either change and still hold.

That the grep needs disambiguating is itself the point: a bare name search
across a 33-crate workspace answers a different question than the one asked
(→ [`integration/001`](../integration/001_one_dependency_one_consumer.md) § The
Fifth Mention).

### Alternatives Considered

| # | Alternative | Why not |
|---|-------------|---------|
| Y1 | Bound the impl `T : Copy` | Compiles, and excludes every non-`Copy` payload from the wrapper for the benefit of a function nobody calls |
| Y2 | Two impls — a `const` one under `T : Copy`, a plain one otherwise | Overlapping impls; not expressible without specialisation |
| Y3 | `core::mem::ManuallyDrop` to suppress the destructor | Turns a three-line safe crate into one reasoning about drop elision, against a workspace that denies `unsafe` and for no caller |
| Y4 | **Drop the `const`** | Chosen |

### Deletion Condition

Delete this workaround when const-eval handles dropping the remainder of a
destructured value for an unbounded generic — i.e. when the probe above
compiles. Then add `const` at `src/lib.rs:99`, delete this file, and drop the
"all but one" qualifier from
[`non_functional_requirement/002`](../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md)'s
evidence table.

**There is no reason to hurry this.** X1 is zero and X3 is mildly positive, so
the workaround costs nothing and the deletion is cosmetic.

### AL50 — The Cost Is Measured at Zero Rather Than Argued to Be Small

`ring_cursor/src/lib.rs` is the only file outside this crate that names
`CacheAligned` at all, and it calls `into_inner` zero times. Every hit in the
census above belongs to a different function that happens to share the spelling
— `Mutex::into_inner`, `PoisonError::into_inner`, and `ring_shutdown`'s own
method on `Producer`.

**Finding.** So X1 ("no caller loses anything") is a measurement, not an
estimate: the set of const-context callers is empty because the set of callers
outside this crate is empty. That is a stronger basis for Y4 than the prose
gives it, and it is also the reason the finding is an observation rather than a
coverage gap — there is nothing uncovered, only nothing there.

---

### AL52 — Two Crates Absorbed the Same Language Limit Without Either Noticing

`ring_shutdown/src/lib.rs:505` declares its own non-`const` `into_inner`, and
`tests/shutdown_test.rs:273` documents the limitation deliberately — "this
asserts the crate's limitation on purpose". Same `E0493`, same shape of
workaround, arrived at independently.

**Finding.** Neither workaround document mentions the other, so the family
carries two separate records of one constraint and no place that says it is one
constraint. The cost today is nil; the cost when const-eval changes is that
whoever deletes this file has no reason to look at `ring_shutdown`, and the
second workaround outlives the reason for both.

### Items

| File | Relationship |
|------|--------------|
| [../item/001_cache_aligned_and_its_associated_functions.md](../item/001_cache_aligned_and_its_associated_functions.md) | The four functions and which are `const` |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_the_wrapped_values_arc.md](../lifecycle/001_the_wrapped_values_arc.md) | L2 — this constraint as the one friction point on the silent exit |

### Non Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md](../non_functional_requirement/002_the_crate_costs_nothing_at_runtime.md) | The "all but one" row this workaround explains |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_cache_aligned.md](../type/002_cache_aligned.md) | The type whose unbounded `T` is the actual cause |

### Workarounds

| File | Relationship |
|------|--------------|
| [001_the_alignment_literal_cannot_be_the_constant.md](001_the_alignment_literal_cannot_be_the_constant.md) | The crate's other absorbed language constraint, which costs considerably more |

### Sources

| File | Relationship |
|------|--------------|
| `ring_align/src/lib.rs:109-118` | The declaration and its doc comment |

### Tests

| File | Relationship |
|------|--------------|
| `src/lib.rs` doctest | `assert_eq!( CacheAligned::new( 3u16 ).into_inner(), 3 )` — a runtime assertion, which is all the function permits |
