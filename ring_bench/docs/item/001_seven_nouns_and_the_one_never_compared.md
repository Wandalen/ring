# Item: Seven Nouns, and the One That Is Copied but Never Compared

### Scope

- **Purpose**: Catalogue the seven nouns as a set, and record the two properties that are invisible from any one of them — a refusal type whose variants disagree about whether they know who refused, and a derive list this crate does not fully own.
- **Responsibility**: State each noun's shape, its derive list, and what that list permits a consumer to ask.
- **In Scope**: `Record`, `WorkloadError`, `Workload`, `Candidate`, `Outcome`, `RunError`, `Comparison`; the two `Display` and two `Error` impls that attach to them.
- **Out of Scope**: The verbs (→ [`002`](002_forty_verbs_twenty_five_of_them_const.md)); why `Outcome` carries three counts rather than one (→ [`data_structure/002`](../data_structure/002_three_counts_that_are_not_interchangeable.md)); why `Candidate` has five variants for four named paths (→ [`decisions/001`](../decisions/001_five_candidates_for_four_named_paths.md)).

### The Set

Seven nouns, and they fall into three groups that the individual rustdoc on each
one cannot show, because each group is defined by what the *others* do.

| Group | Nouns | Derives | What a consumer can ask |
|-------|-------|---------|-------------------------|
| Full value types | `WorkloadError`, `Candidate`, `RunError` | `Debug, Clone, Copy, PartialEq, Eq` | Equality, and free duplication |
| Copied, not compared | `Workload` | `Debug, Clone, Copy` | Free duplication only |
| Owned results | `Outcome`, `Comparison` | `Debug` | Neither |

`Record` is an alias for `u64` and inherits whatever `u64` has, so it is in no
group and is the only noun this crate does not define.

**`Workload` sits alone, and the position is load-bearing.** It is the input to
every measurement in the crate, it is `Copy` so it costs nothing to hand around,
and it cannot be compared to another `Workload`. `Outcome` keeps two of its four
dimensions — `producers` and `offered` — and drops `records_per_producer` and
`batch`. So two outcomes from two different workloads are indistinguishable on
the two dimensions that differ, and the type system offers no way to check they
came from the same description. `Comparison` closes the gap by holding the
`Workload` itself, which is why [`invariant/001`](../invariant/001_received_never_exceeds_reported_never_exceeds_offered.md)'s
ordering is stated over one `Comparison` rather than over a bag of `Outcome`s.

### Sources

| File | Relationship |
|------|-----------------|
| [`src/lib.rs`](../../src/lib.rs) | Every declaration, measured rather than described |

### Types

| File | Relationship |
|------|-----------------|
| [`../type/001_candidate.md`](../type/001_candidate.md) | `Candidate` by role — the enumeration a comparison iterates |
| [`../type/002_run_error.md`](../type/002_run_error.md) | `RunError` by role — the four ways a run is refused |

### Data Structures

| File | Relationship |
|------|-----------------|
| [`../data_structure/001_the_workload_description.md`](../data_structure/001_the_workload_description.md) | `Workload` by role — the two fields both called "producers" |
| [`../data_structure/002_three_counts_that_are_not_interchangeable.md`](../data_structure/002_three_counts_that_are_not_interchangeable.md) | `Outcome` by role — why three counts rather than one |

### Items

| File | Relationship |
|------|--------------|
| [`002_forty_verbs_twenty_five_of_them_const.md`](002_forty_verbs_twenty_five_of_them_const.md) | The other half of the catalogue |

### Tests

| Test | Relationship |
|------|--------------|
| `every_error_renders` | Constructs each `RunError` variant and renders it — the closest the suite comes to exercising the anonymous three |
| `a_comparison_lists_refusals_rather_than_shortening_the_table` | Asserts refusals are kept, not that they can be attributed |
| `the_report_names_every_candidate_and_every_refusal` | The name that claims attribution; the body checks the one variant that carries it |
| `a_policy_refusal_names_the_crate_that_refused` | Names a `Display` property and asserts a variant tag |

### BN25 — Three of Four Refusal Variants Could Not Say Who Refused

`RunError` carried a candidate in one variant and not in the other three; it now
carries one in all four, which is what the census above reads:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the four variants, and which names a candidate --'
awk '/^pub enum RunError/{ p = 1; next } p && /^\}/{ exit }
     p && /^  [A-Z]/{ v = $1 }
     p && /candidate :/{ printf "    %-18s carries a candidate\n", v; seen[v] = 1 }
     p && /^  [A-Z][A-Za-z]*\(/{ printf "    %-18s does not\n", $1 }' src/lib.rs
echo '  -- what the refusal list is a list of --'
command grep 'refusals : Vec<\|pub fn refusals' src/lib.rs
echo '  -- and how the report prints one --'
command grep 'refused: ' src/lib.rs tests/bench_test.rs
```

Live output:

```
  -- the four variants, and which names a candidate --
    ProducerCeiling    carries a candidate
    Build              carries a candidate
    Ring               carries a candidate
    Flush              carries a candidate
  -- what the refusal list is a list of --
  refusals : Vec< RunError >,
  pub fn refusals( &self ) -> &[ RunError ]
  -- and how the report prints one --
src/lib.rs:      let _ = writeln!( out, "refused: {refusal}" );
tests/bench_test.rs:  assert!( parallel_report.contains( "refused: contract_ring" ) );
tests/bench_test.rs:  assert!( evicting.contains( "refused: contract_ring: " ), "{evicting}" );
tests/bench_test.rs:  assert!( evicting.contains( "refused: tls_over_ring: " ), "{evicting}" );
```

The one variant that knew was `ProducerCeiling`, which this crate constructs
itself. The three that did not were exactly the three that relay somebody else's
refusal — `ring_factory`'s, `ring_core`'s and `ring_flush`'s — and those are the
cases where a reader most needs to know which candidate was being built, because
the same `RingConfig` is offered to all five.

`Comparison::report` prints `refused: {refusal}` and `Display` forwarded to the
inner error for those three, so the line a human read was the dependency's own
message with no candidate anywhere on it. The table above it has seven columns
keyed on `candidate.name()`; the refusal lines below shared none of them.

**The test named for this property did not check it.**
`the_report_names_every_candidate_and_every_refusal` asserts
`parallel_report.contains( "refused: contract_ring" )` — a `ProducerCeiling`
refusal, the one variant that carries a name. No test in the suite builds a
`Comparison` that produces a `Build`, `Ring` or `Flush` refusal, so the
anonymous rendering has never appeared in a report the suite looked at.
`a_policy_refusal_names_the_crate_that_refused` comes closest and stops one step
short: it calls `run` directly and asserts `matches!( …, Err( RunError::Build( _ ) ) )`,
which is a variant tag, not a name.

The general shape, worth carrying: **a test name is a claim, and the two here
were claims about rendering asserted by matching on a discriminant.**

**Disposition:** applied — the three relaying variants became struct variants
carrying the candidate alongside the inner error, and `Display` now renders
`{candidate}: {inner}` for each, so a refusal line in the report is keyed on the
same name as the results table above it. Both tests named for the property now
assert the property: `a_policy_refusal_names_the_crate_that_refused` keeps its
two `matches!` tags but adds `to_string().starts_with( "contract_ring: " )` and
`starts_with( "tls_over_ring: " )`, and `the_report_names_every_candidate_and_every_refusal`
gained a `DropOldest` comparison whose report must contain
`refused: contract_ring: ` and `refused: tls_over_ring: ` — the two relayed
refusals that had never appeared in a report the suite looked at. Both were
proven able to fail by restoring `Display`'s bare forward for one variant. What
this does not buy: `Flush` still has no workload that produces it, so its
rendering is asserted on a hand-constructed value rather than on one the harness
generated — the finding's observation that no input reaches that variant is
unchanged. Now prints: `Build              carries a candidate`

### BN26 — `RunError` Is `Copy` Because Three Other Crates Are, and One of Them Reserves the Right to Stop

The derive list is four traits wide and only partly this crate's decision:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what RunError derives --'
command grep -B1 '^pub enum RunError' ring_bench/src/lib.rs | command grep derive
echo '  -- what its three payloads derive --'
command grep -B1 '^pub enum BuildError' ring_factory/src/lib.rs | command grep derive
command grep -B2 '^pub enum RingError' ring_types/src/error.rs | command grep -E 'derive|non_exhaustive'
command grep -B1 '^pub enum ConfigError' ring_flush/src/lib.rs | command grep derive
echo '  -- which family declarations carry the attribute --'
command grep -r '^#\[ non_exhaustive \]' ring_*/src/ | sed 's/:/:  /; s/^/    /'
echo '  -- and what a looser pattern would have swept in --'
command grep -r 'non_exhaustive' ring_*/src/ | command grep -vE ': *//' \
  | command grep -v '#\[ non_exhaustive \]' | sed 's/:/:  /; s/^/    /'
```

Live output:

```
  -- what RunError derives --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
  -- what its three payloads derive --
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
#[ derive( Debug, Clone, Copy, PartialEq, Eq, Hash ) ]
#[ non_exhaustive ]
#[ derive( Debug, Clone, Copy, PartialEq, Eq ) ]
  -- which family declarations carry the attribute --
    ring_testkit/src/lib.rs:  #[ non_exhaustive ]
    ring_types/src/error.rs:  #[ non_exhaustive ]
  -- and what a looser pattern would have swept in --
    ring_spsc/src/lib.rs:        .finish_non_exhaustive()
```

Three of `RunError`'s four variants wrap another crate's error, so every trait
in its derive list is a conjunction across four crates. `Copy` is the load-bearing
one: `RunError` is returned by value from `run`, collected into
`Comparison::refusals`, and read back through `&[ RunError ]` — none of which
needs `Copy`, but the derive is written and so the constraint is live.

**`RingError` is `#[ non_exhaustive ]`, one of two declarations across the
thirty-three crates that carry the attribute** — `ring_testkit::Anomaly` is the
other, and it is nowhere on `RunError`'s payload path, so this coupling is
`RingError`'s alone. The line under the looser pattern above is a `Debug`
formatter's `finish_non_exhaustive()` call, which an unanchored search for the
word sweeps in and which reserves nothing — the measurement needed the anchor
to mean what it says. That attribute is an
explicit reservation: variants may be added. A
variant carrying a `String` — a path, a message, a name — would be an ordinary,
unremarkable addition to an error type, and it would break `RunError`'s derive
in a crate that does not appear in `ring_types`' dependents by name in any
document either crate holds.

Neither side recorded the coupling. `ring_types` documented `RingError` as the
family's shared failure type and said nothing about who had derived `Copy` over
it; this crate documented `RunError` as "why a candidate could not be run" and
said nothing about what its derive list costs somebody else's freedom to change.

The general shape: **`#[ non_exhaustive ]` announces that variants may be added
and says nothing about which traits must keep holding when they are** — and the
consumer that quietly fixed one is three crates away.

**Disposition:** applied — both sides now record it. `RingError` gained an
`# Adding a Variant` section naming `ring_bench::RunError`, its `Copy` derive,
and the concrete failure mode: a variant carrying a `String` would be an
unremarkable addition there and would fail to compile three crates away. This
crate's `RunError` gained a `# What the Derive List Costs Somebody Else` section
naming all three pinned types and saying plainly that nothing here needs `Copy`
— it is written, so the constraint is live. The derive was not removed, which
was the other option: removing it would relax the coupling but silently change
`&[ RunError ]` ergonomics for any consumer, and the derive is not the defect —
the silence was. `both_halves_of_the_copy_coupling_are_named` states the
conjunction as four explicit `Copy` bounds, so the break arrives with a test name
rather than as an unsatisfied bound on an expanded impl, and additionally asserts
that both docs still carry their half of the note. What this does not buy: the
test cannot detect a *new* `#[ non_exhaustive ]` type entering the family, and
nothing generalises the check to the next one — which has since arrived.
`ring_testkit::Anomaly` carries the attribute today, no test reported it, and
the only reason it costs nothing is that no derive list in this crate is a
conjunction over it. Now prints: `ring_types/src/error.rs:  #[ non_exhaustive ]`

**Correction (2026-09-20):** the paragraph under the measurement read
"`RingError` is `#[ non_exhaustive ]`, and it is the only declaration in all
thirty-three crates that carries the attribute", and the Disposition's closing
caveat read "`RingError` remains the only declaration carrying the attribute".
Both were false, and this file printed its own disproof: the Live output four
lines above the first of them lists **two** files under
`-- which family declarations carry the attribute --`,
`ring_testkit/src/lib.rs` and `ring_types/src/error.rs`. Re-running the recipe
today reproduces that block byte-identical, so the evidence was correct and
current the whole time — `ring_testkit::Anomaly` acquired the attribute after
the prose was written and nothing ever read the two against each other. The
neighbouring sentence pointed at "the second line in the output above" to mean
the `finish_non_exhaustive()` call, an ordinal that stopped selecting that line
once the list above it grew; it now names the section instead. Nothing about
the finding moves: the `Copy` coupling is still a conjunction across four
crates, `RingError` is still the only one of the four that reserves the right
to add variants, and neither side recorded it before the **Disposition** above
made them. The caveat is in fact stronger for being corrected — the test's
blind spot is no longer hypothetical.
