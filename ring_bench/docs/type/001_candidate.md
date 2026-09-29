# Type: Candidate

### Scope

- **Purpose**: Define the enumeration of write paths under comparison, establish that it has more variants than named paths, and record why each variant's producer ceiling is a value rather than a comment.
- **Responsibility**: State the variants, the derives, and the validation rules.
- **In Scope**: The six variants; `ALL`; `name`; `producer_ceiling`; `admits`; the cargo feature gating the sixth.
- **Out of Scope**: Why there are five rather than four (→ [`decisions/001`](../decisions/001_five_candidates_for_four_named_paths.md)); where the ceilings come from (→ [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md)).

### Definition

```rust
pub enum Candidate
{
  MutexQueue,
  ContractRing,
  TlsOverRing,
  DirectSpsc,
  DirectMpsc,
  #[ cfg( feature = "crossbeam" ) ]
  OffTheShelf,
}
```

| Variant | Reaches | Through |
|---|---|---|
| `MutexQueue` | `Mutex< VecDeque< Record > >` | the standard library — the baseline every other candidate has to beat to justify existing |
| `ContractRing` | the in-house ring | `ring_factory::build` — what a Contract-bound consumer gets |
| `TlsOverRing` | the in-house ring, staged | `ring_tls::TlsBuffer` + `ring_flush::Flusher`, over a `ring_core::Ring` built directly |
| `DirectSpsc` | `ring_spsc::Ring` | `with_config`, two levels below the Contract |
| `DirectMpsc` | `ring_mpsc::Ring` | the same, and the only in-house candidate with no producer ceiling |
| `OffTheShelf` | crossbeam's `ArrayQueue` | `ring_factory::build_crossbeam`, behind a cargo feature |

**The baseline is bounded to the workload's capacity**, so it competes under the
same back-pressure as the rings rather than absorbing the whole load. An
unbounded `VecDeque` would win every cramped fixture by not being a queue with
a capacity, which measures nothing.

### `ALL` Is Two Constants

```rust
#[ cfg( feature = "crossbeam" ) ]
pub const ALL : &'static [ Self ] = &[ /* six */ ];
#[ cfg( not( feature = "crossbeam" ) ) ]
pub const ALL : &'static [ Self ] = &[ /* five */ ];
```

**The comparison has five candidates by default and six with the feature on, and
`ALL` reflects that rather than hiding it.** The alternative — one constant with
a `cfg`'d element — is the same thing written less legibly; the alternative that
was rejected is a runtime `Vec` filtered by a feature flag, which would make
`ALL` non-`const` and the accounting identity
(`outcomes + refusals == ALL.len()`) a runtime property rather than a
structural one.

The off-the-shelf backend is optional family-wide; this crate's
`crossbeam` feature forwards to `ring_factory/crossbeam` rather than declaring
its own dependency, so there is one switch and not two.

### `producer_ceiling` Carries the Attribution

```rust
pub const fn producer_ceiling( self ) -> Option< usize >
```

`None` is unbounded; `Some( 1 )` is single-producer. **Three variants return
`Some( 1 )` by default — four with the `crossbeam` feature on — and only one
of them is bounded by its own data structure** — the method's own rustdoc
carries the table naming which layer imposes each, because a table of
ceilings without attribution reads as facts about every data structure and is
wrong for most of them: three-fifths by default, four-sixths with the feature
on.
→ [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md).

`admits( n )` is the predicate form: `n <= ceiling`, or `true` when unbounded.
`run` calls it before building anything, so a refusal costs no allocation.

### Validation

**Nothing to validate — the type is a fieldless enumeration — but three
properties hold and are asserted:**

| Rule | Holds because | Checked by |
|---|---|---|
| Every candidate has a unique name | Hand-written `match`, one arm per variant | `every_candidate_declares_a_name_and_a_ceiling` sorts and dedups |
| Every candidate admits one producer | Every ceiling is `None` or `≥ 1` | the same test, per variant |
| A bounded candidate refuses `ceiling + 1` | `admits` is `<=` | the same test, per variant |

**The uniqueness check exists because `name()` feeds the report**, and two
candidates sharing a name would produce two rows a reader cannot tell apart —
with no compile error, since the strings are unrelated to the variants.

### Derives

`Debug, Clone, Copy, PartialEq, Eq`. `Copy` because it is a tag passed by value
into `run` and stored in every `Outcome`; `PartialEq` because `RunError::ProducerCeiling`
carries one and tests compare the whole error by value.

**No `Default`.** There is no candidate a comparison should silently fall back
to, and a default would make `Candidate::default()` a quiet answer to "which
write path?" — which is the question this crate exists to ask.

**No `Hash`, no ordering.** Nothing keys a map by candidate, and an `Ord`
instance would invite sorting the report by variant order, which is already the
report's order for a different reason.

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_run_surface.md](../api/001_the_run_surface.md) | Where `ALL` and `admits` are consumed |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_workload_description.md](../data_structure/001_the_workload_description.md) | The producer count `admits` is checked against |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_five_candidates_for_four_named_paths.md](../decisions/001_five_candidates_for_four_named_paths.md) | Why the enumeration has more variants than the feature has named paths |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/001_the_door_caps_what_the_structure_does_not.md](../pitfall/001_the_door_caps_what_the_structure_does_not.md) | The attribution table, and what a report without it implies |

### State Machines

| File | Relationship |
|------|--------------|
| [../lifecycle/002_one_candidate_through_one_run.md](../lifecycle/002_one_candidate_through_one_run.md) | The states a variant passes through |

### Types

| File | Relationship |
|------|--------------|
| [002_run_error.md](002_run_error.md) | `ProducerCeiling`, which carries a `Candidate` by value |

### Sources

| File | Relationship |
|------|--------------|
| [`../../../ring_factory/readme.md`](../../../ring_factory/readme.md) | `build_crossbeam`, the door the sixth candidate reaches through, and the feature that gates it |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `every_candidate_declares_a_name_and_a_ceiling` asserts all three validation rules and every ceiling by value, so a change to any of them breaks a test rather than silently altering the comparison |

### BN45 — This Document Names the Two Builds in a Section Heading, Then Spends Three Bare Cardinals

`### ALL Is Two Constants` states the fact plainly: *"The comparison has five
candidates by default and six with the feature on, and `ALL` reflects that rather
than hiding it."* Twenty lines later the next section opens **"Four variants
return `Some( 1 )`"** and closes on **"four-sixths wrong"** — a fraction with the
build baked into both halves. In the build a bare `cargo test -p ring_bench`
compiles, it is three variants and three-fifths.

That is one document. The habit is the crate's:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
printf -- '--- bare cardinals for a build-dependent quantity, findings excluded ---\n'
for f in $( find docs -name '*.md' -not -path 'docs/definition/*' | sort ) readme.md; do
  awk -v F="$f" '/^### BN|^### Findings Recorded Here/{ exit } { print F ":" NR ":" $0 }' "$f"
done | command grep -oE '^[^:]+:[0-9]+:.*(four|five|six)[ -](of six|of five|variants|candidates|rows|refusals?|sixths|fifths)' \
  | sed -E 's/^([^:]+):([0-9]+):.*((four|five|six)[ -](of six|of five|variants|candidates|rows|refusals?|sixths|fifths))$/  \1:\2  \3/' \
  | sort | uniq
printf -- '--- and how many documents say which build they mean ---\n'
printf '  .md files under docs/, index excluded        : %s\n' "$( find docs -name '*.md' -not -path 'docs/definition/*' | wc -l )"
printf '  naming the feature or --all-features anywhere : %s\n' \
  "$( for f in $( find docs -name '*.md' -not -path 'docs/definition/*' | sort ); do
        awk '/^### BN|^### Findings Recorded Here/{ exit } { print }' "$f" \
          | command grep -q 'features crossbeam\|all-features\|feature = "crossbeam"\|`crossbeam` feature' && echo "$f"
      done | wc -l )"
printf '  the two ALL lengths                          : %s\n' \
  "$( awk '/pub const ALL/{ inb = 1; c = 0 } inb && /Self::/{ c++ } inb && /\];/{ printf "%s ", c; inb = 0 }' src/lib.rs )"
```

Live output:

```
--- bare cardinals for a build-dependent quantity, findings excluded ---
  docs/algorithm/001_one_workload_through_six_runners.md:56  six candidates
  docs/api/001_the_run_surface.md:138  four refusals
  docs/data_structure/001_the_workload_description.md:183  four refusals
  docs/data_structure/001_the_workload_description.md:97  five rows
  docs/decisions/002_no_test_asserts_an_ordering.md:92  five candidates
  docs/decisions/readme.md:38  four candidates
  docs/integration/001_declared_edges_and_the_three_that_were_missing.md:101  four candidates
  docs/item/001_seven_nouns_and_the_one_never_compared.md:8  five variants
  docs/item/002_forty_verbs_twenty_five_of_them_const.md:72  four of six
  docs/item/readme.md:23  four rows
  docs/item/readme.md:77  four variants
  docs/lifecycle/001_from_a_description_to_a_verdict.md:42  six candidates
  docs/lifecycle/002_one_candidate_through_one_run.md:41  six candidates
  docs/non_functional_requirement/001_the_comparison_is_reproducible_and_same_conditions.md:105  four candidates
  docs/pattern/002_a_refusal_is_a_row.md:12  six candidates
  docs/pattern/002_a_refusal_is_a_row.md:23  six candidates
  docs/pattern/002_a_refusal_is_a_row.md:87  four refusal
  docs/pitfall/001_the_door_caps_what_the_structure_does_not.md:68  six rows
  docs/pitfall/001_the_door_caps_what_the_structure_does_not.md:83  six rows
  docs/type/001_candidate.md:147  six variants
  docs/type/001_candidate.md:48  five candidates
  docs/type/001_candidate.md:71  four-sixths
  docs/type/001_candidate.md:7  six variants
  docs/type/readme.md:7  four variants
  docs/type/readme.md:8  four variants
--- and how many documents say which build they mean ---
  .md files under docs/, index excluded        : 41
  naming the feature or --all-features anywhere : 7
  the two ALL lengths                          : 6 5 
```

Not all of these are errors — "four candidates" in `decisions/readme.md` and
`integration/001` means the four named write paths, which is a different
four and correct under either build. That is the point rather than a mitigation of it: **the same cardinals
denote several different subsets, and none of them carries the qualifier that
would tell them apart.** A reader who meets "four variants" in `item/readme.md`,
"four of six" in `item/002`, "four candidates" in `integration/001` and
"four-sixths" here has no local way to know that the first three are subsets of
different sizes and the fourth is a ratio that changes with a cargo flag.

Four documents out of forty-one name a build in their own prose, and they are
the ones whose subject *is* the feature. The other thirty-seven inherit whichever
build their author happened to have compiled, and the two builds are both
routinely used — `verb/test` passes `--all-features`, a bare `cargo test -p
ring_bench` does not.

**The general shape:** a quantity that varies with a build flag is not a number,
and prose has no type system to stop it being written as one. The place this
document already solved it is `ALL` itself — two constants, both `const`, so the
compiler picks. Every prose cardinal downstream of that decision reintroduces by
hand the ambiguity the two constants removed.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
# -m1 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m1 'three-fifths by default, four-sixths' docs/type/001_candidate.md
```

Live output:

```
wrong for most of them: three-fifths by default, four-sixths with the feature
```

**Disposition:** applied — `producer_ceiling`'s own paragraph now states both
counts with their build attribution (three by default, four with `crossbeam`
on) rather than the six-candidate-only "four variants"/"four-sixths" pair.
What is not fixed: this crate's the other 19 bare-cardinal sites the finding's
own survey lists (`api/001`, `data_structure/001`, `decisions/readme`,
`integration/001`, `item/001`, `item/002`, `item/readme`, `lifecycle/001`,
`lifecycle/002`, `pattern/002`, `pitfall/001` — some already build-qualified
by this window's other fixes, most not) — a full sweep is a separate,
larger edit than this one document's own finding calls for.
Now prints: `three-fifths by default, four-sixths`

### BN46 — One `name()` String Is a Load-Bearing Identifier, and the Validation Section Says There Is Nothing to Validate

`### Validation` opens *"Nothing to validate — the type is a fieldless
enumeration"* and lists three properties, one of which is that names are unique.
Uniqueness is not the property that matters most about these strings.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
printf -- '--- the six strings ---\n'
awk '/fn name/, /^  \}/' src/lib.rs | command grep -E '=>' | sed 's/^ */  /'
printf -- '--- every place a name is compared against a literal ---\n'
command grep -r 'name() ==\|name() !=' src/ tests/ examples/ | sed 's/^/  /'
printf -- '--- what runs that file ---\n'
printf '  #[ test ] fns in examples/comparison.rs : %s\n' "$( command grep -c '#\[ test \]' examples/comparison.rs )"
printf '  the word example in tests/             : %s\n' "$( command grep -rc 'example' tests/ | tr '\n' ' ' )"
printf '  [[example]] sections in Cargo.toml     : %s\n' "$( command grep -c '\[\[example\]\]' Cargo.toml )"
printf -- '--- what now fails if the literal stops matching ---\n'
command grep -c 'the_example_switches_on_a_name_a_candidate_returns' tests/bench_test.rs \
  | sed 's/^/  citations of the guard in tests: /'
printf -- '--- what the example prints if the literal stops matching ---\n'
python3 -c "m = ( 1 << 128 ) - 1; print( f'  spread : {m} - 0 ns' ); print( f'  ratio  : {0.0 / float( m ):.1f}x' )"
```

Live output:

```
--- the six strings ---
  Self::MutexQueue => "mutex_queue",
  Self::ContractRing => "contract_ring",
  Self::TlsOverRing => "tls_over_ring",
  Self::DirectSpsc => "direct_spsc",
  Self::DirectMpsc => "direct_mpsc",
  Self::OffTheShelf => "off_the_shelf",
--- every place a name is compared against a literal ---
  examples/comparison.rs:      if outcome.candidate().name() == "contract_ring"
--- what runs that file ---
  #[ test ] fns in examples/comparison.rs : 0
  the word example in tests/             : tests/manual/readme.md:0 tests/bench_test.rs:9 
  [[example]] sections in Cargo.toml     : 0
--- what now fails if the literal stops matching ---
  citations of the guard in tests: 2
--- what the example prints if the literal stops matching ---
  spread : 340282366920938463463374607431768211455 - 0 ns
  ratio  : 0.0x
```

`examples/comparison.rs` selects the candidate whose run-to-run spread it reports
by string equality against `"contract_ring"`. Rename that arm of `name()` — a
one-line edit inside the very `match` this document calls hand-written — and
every consequence is silent. It compiles; `Candidate::ContractRing` still exists
and nothing references the string type-safely. The uniqueness test still passes,
because six distinct strings are still six distinct strings. Every other test
passes, because none of them reads the example. `cargo nextest run` does not run
examples, and the file declares no tests of its own.

What the reader gets instead is the output above: `lowest` never leaves
`u128::MAX`, `highest` never leaves `0`, and the division prints a ratio of
`0.0x` under a heading that says `contract_ring's own spread`. The program exits
0. There is no candidate named in the output that could tip anyone off, because
the label is a format string, not the value that was matched.

**The rule the Validation table is missing is stability, not uniqueness.** These
six strings are the report's column labels *and* one of them is a program
identifier, and only the first role is guarded. The second role was created in a
file the documentation does not list — see
[`pattern/001`](../pattern/001_the_measurement_is_a_value.md) — which is how a
type documented as having nothing to validate acquired an external consumer of
its string values.

The second role is now guarded too.
`the_example_switches_on_a_name_a_candidate_returns` reads the example's own
source with `include_str!` and asserts the literal it compares against is exactly
what `Candidate::ContractRing::name` returns — so the rename that used to be
silent now fails a test rather than printing a `usize::MAX` spread. Renaming the
arm and renaming the literal were both tried against the restored source and each
failed on its own.

What did not change is the shape that produced the gap. `### Validation` still
opens on uniqueness, the example is still not a test target, and nothing stops a
*second* file acquiring the same dependency on a `name()` string without a guard
of its own — the guard is per-consumer, written by hand, and there is one
consumer. Stability is now enforced for the one string that has an external
reader, not established as a property of the six.

**Disposition:** applied — the one load-bearing identifier has a test that fails
on either side of the rename; the Validation section's claim about the type is
unchanged, because it was never the type that needed validating. Now prints: `citations of the guard in tests: 2`

