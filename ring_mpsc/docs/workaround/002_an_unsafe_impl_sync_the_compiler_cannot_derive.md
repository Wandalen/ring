# An `unsafe impl Sync` the Compiler Cannot Derive

### Scope

- **Purpose**: Record why `Ring< S >` asserts `Sync` by hand, what the assertion promises, and what would let the compiler conclude it instead.
- **Responsibility**: The auto-trait gap, the four-part safety argument, and the removal trigger.
- **In Scope**: `unsafe impl< S : Send > Sync for Ring< S > {}`.
- **Out of Scope**: The other nine unsafe sites (→ [`001`](001_the_unsafe_code_opt_out_and_its_obligations.md)).

### The Gap

`Ring< S >` holds `Buffer< UnsafeCell< S > >`. `UnsafeCell< T >` is deliberately
never `Sync`, so the auto-derive stops there and `Ring` is not `Sync` — which
means it cannot be shared across threads, which is the entire point of a
multi-producer ring.

**The compiler is right to refuse.** `UnsafeCell` marks "the rules here are not
the ones you can check", and nothing in the type says two producers touch
different slots. That fact lives in the claim protocol.

### What the Assertion Promises

The impl carries a four-part `SAFETY` argument, and each part is separately
checkable:

| Part | Claim | Where it is enforced |
|------|-------|----------------------|
| Consumer uniqueness | At most one `Consumer` exists at a time | `ends` takes `&mut self`, `split` takes `&mut Ends`; `Consumer` is neither `Clone` nor `Sync` |
| Slot disjointness | A producer holds `&mut` to exactly the slot it claimed and has not stamped | The claim protocol, gated by `Claimer`'s headroom check |
| Producer→consumer edge | The payload write precedes the stamp store | `PUBLISH` store paired with `OBSERVE` load |
| Consumer→producer edge | The payload read precedes the cursor advance | `COMMIT` store paired with `ring_cursor::GATING` load |

`S : Send` is the bound rather than `S : Sync` because a record is written on
one thread and read on another — moved, never shared.

### What the Tests Reach

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep -hoE '^\s*fn [a-z_0-9]+' tests/*.rs | sed 's/^ *fn //' \
  | grep -E 'send|sync|thread|producers|race|exactly_once|distinct'
```

Live output:

```
four_producers_exchange_one_hundred_thousand_items_with_byte_parity
producers_under_measured_contention_at_small_capacity_show_no_torn_or_duplicated_records
the_producer_is_send_and_sync_and_copy_which_is_what_multi_producer_means
assert_send
assert_sync
the_claim_cursor_and_the_consumer_cursor_are_on_distinct_cache_lines
every_record_written_is_destroyed_exactly_once
a_sibling_producer_recovers_a_lock_poisoned_by_another_producers_panic
the_consumer_never_drains_further_than_the_producers_published
```

The first three parts have a test each. The fourth — the consumer→producer edge
— has `the_orderings_are_the_ones_the_publication_invariant_names`, which checks
the constants rather than the behaviour.

**That is the honest weak point of this workaround, and it is narrower than it
looks.** The reason a value assertion is used is not that the behaviour is
unobservable: this workspace's host is `aarch64-unknown-linux-gnu`, and
[`../invariant/002`](../invariant/002_publication_ordering.md) records a
mutation run on it where weakening `PUBLISH` to `Relaxed` failed 14 of 60 runs.
The behaviour is observable here; what the constants buy is a check that fires
deterministically instead of one in four, and that keeps firing if the family
ever builds on a total-store-order host where the hardware would hide the
defect.

The two checks that do reach the behaviour — the loom model and that mutation
run — are both outside the default test path
(→ [`../invariant/002`](../invariant/002_publication_ordering.md), MP24).

### Removal Trigger

A `Sync` that the compiler can conclude, which needs the slot array's exclusivity
to be expressed in a type rather than in cursor arithmetic. That is the same
condition as [`001`](001_the_unsafe_code_opt_out_and_its_obligations.md)'s and
it retires both together.

### MP52 — Changing `ends` to `&self` Would Compile and Break Soundness

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep -E '^  pub fn ends|^  pub fn split' src/lib.rs
```

Live output:

```
  pub fn ends( &mut self ) -> Ends< '_, S >
  pub fn split( &'a mut self ) -> ( Producer< 'a, S >, Consumer< 'a, S > )
```

Both take `&mut`. The `unsafe impl Sync` argues from that fact — at most one
`Consumer` can exist — and nothing connects the two textually except the SAFETY
comment.

**A reviewer relaxing `ends` to `&self` sees a signature change and no error.**
The suite would still pass: no existing test constructs two consumers, because
until that change no test could. This is the sharpest edge in the crate and it
is guarded by a comment.

These guards live in the module doc comment rather than on an item, so rustdoc
has no name to give them and falls back to the line each fence opens on — an
absolute source address that moves under any edit above it, and one rustdoc
prints in a different order every run, since the `compile_fail` set is compiled
as its own batch. Neither the address nor the order is the claim. What is: that
there are five, that the fourth is this one, and that all five still refuse.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
awk '/compile_fail$/ { n += 1; sub( /^\/\/! /, "", start ); print n ". " start; next }
     /^\/\/!$/ { blank = 1; next }
     /^\/\/!/  { if ( blank ) { start = $0; blank = 0 } }' src/lib.rs
printf 'guards that refused to compile: %s\n' \
  "$( cargo test --doc --all-features -p ring_mpsc 2>&1 | command grep -c -- '- compile fail ... ok' )"
```

Live output:

```
1. A second consumer cannot be made by cloning the first:
2. Nor by sharing one with a second thread:
3. Nor by splitting twice while the first pair is live:
4. Nor by holding two [`Ends`] from one ring at once — `ends` takes `&mut
5. And a [`Reserved`] cannot outlive the batch it would publish into, because
guards that refused to compile: 5
```

**Disposition:** applied — the module doc comment in `src/lib.rs` now carries a
fifth `compile_fail` regression guard, immediately after the existing
"splitting twice" one, that holds two `Ends` from one ring live at once and
names the exact fact this finding says only a comment protects: that `ends`
takes `&mut self`. Today the second `ring.ends()` call is refused by the
borrow checker while the first is still alive (guard 4 above), so relaxing
`ends` to `&self` — the exact change this finding warns a reviewer could make
"and see no error" — would make that borrow legal, the doctest would compile
where it is required to fail, and `cargo test --doc` would report it as a
failing test. The suite is no longer silent on this signature. Full crate
suite re-verified passing (`cargo test --all-features -p ring_mpsc`,
2026-09-04): 31 integration tests, 28 regular doctests, 5 `compile_fail`
doctests, 0 failures. Now prints: `guards that refused to compile: 5`
