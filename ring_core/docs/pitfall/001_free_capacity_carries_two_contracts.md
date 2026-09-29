# Pitfall: `free_capacity` Carries Two Contracts Under One Signature

### Scope

- **Purpose**: Name the trap this crate's uniform surface creates — that `free_capacity( &self ) -> usize` means something strictly stronger at one backend than at the other two, with nothing in the signature to say which one a caller holds.
- **Responsibility**: The trap's shape, why the compiler cannot catch it, the failure it produces, and what actually mitigates it.
- **In Scope**: `Producer::free_capacity` and `Producer::is_full` across the three backends.
- **Out of Scope**: `free_capacity`'s derivation within either in-house ring (→ [`ring_spsc` type/002](../../../ring_spsc/docs/type/002_free_capacity.md)); drain-order differences, which are documented but do not share this failure mode.

### Trap

The surface reports room the same way on every backend:

```rust
let room = producer.free_capacity();
```

At **SPSC** a reported `n` means *`n` pushes will succeed* — nothing else can
take the space, because nothing else exists. At **MPSC** and **crossbeam** it
means *`n` slots were free at some instant that has already passed*, and
another producer may have taken them before the caller acts.

The two readings differ in kind, not degree. One is a guarantee; the other is a
sample. And the signature is byte-for-byte identical.

```rust
// Correct against an SPSC ring. Silently wrong against the other two.
let room = producer.free_capacity();
for record in batch.drain( ..room )
{
  producer.try_push( record ).expect( "we checked" );   // panics under contention
}
```

**Nothing about that code changes when the backend changes.** No signature
moves, no trait bound fails, no lint fires. The only thing that changes is
whether `expect` is reachable — and it becomes reachable as a function of
*load*, so the configuration used during development (one producer, for
simplicity) is the configuration where it never fires.

The trap is a direct cost of this crate's own purpose. This crate's
uniform-surface requirement makes the backends interchangeable, and
interchangeable at the surface is exactly the condition under which a caller
stops tracking which one is underneath —
which is the same mechanism
[`ring_spsc`'s own pitfall/001](../../../ring_spsc/docs/pitfall/001_spsc_correctness_does_not_transfer.md)
warns about one layer down. This instance is that pitfall's row 2, arriving at
the layer that made it reachable.

### Failure

| Carried assumption | Failure at MPSC or crossbeam | Visibility |
|---|---|---|
| `free_capacity()` binds, so a checked batch cannot be refused | Mid-batch `try_push` returns `Err( record )`; a caller that unwrapped panics, one that ignored the result drops records | **Load-dependent** — passes at one producer, degrades as producers are added |
| `!is_full()` implies the next push succeeds | Same, one record at a time rather than mid-batch | Load-dependent, and rarer, which is worse |
| A reported `0` means the ring is full *now* | It means it was full; a drain may already have made room | Benign — the reading only ever understates room, so acting on it is safe |

**Only the first two are real.** The third is listed because it is the one
readers expect to be the problem, and it is not: understating is permitted at
every backend, which is what "advisory" licenses. The dangerous direction is
overstating, and it is the direction a single-threaded test cannot exercise
(→ `tests/manual/readme.md` C7).

**Row 2 is structural, not incidental.** `is_full` is not dispatched per
backend at all — it is one line, `self.free_capacity() == 0`, with no `match`:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'fn is_full' -A 3 ring_core/src/lib.rs
```

Live output:

```
  pub fn is_full( &self ) -> bool
  {
    self.free_capacity() == 0
  }
```

So it inherits this trap *exactly*, by definition rather than by coincidence.
There is no configuration in which `is_full` is binding and `free_capacity` is
not, and no future change to one that leaves the other alone. A reader who
mitigates `free_capacity` and forgets `is_full` has mitigated nothing.

`ring_spsc` has its own `is_full`; this crate never calls it
(→ `ring_spsc/tests/manual/readme.md` S10, where the definition was measured).

### Mitigation

1. **`try_clone` is the machine-checkable discriminator, and it is the only
   one.** It returns `None` at SPSC and `Some` at MPSC and crossbeam. A caller
   holding a producer that refuses to clone holds the binding contract; one
   whose clone succeeded does not. `Backend` can also be read from the ring, but
   `try_clone` is available on the *producer itself*, which is where the
   question is asked.

2. **Treat the returned value as a lower bound and check the result anyway.**
   `try_push` returns the record on refusal precisely so that the recovery is
   free — no copy, no allocation, the record is handed back. Code written this
   way is correct at every backend and pays nothing at SPSC.

3. **Do not build a `free_capacity`-then-push helper on this surface.** Any
   such helper has to pick one of the two contracts, and whichever it picks it
   is wrong at two backends or one. If a batch API is wanted, `try_push_batch`
   is it: it reports how many it accepted rather than requiring the caller to
   predict it.

**What does not mitigate it: documenting it only in the module docs.** The
hazard is at the call site, and the call site is in another crate. That is why
mitigation 1 is a method that returns a different value per backend rather than
a paragraph — a caller can branch on it.

**What also does not mitigate it: a test.** `free_capacity_never_overstates_the_room_available`
checks the safe direction, and that is all a single-threaded test can check:
binding and advisory are indistinguishable without a second thread racing for
the room. What actually guards the SPSC contract is structural — no second
producer can exist there, because `try_clone` refuses — so
`try_clone_refuses_at_spsc_and_permits_elsewhere` is the real guard, and it
guards by construction rather than by observation.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_producer_surface.md](../api/001_producer_surface.md) | Where the two contracts are stated as a contract rather than a warning |

### Invariants

| File | Relationship |
|------|--------------|
| [../invariant/002_uniform_delivery_across_backends.md](../invariant/002_uniform_delivery_across_backends.md) | Why `free_capacity`'s exactness is explicitly excluded from what the reached-test asserts |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/001_uniform_surface_over_unequal_backends.md](../pattern/001_uniform_surface_over_unequal_backends.md) | The general form: which differences a uniform surface may hide and which it must expose |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_producer_cardinality.md](../type/002_producer_cardinality.md) | `try_clone`'s return as the discriminator mitigation 1 relies on |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_spsc/docs/pitfall/001`](../../../ring_spsc/docs/pitfall/001_spsc_correctness_does_not_transfer.md) | Row 2 of its trap table, which this instance is the composed-layer occurrence of |
| [`ring_spsc/docs/type/002_free_capacity.md`](../../../ring_spsc/docs/type/002_free_capacity.md) | The binding contract worked out where it holds |
| [`ring_handle/docs/api/001_producer_surface.md`](../../../ring_handle/docs/api/001_producer_surface.md) | Specifies the same method with the same "advisory at MPSC, binding at SPSC" note, one layer up |

### Tests

| File | Relationship |
|------|--------------|
| `tests/core_test.rs` | `try_clone_refuses_at_spsc_and_permits_elsewhere` — mitigation 1, and the actual structural guard |
| `tests/core_test.rs` | `free_capacity_never_overstates_the_room_available` — the safe direction, which is the only one a single-threaded test reaches |
| `tests/manual/readme.md` | C7 — the record that no binding-contract test exists, and why one cannot be written here |

### CO47 — The Documented Escape Hatch Has Zero Callers

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
command grep -m1 -B7 -A1 -F '  pub fn free_capacity( &self ) -> usize' src/lib.rs
```

Live output:

```
  /// Room for at least this many more records.
  ///
  /// **Binding at SPSC, advisory at MPSC and crossbeam** — the asymmetry the
  /// uniform surface cannot express. Use [`try_clone`](Self::try_clone) to
  /// learn which reading applies, or treat every reading as advisory and let
  /// [`try_push`](Self::try_push) be the authority, which is always correct.
  #[ must_use ]
  pub fn free_capacity( &self ) -> usize
  {
```

Two remedies offered, one adopted. The `try_clone` route has no production
caller anywhere in the family
(→ [`../item/002`](../item/002_the_backend_discrimination_surface.md)); the
advisory route is what all four call sites do.

**That is the pitfall's real resolution and it is the weaker of the two.** It
works — `try_push` is the authority and always correct — but it means the
binding SPSC guarantee is unavailable in practice to anyone who wanted it.

### CO48 — The Pitfall Is Documented on the Producer Side Only

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
for m in free_capacity is_full len is_empty; do
  printf '%-14s ' "$m"
  grep -B8 "pub fn $m" src/lib.rs | grep -c 'Binding at SPSC\|advisory'
done
```

Live output:

```
free_capacity  2
is_full        1
len            1
is_empty       1
```

**What was found.** One method warned; three inherited the property and said
nothing. `is_full` was literally `free_capacity() == 0` and its rustdoc said "by
the same reading", which pointed at the warning without repeating it —
defensible. `len` and `is_empty` are on the other handle and had no such
pointer.

**A hazard documented on one of four affected methods reads as a property of
that method.** The pitfall instance itself is scoped to `free_capacity` in its
title, which propagated the same narrowing into the corpus.

The gap was closed from two other documents before this one was read again,
which is a fact about the corpus rather than about the code. `api/002` § CO7
found the consumer half and `decisions/002` § CO17 found `is_full`, each
arriving at one of the three unwarned methods without seeing this table. Three
documents, three angles, one gap — and this count is the only place all four
readings are put side by side, which is why it is the one that can say the gap
is closed rather than that a method was fixed.

**Disposition:** applied — `is_full`, `len` and `is_empty` each state the SPSC/MPSC asymmetry in their own rustdoc and name the authoritative call to make instead, so all four occupancy readings carry the warning rather than one; the count above is the check, and the narrowing left in this instance's title is now the instance's scope rather than the hazard's. Now prints: `is_full        1`
