# The Forty Items, and the Six the Family Calls

### Scope

- **Purpose**: Read the forty catalogued items as one set and report which of them the other thirty crates actually reach, so the catalogue's per-item Crate Usage sections have a total that can be checked against them.
- **Responsibility**: State the reach of every callable item, name the ones with no library caller, and separate a doc-example reference from a production one everywhere the two differ.
- **In Scope**: The 13 associated functions, 3 associated constants, and 6 exported names, measured against `ring_*/src`.
- **Out of Scope**: Each item's own definition and file-local usage — those are the forty catalogue files under this directory; the field-privacy asymmetry that shapes the reach — see [`002`](002_three_newtypes_and_two_open_fields.md).

### Why a Synthesis Rather Than a Forty-First Item

The catalogue answers *who touches this item*, forty times, correctly. It cannot
answer *how much of this crate is load-bearing*, because that question is about
the set and every catalogue file sees one member of it. The two questions have
different answers here, and the difference is the point: the per-item sections
report healthy usage almost everywhere, and the set reports that most of the
surface has one caller or none.

### The Callable Surface, by Reach

Production call sites only — every count below excludes `//`, `///` and `//!`
lines, because the raw figures are dominated by doc examples
(→ [TY10](#ty10--capacitynew-has-one-production-call-site-and-112-doc-references)).
Three of the thirteen names are ambiguous family-wide — `new`, `get` and `next`
collide with `Vec::new`, `slice::get` and `Iterator::next` — so those are matched
by their qualified form and the rest by their bare method call:

```sh
cd "$(git rev-parse --show-toplevel)"
for p in 'Capacity::new' 'Capacity::default' '\.mask()' '\.is_configuration()' \
         '\.is_transient()' '\.is_non_blocking()' '\.reports_failure()' \
         '\.drops_silently()' '\.advanced_by(' '\.distance_to('; do
  n=$( grep -r "$p" ring_*/src/*.rs 2>/dev/null \
       | grep -v '^ring_types/' | grep -vE ': *(//|///|//!)' | wc -l )
  printf '%-22s %3d\n' "$p" "$n"
done
```

Live output:

```
Capacity::new            1
Capacity::default        0
\.mask()                 2
\.is_configuration()     0
\.is_transient()         0
\.is_non_blocking()      1
\.reports_failure()      0
\.drops_silently()       0
\.advanced_by(          14
\.distance_to(          11
```

**Four of the five classifier predicates have no production caller outside this
crate at all**, and the fifth has one. The crate that thirty others depend on
exports a classification surface that no library in the family calls.

The figure changes with the comment filter, and that is the finding's whole
point: `is_configuration` and `is_transient` each *look* adopted — `ring_gating`
and `ring_shutdown` respectively name them — but both mentions are `///`
examples. Strip comments and the count is zero.

### The Two Halves of the Crate Are Not Alike

| Half | Items | Reach |
|------|-------|-------|
| Positions and capacity | `Seq`, `SlotIndex`, `Capacity` and their methods | Heavy — `Seq` named in 16 crates, `Capacity` in 11, `advanced_by` called from 7 and `distance_to` from 4 |
| Classification | The five predicates over `RingError`, `WaitKind`, `OverflowPolicy` | Zero to one production caller each |

The classification half is not unused; it is *used by tests*. `is_transient` has
six references in this crate's own suite and seven across the rest of the
family's, against **no** production caller anywhere. That is the profile of an
API that documents an intent rather than one that carries traffic — a defensible
thing for tier 0 to export, but not what a reader would infer from thirteen
`#[ must_use ]` methods.

### TY9 — Four of the Five Classifier Predicates Have No Production Caller

`is_configuration`, `is_transient`, `reports_failure` and `drops_silently` are
called from no library in the family outside this crate; `is_non_blocking` is
called once, by `ring_config`. Across thirty dependent crates the
classification surface is exercised entirely from test code.

### TY10 — `Capacity::new` Has One Production Call Site and 112 Doc References

```sh
cd "$(git rev-parse --show-toplevel)"
all=$( command grep -rn 'Capacity::new' ring_*/src/*.rs 2>/dev/null )
printf 'references across ring_*/src : %s\n' "$( printf '%s\n' "$all" | wc -l )"
printf 'of them, production calls    : %s\n' "$( printf '%s\n' "$all" | command grep -vE ':[0-9]+: *(//|///|//!)' | command grep -v '^ring_types/' | wc -l )"
printf 'and where that call is       : %s\n' "$( printf '%s\n' "$all" | command grep -vE ':[0-9]+: *(//|///|//!)' | command grep -v '^ring_types/' | cut -d: -f1,2 | tr '\n' ' ' )"
```

Live output:

```
references across ring_*/src : 113
of them, production calls    : 1
and where that call is       : ring_config/src/lib.rs:71 
```

Every figure above is printed rather than written down, because this finding's
own numbers moved once already: it was filed reading *two* production call
sites, and [`algorithm/001` § TY20](../algorithm/001_validating_a_slot_count_to_a_power_of_two.md#ty20--an-infallible-accessor-re-runs-the-validation-and-panics-on-failure)
removed the second — `ring_core`'s accessor stopped rebuilding a `Capacity` and
now carries the validated one instead. Correcting that hazard cost this crate a
caller.

**Finding.** The ratio is the point and it survived the change intact: almost
every appearance of `Capacity::new` in the family is a `///` or `//!` example.
Any reach figure for this crate that does not strip comment lines overstates it
by roughly a hundredfold — worse now than the fiftyfold this finding was filed
at, because the correction removed a real call and left every doc example
standing.

### TY11 — `Capacity::mask` Has Two Production Callers and `Capacity::get` Has Twenty-Nine

The accessor that returns the raw slot count is called fifteen times as often as
the one that returns the mask, and both of `mask`'s callers use it for the same
expression (→ [`data_structure/001`](../data_structure/001_two_position_types_and_the_fold_between_them.md),
TY26). The method that encodes the power-of-two invariant is the least-used
thing the type exports.

### TY12 — Every Callable Is Associated With a Type

The crate declares no free function, no trait, no static, no type alias and no
macro — 40 items across 7 of the taxonomy's 18 kinds. Nothing here can be called
without first naming one of four types, which is why the export surface is
exactly six names and why a consumer cannot partially adopt the crate.

### Items

| File | Relationship |
|------|--------------|
| [`readme.md`](readme.md) | The catalogue this reads as a set |
| [`002_three_newtypes_and_two_open_fields.md`](002_three_newtypes_and_two_open_fields.md) | The construction paths that bypass the callable surface measured here |
