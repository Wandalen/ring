# Item: The Two That Nothing Calls

### Scope

**Purpose:** Read `aliases` and `run` as items — the one whose doc comment
justifies its own uselessness correctly and resolves a two-hop cross-crate
reference, and the one whose doc comment cites the feature implemented by the
function that replaced it.

**Responsibility:** `ring_index/src/lib.rs:54-122` as two units, read for
what each says about why it exists.

**In Scope:** `aliases`' and `run`'s doc comments, doctests, and bodies;
`ring_cursor/src/lib.rs:417-419`.

**Out of Scope:** the reach census is
[`api/001`](../api/001_three_functions_three_must_use_one_reached.md) IX6. `run`'s
return type is
[`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md).

---

## Both Items

```sh
cd "$(git rev-parse --show-toplevel)"
sed -n '/^\/\/\/ Whether two sequences address the same slot — true exactly when they are a$/,/^\/\/\/ can be tested against it; it does not prevent anything itself\.$/p;/^\/\/\/ The slots a contiguous run of `count` sequences starting at `start`$/,/^\/\/\/ lands, per `docs\/feature\/177_batch_claim_and_batch_drain\.md`\.$/p' ring_index/src/lib.rs
```

Live output:

```
/// Whether two sequences address the same slot — true exactly when they are a
/// whole number of laps apart.
///
/// Aliasing is the ring's defining behaviour rather than a defect: a fixed
/// number of slots reused forever is what the structure *is*. What keeps it
/// from being observable is the gate — `ring_seqno`'s `may_claim`, and the
/// barrier built on it — which refuses a claim that would land on a slot a
/// consumer has not yet passed. This function reports the aliasing so a gate
/// can be tested against it; it does not prevent anything itself.
/// The slots a contiguous run of `count` sequences starting at `start`
/// addresses, in order.
///
/// A batch claim is contiguous in sequence space, so its slots wrap at most
/// once — which is what preserves a staged buffer's relative order when it
/// lands, per `docs/feature/177_batch_claim_and_batch_drain.md`.
```

Neither function has a caller in any of the 33 crates. Their doc comments handle
that fact very differently.

---

### IX35 — `aliases` Documents Why It Has No Callers, and the Reason Holds

**Finding.** "This function reports the aliasing so a gate can be tested against
it; it does not prevent anything itself" is an unusually honest sentence for a
public function. It says: this is a testing affordance, not machinery. A
zero-caller count is the *expected* outcome for such a function, not a symptom.

The comment then does something a doc comment rarely does successfully — it
names the real gate, in another crate, by function:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'pub fn may_claim' ring_seqno/src/lib.rs
command grep -m1 -A3 -F '  pub fn may_claim( &self ) -> bool' ring_cursor/src/lib.rs
echo '  -- ring_barrier'\''s dependencies --'
sed -n '/\[dependencies\]/,/^\[lints\]/p' ring_barrier/Cargo.toml | command grep -o 'ring_[a-z]*' | sort -u | tr '\n' ' '
echo
```

Live output:

```
pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
  pub fn may_claim( &self ) -> bool
  {
    ring_seqno::may_claim( self.producer.load( GATING ), self.consumer.load( GATING ), self.capacity )
  }
  -- ring_barrier's dependencies --
ring_cursor ring_gating ring_types ring_wait 
```

The reference resolves, through a hop the comment does not mention.
`ring_seqno::may_claim` exists as a free function; every caller in the family
reaches it through `ring_cursor::CursorPair::may_claim`, which is a thin
delegation; and `ring_barrier` depends on `ring_cursor`, so "the barrier built
on it" is accurate transitively even though `ring_barrier` never names
`ring_seqno`.

That is worth recording as a correct cross-reference rather than a defect,
because most of this corpus's findings about cross-crate prose are the opposite
— [`integration/001`](../integration/001_two_dependents_and_a_third_that_did_it_again.md)
IX11's claim-path error, [`pitfall/001`](../pitfall/001_the_run_that_panics_in_debug_and_wraps_in_release.md)
IX17's saturation claim. `aliases`' comment is the one in this crate that names
another crate's internals and is still right two hops out.

What it does not say is that the zero callers include zero *tests*. The stated
purpose is "so a gate can be tested against it," and no gate test anywhere uses
it — `ring_seqno`, `ring_gating`, `ring_cursor` and `ring_barrier` all test their
gates without reference to `aliases`. The justification is coherent and
unexercised.

---

### IX36 — `run`'s Comment Cites the Feature Implemented by the Function That Replaced It

**Finding.** `run`'s justification is that a batch claim is contiguous, so its
slots wrap at most once, "per `docs/feature/177_batch_claim_and_batch_drain.md`."

This family's own batch-claim contract is implemented by `ring_batch`. `ring_batch` does not call `run`. It
imports `of` and builds `drain_order`, which is `run` as a lazy iterator
([`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md)).

So the citation is the strongest possible evidence against the function that
carries it: `run` names a feature as its reason to exist, and the crate that
implements that feature looked at `run`, did not use it, and wrote the version
it needed. The comment is not wrong about the feature — batch claims really are
contiguous — it is wrong about being the code that serves it.

Two smaller things compound it. The "wraps at most once" clause states a property
of batch claims as though it were a property of `run`, which accepts any `count`
at all ([`algorithm/002`](../algorithm/002_a_run_is_the_fold_applied_count_times.md)
IX3). And the doctest demonstrates precisely the case the clause describes —
`run( Seq( 2 ), 4, cap 4 )` wrapping once — while the crate's own
`an_oversized_run_repeats_slots` test asserts the case it does not.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A9 -F '/// lands, per `docs/feature/177_batch_claim_and_batch_drain.md`.' ring_index/src/lib.rs | tail -n 8
```

Live output:

```
/// That guarantee belongs to the caller's batch claim, not to this function:
/// `run` neither clamps nor deduplicates, so a `count` larger than `capacity`
/// returns repeated slots rather than stopping at one wrap.
///
/// `run` also allocates once per call — exactly sized with no growth slack,
/// but an allocation regardless. That disqualifies it from the family's
/// lock-free claim and consume paths at any speed; `of` and `aliases`
/// measure zero allocations over 10,000 calls each and are what those paths
```

Two assertions, the fewest of the three functions. Both are inside the region the
comment describes, so the doctest cannot contradict the comment and does not.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`item/001`](001_the_fold_itself.md) | The one function with callers, whose comment states the aliasing property narrowly |
| [`api/001`](../api/001_three_functions_three_must_use_one_reached.md) | The zero-caller census both items sit on |
| [`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md) | What `ring_batch` built rather than call `run` |
| [`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md) | `run`'s return type, the reason `drain_order` exists |

### Sources

| Fact | Where |
|------|-------|
| Both doc comments | `ring_index/src/lib.rs:54-62, 79-84` |
| `run`'s doctest | `ring_index/src/lib.rs:110-117` |
| `ring_seqno::may_claim` and its wrapper | `ring_seqno/src/lib.rs:73`; `ring_cursor/src/lib.rs:417-419` |
| `ring_barrier` reaching it transitively | Manifest census above |

### Tests

| Test | Covers |
|------|--------|
| `aliasing_is_exactly_whole_laps`, `a_sequence_aliases_itself` | `aliases`, from inside this crate only |
| `a_run_wraps_at_most_once_within_one_capacity` | The case `run`'s comment describes |
| `an_oversized_run_repeats_slots` | The case it does not |
| *(to create)* | A gate test that actually uses `aliases`, which is the purpose its comment claims |
