# Eighty Items, and the Four Names That Leave the Crate

### Scope

- **Purpose**: Read the census as a set and report which public items the two dependent crates reach, because a 57-item public surface consumed by two crates is a ratio worth stating rather than assuming.
- **Responsibility**: The import reach of every public type and constant, and the call reach of every public associated function.
- **In Scope**: The 57 public items, measured against `ring_core/src` and `ring_bench/src` — the only two crates declaring a `ring_mpsc` dependency.
- **Out of Scope**: The five constants, which get their own instance (→ [`002`](002_five_public_ordering_constants.md)); what each item does.

### Who Depends on This Crate at All

```sh
cd "$(git rev-parse --show-toplevel)"
for c in $( grep -l 'ring_mpsc' ring_*/Cargo.toml | sed 's|/Cargo.toml||' | grep -v '^ring_mpsc$' ); do
  printf '%-14s ' "$c"
  grep -hoE 'ring_mpsc::\{?[A-Za-z_, ]+' $c/src/*.rs 2>/dev/null | sed 's/ring_mpsc:://' \
    | tr -d '{}' | tr ',' '\n' | tr -d ' ' | sort -u | tr '\n' ' '
  echo
done
```

Live output:

```
ring_bench     Ring 
ring_core      Consumer Ends Producer Ring 
```

**Two crates, four distinct names.** `ring_core` takes the four handles;
`ring_bench` takes `Ring` alone and reaches everything else through it.

### Which Functions Are Called

Production call sites across those two crates, comment lines removed:

```sh
cd "$(git rev-parse --show-toplevel)"
for m in '\.claim()' '\.push(' '\.drain()' '\.drain_up_to(' '\.available()' \
         '\.committed()' '\.published_through()' '\.stamps()' '\.claimed()' \
         '\.on_distinct_lines()' '\.sequences()' '\.position()' '\.with_config('; do
  L=0
  for c in ring_core ring_bench; do
    L=$(( L + $( cat $c/src/*.rs | grep -vE '^\s*(//|///|//!)' | grep -c "$m" ) ))
  done
  printf '%-22s %d\n' "$m" "$L"
done
```

Live output:

```
\.claim()              1
\.push(                5
\.drain()              4
\.drain_up_to(         2
\.available()          2
\.committed()          0
\.published_through()  0
\.stamps()             0
\.claimed()            0
\.on_distinct_lines()  0
\.sequences()          0
\.position()           0
\.with_config(         0
```

**Eight of the thirteen have no caller**, and they are not scattered: `committed`,
`published_through`, `stamps`, `claimed`, `on_distinct_lines`, `sequences` and
`position` are the crate's whole observation surface — every method that reports
where the cursors are rather than moving them. `with_config` is the eighth, and
it is the constructor the family does not use.

That grouping is the subject of its own decision
(→ [`../decisions/002`](../decisions/002_the_observation_surface_kept_without_a_caller.md)).

### The Shape This Gives

Fifty-seven public items; four names imported; five methods carrying all the
family's traffic. `ring_mpsc` is reached almost entirely through `ring_core`,
which is exactly what `integration/001` describes — and it means this crate's
public surface is sized for a consumer that does not yet exist.

### MP26 — Fifty-Seven Public Items, Four Names Imported

`Reserved` and `Batch` are never named outside this crate — both are obtained
from a method and used inline, never spelled in a signature. The five constants
are never named either (→ [`002`](002_five_public_ordering_constants.md)).

**A 1,214-line crate whose entire imported vocabulary is four types.** That is
not a defect; it is what a well-encapsulated data structure looks like, and it
means the item census is the only place the other fifty-three are visible at
all.

### MP27 — No Enum and No Trait in Eighty Items

`lifecycle/003` describes four slot states across one lap. None is a variant:
a slot's state is the relation between its stamp and the sequence addressing it,
plus the two cursors' positions. There is nothing to `match` on.

**That is why the stale-stamp trap exists**
(→ [`../pitfall/002`](../pitfall/002_a_stale_stamp_reads_as_unpublished_not_as_wrong.md)) —
an enum would make the four states exhaustive and the wrong test uncompilable;
arithmetic makes all three plausible tests compile and one correct.

### MP28 — Sixteen Implementations for Six Structs

```sh
cd "$(git rev-parse --show-toplevel)"/ring_mpsc
grep -E '^impl|^unsafe impl' src/lib.rs | sed 's/ *$//'
```

Live output:

```
unsafe impl< S : Send > Sync for Ring< S > {}
impl< S : Slot + Default > Ring< S >
impl< S > Ring< S >
impl< S > core::fmt::Debug for Ring< S >
impl< 'a, S > Ends< 'a, S >
impl< S > Clone for Producer< '_, S >
impl< S > Copy for Producer< '_, S > {}
impl< 'a, S > Producer< 'a, S >
impl< 'a, T > Producer< 'a, TypedSlot< T > >
impl< S > Reserved< '_, S >
impl< S > Deref for Reserved< '_, S >
impl< S > DerefMut for Reserved< '_, S >
impl< S > Drop for Reserved< '_, S >
impl< 'a, S > Consumer< 'a, S >
impl< S > Batch< '_, S >
impl< S > Drop for Batch< '_, S >
```

Reading the list shows the pattern: `Ring< S >` has one block for `S : Slot +
Default` and another for the unbounded case, because construction needs
`Default` and observation does not. Splitting by bound rather than gating
individual methods keeps each block's requirements visible in one place.
