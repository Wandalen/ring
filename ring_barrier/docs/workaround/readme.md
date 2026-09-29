# workaround

Two workarounds, both of them checks that exist because the property they guard
is an *absence* — no capacity in the arithmetic, no library dependency on
`ring_gating` — and absences are not reachable by calling the code. Both are
greps over text rather than assertions over behaviour, and both instances record
what their grep can no longer see.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Check That Capacity Stays Out](001_the_check_that_capacity_stays_out.md) | B1's grep, BR10's weakening, and where the load-bearing assertion moved to |
| 002 | [`ring_gating` as a Dev-Dependency](002_ring_gating_as_a_dev_dependency.md) | The placement, the three tests that need it, and BR14 — half the guard goes silent on re-promotion |

### Run Both

```sh
cd "$(git rev-parse --show-toplevel)"

# B1 — no capacity anywhere in the file
grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
  | grep -nE "capacity|Capacity"

# B4 — library dependencies against the library
comm -23 \
  <( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_barrier/Cargo.toml \
     | grep -oE "^ring_[a-z_]+" | sort -u ) \
  <( grep -vE "^[[:space:]]*//" ring_barrier/src/lib.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )

# B4 — the dev-dependency against the tests
comm -23 \
  <( awk '/^\[dev-dependencies\]/{f=1;next} /^\[/{f=0} f' ring_barrier/Cargo.toml \
     | grep -oE "^ring_[a-z_]+" | sort -u ) \
  <( grep -hvE "^[[:space:]]*//" ring_barrier/tests/*.rs \
     | grep -oE "ring_[a-z_]+" | sort -u )
```

All three produce no output today.

### Both Checks Have a Degenerate Pass Mode

That is the through-line of this definition, and it is why each instance says
so rather than reporting a green result:

| | B1 | B4 half 2 |
|--|----|-----------|
| Passes when | no `Capacity` in the file | every dev-dependency is used |
| **Also** passes when | the signature gives capacity no way in — which is now the case | there are no dev-dependencies at all |
| Currently passing for | the degenerate reason | the real reason |
| Backed up by | `available_ignores_capacity_entirely` — 4 against 1,000 | B4 half 1, which fires on the same edit |

Neither degenerate mode is a defect in the check. Both are what happens when a
textual check outlives the shape of the code it was written against, and the
value of recording them is that the next reader gets the check's *current*
strength rather than its original one.

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| BR10 | `ring_barrier` | n/a — coverage | B1's grep now passes because the slice signature left capacity no way into the file, not because capacity was kept out — the manual plan says so itself |
| BR14 | `ring_barrier` | n/a — coverage | Simulated, re-promoting `ring_gating` makes B4's first command print `ring_gating` and its second go **silent** — an emptied `[dev-dependencies]` satisfies "expected: no output" by having no subject left |
| BR50 | `ring_barrier` | n/a — observation | All six mentions of capacity in the source are `//!` prose and none is code, so the guard's comment filter is the entire check — drop it and the guard fails on a correct crate, which is the right failure direction and the opposite of the two guards in this family that cannot fail at all |
| BR51 | `ring_barrier` | n/a — doc gap | The reason `ring_gating` is a dev-dependency is explained in three lines of manifest comment that no rustdoc renders, no test reads and no gate checks, while forty lines of module documentation discuss the two crates' relationship without mentioning it |
