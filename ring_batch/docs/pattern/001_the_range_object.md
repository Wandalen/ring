# Pattern: The Range Object

### Scope

**Purpose:** Record the shape `BatchClaim` shares with three other types in the
family — a start, a length, and a method surface derived entirely from those two
numbers — and the one attribute that separates it from its nearest twin.

**Responsibility:** `BatchClaim` against `ring_claim::Claim`, `ring_mpsc::Batch`,
and `ring_spsc::Batch`: fields, method names, and derives.

**In Scope:** `ring_batch/src/lib.rs:55-60`; `ring_claim/src/lib.rs:94-100`;
`ring_mpsc/src/lib.rs:1143-1148`; `ring_spsc/src/lib.rs:962-967`.

**Out of Scope:** The destructors two of the four carry are
[`lifecycle/001`](../lifecycle/001_no_lifecycle_and_no_rollback.md) BA31. The
field-by-field comparison with `ring_claim::Claim` alone is
[`data_structure/002`](../data_structure/002_the_struct_ring_claim_wrote_again.md).

---

## Four Types, Two Shapes

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- Tier 2 --'
command grep -m1 -A4 -F 'pub struct BatchClaim' ring_batch/src/lib.rs
echo '  -- Tier 5 --'
command grep -m1 -B2 -A2 -F '  start : Seq,' ring_claim/src/lib.rs
echo '  -- Tier 6, twice, with the ring attached --'
command grep -m1 -A5 -F 'pub struct Batch< '"'"'a, S >' ring_mpsc/src/lib.rs
command grep -m1 -A5 -F 'pub struct Batch< '"'"'a, S >' ring_spsc/src/lib.rs
```

Live output:

```
  -- Tier 2 --
pub struct BatchClaim
{
  start : Seq,
  count : usize,
}
  -- Tier 5 --
pub struct Claim
{
  start : Seq,
  len : usize,
}
  -- Tier 6, twice, with the ring attached --
pub struct Batch< 'a, S >
{
  ring : &'a Ring< S >,
  start : Seq,
  len : usize,
}
pub struct Batch< 'a, S >
{
  ring : &'a Ring< S >,
  start : Seq,
  len : usize,
}
```

---

### BA38 — The Same Eight Methods, in Two Crates, With No Edge Between Them

The two-field pair is not merely similar in shape. `ring_claim::Claim` exposes
the identical method surface:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the same eight methods, two crates, no dependency between them --'
printf '    %-12s %-10s %s\n' 'method' 'ring_batch' 'ring_claim'
for m in new start len is_empty end contains sequences overlaps
do
  b=$( command grep -c "fn $m(" ring_batch/src/lib.rs || true )
  c=$( command grep -c "fn $m(" ring_claim/src/lib.rs || true )
  printf '    %-12s %-10s %s\n' "$m" "$b" "$c"
done
echo '  -- does the Tier 5 crate know about the Tier 2 one --'
command grep 'ring_batch' ring_claim/Cargo.toml ring_claim/src/lib.rs || echo '    (no mention anywhere in ring_claim)'
```

Live output:

```
  -- the same eight methods, two crates, no dependency between them --
    method       ring_batch ring_claim
    new          1          2
    start        1          1
    len          1          1
    is_empty     1          1
    end          1          1
    contains     1          1
    sequences    1          1
    overlaps     1          1
  -- does the Tier 5 crate know about the Tier 2 one --
    (no mention anywhere in ring_claim)
```

**Finding.** Eight names, eight matches, same spelling — `ring_claim`'s second
`new` is `Claimer::new`, a different type. Both crates independently arrived at
`start`/`len`/`is_empty`/`end`/`contains`/`sequences`/`overlaps` on a two-field
range, and `ring_claim` sits three tiers above `ring_batch` with no dependency
edge and no mention of it anywhere in its manifest or its source.

That is the pattern working and the pattern going unnoticed at the same time.
Convergent naming on a well-understood shape is a good sign — it means the shape
is right and the vocabulary is obvious. What it costs here is that a family with
a Tier 2 crate whose entire job is this shape has four implementations of it, and
the highest-tier ones do not import the lowest.

The two `Batch` types diverge for a stated reason: adding `ring : &'a Ring< S >`
is what makes a committing `Drop` possible, which is why they are three fields
rather than two. `ring_claim::Claim` has no such reason — it is
`BatchClaim` with `count` renamed to `len`.

---

### BA39 — The Twin Carries the Sentence This Crate Never Writes

The derives on the two two-field types are identical. One attribute is not:

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- Tier 5 --'
command grep -m1 -A2 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]' ring_claim/src/lib.rs
echo '  -- Tier 2 --'
command grep -m1 -A1 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]' ring_batch/src/lib.rs
echo '  -- type-level must_use across the family --'
command grep -r 'must_use = ' --include=lib.rs ring_*/src/ | sed 's|ring/||'
```

Live output:

```
  -- Tier 5 --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
#[ must_use = "a claimed range that is never published strands its slots and stalls every consumer" ]
pub struct Claim
  -- Tier 2 --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
pub struct BatchClaim
  -- type-level must_use across the family --
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
```

**Finding.** `derive( Debug, Clone, Copy, PartialEq, Eq )` on both, character for
character. `ring_claim::Claim` then adds a type-level `#[ must_use ]`, which
propagates to every function returning it — so `Claimer::claim` and
`claim_up_to` both refuse to be dropped silently. `BatchClaim` has no such
attribute, which is why `ring_batch::claim` is the one callable item in this
crate that can be discarded without a warning
([`api/001`](../api/001_twelve_items_seven_must_use.md) BA6).

The message is the more interesting half. "A claimed range that is never
published strands its slots and stalls every consumer" is precisely the rule
[`lifecycle/001`](../lifecycle/001_no_lifecycle_and_no_rollback.md) BA30 records
as missing from `ring_batch` — that a claim is irreversible, and that abandoning
one leaves sequences owned by nothing. `ring_claim` states it. `ring_tls` states
it, in a subordinate clause, as a justification for something else. The crate
that defines the type states it nowhere, and the one-line attribute that would
both state it and enforce it is already written three tiers up.

The census also shows the pattern half-applied one tier above that. Four
type-level `must_use` attributes exist in the family, and all four name a
consequence rather than a rule — what goes wrong, not what you must do.
`ring_spsc` puts one on both of its drop-committing types, `Reservation` and
`Batch`. `ring_mpsc` has the same two types with the same destructors
([`lifecycle/001`](../lifecycle/001_no_lifecycle_and_no_rollback.md) BA31) and
guards neither, so dropping a `ring_mpsc::Batch` silently commits records nobody
wrote. Two crates, one convention, applied in one of them.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`data_structure/002`](../data_structure/002_the_struct_ring_claim_wrote_again.md) | The same two types compared as data rather than as a pattern |
| [`lifecycle/001`](../lifecycle/001_no_lifecycle_and_no_rollback.md) | The rule this attribute states and this crate does not |
| [`api/001`](../api/001_twelve_items_seven_must_use.md) | The `must_use` census, and the one item it misses |
| [`pattern/002`](002_two_functions_where_one_would_have_hidden_it.md) | The other shape decision the crate made deliberately |

### Sources

| Fact | Where |
|------|-------|
| `BatchClaim` and its derives | `ring_batch/src/lib.rs:55-60` |
| `ring_claim::Claim`, its derives and its `must_use` | `ring_claim/src/lib.rs:94-100` |
| The two ring-carrying variants | `ring_mpsc/src/lib.rs:1143-1148`, `ring_spsc/src/lib.rs:962-967` |
| The identical method surface, and the absent edge | Census above |

### Tests

| Test | Covers |
|------|--------|
| `claims_are_copied_not_moved_and_compare_by_value` | The derives the two types share |
| *(to create)* | Nothing anywhere compares the four range types; the duplication is invisible to the suite |
