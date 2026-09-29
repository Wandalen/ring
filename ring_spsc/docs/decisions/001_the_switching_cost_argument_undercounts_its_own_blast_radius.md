# Decision: The Switching-Cost Argument Undercounts Its Own Blast Radius

- **Status**: Open
- **Deciders**: the ring family
- **Turns on**: Whether the Decision Gate ruling that kept three shape questions unfiled survives a corrected blast radius

### Context

This crate's [`decisions/readme.md`](readme.md) records three deliberately
undecided shape questions — the producer's candidate shapes, the borrow-versus-copy
drain, and the cursor initialization value — and explains that none was filed as
an ADR because the Decision Gate tests switching cost, and the cost here is low:
changing any of the three "reaches `ring_core` and `ring_handle` and stops".

The measured reach is a different set, of a different size, coupled three
different ways.

```sh
cd "$(git rev-parse --show-toplevel)"
for c in $( ls -d ring_*/ | tr -d / ); do
  [ "$c" = ring_spsc ] && continue
  code=$( find $c -name '*.rs' -exec cat {} + 2>/dev/null \
          | grep -vE '^\s*(//|///|//!)' | grep -c 'ring_spsc' )
  fixture=$( find $c -name '*.stderr' -exec cat {} + 2>/dev/null | grep -c 'ring_spsc' )
  manifest=$( grep -c 'ring_spsc' $c/Cargo.toml )
  [ "$code$fixture$manifest" = 000 ] && continue
  printf '%-12s code=%-3s fixture=%-3s manifest=%s\n' "$c" "$code" "$fixture" "$manifest"
done
```

Live output:

```
ring_bench   code=2   fixture=0   manifest=2
ring_core    code=5   fixture=0   manifest=1
ring_handle  code=0   fixture=2   manifest=0
```

**Three crates, and the argument named two of them — one of which is the one
that reaches this crate in no code line at all.** `ring_bench`, which depends on
this crate and calls into it, was not named. `ring_handle`, which was, declares
no dependency and imports nothing — yet is coupled to this crate more tightly
than either of the other two, through a mechanism the dependency graph does not
show.

### The Third Coupling

`ring_handle/tests/ui/producer_shared_across_threads.stderr` is a `trybuild`
fixture: a pinned copy of the exact compiler error a program is required to
produce. It is checked on every ordinary test run
(`ring_handle/tests/ui_test.rs`, not `#[ ignore ]`d), and it contains this
crate's *declaration text*.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_handle
printf 'the pinned lines:\n'
grep 'ring_spsc\|pub struct Producer' tests/ui/producer_shared_across_threads.stderr
printf 'ring_spsc in this manifest:  %s\n' "$( grep -c 'ring_spsc' Cargo.toml )"
```

Live output:

```
the pinned lines:
note: required because it appears within the type `ring_spsc::Producer<'_, ring_slot::TypedSlot<u32>>`
  --> $WORKSPACE/ring_spsc/src/lib.rs
   | pub struct Producer< 'a, S >
   | pub struct Producer< 'a, T >
   | pub struct Producer< 'a, T >
ring_spsc in this manifest:  0
```

Line 16 is a byte-for-byte copy of a line in `ring_spsc/src/lib.rs`. **Adding a
bound, renaming the type parameter, or reflowing the declaration across two
lines fails a test in a crate two hops away that never mentions this one in its
manifest** — and it fails as a diff against expected output, not as a
compilation error, so the message names `ring_handle` rather than the file that
changed.

That crate's own harness states the policy that makes this consequential: "a
changed `.stderr` is a change to what this crate guarantees." True — but the
change can originate here, where nobody reads `ring_handle`'s guarantees.

### Readings

**1 — A typo with no consequence.** Two names in, one name out; the conclusion —
a handful of in-repo consumers, all within this family — is unchanged. Under this
reading the fix is to correct the sentence and leave the three questions unfiled.

**2 — The argument was never checked, which is the finding.** The switching-cost
claim is the entire justification for three questions going unrecorded. A
justification that names its blast radius from memory rather than from a
measurement is the kind that stays wrong after the radius changes — and this one
was wrong on the day it was written, in both directions at once.

**3 — The two omitted couplings are both the quiet kind, and that is not
coincidence.** The consumer the argument *did* name is the one whose breakage is
a compile error in this family's own code. `ring_bench` breaks by continuing
to compile while measuring something else; `ring_handle` breaks by failing a
pinned-text comparison in a suite whose failure message points away from the
edit. An argument assembled from memory recalls the loud consumer and forgets
the quiet ones, because the loud one is the one anyone has ever seen break.

### Decision

Open. What is filed here is the measurement and the three readings, not the
change: whether the three shape questions should now be filed as ADRs is a
ruling for the family's own layering rules to make, and settling it would mean
revisiting how those rules currently set this crate's switching cost.

The family's layering rule sets switching cost by export position —
`ring_spsc` is internal, therefore cheap. The `ring_handle` fixture is a
counter-example to the rule itself and not just to this crate's application of
it: export position measures who may *depend* on a crate, and a pinned
diagnostic couples without depending.

### Consequences

Until it is ruled, this crate's `decisions/` index states a blast radius that
does not match the dependency graph, and the reader has no way to tell from the
index alone. Separately and regardless of the ruling, the `ring_handle`
coupling was undocumented on both ends; SP13's disposition closed this crate's
side of that gap — `Producer`'s own doc comment in `ring_spsc/src/lib.rs` now
names the fixture that pins it. `ring_handle` still records nothing of the
reverse edge.

### Sources

| File | Relationship |
|------|-----------------|
| `readme.md` | States the switching-cost argument this instance measures |
| `../../../ring_handle/tests/ui/producer_shared_across_threads.stderr` | Pins this crate's `Producer` declaration verbatim |
| `../../../ring_handle/tests/ui_test.rs` | Runs that fixture on every ordinary test run |
| `../../../ring_handle/Cargo.toml` | Declares no dependency on this crate |
| `../../../ring_bench/Cargo.toml` | Declares one, and was not named |
| `../integration/002_reached_through_the_export_surface.md` | The instance the readme cites for the export position |

### SP13 — A Pinned Diagnostic Couples Two Crates With No Edge Between Them

`ring_handle` has no dependency on `ring_spsc`, imports no name from it, and
fails its own test suite if this crate's `pub struct Producer< 'a, S >` line is
reformatted. The coupling runs through `ring_core`, which depends on both, and
is recorded only in the expected text of a compiler error.

**No tool in the family reports this edge.** `cargo tree` does not — there is no
dependency. A grep for `ring_spsc` in `*.rs` does not — the text is in a
`.stderr`. The manifest does not. It is visible only by reading a fixture file
that exists to describe a *rejection*.

The failure is also misattributed by construction: the test that goes red names
`ring_handle`, and the edit that turned it red was in `ring_spsc`.

```sh
cd "$(git rev-parse --show-toplevel)"
grep -c 'pinned outside this crate' ring_spsc/src/lib.rs
```

Live output:

```
1
```

**Disposition:** applied — `Producer`'s own doc comment in `ring_spsc/src/lib.rs`
now states that this declaration is pinned by `ring_handle`'s trybuild fixture
and names the fixture path, so a contributor reading the struct before editing
it sees the coupling `cargo tree`, a `.rs` grep and the manifest all miss.
Now prints: `1`

### SP14 — The Scan That Found Two Consumers Had Only Ever Looked at Four Crates

The first version of this instance measured reach by looping over a hardcoded
list — `ring_bench ring_core ring_debug ring_handle` — assembled from the crates
already under discussion. It reported two code consumers, which is right, from a
scan that never examined the other twenty-eight.

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'crates mentioning ring_spsc at all:  '
grep -rl 'ring_spsc' --include='*.rs' --include='*.toml' --include='*.stderr' . \
  | grep -v '^./ring_spsc/' | grep -v '^./Cargo.toml$' | cut -d/ -f2 | sort -u | wc -l
printf 'of those, reaching it in code:       '
for c in $( ls -d ring_*/ | tr -d / ); do
  [ "$c" = ring_spsc ] && continue
  find $c -name '*.rs' -exec cat {} + 2>/dev/null \
    | grep -vE '^\s*(//|///|//!)' | grep -q 'ring_spsc' && echo $c
done | wc -l
```

Live output:

```
crates mentioning ring_spsc at all:  22
of those, reaching it in code:       2
```

Twenty-two crates name this one somewhere; twenty do so only in prose. The
hardcoded four happened to contain both real consumers, so the answer was
correct and the method was not — **a scan whose universe is a list written from
memory measures the memory, not the codebase**, and reports the same number
whether or not it is right.

This is the fourth silent-filter defect recorded in this family and the
first in a recipe written to catch the other three.

**Correction (2026-09-28):** this section previously quoted a total of
twenty-one with the prose reading "Nineteen crates name this one somewhere;
seventeen do so only in prose" — inconsistent with the twenty-one and two
directly above it, which give nineteen prose-only, not seventeen; the "name
this one somewhere" figure should have read the total itself, twenty-one, not
nineteen. Independently, the total had also gone stale: the untracked
`ring/Cargo.toml` introduced by an in-progress workspace restructuring lists
`ring_spsc` as a member, and the recipe's `cut -d/ -f2` was reading its bare
`./Cargo.toml` match as a pseudo-crate literally named `Cargo.toml` — one
inflated entry the recipe's own `ring_spsc/` exclusion does not catch because
it isn't a `ring_spsc/`-prefixed path. The recipe above now excludes that
match explicitly. The corrected reading is twenty-two crates mentioning this
one, two reaching it in code, twenty in prose only.

### SP15 — Two of the Three Consumers Break Quietly

`ring_core` names `Ring`, `Producer` and `Consumer` in five code lines; a shape
change there fails to compile, which is the loud case and the only one the
switching-cost argument accounted for.

`ring_bench` names `Ring` and `Ring::with_config`. A change that still compiles
but alters what is measured — a different default capacity, a different
occupancy accounting — produces numbers not comparable to yesterday's and says
nothing about it.

`ring_handle` names nothing, and breaks on the *spelling* of a declaration.

**Three consumers, three failure modes, and the two the argument omitted are the
two that do not point at the file that caused them.**
