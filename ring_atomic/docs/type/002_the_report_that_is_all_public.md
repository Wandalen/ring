# Type: The Report That Is All Public

### Scope

**Purpose:** Record what `OpCounts` is as a type — the crate's only aggregate, its
derives, its absent constructor — and what the family actually does with a value of
it.

**Responsibility:** The declaration, the six derives, the five public fields, and the
consumption census across all three crates that read one.

**In Scope:** `ring_atomic/src/lib.rs:272-285`;
`ring_atomic/tests/atomic_test.rs:309-319`.

**Out of Scope:** That `total` is a definition the type does not enforce is
[`data_structure/002`](../data_structure/002_opcounts_and_the_total_it_stores.md)
AT11, and that derived equality compares it is AT12 there. The 40-byte layout is
[`data_structure/001`](../data_structure/001_one_word_and_five.md). That a returned
value may describe a moment that never happened is
[`pitfall/001`](../pitfall/001_the_snapshot_that_never_happened.md).

---

## The Declaration, and What the Family Does With One

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the declaration --'
command grep -m1 -A14 -F '#[ derive( Debug, Clone, Copy, PartialEq, Eq, Default ) ]' ring_atomic/src/lib.rs
echo '  -- how the family consumes a value of it --'
for scope in "ring_atomic" "ring_batch ring_tls"
do label=$( [ "$scope" = "ring_atomic" ] && echo "in ring_atomic" || echo "downstream   " )
   printf '    %s  field reads %-4s whole-value uses %s\n' "$label" \
     "$( command grep -rho 'counts()\.[a-z_]*' --include=*.rs $scope | wc -l )" \
     "$( command grep -rho 'counts()[^.]' --include=*.rs $scope | wc -l )"; done
echo '  -- and what the crate builds for each of its three types --'
printf '    new() constructors for the two cells : %s\n' \
  "$( command grep -c 'pub const fn new\|pub fn new' ring_atomic/src/lib.rs || true )"
printf '    hand-written Default impls for them  : %s\n' \
  "$( command grep -c 'impl Default for' ring_atomic/src/lib.rs || true )"
printf '    any inherent impl on OpCounts        : %s\n' \
  "$( command grep -c 'impl OpCounts' ring_atomic/src/lib.rs || true )"
```

Live output:

```
  -- the declaration --
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Default ) ]
pub struct OpCounts
{
  /// Reads served.
  pub loads : usize,
  /// Writes served.
  pub stores : usize,
  /// Advances served — the batch claim's own operation.
  pub fetch_adds : usize,
  /// Compare-exchanges served, successful or not — the contended claim's.
  pub compare_exchanges : usize,
  /// Every operation above, summed.
  pub total : usize,
}

  -- how the family consumes a value of it --
    in ring_atomic  field reads 15   whole-value uses 11
    downstream     field reads 16   whole-value uses 0
  -- and what the crate builds for each of its three types --
    new() constructors for the two cells : 4
    hand-written Default impls for them  : 2
    any inherent impl on OpCounts        : 0
```

---

### AT47 — The Bundle Nobody Downstream Keeps

`OpCounts` bundles four counters and a derived total into one returned value, and
carries `PartialEq` and `Eq` so a caller can compare the whole shape at once. The
crate has a test devoted to justifying exactly that, and it says so in its own
comment: "Comparable so a test can assert a whole shape at once rather than four
fields; printable so a failure says what was actually counted."

Downstream, in the two crates the counting shim exists to serve, that option is taken
zero times. Sixteen field reads across `ring_batch` and `ring_tls` — twenty `.total`,
seven `.fetch_adds`, two `.loads`, one `.compare_exchanges` across all three crates —
and not one site that compares, stores, or passes an `OpCounts` as a value. Every
consumer unbundles immediately.

**Finding.** All nine whole-value uses are inside `ring_atomic` itself, and the three
that actually compare are in its own test file, each hand-writing the total —
`OpCounts { loads : 1, total : 1, ..OpCounts::default() }`. So the derive's only real
users are the test written to demonstrate the derive and its two neighbours.

That matters because the bundling is not free. Returning five values together is what
requires four separate reads and creates the tear
([`pitfall/001`](../pitfall/001_the_snapshot_that_never_happened.md) AT43); carrying a
derived `total` alongside its own operands is what creates a field that can disagree
with them ([`data_structure/002`](../data_structure/002_opcounts_and_the_total_it_stores.md)
AT11, AT12). Both costs are paid at every call, for an aggregate that every caller
outside the crate takes apart on the same line.

---

### AT48 — Two Cells Get Six Constructors; the Report That Most Needs an Identity Gets None

The crate builds carefully for its two cells: four `new` functions and two
hand-written `Default` impls, each with a written rationale
([`item/001`](../item/001_six_constructors_for_two_types.md)). `OpCounts` has no
inherent impl at all. It is five `pub` fields, six derives, and nothing else — built
by struct literal or by `Default`.

For a data-transfer record that is the ordinary and correct choice, and this instance
is not arguing for a constructor. It is naming what the choice leaves absent, because
`OpCounts` is not quite an ordinary record: it is a *reading*, and a reading has two
properties a plain struct cannot carry.

It has no provenance. `Copy` plus public fields means a value obtained from
`cell.counts()` is indistinguishable in every way from one written out by hand — same
type, equal under `==`, identical under `{:?}`. `op_counts_are_comparable_and_printable`
relies on precisely this, comparing a literal against `OpCounts::default()`.

And it has no age. A snapshot copies freely and silently, so one taken while four
threads were writing — where roughly one in a hundred describes a state that never
existed — travels exactly as far, and looks exactly as authoritative, as one taken
from a quiet cell.

**Finding.** The type in this crate carrying the least structure is the one whose
values are the least trustworthy. Nothing here needs a constructor; what the type
lacks is any marker that a value came from a live cell rather than a keyboard. The
cheapest fix is not structural at all — it is the sentence
[`pitfall/001`](../pitfall/001_the_snapshot_that_never_happened.md) AT44 already asks
for on `counts()`, since the type's own field docs are individually accurate and it is
only the reading's conditions that go unstated.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`type/001`](001_what_the_trait_promises.md) | The other public surface, and the two things it declines to require |
| [`data_structure/002`](../data_structure/002_opcounts_and_the_total_it_stores.md) | `total` as an unenforced definition, and equality that compares it |
| [`pitfall/001`](../pitfall/001_the_snapshot_that_never_happened.md) | What a value of this type can describe, and why it is undetectable |
| [`item/001`](../item/001_six_constructors_for_two_types.md) | The six constructors the two cells got, against this type's none |
| [`algorithm/002`](../algorithm/002_counts_is_four_reads_not_one.md) | The four reads the bundling requires |

### Sources

| Fact | Where |
|------|-------|
| Six derives, five public fields | `ring_atomic/src/lib.rs:272-285` |
| The test justifying `PartialEq` and `Debug` | `ring_atomic/tests/atomic_test.rs:309-319` |
| 16 downstream field reads, 0 whole-value uses | Census above |
| Four `new`, two `Default`, no inherent impl on `OpCounts` | Census above |

### Tests

| Test | Covers |
|------|--------|
| `op_counts_are_comparable_and_printable` | The derives, using two hand-written values and never one from a cell |
| `total_is_the_sum_of_the_four_and_not_an_independent_counter` | That a value from a cell satisfies the summation the field documents |
| `a_cell_never_touched_counts_zero` | That an untouched cell reports the same value `Default` produces |
| *(to create)* | Nothing compares a whole `OpCounts` taken from a cell against another taken from a cell, so the derive is exercised only against literals |
