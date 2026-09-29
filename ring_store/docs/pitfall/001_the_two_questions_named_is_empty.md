# Pitfall: The Two Questions Named `is_empty`

### Scope

**Purpose:** Record that `Buffer::is_empty` answers "does this buffer have zero
slots" and is therefore always false, that the question a caller means is
`all_empty`, and that the family runs seventeen `is_empty` methods including one
on a differently-named buffer type whose answer is the opposite.

**Responsibility:** The predicate collision: what each method asks, which one a
caller reaches for, and where the two meet in one file.

**In Scope:** `ring_store/src/lib.rs:127-191`;
`ring_tls/tests/tls_test.rs`.

**Out of Scope:** Why `len` exists at all is
[`decisions/002`](../decisions/002_a_length_kept_to_be_checked_against_itself.md).
That `all_empty` has no caller is
[`integration/001`](../integration/001_four_dependents_two_that_build_on_it.md).

---

### BF38 — The Method Named `is_empty` Is False for an Empty Buffer

Both predicates, measured on a buffer that has just been constructed and holds
nothing:

```
--- (2) the two questions named is_empty ---
  a freshly built buffer, every slot empty:
    is_empty()  = false
    all_empty() = true
```

The source says why, in its first line:

```sh
cd "$(git rev-parse --show-toplevel)"
awk '/^  \/\/\/ Always false — a `Capacity` cannot be zero, so a buffer always has slots\.$/{ n1 = NR } n1 && NR >= n1 && NR <= n1 + 5 { print } /^  \/\/\/ assert!\( !buffer\.is_empty\(\) \);$/{ n2 = NR } n2 && NR >= n2 + 2 && NR <= n2 + 6 { print }' ring_store/src/lib.rs
```

Live output:

```
  /// Always false — a `Capacity` cannot be zero, so a buffer always has slots.
  ///
  /// Exists because [`Buffer::len`] does; a `len` without an `is_empty` is a
  /// lint, and a hand-written `is_empty` that could disagree with `len` is
  /// worse than one that provably cannot.
  ///
  #[ must_use ]
  pub const fn is_empty( &self ) -> bool
  {
    self.slots.is_empty()
  }
```

**Finding.** The method exists to satisfy `clippy::len_without_is_empty`, and the
implementation chosen is the only correct one — `self.slots.is_empty()` cannot
disagree with `len` because both read the same slice. The doc leads with the
consequence rather than burying it. Every part of the handling is right.

The pitfall survives all of it, because it is in the name. `is_empty` on a
container reads as "does this container hold nothing", and the answer here is a
constant `false` regardless of contents. A caller writing
`if buffer.is_empty() { … }` gets a branch that can never be taken, under a
predicate that compiles, is `#[ must_use ]`, is `const`, and is named exactly
what they meant to ask. Nothing warns, and the test suite's own use of it —
`a_buffer_is_never_empty_because_a_capacity_is_never_zero` — asserts the negation
rather than demonstrating the trap.

The question a caller means is `all_empty`, which is four characters away, sits
in a different `impl` block ninety lines earlier, and has no caller in the family
at all.

`ring_slot`'s corpus reached the same fact from the other side and classified it
one tier lower: its
[`algorithm/002`](../../../ring_slot/docs/algorithm/002_emptiness_three_ways.md)
SL16 records `Buffer::is_empty` as a **misleading doc** — a differently-named
method that is always `false` and exists to satisfy a lint — while tracing where
the real emptiness fold lives. Read from the slot, that is the whole of it. Read
from here, the consequence is the branch, which is why the same fact carries the
heavier tier in this corpus.

The disagreement this finding describes is now pinned on one buffer, in one
test. `2>/dev/null` below drops cargo's own build-progress chatter (compile
timing and the hashed binary path both vary run to run), leaving only the
test's own deterministic stdout:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_store
cargo test --test buffer_test is_empty_and_all_empty_disagree_on_a_freshly_built_buffer 2>/dev/null
```

Live output:

```

running 1 test
test is_empty_and_all_empty_disagree_on_a_freshly_built_buffer ... ok

test result: ok. 1 passed; 0 failed; 0 ignored; 0 measured; 16 filtered out; finished in 0.00s
```

**Disposition:** applied — added
`is_empty_and_all_empty_disagree_on_a_freshly_built_buffer` to
`tests/buffer_test.rs`, asserting `!is_empty()` and `all_empty()` together on
one freshly built buffer — the exact disagreement this finding names, visible
in one place rather than inferred from two separate tests elsewhere in the
suite. Renaming either method is a breaking API change and out of scope for
a documentation-and-test pass; the doc comment on `is_empty` already leads
with the consequence, which is the mitigation this finding credits it with.
The crate's 17 unit tests plus 8 doctests re-verified passing
(`cargo test --all-features`, 2026-09-04). Now prints:
`test is_empty_and_all_empty_disagree_on_a_freshly_built_buffer ... ok`

---

### BF39 — Seventeen `is_empty` Methods, and Two of Them Meet in One Test File

The predicate is family-wide:

```sh
cd "$(git rev-parse --show-toplevel)"
# sorted: grep is shimmed to a parallel ugrep here, so hits arrive in completion order
grep -rn 'pub \(const \)\?fn is_empty' ring_*/src/*.rs | sed 's|ring/||' | sort | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
ring_barrier/src/lib.rs:  pub const fn is_empty( &self ) -> bool
ring_batch/src/lib.rs:  pub const fn is_empty( &self ) -> bool
ring_store/src/lib.rs:  pub const fn is_empty( &self ) -> bool
ring_claim/src/lib.rs:  pub const fn is_empty( self ) -> bool
ring_consume/src/lib.rs:  pub const fn is_empty( self ) -> bool
ring_core/src/lib.rs:  pub fn is_empty( &self ) -> bool
ring_flush/src/lib.rs:  pub fn is_empty( &self ) -> bool
ring_gating/src/lib.rs:  pub fn is_empty( &self ) -> bool
ring_handle/src/lib.rs:  pub fn is_empty( &self ) -> bool
ring_mpsc/src/lib.rs:  pub fn is_empty( &self ) -> bool
ring_mpsc/src/lib.rs:  pub const fn is_empty( &self ) -> bool
ring_registry/src/lib.rs:  pub fn is_empty( &self ) -> bool
ring_slot/src/lib.rs:  pub const fn is_empty( &self ) -> bool
ring_spsc/src/lib.rs:  pub const fn is_empty( &self ) -> bool
ring_spsc/src/lib.rs:  pub fn is_empty( &self ) -> bool
ring_tls/src/lib.rs:  pub fn is_empty( &self ) -> bool
ring_trace/src/lib.rs:  pub fn is_empty( &self ) -> bool
```

Seventeen, and the nearest neighbour is the sharpest. `ring_tls::TlsBuffer` is
also called a buffer, also has `is_empty`, and its answer is the useful one —
`self.items.is_empty()`, true when nothing is staged. Its test file imports both
types and resolves the collision by naming:

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'let mut buffer = TlsBuffer\|let mut ring : Buffer\|buffer\.is_empty()' ring_tls/tests/tls_test.rs | head -8
```

Live output:

```
  let mut buffer = TlsBuffer::with_capacity( 512 );
  let mut buffer = TlsBuffer::with_capacity( 64 );
  let mut buffer = TlsBuffer::with_capacity( 128 );
  let mut buffer = TlsBuffer::with_capacity( 8 );
  let mut ring : Buffer< TypedSlot< u32 > > = Buffer::new( capacity );
  let mut buffer = TlsBuffer::with_capacity( 3 );
  let mut buffer = TlsBuffer::with_capacity( 2 );
  let mut buffer = TlsBuffer::with_capacity( 4 );
```

**Finding.** In `ring_tls`'s tests the identifier `buffer` is a `TlsBuffer` and
the `ring_store::Buffer` is bound to `ring` — the naming is inverted relative to
the types precisely so that `buffer.is_empty()`, which appears ten times in that
file and is asserted true in eight of them, means what it looks like it means.

That inversion is evidence the collision is real and was felt at the one site
where both types are in scope. It is also undocumented on either side: nothing in
`ring_store` mentions `TlsBuffer`, nothing in `ring_tls` mentions that the other
buffer's `is_empty` is a constant, and the convention holding the test file
together is a variable-naming choice with no comment attached. A future test that
brings a `Buffer` into scope as `buffer` inherits the trap.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pitfall/002`](002_the_exclusive_accessor_you_can_throw_away.md) | The other way this API compiles into a no-op |
| [`decisions/002`](../decisions/002_a_length_kept_to_be_checked_against_itself.md) | Why `len` exists, and therefore why `is_empty` does |
| [`integration/001`](../integration/001_four_dependents_two_that_build_on_it.md) | That `all_empty`, the useful predicate, has no caller |
| [`api/001`](../api/001_twelve_functions_three_const_seven_must_use.md) | Both predicates as API surface |
| `ring_slot` — [`algorithm/002`](../../../ring_slot/docs/algorithm/002_emptiness_three_ways.md) | The same method, read from the slot side and tiered one rung lower |

### Sources

| Fact | Where |
|------|-------|
| `is_empty`'s doc and body | `ring_store/src/lib.rs:173-191` |
| `all_empty`'s body | `ring_store/src/lib.rs:139-141` |
| Both measured on a fresh buffer | Release probe, quoted above |
| The seventeen predicates | Census above |
| `TlsBuffer::is_empty` | `ring_tls/src/lib.rs:150-153` |
| The naming inversion | `ring_tls/tests/tls_test.rs:40-332` |
| The same method, classified from the slot side | `ring_slot` — SL16 |

### Tests

| Test | Covers |
|------|--------|
| `a_buffer_is_never_empty_because_a_capacity_is_never_zero` | The constant, asserted as a negation |
| `every_slot_starts_empty` | `all_empty`, the question a caller means |
| `ring_tls` — the ten `buffer.is_empty()` assertions | The other predicate, under the inverted naming |
| *(to create)* | A single test asserting both predicates on one fresh buffer, so the disagreement is visible in one place |
