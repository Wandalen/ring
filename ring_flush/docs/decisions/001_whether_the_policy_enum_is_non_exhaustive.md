# Decision: Whether `FlushPolicy` Is `#[non_exhaustive]`

**Status:** open. Rules Pending 3, and rules it only as far as one crate can:
the question is family-grain — `OverflowPolicy` in `ring_types` faces the same
question at five times the price (FL14) — so what is settled here is the
*evidence*, not the answer. Owned by whoever rules the family's enum-evolution
convention.

### Scope

- **Purpose**: Record what marking this crate's exported enums `#[non_exhaustive]` would cost today, measured rather than argued, so the family-grain ruling has a number to work from instead of a preference.
- **Responsibility**: The measurement of the current consumer surface, the two failure modes the choice trades between, and why this crate declines to rule unilaterally.
- **In Scope**: `FlushPolicy`, `FlushCause`, `FlushOutcome` and `ConfigError` — the four enums this crate exports; the family's existing precedent; the one external consumer.
- **Out of Scope**: Whether a fourth variant should ever be added (nothing proposes one); the `serde` question, which is not local (→ [`readme.md`](readme.md)'s closing section); `drain_final`'s signature (→ [`002`](002_the_final_drains_signature.md)).

### The Question, Restated

Adding a variant to a public enum is a breaking change for every external
`match` that was exhaustive. `#[non_exhaustive]` makes such additions
non-breaking, at the price of forcing **every** consumer to write a wildcard arm
today — including the consumers who wanted the compiler to tell them when a
variant appeared.

The trade is not between safety and convenience. It is between **a break that is
loud** and **a break that is silent**:

| Choice | When a variant is added | What a consumer experiences |
|--------|------------------------|----------------------------|
| Exhaustive (today) | Compile error at every external `match` | Loud. Every site is visited. Nobody can miss it |
| `#[non_exhaustive]` | Nothing | Silent. The wildcard arm absorbs it, and whatever that arm does is now the new variant's behaviour |

For a *policy* enum the silent case is worse than usual, because the wildcard
arm's behaviour is a policy nobody chose — the same failure this crate exists to
prevent, arriving one level up.

### What This Crate Actually Contributes

Four of the family's eight exported enums are declared here, which makes this
crate the largest single stakeholder in a convention it cannot set. It
contributes three things and no ruling: the count, the observation that the
family has already answered the question twice in opposite directions without
recording either as a decision (FL13), and the price list (FL14).

### Why It Is Filed Rather Than Decided

Ruling it here would set a convention for `ring_types`' three enums by
precedent, from a crate that does not own them — and FL14 shows those three are
where the whole cost sits. Marking this crate's four is free; marking
`OverflowPolicy` forces a wildcard into four functions across two crates that
did not declare it.

**A single family-wide ruling is therefore not a single price**, which is the
one thing this decision can say that the crate ruling alone could not. The
reverse order — `ring_types` ruling on its own numbers and this crate following
— produces one convention instead of two and charges the decision to the crate
that actually pays for it.

**What is not deferred is the timing.** The measurement below says this crate's
share of the cost is currently zero, and it rises monotonically with every
consumer the family acquires. Recording that, and the fact that the sibling's
share is already nonzero, is this crate's whole contribution.

### Related

- [`integration/002`](../integration/002_a_decision_on_the_export_surface.md) —
  X1, which states the obligation this decision is about
- [`type/001`](../type/001_flush_policy.md) — the trait table, and the `Default`
  that was withheld for a neighbouring reason
- [`item/001`](../item/001_seven_nouns_thirteen_variants.md) — the seven nouns,
  four of which are the enums at issue

### FL13 — The Family Has Ruled This Twice, in Opposite Directions, and Neither Ruling Is in a Decision Record

Twenty-three public enums exist across the family, eight of them on the Contract.
One carries the attribute, and exactly one other has a written reason for not
carrying it:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- public enums on the five-crate Contract --'
for c in ring_factory ring_handle ring_tls ring_flush ring_types
do
  printf '    %-14s %s\n' "$c" \
    "$( command grep -hoE '^pub enum [A-Za-z]+' $c/src/*.rs | sed 's/pub enum //' | tr '\n' ' ' )"
done
printf '    %-14s %s\n' 'family total' \
  "$( command grep -rhcE '^pub enum ' ring_*/src/*.rs | paste -sd+ | bc )"
echo '  -- every mention of the attribute anywhere in the family --'
command grep -r 'non_exhaustive' ring_*/src ring_*/tests 2>/dev/null \
  | command grep -v 'finish_non_exhaustive' | command grep -vE ': *(//|#)' \
  | sed 's|ring/||; s|:|:  |'
```

Live output:

```
  -- public enums on the five-crate Contract --
    ring_factory   BuildError 
    ring_handle    
    ring_tls       
    ring_flush     FlushPolicy FlushCause FlushOutcome ConfigError 
    ring_types     RingError WaitKind OverflowPolicy 
    family total   23
  -- every mention of the attribute anywhere in the family --
ring_core/tests/manual/readme.md:  the roster is maintained by hand and — the enum being `#[ non_exhaustive ]` —
ring_overflow/tests/manual/readme.md:  A `#[non_exhaustive]` enum forces downstream wildcards, which would defeat M2
ring_overflow/tests/manual/readme.md:  **Expected:** no `#[ non_exhaustive ]`. (Contrast `RingError` in `ring_types`,
ring_overflow/tests/manual/readme.md:  | 2026-08-28 | M3 | ✅ | Derives are `Debug, Clone, Copy, PartialEq, Eq, Hash`; no `#[ non_exhaustive ]`, so downstream matches stay exhaustive too. |
```

Eight enums on the Contract, twenty-two in the family, one attribute — and the
one that has it is `ring_types::RingError`, the shared error vocabulary, the type
most likely to grow a variant and the one whose consumers least want a compile
error when it does.

**Four of the eight are declared here.** This crate is half the Contract's enum
surface and none of its four carries the attribute, which is a decision by
default rather than by argument: nobody wrote it down, so the exhaustive
behaviour is what shipped.

**The second ruling is the one worth reading.** `ring_overflow`'s manual test
plan has a whole stage, M3, whose subject is that `Resolution` is *not*
`#[non_exhaustive]` — with the rationale stated ("new error kinds are expected;
new overflow outcomes are not") and `RingError` named as the deliberate contrast.
So the family has answered this question twice, coherently, in opposite
directions, for good reasons.

Neither answer is in a decision record. One is an attribute with its reasoning
nowhere; the other is reasoning that lives in a *manual test stage of a crate
that is not on the Contract* — the one place a reader looking for the family's
enum-evolution convention would not think to look. The remaining twenty enums,
including all four declared here, have neither the attribute nor a sentence.

### FL14 — This Crate's Share of the Cost Is Zero and Its Sibling's Is Five

The measurement that was supposed to say the ruling is free says something more
useful:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- crates naming FlushPolicy at all --'
command grep -rl 'FlushPolicy' ring_*/src ring_*/tests 2>/dev/null \
  | sed 's|^ring/||; s|/.*||' | sort -u
echo '  -- how the one outside consumer uses it --'
command grep 'FlushPolicy' ring_bench/src/lib.rs ring_bench/tests/bench_test.rs
echo '  -- and every match on it, anywhere --'
command grep -r 'match .*[Pp]olicy' ring_*/src ring_*/tests 2>/dev/null | sed 's|ring/||'
```

Live output:

```
  -- crates naming FlushPolicy at all --
ring_bench
ring_flush
  -- how the one outside consumer uses it --
ring_bench/src/lib.rs:use ring_flush::{ ConfigError, FlushOutcome, FlushPolicy, Flusher };
ring_bench/src/lib.rs:  /// and the staged candidate binds it as its [`FlushPolicy::OnBatch`]
ring_bench/src/lib.rs:  let mut flusher = Flusher::new( buffer, producer, FlushPolicy::OnBatch( workload.batch() ) )
ring_bench/tests/bench_test.rs:/// `FlushPolicy::OnBatch( workload.batch() )`, so `ring_flush`'s two binding
  -- and every match on it, anywhere --
ring_flush/src/lib.rs:    match self.policy
ring_overflow/src/lib.rs:  match policy
ring_overflow/src/lib.rs:  match policy
ring_stats/src/lib.rs:    let counter = match policy
ring_stats/src/lib.rs:    let counter = match policy
ring_factory/tests/factory_test.rs:    match Factory.build::< u32 >( cfg.with_overflow( policy ) )
ring_types/tests/types_test.rs:    match policy
```

Two crates name it, and one of them is this one. `ring_bench` mentions it four
times across two files — an import, two doc references, and exactly one
construction of `FlushPolicy::OnBatch`. It never matches on it, and the sole
`match` on a `FlushPolicy` value in the repository is this crate's own private
`trigger` at line 571. Marking it costs nothing anywhere.

**The third block is the one that changes this decision's shape.** Every other
policy match in the family is on `OverflowPolicy` — twice in `ring_overflow`'s
`resolve` and `would_resolve`, twice in `ring_stats`' `record_drop` and
`dropped`, once in `ring_types`' own test. (`ring_factory`'s line matches a
`Result`, not the policy.) Four of the five are in crates that do not declare
the enum, and two of them are exhaustive matches inside published `pub fn`
bodies.

So the premise this decision was filed on — that the two policy enums face the
question identically, so the family should rule once — is **half wrong in the
direction that matters**. They face the same question at five times the price:
marking `FlushPolicy` breaks nothing, marking `OverflowPolicy` forces a wildcard
into four functions across two crates. A single family-wide ruling therefore
cannot be free; whoever rules it is choosing on `OverflowPolicy`'s numbers, and
this crate's zero is not evidence about the family, only about this crate.

**`#[non_exhaustive]` on an enum restricts matching, not construction**, so
adding the attribute today breaks nothing anywhere in the repository. That is
not a permanent property. It holds because the family has exactly one external
consumer and that consumer happens to configure rather than dispatch — and the
first consumer that writes `match policy { … }` converts a free decision into a
breaking one.

The instance this decision was raised from called it "an argument for ruling
once at family grain rather than twice locally." That is still right. What is
added here is that the argument has an expiry: the cheapest moment to rule is
now, and it gets more expensive monotonically and without warning.

**Disposition:** declined — the finding's own measurement is the deliverable,
not a trigger to act on it locally: this document's own "Why It Is Filed
Rather Than Decided" section already argues that marking `FlushPolicy`
`#[non_exhaustive]` here, unilaterally, would set a convention for
`ring_types`'s `OverflowPolicy` by precedent from a crate that pays five times
less for it and does not own the sibling enum. Applying the attribute to
`FlushPolicy` alone would be exactly the premature single-crate ruling the
decision document exists to head off; the family-grain ruling it is filed for
is out of a docs-corpus disposition pass's authority, same as `FL13`'s twin
question one section above.
