# Workaround: An `unsafe impl Sync` on a Type Whose Ends Are Not `Sync`

### Scope

- **Purpose**: Record the hand-written `Sync` impl, the bound it carries, and the asymmetry that makes it safe — the ring is `Sync`, the handles onto it deliberately are not.
- **Responsibility**: `unsafe impl< S : Send > Sync for Ring< S >` and the test that pins its counterpart.
- **In Scope**: The impl, its bound, and the negative it depends on.
- **Out of Scope**: The other nine `unsafe` lines (→ [`001`](001_the_unsafe_code_opt_out_and_what_bounds_it.md)).

### The Impl

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
grep 'unsafe impl' src/lib.rs
grep 'fn both_ends_are_send_and_neither_is_sync' tests/spsc_test.rs
```

Live output:

```
unsafe impl< S : Send > Sync for Ring< S > {}
  fn both_ends_are_send_and_neither_is_sync()
```

`Ring< S >` contains an `UnsafeCell`, so the compiler will not derive `Sync` and
cannot be argued with. The impl asserts what the compiler cannot see: the two
ends touch disjoint slots, so sharing the ring between them is sound.

**`S : Send`, not `S : Sync`.** A record is written on the producer's thread and
read on the consumer's — moved across a boundary, never shared across one. `Sync`
would be the wrong bound and a stricter one, excluding payload types that are
perfectly usable here.

### The Negative It Rests On

The ring is `Sync`; **neither end is**, and that is asserted rather than assumed.
`both_ends_are_send_and_neither_is_sync` is the test. If `Producer` were `Sync`,
two threads could hold `&Producer` and both call `claim` — and `claim` takes
`&mut self`, so this is not reachable today, but it is reachable by any future
change that relaxes that receiver.

**Compare the multi-producer sibling, where the opposite is true.**
`ring_mpsc::Producer` is `Send + Sync + Copy` on purpose — duplicating it *is*
what multi-producer means there. The same word on the same-named type carries
opposite intent in the two crates, which is the single easiest thing to get
wrong when moving code between them.

### Removal Trigger

A `Sync` the compiler can conclude, which needs the slot array's exclusivity
expressed in a type rather than in cursor arithmetic. Same condition as
[`001`](001_the_unsafe_code_opt_out_and_what_bounds_it.md)'s; the two retire
together.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | The impl and its `SAFETY` comment |
| `tests/spsc_test.rs` | `both_ends_are_send_and_neither_is_sync` |
| `../../../ring_mpsc/docs/workaround/002_an_unsafe_impl_sync_the_compiler_cannot_derive.md` | The sibling impl, with the opposite handle-level answer |

### SP52 — The Same Type Name Carries Opposite `Sync` Intent in the Two Crates

```sh
cd "$(git rev-parse --show-toplevel)"
for c in ring_spsc ring_mpsc; do
  printf '%-11s ' "$c"; grep -vE '^\s*(//|///|//!)' $c/src/lib.rs | grep -E 'unsafe impl' | head -2
done
```

Live output:

```
ring_spsc   unsafe impl<S: Send> Sync for Ring<S> {}
ring_mpsc   unsafe impl<S: Send> Sync for Ring<S> {}
```

`ring_core`'s `ProducerInner` has one variant per backend and hands out a uniform
`Producer` over both, so the union's thread-safety is the weaker of the two — as
it must be.

**The hazard is textual rather than structural.** Nothing composes wrongly today;
what is easy is reading a `SAFETY` comment in one crate while editing the other,
since the types share a name, a shape, and a file position.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c 'identical line under a different justification' ring_spsc/src/lib.rs
```

Live output:

```
1
```

**Disposition:** applied — `ring_spsc/src/lib.rs`'s `SAFETY` comment above
`unsafe impl< S : Send > Sync for Ring< S >` now names `ring_mpsc` explicitly
and states the two crates justify the identical line on different grounds
(exactly two threads here, many producers on disjoint slots there), so a
reader mid-edit sees the divergence flagged instead of needing tribal
knowledge. Now prints: `1`
