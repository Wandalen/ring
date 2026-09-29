# Workaround: A Feature That Cannot Be Negated at the Use Site

### Scope

- **Purpose**: Record W2 — a Cargo feature is additive and global, so an optional candidate cannot be excluded locally where it is inconvenient — what the crate pays for absorbing it, and the two places the cost has already landed.
- **Responsibility**: State the constraint, the compensation, its cost, and the deletion condition.
- **In Scope**: The `crossbeam` feature; the seven `cfg` sites; the two copies of `Candidate::ALL`; the `producer_ceiling` doc table.
- **Out of Scope**: Why an off-the-shelf queue is a candidate at all (→ [`decisions/001`](../decisions/001_five_candidates_for_four_named_paths.md)); what the ceiling means (→ [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md)); W1 (→ [`001`](001_the_clock_is_the_platforms_and_the_tie_break_is_the_list.md)).

### The Constraint

Cargo features are additive and crate-global. A feature that is on is on
everywhere in the compilation, and there is no way to say "not here" at a use
site. So an optional variant is not one optional thing — it is one optional
thing plus a compile-time fork in every construct that enumerates variants, and
the fork must be written out by hand at each one.

`ring_core` already carries a `crossbeam` backend behind such a feature, and
`ring_factory` exposes it through `build_crossbeam`. This crate takes both, adds
a sixth `Candidate`, and pays for the fork.

| | |
|---|---|
| **Constraint** | Cargo features are additive and global; a variant cannot be excluded at one use site |
| **Compensation** | Every enumerating construct is written twice, once per build, and the default build is the one the suite runs |
| **Cost** | Seven `cfg` sites, two hand-maintained copies of `Candidate::ALL` carrying identical doc text, and one doc table whose arithmetic is true in only one of the two builds |
| **Deleted when** | The optional candidate becomes unconditional, or is removed. Not otherwise — the constraint is Cargo's |

### Sources

| File | Relationship |
|------|-----------------|
| [`src/lib.rs`](../../src/lib.rs) | The seven `cfg` sites and the two `ALL` copies |
| `Cargo.toml` | The one-line `[features]` section that forwards to `ring_factory` |

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_candidate.md`](../type/001_candidate.md) | `Candidate` by role, including the variant that is not always there |

### Decisions

| File | Relationship |
|------|--------------|
| [`../decisions/001_five_candidates_for_four_named_paths.md`](../decisions/001_five_candidates_for_four_named_paths.md) | Why the sixth candidate is worth its cost |

### Workarounds

| File | Relationship |
|------|--------------|
| [`001_the_clock_is_the_platforms_and_the_tie_break_is_the_list.md`](001_the_clock_is_the_platforms_and_the_tie_break_is_the_list.md) | W1, whose undocumented tie-break lives in the list this entry duplicates |

### Tests

| Test | Relationship |
|------|--------------|
| `every_candidate_declares_a_name_and_a_ceiling` | Walks `Candidate::ALL`, then asserts five ceilings by name — the sixth cannot be named |
| `a_path_that_dropped_records_is_not_eligible_to_be_fastest` | Asserts `outcomes().len() == Candidate::ALL.len()`, a length that changes with the feature |
| `the_report_names_every_candidate_and_every_refusal` | Iterates `Candidate::ALL`, so its coverage varies with the build |

### BN51 — The Ceiling Table's Arithmetic Holds in the Build the Suite Does Not Run

`producer_ceiling`'s doc counts the bounded candidates, and the count is
build-dependent:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the pre-BN51-fix summary sentence ("for three of the four..."), expect 0 -- see fix below --'
printf '    hits: %s\n' "$( command grep -c 'for three of the four bounded ones' src/lib.rs )"
echo '  -- what the match arms actually return --'
awk '/pub const fn producer_ceiling/, /^  \}/' src/lib.rs | command grep -E 'Self::|cfg' | sed 's/^/    /'
echo '  -- which ceilings the suite pins by name --'
command grep 'producer_ceiling(), ' tests/bench_test.rs | sed 's/^/    /'
```

Live output:

```
  -- the pre-BN51-fix summary sentence ("for three of the four..."), expect 0 -- see fix below --
    hits: 0
  -- what the match arms actually return --
          Self::MutexQueue | Self::DirectMpsc => None,
          Self::ContractRing | Self::TlsOverRing | Self::DirectSpsc => Some( 1 ),
          #[ cfg( feature = "crossbeam" ) ]
          Self::OffTheShelf => Some( 1 ),
  -- which ceilings the suite pins by name --
      assert_eq!( Candidate::MutexQueue.producer_ceiling(), None );
      assert_eq!( Candidate::DirectMpsc.producer_ceiling(), None );
      assert_eq!( Candidate::ContractRing.producer_ceiling(), Some( 1 ) );
      assert_eq!( Candidate::TlsOverRing.producer_ceiling(), Some( 1 ) );
      assert_eq!( Candidate::DirectSpsc.producer_ceiling(), Some( 1 ) );
      assert_eq!( Candidate::OffTheShelf.producer_ceiling(), Some( 1 ) );
```

Count the bounded candidates in the default build: `ContractRing`,
`TlsOverRing`, `DirectSpsc` — three. Of those, the door imposes the bound on
two; `DirectSpsc`'s is real, and the table says so ("the only honest 1 in the
table"). So the sentence "for three of the four bounded ones the bound belongs
to the *door*" is arithmetic about a four-element set that has three elements
unless `--features crossbeam` is passed.

**The suite cannot catch this, and the shape of why is the interesting part.**
`every_candidate_declares_a_name_and_a_ceiling` asserts five ceilings by name
and stops, because naming `Candidate::OffTheShelf` in a test would need the test
to carry its own `cfg` — and the suite carries none at all. A doc sentence about
a six-row table is therefore checked against a five-row reality by nothing, in a
crate where that table is the primary statement of the largest finding.

The row itself is accurate; it is the *summary sentence over the rows* that is
build-dependent. Prose that counts is prose that has an arity, and a `cfg` gives
a table two arities while leaving the sentence above it with one.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
command grep -A4 'None. means unbounded' src/lib.rs
```

Live output:

```
  /// `None` means unbounded. `Some( 1 )` means the candidate can only be run
  /// single-producer — and of the bounded ones, the door imposes the ceiling
  /// rather than the data structure behind it for most of them: two of three
  /// by default, three of four with the `crossbeam` feature on (the table
  /// below is the six-candidate build):
```

**Disposition:** applied — the summary sentence now carries both arities (two
of three by default, three of four with `crossbeam`) instead of one fixed
"three of the four", and names which build the table itself renders under.
The table's own rows are untouched, since the finding's own reading is that
they are accurate and only the sentence above them was not.
Now prints: `two of three`

### BN52 — One List, Written Twice by Hand, Carrying Two Copies of the Same Doc Comment

The fork is not abstract — it is a duplicated declaration:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- every cfg site in this crate --'
command grep 'feature = "crossbeam"' src/lib.rs tests/bench_test.rs | sed 's/^/    /'
echo '  -- the two declarations of ALL, with their doc lines --'
command grep -B2 'pub const ALL' src/lib.rs | sed 's/^/    /'
echo '  -- what the manifest says --'
sed -n '/^\[features\]/,/^$/p' Cargo.toml
echo '  -- and whether anything asserts the two agree --'
printf '    tests naming ALL.len()  : %s\n' "$( command grep -c 'ALL.len()' tests/bench_test.rs )"
printf '    tests with a cfg        : %s\n' "$( command grep -c 'feature = "crossbeam"' tests/bench_test.rs )"
printf '    names checked outside   : %s\n' "$( command grep -c 'const COMMON : \[ &str; 5 \]' tests/bench_test.rs )"
```

Live output:

```
  -- every cfg site in this crate --
    src/lib.rs:  #[ cfg( feature = "crossbeam" ) ]
    src/lib.rs:  #[ cfg( feature = "crossbeam" ) ]
    src/lib.rs:  #[ cfg( not( feature = "crossbeam" ) ) ]
    src/lib.rs:      #[ cfg( feature = "crossbeam" ) ]
    src/lib.rs:      #[ cfg( feature = "crossbeam" ) ]
    src/lib.rs:    #[ cfg( feature = "crossbeam" ) ]
    src/lib.rs:#[ cfg( feature = "crossbeam" ) ]
    tests/bench_test.rs:  #[ cfg( feature = "crossbeam" ) ]
    tests/bench_test.rs:  #[ cfg( feature = "crossbeam" ) ]
    tests/bench_test.rs:  #[ cfg( feature = "crossbeam" ) ]
    tests/bench_test.rs:  #[ cfg( not( feature = "crossbeam" ) ) ]
    tests/bench_test.rs:  #[ cfg( feature = "crossbeam" ) ]
  -- the two declarations of ALL, with their doc lines --
      /// a reorder of either `cfg` arm fails rather than silently changing a verdict.
      #[ cfg( feature = "crossbeam" ) ]
      pub const ALL : &'static [ Self ] =
    --
      /// [`Comparison::fastest`] — see the crossbeam-gated copy above (BN50).
      #[ cfg( not( feature = "crossbeam" ) ) ]
      pub const ALL : &'static [ Self ] =
  -- what the manifest says --
[features]
# Forwards to `ring_factory`'s crossbeam door, which gates the off-the-shelf
# candidate. Feature 186 names that candidate; feature 187 makes it optional.
# The comparison therefore has five candidates by default and six with this on,
# which `Candidate::ALL` reflects rather than hides.
crossbeam = [ "ring_factory/crossbeam" ]

  -- and whether anything asserts the two agree --
    tests naming ALL.len()  : 3
    tests with a cfg        : 5
    names checked outside   : 1
```

Seven `cfg` sites in the source: the variant, its `name` arm, its ceiling arm,
its dispatch arm in `run`, the runner function itself, and both copies of `ALL`.
Zero in the suite.

**The duplicated doc comment is the part that will rot.** Both copies of `ALL`
carry the identical line "Every candidate, in a fixed order." — which is true of
each and false of the pair, since the two orders are separate literals that
nothing compares. Rustdoc renders one of them depending on which build it was
invoked in, and `#![ deny( missing_docs ) ]` is satisfied twice over without
either copy being checked against the other.

That list is also W1's tie-break
(→ [`001`](001_the_clock_is_the_platforms_and_the_tie_break_is_the_list.md)'s
BN50). So the crate's undocumented rule for resolving a tie between two equally
fast candidates is encoded in the declaration order of a literal that exists
twice, by hand, with no assertion that the two agree — and the two tests that
touch the list's shape both compare a length back to `Candidate::ALL.len()`,
which is a tautology in either build and would pass if the two copies listed
different candidates in different orders.

The general shape: **an additive feature buys optionality by duplicating every
construct that enumerates**, and duplication of a *list* is worse than
duplication of code, because a list's meaning includes its order and nothing
type-checks an order.

The duplication is unchanged and is not the part that was fixable — one `cfg`,
one list, is what an additive feature costs. What was fixable is that nothing
compared either copy to anything outside itself.
`the_candidate_list_matches_a_copy_written_outside_the_declaration` spells the
five common names out in the test, in order, and asserts the declaration against
that; the crossbeam arm additionally pins that `off_the_shelf` is *appended*
rather than inserted, which is the specific reordering BN50's tie-break would
silently reinterpret. Reordering either copy alone now fails, and so does adding
a candidate to one and not the other — both were tried against the restored
source and both failed for the right reason.

The two tautological length checks are still there, and deliberately: they read
`ALL.len()` because that is what they are about. They are simply no longer the
only thing looking at the list.

**Disposition:** applied — the second copy still exists and still has to be
maintained by hand; what no longer holds is that its contents and its order were
unchecked. Now prints: `names checked outside   : 1`
