# Type: Two Counts That Are `usize` and Three Fields That Are Not

### Scope

**Purpose:** Record the split between the fields carrying a domain type and the
fields carrying a bare integer, and what follows the same line.

**Responsibility:** The three domain-typed fields and what their types refuse, the
two `usize` fields and the runtime clamps that stand in, and the three separate
censuses in this corpus that partition the record the same way.

**In Scope:** `ring_config/src/lib.rs:42-49`, `:91`, `:106`, `:124`,
`:143`; `ring_types/src/capacity.rs:42-48`;
`ring_bench/src/lib.rs:206-212`.

**Out of Scope:** The derives over these types are
[`type/001`](001_five_derives_and_the_one_that_is_free_and_absent.md). What the
clamps guarantee is
[`invariant/001`](../invariant/001_two_ranges_that_cannot_be_violated.md).

---

## Which Fields Carry a Type and Which Carry a Number

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the five fields, by whether they carry a domain type or a bare integer --'
command grep -m1 -A7 -F 'pub struct RingConfig' ring_config/src/lib.rs
echo '  -- what the domain types refuse that a usize cannot --'
command grep -m1 -A6 -F '    if slots == 0' ring_types/src/capacity.rs
echo '  -- the setters that clamp, against the setters that assign --'
command grep 'self\.[a-z_]* = ' ring_config/src/lib.rs
echo '  -- production reads of each field outside this crate --'
for m in capacity wait overflow producers batch; do
  n=$( command grep -r "\(config()\|config\|cfg\)\.$m()" --include=*.rs */src | command grep -v '^ring_config/' | command grep -vc '///\|//!' || true )
  printf '  %-10s %s\n' "$m" "$n"
done
echo '  -- and the fields ring_bench keeps its own copy of --'
command grep -m1 -A6 -F 'pub struct Workload' ring_bench/src/lib.rs
```

Live output:

```
  -- the five fields, by whether they carry a domain type or a bare integer --
pub struct RingConfig
{
  capacity : Capacity,
  wait : WaitKind,
  overflow : OverflowPolicy,
  producers : usize,
  batch : usize,
}
  -- what the domain types refuse that a usize cannot --
    if slots == 0
    {
      return Err( RingError::CapacityZero );
    }
    if !slots.is_power_of_two()
    {
      return Err( RingError::CapacityNotPowerOfTwo( slots ) );
  -- the setters that clamp, against the setters that assign --
    self.wait = wait;
    self.overflow = overflow;
    self.producers = if producers == 0 { 1 } else { producers };
    self.batch = if capped == 0 { 1 } else { capped };
  -- production reads of each field outside this crate --
  capacity   5
  wait       0
  overflow   5
  producers  0
  batch      1
  -- and the fields ring_bench keeps its own copy of --
pub struct Workload
{
  config : RingConfig,
  producers : usize,
  records_per_producer : usize,
  cells : usize,
  semantics : AccumulatorSemantics,
```

---

### RC47 — The Record Enforces the Same Kind of Rule Two Different Ways and Never Says Which to Use

Three fields carry a type that cannot hold an illegal value. `Capacity` is a
newtype over `usize` whose constructor rejects zero and any non-power-of-two, so
by the time one reaches the struct the two rules are already true and no later
code re-checks them. `WaitKind` and `OverflowPolicy` are enums, which is the same
guarantee by a different construction — the illegal states have no spelling.

Two fields carry a bare `usize` and get their rules at assignment time instead.
`with_producers` writes `if producers == 0 { 1 } else { producers }` and
`with_batch` caps against the capacity, and those two lines are the only thing
between the fields and any value a `usize` can hold. The setter bodies show the
split exactly: `with_wait` and `with_overflow` are plain assignments because their
types have already done the work, and the other two are the ones with an
expression on the right-hand side.

**Finding.** Both mechanisms are correct and the record uses them for the same
job. What is missing is a rule. `Capacity` is the family's own demonstration that
a bounded count deserves a newtype — it lives one crate away, in `ring_types`,
next to the two enums — and `producers` is a bounded count with an identical
shape: a `usize` with a rule about zero.

A `Producers` newtype would move `with_producers`' clamp into a constructor and
make the range structural rather than maintained, which is what
[`invariant/001`](../invariant/001_two_ranges_that_cannot_be_violated.md) records
as currently resting on two lines with no test that they are the only two. It
would also cost something real: `Capacity::new` returns `Result`, and
[`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md)
records the deliberate choice to keep setters infallible, so a fallible newtype
would put back the `?` the setters were shaped to avoid. That tradeoff is a
genuine design question. Nothing in the crate poses it.

---

### RC48 — Three Separate Censuses in This Corpus Cut the Record Along the Same Line

The two bare-`usize` fields are the same two fields in three other places, each
established independently.

They are exactly the two with clamps — `:124` and `:143`, the only setter bodies
that are not assignments. They are exactly the two `ring_bench::Workload` keeps
its own copies of, beside the whole record that already holds them, and gets out
of step from its own constructor
([`item/001`](../item/001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md)).
And they are two of the three fields with no production reader anywhere in the
family: the census returns `capacity` 4, `overflow` 5, and zero for `wait`,
`producers` and `batch`.

**Finding.** That last one is a subset rather than a match, and the exception is
worth as much as the rule. `wait` is domain-typed and unread, so the alignment is
not "domain types get used and integers do not" — it is narrower and more
specific: the two fields that needed a rule written by hand are the two fields
whose value another crate felt entitled to keep a copy of, and the two whose
correction nothing can observe
([`pitfall/001`](../pitfall/001_a_clamp_with_no_way_to_detect_it.md)).

A field with a domain type carries its rule wherever it goes, including into
`ring_bench`'s struct, which is why `Workload` holds a whole `RingConfig` and two
loose integers rather than five loose fields. The two the copy could be taken of
are the two where taking it loses nothing the type system was tracking — and
therefore the two where the copy going wrong produces no error, only a different
number.

None of the four documents involved names the line. This one records it because
it is the only property of the record that four independent measurements agree
on, and because it is the reason a `Producers` newtype would be worth more than
the clamp it replaced: the type would have to be carried, not copied out.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/001`](001_five_derives_and_the_one_that_is_free_and_absent.md) | The derives that are the intersection of these types' |
| [`invariant/001`](../invariant/001_two_ranges_that_cannot_be_violated.md) | The two ranges the bare integers maintain by hand |
| [`item/001`](../item/001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md) | The copies taken of exactly these two fields |
| [`pitfall/001`](../pitfall/001_a_clamp_with_no_way_to_detect_it.md) | Why an unobservable correction is the cost of the second mechanism |
| [`decisions/002`](../decisions/002_clamping_instead_of_a_question_mark_mid_chain.md) | The infallibility a newtype would trade against |

### Sources

| Fact | Where |
|------|-------|
| The five fields and their types | `ring_config/src/lib.rs:42-49` |
| What `Capacity::new` refuses | `ring_types/src/capacity.rs:42-48` |
| Two assignments and two clamps | `ring_config/src/lib.rs:91`, `:106`, `:124`, `:143` |
| Reader counts per field outside the crate | Census above |
| The two fields `Workload` copies | `ring_bench/src/lib.rs:206-212` |

### Tests

| Test | Covers |
|------|--------|
| `capacity_is_validated_at_construction` | The rule a domain type carries rather than maintains |
| `zero_producers_clamps_to_one` | The same kind of rule, maintained by an assignment |
| `batch_clamps_into_one_through_capacity` | The other maintained rule, and its dependence on a domain-typed field |
| `every_named_field_is_carried` | All five fields, across both mechanisms |
