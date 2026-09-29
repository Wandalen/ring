# API: Twelve Items, Seven `must_use`

### Scope

**Purpose:** Record the whole public surface, how `#[ must_use ]` is distributed
across it, and the one item where the distribution is exactly backwards.

**Responsibility:** The twelve public items of `ring_batch` and the attribute
above each.

**In Scope:** `ring_batch/src/lib.rs:56-362`.

**Out of Scope:** Which of the twelve anything actually calls is
[`api/002`](002_two_claim_functions_one_caller.md). The individual doc comments
are [`item/001`](../item/001_the_method_whose_reason_was_declined.md)
and [`item/002`](../item/002_one_past_the_end.md).

---

## The Surface, With Its Attributes

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -E '^\s*(#\[ must_use \]|pub (const )?fn |pub struct )' ring_batch/src/lib.rs
echo "  public items: $( command grep -cE '^\s*pub (const )?fn |^pub struct ' ring_batch/src/lib.rs )   must_use: $( command grep -c '#\[ must_use \]' ring_batch/src/lib.rs )"
```

Live output:

```
pub struct BatchClaim
  #[ must_use ]
  pub const fn new( start : Seq, count : usize ) -> Self
  #[ must_use ]
  pub const fn start( &self ) -> Seq
  #[ must_use ]
  pub const fn len( &self ) -> usize
  #[ must_use ]
  pub const fn is_empty( &self ) -> bool
  #[ must_use ]
  pub const fn end( &self ) -> Seq
  #[ must_use ]
  pub const fn contains( &self, seq : Seq ) -> bool
  pub fn sequences( &self ) -> impl Iterator< Item = Seq > + use< >
  #[ must_use ]
  pub const fn overlaps( &self, other : &Self ) -> bool
#[ must_use ]
pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim
pub fn claim_gated< P : SeqCell, C : SeqCell >
pub fn drain_order( claim : &BatchClaim, capacity : Capacity )
  public items: 12   must_use: 8
```

One struct, eight inherent methods, three free functions.

---

### BA5 — The Attribute Is Exactly on the Seven Items Where It Buys Nothing

Seven of the eight methods on `BatchClaim` carry `#[ must_use ]`. Every one of
them is a `const fn` taking `&self`, reading two fields, and returning a value.
Dropping any of their results does nothing at all — no state changes, no memory
is touched, and the optimiser removes the call. The attribute is a tidiness
warning there, not a safety net.

The three free functions carry no attribute. `claim_gated` is protected anyway,
because `Result` is `#[ must_use ]` by its own definition; `drain_order` returns
a lazy iterator whose construction has no effect. That leaves `claim`.

**Finding.** The distribution is inverted with respect to consequence. The
attribute is present on all seven items where forgetting the result is free, and
absent from the one item where forgetting it is unrecoverable.

---

### BA6 — A Dropped `claim` Silently Burns Sequences, and the Compiler Says Nothing

`claim` performs a `fetch_add`. The cursor moves whether or not anyone keeps the
range it returns. There is no attribute on it and no `Drop` impl on `BatchClaim`
to notice, so a bare call statement is well-formed code:

```
   Compiling ring_index v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_index)
   Compiling ring_atomic v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_atomic)
   Compiling ring_batch v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_batch)
   Compiling batch_probe v0.0.0 (/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/-batch_probe)
    Finished `release` profile [optimized] target(s) in 0.50s
  two claims taken and both ranges thrown away
  cursor is now at 16, so sequences 0..16 are owned by nobody
  a consumer waiting on sequence 0 will wait forever
```

The probe body is two lines — `claim( &cursor, 8, order );` and
`let _ = claim( &cursor, 8, order );` — built with `RUSTFLAGS="-D warnings"`.
Both forms compile.

**Finding.** This is the crate's one **latent hazard** reachable by a caller who
writes nothing unusual. A claim is a promise to write into a range that the
consumer is now waiting on; dropping it does not release the range, because
[`algorithm/001`](../algorithm/001_one_fetch_add_whatever_the_count.md) BA1
shows there is nothing to release it *to*. The cursor advanced, and the only
protocol for those sequences was the value that was discarded.

`#[ must_use ]` on `claim` would catch both forms shown above at compile time
and costs one line. Its absence is not a considered decision — the seven pure
accessors that do carry it demonstrate the author's default, and `claim` is the
item that fell outside it.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -B1 -F 'pub fn claim< C : SeqCell >' ring_batch/src/lib.rs
```

Live output:

```
#[ must_use ]
pub fn claim< C : SeqCell >( cursor : &C, count : usize, order : Ordering ) -> BatchClaim
```

**Disposition:** applied — added `#[ must_use ]` to `claim` in `src/lib.rs`,
matching the attribute the seven pure accessors already carry, so both forms
shown in the probe above (`claim( &cursor, 8, order );` and
`let _ = claim( &cursor, 8, order );`) now diverge — the first becomes a
compile warning (a hard error under this project's own `-D warnings`
verification levels), the second stays silent and explicit. The crate's 21
unit tests plus 10 doctests re-verified passing
(`cargo test --all-features`, 2026-09-04). Now prints:
`#[ must_use ]`

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`api/002`](002_two_claim_functions_one_caller.md) | Which of the twelve items has a caller |
| [`algorithm/001`](../algorithm/001_one_fetch_add_whatever_the_count.md) | Why an advanced cursor cannot be walked back |
| [`lifecycle/001`](../lifecycle/001_no_lifecycle_and_no_rollback.md) | The absence of `Drop` and what it forecloses |
| [`item/002`](../item/002_one_past_the_end.md) | `end()`, which three of the twelve route through |

### Sources

| Fact | Where |
|------|-------|
| The twelve public items | `ring_batch/src/lib.rs:56-362` |
| `#[ must_use ]` count | Census above |
| A dropped claim compiling under `-D warnings` | Release probe, quoted above |

### Tests

| Test | Covers |
|------|--------|
| `a_claim_reports_its_own_extent` | Four of the seven attributed accessors |
| `overlap_is_exactly_range_intersection` | The seventh |
| *(to create)* | Nothing can test a missing attribute; a `compile_fail` doctest on `claim` would be the check |
