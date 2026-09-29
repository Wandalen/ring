# lifecycle

A buffer's life has four events and one of them is optional. It is allocated once
in `new`, borrowed for as long as anyone holds it, optionally swept by `clear`,
and dropped. There is no resize, no reopen, no second constructor, and no `Drop`
impl — the whole arc fits in a paragraph, which is what makes the two instances
here able to say something beyond restating it.

What they find is at the two ends. At the drop end: destructors do run, the
buffer is the family's only owner of a payload, and because nothing anywhere ever
empties a slot on read, a dropped buffer destroys payloads the ring delivered a
lap ago — residency is not liveness, and nothing says so. At the sweep end:
`clear` documents a requester that has no dependency on this crate, has exactly
one caller in the family and that caller is a test, and states a guarantee about
previous-world payloads that one of the two slot shapes does not deliver.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Allocate Once, Borrow Forever, Drop Plainly](001_allocate_once_borrow_forever_drop_plainly.md) | BF30, BF31 — no `Drop` impl and destructors anyway, and a slot nothing ever empties |
| 002 | [The Sweep Nothing Calls](002_the_sweep_nothing_calls.md) | BF32, BF33 — a requester that cannot reach the method, and a guarantee half the shapes deliver |

### The Four Events

| Event | Mechanism | Who triggers it |
|-------|-----------|-----------------|
| Allocate | `new` — `Vec::with_capacity` + `resize_with` + `into_boxed_slice` | Every ring, once |
| Borrow | `get` / `get_mut` / `at` / `at_mut` / `iter` / `iter_mut` | Producers and consumers, per operation |
| Sweep | `clear` — a `for` loop over `Slot::clear` | Nothing, outside this crate's own test |
| Drop | `Box< [ S ] >`'s own destructor | Scope exit, dropping every resident payload |

Two of the four are unconditional and two are not. The sweep has no library
caller at all; the drop runs whether or not the ring considers its payloads live.

### Why a Payload Outlives Its Delivery

No consumer path in the family calls `TypedSlot::take` — its only occurrence in
library code is its own body. A consumer reads through a borrow and leaves the
payload in place, so a delivered payload stays resident until a producer
overwrites it a lap later, at which point the `set` return carries it out.
`ring_spsc` states the rule in a line comment at the one site that has to reason
about it, and that comment is the family's only written trace of it.

The consequence a reader is most likely to miss: a payload holding an external
resource does not release it when the consumer finishes with it. It releases a
lap later, or at ring teardown, whichever comes first.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the family's Drop impls — sorted: grep -r has no stable multi-file order
grep -rn '^impl.*Drop for' ring_*/src/*.rs | sort

# the operation that would empty a slot on read, and its whole call census
grep -rn '\.take(' ring_*/src/*.rs | grep -vE ':[[:space:]]*(///|//!|//)' | sort

# the one written statement of the consumer-does-not-clear rule
command grep -m1 -A6 -F '    let Ok( mut reservation ) = self.claim() else { return Err( record ) };' ring_spsc/src/lib.rs

# clear's doc, and the requester it names
command grep -m1 -A22 -F '  /// Empty every slot, keeping the allocation.' ring_store/src/lib.rs
sed -n '/^\[dependencies\]/,/^$/p' ring_shutdown/Cargo.toml
printf 'lines mentioning "buffer" in ring_shutdown/src/lib.rs: '
grep -ci 'buffer' ring_shutdown/src/lib.rs || true

# every caller of the sweep
grep -rn 'buffer\.clear()\|Buffer::clear' ring_*/src/*.rs ring_*/tests/*.rs 2>/dev/null \
  | grep -vE ':[[:space:]]*(///|//!|//)' | sort
```

The drop counts and the `BytesSlot` residue come from a release probe; both are
quoted in the instances.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BF30 | `ring_store` | n/a — observation | The buffer needs no `Drop` impl because `Box< [ S ] >` has one, making storage the family's single owner of every payload; the family's four `Drop` impls are all RAII publish guards one tier up |
| BF31 | `ring_store` | n/a — doc gap | Nothing in the family calls `take`, so a delivered payload stays resident for up to a lap and is destroyed by the buffer's drop if the ring dies first; that residency is not liveness is written only in one `ring_spsc` line comment |
| BF32 | `ring_store` | **misleading doc** | `clear`'s doc attributes the operation to `ring_shutdown`, which has no dependency on this crate and never mentions a buffer; the sweep's only caller family-wide is this crate's own test |
| BF33 | `ring_store` | **misleading doc** | `clear` states the family's strongest payload-visibility guarantee — a recycled ring must not hand a consumer the previous world's payloads — at a tier that can only forward `Slot::clear`, which for `BytesSlot` leaves every byte resident while `all_empty` reports true |
