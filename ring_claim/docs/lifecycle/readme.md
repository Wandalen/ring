# lifecycle

Two lives, and they are almost opposites. A *range* has six states, five
transitions, a blind window in the middle, and exactly one way to leave that
window correctly. A *`Claimer`* has a counter that goes up and a borrow that
bounds it, and that is the whole of it — no reset, no destructor, no state
machine worth drawing.

The interesting material is at the seams in both cases. The range's window is
opened by this crate and closed by `ring_publish`, so what happens inside it is
governed by an assumption neither crate states in the other's terms. The
`Claimer`'s monotonicity is guaranteed by its methods and then handed away by
its one accessor, so the guarantee turns out to be a convention above the public
API rather than an encapsulated invariant.

Both files end up documenting the same kind of thing: a property this crate is
responsible for, enforced somewhere weaker than where it is relied upon.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [A Range from Grant to Publication](001_a_range_from_grant_to_publication.md) | CL31, CL32 — the six states and their owners, the sibling crate's budget-free spin and what keeps it terminating, and why width does not scale the damage |
| 002 | [The Claimer Over a Ring's Life](002_the_claimer_over_a_rings_life.md) | CL33, CL34 — the accessor that exports `store` and `fetch_add`, reproduced in eight safe lines, and the overflow comment that named the wrong failure mode until it was corrected |

### Two Lifecycles, Side by Side

| | A range | A `Claimer` |
|--|---------|-------------|
| States | 6 | 1 |
| Transitions that move a cursor | 3 of 5 | 1 |
| Crates involved | 4 | 1 |
| Reversible transitions | **0** | 0 |
| Has a destructor | no — and [`decisions/002`](../decisions/002_must_use_without_drop.md) says why | no |
| Bounded by the type system | no | **yes** — the `'a` borrow |
| Ends by | publication, or stranding | going out of scope |

The one row where the type system helps is the `Claimer`'s borrow. Everything
else in both columns is upheld by a doc comment, a lint, or a convention.

### The Two Enforcement Gaps, Stated Together

Each file finds one, and they are the same shape at different scales:

| | Relied on by | Enforced by | Gap |
|--|--------------|-------------|-----|
| "a claimed range will be published" | `ring_publish::publish`'s unbounded `loop` — whose `# Panics` documents deadlock, not a panic | `#[ must_use ]` on `Claim` | `let _ = …` silences the lint; the loop then never exits |
| "the claim cursor only moves forward" | every gate comparison in the family | `Claimer`'s methods | `cursor()` hands out a `&PaddedCursor`, and `SeqCell` gives any holder `store` and `fetch_add` |

Neither is a live defect and neither is reachable by accident. Both are cases
where the *strength* of the guarantee downstream is not matched by the strength
of the mechanism upstream, and in both cases the downstream crate's
documentation asserts the guarantee flatly — "Never", "the wrap point is
unreachable" — without naming what it rests on.

The second gap is demonstrable in eight lines of safe code, and the
demonstration ends with `Claim::overlaps` returning `true` for two claims from
one `Claimer` — the exact property multi-producer exclusivity forbids, reported
by the exact method the crate exposes so a test can check it.

### What This Definition Deliberately Does Not Claim

That either gap should be closed. The obvious fixes both cost more than they
buy:

| Gap | Obvious fix | Why it is not obviously right |
|-----|-------------|-------------------------------|
| dropped claim | a `Drop` that releases | rewinding hands out sequences twice — the thing this crate's module documentation forbids outright |
| dropped claim | a budget on `publish`'s spin | `ring_publish:42-53` — a budget whose exhaustion has no correct handling |
| writable cursor | remove `cursor()` | the cache-line check and two tests need the cell's identity |
| writable cursor | narrow it to `cursor_addr() -> usize` | plausible, and the only one of the four with no stated counter-argument |
| overflow comment | checked arithmetic | buys nothing at 584 years and costs an instruction on the hot path — but that reasoning is `next`'s, and this crate's actual call is `advanced_by`, where `n` comes from the caller and the 584 years buy nothing at all. The counter-argument holds for one of the two methods and was written as though it held for both |

Four of the five have a recorded reason not to, and one of those four turns out
to be recorded against the wrong method: the 584-year argument answers `next`,
while this crate calls `advanced_by`, whose `n` is a caller's batch length. The
fifth — narrowing the accessor — is the one suggestion that was open from the
start, and even it is churn against a hole nobody has fallen into.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# the six states' owners: who moves which cursor
command grep -r 'compare_exchange\|\.store(' ring_claim/src/lib.rs \
  ring_publish/src/lib.rs | grep -v '///'

# the spin loop and the argument that it terminates
sed -n '/^\/\/! ## Why a plain spin, and not a `WaitKind`$/,/^\/\/! a budget whose exhaustion has no correct handling\.$/p;/^  \/\/\/ # Panics$/,/^  }$/p' ring_publish/src/lib.rs

# what a &PaddedCursor can do
grep -A40 'pub trait SeqCell' ring_atomic/src/lib.rs | grep -E 'fn [a-z_]+\('

# the 584-year argument, and the sentence after it. the second range was
# anchored on "saturates in a release build" and both its endpoints were
# rewritten out of the source, so it silently matched nothing — a dead range
# contributes no lines and no error, and the quoted output below regenerates
# clean around the hole
sed -n '/^\/\/\/ Monotonic and, for every reachable workload, non-wrapping: at 10⁹$/,/^\/\/\/ wrap-around logic anywhere, because the wrap point is unreachable\.$/p;/^  \/\/\/ Panics on overflow in a debug build and wraps to zero in a release$/,/^  \/\/\/ .u64. runs for roughly 584 years, well past any reachable workload\.$/p' ring_types/src/id.rs

# this crate has no destructor and no reset — reported through `printf`, because
# `grep -c` exits 1 on a zero count and zero is the expected answer
printf '  Drop impls, resets and rewinds in ring_claim: %s\n' \
  "$( grep -vE "^[[:space:]]*//" ring_claim/src/lib.rs | grep -cE 'impl.*Drop|fn reset|fn rewind' )"
```

Live output:

```
ring_claim/src/lib.rs:      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
ring_claim/src/lib.rs:      match self.cursor.compare_exchange( current, next, CLAIM_SUCCESS, GATING )
ring_publish/src/lib.rs:    self.cursor.compare_exchange( start, end, PUBLISH, GATING ).map( | _ | end )
//! ## Why a plain spin, and not a `WaitKind`
//!
//! [`Publisher::publish`] loops on a `spin_loop` hint with no wait strategy and
//! no budget, which everywhere else in this family would be a bug. Here it is
//! the correct shape, and the difference is what is being waited *for*.
//!
//! Waiting for space is unbounded: it depends on a consumer that may be slow,
//! stalled, or gone, so it needs a strategy and a give-up. Waiting for your
//! predecessor to publish is bounded by that producer finishing a slot write it
//! has already started and cannot abandon — it is not blocked on anything
//! itself. A `WaitKind` here would offer a `Park` that can only ever hurt, and
//! a budget whose exhaustion has no correct handling.
  /// # Panics
  ///
  /// Never. It deadlocks instead, and there are two ways in.
  ///
  /// A caller that publishes a range it never claimed waits for a turn that
  /// cannot arrive. That is a caller bug this crate cannot detect, and
  /// `try_publish` is the variant for a caller that wants to decide for itself.
  ///
  /// A caller whose *predecessor* dropped its claim without publishing waits
  /// just as long, and that one is not the waiting caller's bug at all. The
  /// module documentation's termination argument — a predecessor "cannot
  /// abandon" a slot write it has already started — describes correct
  /// producers rather than a property the types enforce:
  /// `ring_claim::Claim` has no destructor, so an abandoned claim is a
  /// `#[ must_use ]` warning and nothing more, and `let _ = …` silences even
  /// that. Of the two deadlocks this is the reachable one, and the only
  /// defence against it is that every producer publishes what it claims.
  pub fn publish( &self, start : Seq, len : usize ) -> Seq
  {
    loop
    {
      if let Ok( end ) = self.try_publish( start, len )
      {
        return end;
      }
      core::hint::spin_loop();
    }
  }
  fn load( &self, order : Ordering ) -> Seq;
  fn store( &self, value : Seq, order : Ordering );
  fn fetch_add( &self, n : u64, order : Ordering ) -> Seq;
/// Monotonic and, for every reachable workload, non-wrapping: at 10⁹
/// publications per second a `u64` runs for roughly 584 years. The family
/// depends on that — `Seq` comparison is plain `<`, with no lap-aware
/// wrap-around logic anywhere, because the wrap point is unreachable.
  /// Panics on overflow in a debug build and wraps to zero in a release
  /// build — the standard `u64` addition behaviour. A wrapped `Seq` would
  /// silently invert every gate comparison in the family, which is why the
  /// non-wrapping argument has to hold: at 10⁹ publications per second a
  /// `u64` runs for roughly 584 years, well past any reachable workload.
  Drop impls, resets and rewinds in ring_claim: 0
```

| | Value |
|--|------:|
| States a sequence passes through | 6 |
| …transitions that move no cursor | 3 |
| Crates owning a transition | 4 |
| Ways out of the claimed-but-unpublished window | **2** |
| …that anything observes | **1** |
| `Drop` impls in this crate | **0** |
| `reset` / `rewind` methods | **0** |
| `SeqCell` methods reachable from `cursor()` | **4** |
| …that break monotonicity | **2** |
| …requiring `unsafe` | **0** |
| Budget on `ring_publish::publish`'s spin | **none** |
| Documented panic modes of that function | "Never" |
| Years to exhaust a `u64` at 10⁹ claims/s | ~585 |
| Release-build behaviour at that point | **wraps to 0** (the doc said "saturates"; since corrected, and the recipe above prints the correction) |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL31 | `ring_publish` | **latent hazard** | `ring_publish::publish` is a budget-free, strategy-free `loop` whose termination argument is that the predecessor "is committed to" publishing; nothing enforces that, so the family's one deliberately unbounded spin is kept terminating by a `#[ must_use ]` lint two tiers down — and its `# Panics` section documented only the caller's own deadlock until the predecessor case was added beside it |
| CL32 | `ring_claim` | **latent hazard** | Because `try_publish` advances only when the published cursor is *exactly* at the claim's start, a dropped claim of width one stops the ring as completely as one of width a thousand: the loss is the ordering token, not the slots, and every later claim keeps succeeding for a while |
| CL33 | `ring_claim` | **latent hazard** | `Claimer::cursor()` returns `&PaddedCursor`, and `SeqCell` is a public trait whose four `&self` methods include `store` and `fetch_add`; the crate's founding invariant is therefore breakable from safe code in one line, and `tests/manual/readme.md § C2` cannot see it because it greps this crate's own source |
| CL34 | `ring_types` | **wrong doc** | `Seq::next`'s doc stated release-mode `u64` addition "saturates … deliberately not wrapping"; it wraps, measured, and the error ran in the unsafe direction — saturation would preserve monotonicity and stop the ring loudly, wrapping inverts every gate comparison. **Since corrected** in `ring_types/src/id.rs`; the recipe above prints the replacement paragraph, which now says "wraps to zero in a release build" and carries the 584-year argument as the reason that is survivable rather than as a claim about the arithmetic |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| That function's `# Panics` section named one deadlock — publishing an unclaimed range — and omitted the reachable one, a predecessor that dropped its claim, until CL20's sibling finding put both in it | [001](001_a_range_from_grant_to_publication.md) |
| The claimed-but-unpublished window has exactly two exits and only one of them is observable by anything in the family; no timeout, watchdog, or reconciliation pass exists anywhere | [001](001_a_range_from_grant_to_publication.md) |
| Reproduced in eight safe lines: a `fetch_add` through the accessor grants past a gate that had just returned `Full`, a `store` rewinds, and `Claim::overlaps` then returns `true` for two live claims from one `Claimer` | [002](002_the_claimer_over_a_rings_life.md) |
| `advanced_by` — the method this crate actually calls, from `Claim::end()` and every cursor advance — is the same `self.0 + n`, and carried no overflow note at all until one was written; the note now says the reachability argument does *not* carry over from `next`, because `n` comes from the caller and reaches the wrap in one call | [002](002_the_claimer_over_a_rings_life.md) |

