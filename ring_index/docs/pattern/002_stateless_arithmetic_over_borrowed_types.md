# Pattern: Stateless Arithmetic Over Borrowed Types

### Scope

**Purpose:** Name the shape `ring_index` shares with exactly one sibling — a
crate that owns no type, holds no state, and exposes only free functions over
`ring_types` values — and record the one function of the eight that breaks it.

**Responsibility:** The crate-level shape, measured against `ring_seqno`, the only
other Tier 1 crate with the same census.

**In Scope:** `ring_index/src/lib.rs`; `ring_seqno/src/lib.rs`.

**Out of Scope:** the one-owner rule about *where* the fold lives is
[`pattern/001`](001_one_owner_for_one_arithmetic_fact.md). What `run`'s
allocation costs is
[`non_functional_requirement/002`](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md).

---

## Two Crates With the Same Census

```sh
cd "$(git rev-parse --show-toplevel)"
printf '  %-12s %-7s %-7s %-7s %-7s %-7s\n' crate 'pub fn' struct trait static 'mut'
for c in ring_index ring_seqno; do
  f=ring/$c/src/lib.rs
  printf '  %-12s %-7s %-7s %-7s %-7s %-7s\n' "$c" \
    "$( command grep -cE '^pub (const )?fn ' "$f" )" \
    "$( command grep -cE '^\s*(pub )?struct ' "$f" )" \
    "$( command grep -cE '^\s*(pub )?trait ' "$f" )" \
    "$( command grep -cE '^\s*(pub )?static ' "$f" )" \
    "$( command grep -cE '\bmut\b' "$f" )"
done
echo '  -- their eight signatures --'
command grep -hE '^pub (const )?fn ' ring_index/src/lib.rs ring_seqno/src/lib.rs | sed 's/^/    /'
```

Live output:

```
  crate        pub fn  struct  trait   static  mut    
  ring_index   3       0       0       0       0      
  ring_seqno     5       0       0       0       0      
  -- their eight signatures --
    pub fn of( seq : Seq, capacity : Capacity ) -> SlotIndex
    pub fn aliases( a : Seq, b : Seq, capacity : Capacity ) -> bool
    pub fn run( start : Seq, count : usize, capacity : Capacity ) -> Vec< SlotIndex >
    pub fn laps_between( earlier : Seq, later : Seq, capacity : Capacity ) -> u64
    pub fn may_claim( producer : Seq, consumer : Seq, capacity : Capacity ) -> bool
    pub fn free_slots( producer : Seq, consumer : Seq, capacity : Capacity ) -> usize
    pub fn pending( producer : Seq, consumer : Seq ) -> u64
    pub fn slowest( cursors : &[ Seq ] ) -> Option< Seq >
```

---

### IX43 — Eight Functions, Two Crates, No State of Any Kind

**Finding.** `ring_index` and `ring_seqno` are the only two crates in the family
whose census is zero across every axis but functions: no struct, no enum, no
trait, no static, and not one occurrence of the token `mut` in either file. Six
of the eight signatures take `Capacity` as their last parameter and return a
plain value. Nothing is borrowed mutably; nothing is stored.

The shape is worth naming because it explains what these two crates *are*.
`ring_index` answers "which slot does this sequence address"; `ring_seqno` answers
"may this sequence be claimed, and how far apart are these two." Both questions
are total functions of their arguments — there is no ring involved, only
arithmetic about one — so a struct would have nothing to hold and a trait would
have nothing to abstract over.

That is also why they sit at Tier 1 with nothing above them but `ring_types`.
They can be tested by enumeration rather than by scenario, which is exactly what
`mask_equals_modulo_over_four_laps_of_every_capacity` does with 8,184
assertions and no ring anywhere in the test.

The division between the two is clean and undocumented in either. `ring_index`
maps sequence space onto slot space; `ring_seqno` compares points within sequence
space and never mentions a slot. Neither crate's module comment states the
boundary, and `aliases` sits exactly on it — it is a `ring_index` function whose
stated purpose is testing a `ring_seqno` gate
([`item/002`](../item/002_the_two_that_nothing_calls.md) IX35).

---

### IX44 — One of the Eight Allocates, and It Is the One Nothing Calls

**Finding.** Seven of the eight signatures return a `Copy` scalar or a small
`Option`. One returns `Vec< SlotIndex >`.

```
--- (2) what each of the three functions allocates ---
  of                          0 allocs over 10,000 calls (sink true)
  aliases                     0 allocs over 10,000 calls (sink false)
  run (count = 8)         10000 allocs over 10,000 calls, 640000 bytes (len 80000)
```

`run` is the only allocating function across both crates. It is also the only one
of the eight with no caller anywhere in the 33 crates, and the two facts are
connected: `ring_batch` needed exactly this computation, looked at the signature,
and wrote `drain_order` as an iterator instead
([`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md)).

So the pattern held everywhere it was tested. Seven functions that allocate
nothing are used; the one that allocates is not. The crate that declined it did
not decline the *arithmetic* — `ring_batch` imports `of` and folds one sequence
at a time — it declined the container the arithmetic was wrapped in.

Reading the shape as a rule rather than a coincidence: a Tier 1 crate is on the
hot path of everything above it, so returning an owned collection from one is a
decision to allocate inside every consumer's inner loop. Seven signatures respect
that and one does not, and the family's response to the one was to route around
it rather than to fix it.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`pattern/001`](001_one_owner_for_one_arithmetic_fact.md) | Where the fold may live, which this shape is the crate-level consequence of |
| [`decisions/002`](../decisions/002_a_vec_where_an_iterator_would_do.md) | The return type as an undocumented decision |
| [`non_functional_requirement/002`](../non_functional_requirement/002_the_one_allocation_and_the_zero_callers.md) | What the allocation costs, measured |
| [`workaround/002`](../workaround/002_the_iterator_ring_batch_built_instead.md) | What `ring_batch` wrote instead |

### Sources

| Fact | Where |
|------|-------|
| The two censuses and the eight signatures | Census above |
| Allocation counts per function | Release probe, section 2, quoted above |
| `ring_seqno`'s signatures | `ring_seqno/src/lib.rs:50, 73, 95, 112, 133` |

### Tests

| Test | Covers |
|------|--------|
| `mask_equals_modulo_over_four_laps_of_every_capacity` | Enumeration rather than scenario, which the shape is what permits |
| `an_empty_run_is_empty` | The one allocating signature, at the size where it allocates nothing |
| *(to create)* | An assertion that `of` and `aliases` allocate nothing, which is measured and unasserted |
