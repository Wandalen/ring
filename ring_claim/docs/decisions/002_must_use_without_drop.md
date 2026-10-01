# Decision: `must_use` Without `Drop`

### Scope

- **Purpose**: Record the second refusal — that `Claim` warns about being dropped and does nothing when it is — and place it against the four `Drop` impls the family built to solve the same problem twice.
- **Responsibility**: Reproduce the argument for why release is impossible, census the family's messaged annotations and destructors, and state what the annotation can and cannot reach.
- **In Scope**: `#[ must_use = … ]` on `Claim`, the absent `Drop`, and the guard types elsewhere.
- **Out of Scope**: What happens when the warning is ignored — see [`pitfall/001`](../pitfall/001_dropping_a_claim.md).

### The Decision

From the module doc comment, § Why a claim is `#[must_use]` and carries no
destructor:

> A [`Claim`] is a promise the producer made to itself: these sequences are mine
> and I will publish them. Dropping one without publishing strands the range —
> the producer cursor has already advanced past it, so those slots are never
> written and never reclaimed, and every consumer stalls at the gap forever.
>
> There is deliberately no `Drop` impl that "releases" the claim. Releasing is
> not possible: another producer may already have claimed the range beyond it,
> so rewinding the cursor would hand out sequences twice, which is the one thing
> `docs/feature/172` forbids outright. The type is `#[must_use]` so the compiler
> objects to the common accident, and the invariant is stated here for the
> uncommon one.

The argument is airtight for *this* type and it is worth being explicit about
its scope: releasing is impossible because a `Claim` is only a range. A type
that also held the ring could publish empty records on drop instead of rewinding
— which is exactly what the family does elsewhere.

### CL21 — Ten of the Twelve Messaged Annotations Have Nothing Behind Them, and Two of Them Name the Same Permanent Failure

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -r '#\[ must_use = ' ring_*/src/*.rs
command grep -r '^impl.*Drop for' ring_*/src/*.rs
```

Live output:

```
ring_atomic/src/lib.rs:  #[ must_use = "the returned sequence is the claim — dropping it claims a range nobody will use" ]
ring_claim/src/lib.rs:#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
ring_flush/src/lib.rs:#[ must_use = "an ignored outcome is exactly how a misconfigured OnBarrier stays silent" ]
ring_shutdown/src/lib.rs:  #[ must_use = "a Stopped is the only route to a drain; bind it, or bind `_` to close and nothing else" ]
ring_shutdown/src/lib.rs:  #[ must_use = "this is the record itself, not a copy — dropping it loses it" ]
ring_shutdown/src/lib.rs:#[ must_use = "a Wake::Closed means stop, not publish" ]
ring_slot/src/lib.rs:  #[ must_use = "this is the only way a payload leaves the slot; dropping it here destroys the record" ]
ring_spsc/src/lib.rs:#[ must_use = "a reservation publishes on drop; dropping it immediately publishes an unwritten slot" ]
ring_spsc/src/lib.rs:#[ must_use = "a batch commits on drop; dropping it immediately discards the records it covers" ]
ring_testkit/src/lib.rs:#[ must_use = "nothing frees this — dropping the reference leaks the ring with no way to reach it again" ]
ring_testkit/src/lib.rs:#[ must_use = "nothing frees either allocation — dropping the pair leaks both with no way to reach them again" ]
ring_testkit/src/lib.rs:  #[ must_use = "the Outcome is the measurement — `script.run( &mut ring );` as a statement drives the ring and discards everything it observed" ]
ring_mpsc/src/lib.rs:impl< S > Drop for Reserved< '_, S >
ring_mpsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Reservation< '_, S >
ring_spsc/src/lib.rs:impl< S > Drop for Batch< '_, S >
```

Two hundred and eighty-two `#[ must_use ]` attributes across the family; twelve
carry a message, and two of those twelve sit on a type with a `Drop` impl:

| Site | What dropping costs | Has a `Drop` that acts |
|------|---------------------|:----------------------:|
| `ring_spsc::Reservation` | publishes one unwritten slot | **✔** |
| `ring_spsc::Batch` | discards the records it covers | **✔** |
| **`ring_claim::Claim`** | **strands its slots; every consumer stalls at the gap** | **—** |
| `ring_atomic`'s `fetch_add` | **claims a range nobody will use** | — (a trait method; the `Seq` it returns has no destructor either) |
| `ring_shutdown`'s `close` | no drain ever happens | — |
| `ring_shutdown`'s `into_record` | loses one record | — |
| `ring_shutdown::Wake` | a close is read as a publish | — |
| `ring_slot`'s `take` | destroys one record | — |
| `ring_flush::FlushOutcome` | a misconfigured `OnBarrier` stays silent | — (a report, not a resource) |
| `ring_testkit`'s `leak` | leaks the ring | — (test helper) |
| `ring_testkit`'s `leak_ends` | leaks both allocations | — (test helper) |
| `ring_testkit`'s `Script::run` | discards the measurement | — (test helper) |

So `Claim` is not unusual in being unbacked — ten of the twelve are. What sets
it apart is the *shape* of the loss. Eight of those ten cost something bounded:
one record, one report, one signal, one test allocation. Two do not, and both
name the same failure — a range handed out and never published, which strands
those slots and stalls every consumer at the gap forever. They are `Claim` and
`ring_atomic`'s `fetch_add`, and neither can be given a destructor: this one
owns no ring, and that one is a trait method returning a `Seq` the whole family
shares.

Against the two that *are* backed, the difference is stark:

| | `ring_spsc::Reservation` | `ring_claim::Claim` |
|--|--------------------------|---------------------|
| Message says | dropping publishes an unwritten slot | dropping strands slots and stalls consumers |
| So dropping is | defined, observable, one empty record | undefined-in-time, silent, permanent |
| Recoverable | yes — the ring keeps moving | **no** — every consumer stops at the gap |
| Enforced by | `Drop` | nothing |

`Claim`'s message names the most severe consequence in the family — jointly with
`ring_atomic`'s, which names the same one — and neither has a runtime mechanism
behind it. That is not an inconsistency to fix — it is the honest shape of a
type that owns no ring — but it does mean the message is doing all the work, and
a message is only read when the warning fires. A `let _ = claimer.claim( 4 );`
silences it.

### CL22 — The Family Built the Guard Three Times, in the Two Crates That Have a Ring

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
command grep -r -A3 '^impl.*Drop for' ring_*/src/*.rs
```

Live output:

```
ring_mpsc/src/lib.rs:impl<S> Drop for Reserved<'_, S> {
ring_mpsc/src/lib.rs-  /// Publish, with the one `Release` store the whole protocol turns on.
ring_mpsc/src/lib.rs-  fn drop(&mut self) {
ring_mpsc/src/lib.rs-    self.ring.stamp(self.seq).store(self.seq, PUBLISH);
--
ring_mpsc/src/lib.rs:impl<S> Drop for ReservedBatch<'_, S> {
ring_mpsc/src/lib.rs-  /// Publish the whole grant, one `Release` store per sequence, in issue
ring_mpsc/src/lib.rs-  /// order. Every sequence gets its stamp whether it was written or not — a
ring_mpsc/src/lib.rs-  /// range that stopped publishing at its first unwritten sequence would
--
ring_mpsc/src/lib.rs:impl<S> Drop for Batch<'_, S> {
ring_mpsc/src/lib.rs-  /// Commit, releasing the slots for reuse.
ring_mpsc/src/lib.rs-  ///
ring_mpsc/src/lib.rs-  /// The `Release` here pairs with the [`ring_cursor::GATING`] load inside
--
ring_spsc/src/lib.rs:impl<S> Drop for Reservation<'_, S> {
ring_spsc/src/lib.rs-  /// Publish, with the one release store that is the producer path's entire
ring_spsc/src/lib.rs-  /// synchronization.
ring_spsc/src/lib.rs-  fn drop(&mut self) {
--
ring_spsc/src/lib.rs:impl<S> Drop for Batch<'_, S> {
ring_spsc/src/lib.rs-  /// Commit, with the one release store that is the consumer path's entire
ring_spsc/src/lib.rs-  /// synchronization — one store for the whole batch, which is why the surface
```

Five `Drop` impls exist in the 33 crates. All five are in `ring_mpsc` and
`ring_spsc`, and all five are on borrow-carrying guard types:

| Crate | Type | What dropping does |
|-------|------|--------------------|
| `ring_mpsc` | `Reserved<'_, S>` | publishes — "the one `Release` store the whole protocol turns on" |
| `ring_mpsc` | `ReservedBatch<'_, S>` | publishes the whole grant — one `Release` store per claimed sequence, in issue order |
| `ring_mpsc` | `Batch<'_, S>` | commits, releasing the slots for reuse |
| `ring_spsc` | `Reservation<'_, S>` | publishes |
| `ring_spsc` | `Batch<'_, S>` | commits |

Zero of them are in a Tier 5 primitive. The doc comment on
`ring_spsc::Producer::claim` argues for the shape explicitly, and names the
alternative this crate *is*:

> The guard shape rather than a bare `claim`/`publish` pair, because an early
> return between the two wedges the ring permanently and no runtime check can
> distinguish "claimed and about to publish" from "claimed and abandoned". …
> Making the publish the drop makes the case unreachable, including on unwind.

So the family's considered position is that the guard shape is better, stated in
a crate that adopted it — and this crate cannot adopt it, because a guard must
publish on drop and publishing requires a ring. The doc comment on
`ring_mpsc`'s `Reserved` records the consequence of getting it right:

> **A guard dropped without a write publishes an empty slot, not a torn one.**
> The slot was left `Default` by the consumer that drained it, so a panic
> between claim and write costs one empty record — an observable, defined
> [outcome]

One empty record against a permanent stall. That is the whole difference between
a claim held by a type that can reach the ring and a claim held by two integers.

### The Unwind Case

The comparison sharpens on panics, which is where a `must_use` offers nothing at
all:

| Event | `ring_spsc::Reservation` | `ring_claim::Claim` |
|-------|--------------------------|---------------------|
| producer panics after claim, before write | `Drop` publishes an empty slot | range stranded, consumers stall |
| producer returns early | compiler warning, then `Drop` | compiler warning only |
| producer does `let _ = …` | `Drop` still runs | **nothing happens, ever** |

The third row is the one worth naming: `#[ must_use ]` is a lint, and the
idiomatic way to silence a lint also destroys the value. `Drop` is not
silenceable. So the guard shape is not merely a nicer ergonomic — it is a
different enforcement class, and this crate is in the weaker one by necessity.

### What `§ C4` Checks

`tests/manual/readme.md § C4` is the check that keeps both halves in place:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -E "must_use = "
grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -E "impl.*Drop"
```

Live output:

```
#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
```

**Expected:** one messaged `must_use` from the first, and **no output at all**
from the second. The plan states why the second matters, and it is the sentence
that most needs preserving:

> a `Drop` impl here would look like careful resource handling and would be a
> correctness bug.

That is the failure mode this check exists for. Someone reading `ring_spsc`,
noticing the guard, and adding a "matching" `Drop` to `Claim` would be writing
plausible, symmetric, well-intentioned code that hands out sequences twice.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seventeen_items_and_nothing_that_drops_silently.md](../api/001_seventeen_items_and_nothing_that_drops_silently.md) | The annotation the constructor's bare-attribute omission protects |

### Decisions

| File | Relationship |
|------|--------------|
| [001_compare_exchange_rather_than_fetch_add.md](001_compare_exchange_rather_than_fetch_add.md) | The other refusal, for the same underlying reason — the cursor cannot go backwards |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_a_range_from_grant_to_publication.md](../lifecycle/001_a_range_from_grant_to_publication.md) | The window the annotation guards, and its two exits |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_dropping_a_claim.md](../pitfall/001_dropping_a_claim.md) | The four ways to reach the stranded state anyway |

### Sources

| File | Relationship |
|------|--------------|
| `ring_claim` module doc, § Why a claim is `#[must_use]` and carries no destructor | The decision, in full |
| `ring_claim`'s `Claim` | The messaged annotation, on the type |
| `ring_spsc`'s `Producer::claim` | The family's argument *for* the guard shape, naming this crate's shape |
| `ring_spsc`'s `Reservation` and `Batch` | The only two messaged annotations with a destructor behind them |
| `ring_mpsc`'s `Reserved` and `Batch` | The defined cost of a dropped guard, and two more destructors |
| `ring_flush`'s `FlushOutcome` | A messaged annotation on a report rather than a resource |

### Tests

| File | Relationship |
|------|--------------|
| `tests/manual/readme.md § C4` | One messaged `must_use`, and no `Drop` — with the reason a `Drop` would be a bug |
