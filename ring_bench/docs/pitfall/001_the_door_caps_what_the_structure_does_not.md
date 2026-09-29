# Pitfall: The Door Caps What the Structure Does Not

### Scope

- **Purpose**: Record that `ring_factory::build` caps at one producer two candidates whose underlying data structures are both multi-producer, and that comparing candidates "under the same producer counts" is therefore unsatisfiable through the export Contract alone.
- **Responsibility**: Name the trap, the failures it produces, and the mitigations, including which ones are not this crate's to apply.
- **In Scope**: The ceiling `ring_handle::Ends::split` imposes; what a comparison run at one producer does and does not establish; why `DirectMpsc` exists.
- **Out of Scope**: Whether `ring_handle` should re-expose `try_clone`, which is that crate's decision (→ [`ring_handle`](../../../ring_handle/readme.md)); the SPSC backend's own single-producer bound, which is honest and not a trap.

### Trap

**The comparison requires the candidates to be compared under the same conditions:**

> One workload run against every candidate write path […] under the same
> producer counts, batch sizes, and payloads.

**Two of the candidates cannot be driven above one producer, and in neither case
is the limit theirs.** Verify against the two crates' own surfaces:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'fn try_clone' ring_core/src/lib.rs
command grep 'fn try_clone' ring_handle/src/lib.rs || true
```

Live output:

```
  pub fn try_clone( &self ) -> Option< Producer< 'a, T > >
```

`ring_factory::build` returns a `ring_handle::Split`. Its `Ends::split` yields
exactly one producer and one consumer, and `ring_handle` deliberately does not
re-expose `ring_core::Producer::try_clone` — the operation that would hand back
a second producer for the MPSC and crossbeam backends. That crate's own
documentation states the omission as intentional:

> There is no operation here that yields a second producer, which is the
> difference from `ring_core::Producer::try_clone`.

So the ceiling table reads:

| Candidate | Ceiling | Imposed by |
|---|---|---|
| `MutexQueue` | none | — |
| `ContractRing` | 1 | `ring_handle::Ends::split` |
| `TlsOverRing` | 1 | `ring_flush::Flusher` owns its producer, and staging is per-thread by definition |
| `DirectSpsc` | 1 | **the backend** — the only honest 1 in the table |
| `DirectMpsc` | none | — |
| `OffTheShelf` | 1 | `ring_handle` again; `ArrayQueue` itself is multi-producer |

**Exactly one of the four `1`s is the structure's, and nothing in a report
distinguishes it from the other three.** Two are the export door's
(`ContractRing`, `OffTheShelf` — both reach a structure that is multi-producer
underneath), one is `ring_flush`'s (`TlsOverRing` — `ring_core::Producer::try_clone`
would hand out more, but a `Flusher` owns exactly one), and one is real
(`DirectSpsc`, where `try_clone` returns `None` by construction).

**So the column is four identical `1`s standing for three different causes.**
A comparison table listing `contract_ring` at
one producer and `mutex_queue` at four does not say whether the ring *cannot* do
four or was merely not asked; the reader supplies "cannot", because that is what
a benchmark table normally means.

**The trap is sharper than an omission, because the single-producer number is a
real measurement of a real path.** `ContractRing` at one producer is exactly
what a Contract-bound consumer gets. Nothing in the figure is wrong. What is
wrong is the sentence a reader forms from a table of six rows in which one
column silently means two different things.

### Failure

| # | Failure | How it presents |
|---|---------|-----------------|
| C1 | A four-producer comparison omits the Contract candidates | A shorter table. **Reads as "these paths were not interesting" rather than "these paths were unreachable"** |
| C2 | A reader concludes the in-house ring does not scale | The MPSC ring scales; the door does not expose it. The conclusion is about `ring_handle` and is attributed to `ring_mpsc` |
| C3 | The comparison is graded reached on a single-producer run | Every candidate runs, every count is right, and the requirement "under the same producer counts" was satisfied by choosing the count at which nothing is excluded |
| C4 | A consumer reads the comparison and adopts the Contract | They get the measured behaviour exactly. **This is the failure that does not happen** — the ceiling is real for them too |
| C5 | `ring_handle` later adds `try_clone` | The ceilings change and no test in this crate fails, because a `Some( 1 )` that becomes `None` only widens what is admitted |

**C3 is the one that matters, because it is invisible and self-inflicted.** The
easiest way to make every candidate run is to run them all at one producer, and
a harness that did so would report six rows, no refusals, and a green gate,
having quietly answered a different question than the feature asked.

**C1 is what the implementation does instead**, and it is why
`Comparison::refusals` exists at all: a candidate excluded by the producer count
appears in the report with its reason, so a four-producer run says *which* paths
could not be reached rather than showing a shorter table.
→ [`pattern/002`](../pattern/002_a_refusal_is_a_row.md).

**C4 is listed because it is the reassuring case and it is genuinely
reassuring.** Nothing here misleads a *consumer*; the ceiling they would hit is
the ceiling measured. The gap is only between what the table shows and what a
reader infers about the data structure, and readers of benchmark tables infer
about data structures.

### Mitigation

**What does not work:**

| Attempt | Why it fails |
|---------|--------------|
| Run everything at one producer | C3. Satisfies the letter of "the same producer counts" by choosing the count that excludes nothing, and answers a question nobody asked |
| Clone the `Split` | It owns the ring. Two `Split`s are two rings, and the candidate under test becomes something else |
| Re-expose `try_clone` through `ring_handle` | That crate's decision, deliberately made, with the omission documented in its own source. One consumer does not amend it |
| Drop the Contract candidates from multi-producer runs silently | C1's failure with the evidence removed |
| Note the ceiling in prose only | A note is not graded. The next person to widen the workload gets the same surprise |

**What works:**

1. **`Candidate::producer_ceiling` is a value, not a comment.** Every candidate
   declares its ceiling; `run` checks it before building anything, so a refusal
   costs no allocation and carries the two numbers that explain it. The
   documentation of *which layer* imposed each ceiling lives on that method,
   next to the values it explains.
2. **`Candidate::DirectMpsc` makes the gap measurable.** It is the same ring as
   `ContractRing`, in the same multi-producer configuration, reached two levels
   lower. A four-producer comparison therefore contains both a row for the ring
   and a refusal for the door, and the difference between them is the price of
   the Contract stated as a number rather than as a paragraph.
   → [`decisions/001`](../decisions/001_five_candidates_for_four_named_paths.md).
3. **The suite asserts the ceiling rather than describing it — locally.**
   `the_contract_door_caps_a_multi_producer_structure_at_one_producer` runs the
   parallel workload against both, asserts the exact `ProducerCeiling` refusal
   from one and a lossless four-producer run from the other. That pins this
   crate's *transcription* of the ceiling, not the ceiling. C5 is a change in
   `ring_handle`, which is not a dependency of this crate, so nothing here can
   observe it — see BN41 below, which retracts the stronger claim this item used
   to make.

**None of the three closes C3 by itself** — a future harness could still choose
a single-producer workload and report six clean rows. What closes it is that the
suite's own fixtures include a four-producer one and assert the refusal list is
non-empty, so "everything ran" is a state the tests would have to be edited to
reach.

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_workload_description.md](../data_structure/001_the_workload_description.md) | Where the producer count lives, and why it and `RingConfig::producers` are set together |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/001_five_candidates_for_four_named_paths.md](../decisions/001_five_candidates_for_four_named_paths.md) | Mitigation 2 — the ADR that added `DirectSpsc` and `DirectMpsc` |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_declared_edges_and_the_three_that_were_missing.md](../integration/001_declared_edges_and_the_three_that_were_missing.md) | The other Contract gap found here — `ring_flush` and `ring_factory` do not compose |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_a_refusal_is_a_row.md](../pattern/002_a_refusal_is_a_row.md) | C1's mitigation — the excluded candidate appears with its reason |

### Pitfalls

| File | Relationship |
|------|--------------|
| [003_ok_is_not_kept_and_the_verdict_inverts.md](003_ok_is_not_kept_and_the_verdict_inverts.md) | The other trap whose failure is a wrong recommendation rather than a wrong record |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_candidate.md](../type/001_candidate.md) | The enumeration carrying `producer_ceiling`, and the table of which layer imposes each |
| [../type/002_run_error.md](../type/002_run_error.md) | `ProducerCeiling`, the refusal this trap produces |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_handle/src/lib.rs`](../../../ring_handle/src/lib.rs) | `Ends::split` and the documented omission of `try_clone`, quoted above |
| [`ring_core/src/lib.rs`](../../../ring_core/src/lib.rs) | `Producer::try_clone` — `None` for SPSC, `Some` for MPSC and crossbeam |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/bench_test.rs`](../../tests/bench_test.rs) | `the_contract_door_caps_a_multi_producer_structure_at_one_producer` asserts the refusal and the direct run side by side; `every_candidate_declares_a_name_and_a_ceiling` transcribes all six ceiling values, which catches a local edit to the `match` but **not** C5 (BN41); `a_comparison_lists_refusals_rather_than_shortening_the_table` asserts C1's mitigation |

### BN41 — The Guard Named Against C5 Cannot See C5, and Pins the Value C5 Would Falsify

C5 is the failure where `ring_handle` later gains `try_clone`. The document says
three things about it, from three sections, and they do not agree:

```sh
cd "$(git rev-parse --show-toplevel)"
D=ring_bench/docs/pitfall/001_the_door_caps_what_the_structure_does_not.md
printf -- '--- three statements about C5, by line ---\n'
for phrase in 'no test in this crate fails' 'so C5 turns into a' 'so C5 breaks a test'; do
  printf '  line %-4s | %s\n' \
    "$( awk '/^### BN/{ exit } { print NR ":" $0 }' "$D" | command grep -F "$phrase" \
        | command grep -oE '^[0-9]+' | tr '\n' ' ' )" "$phrase"
done
printf -- '--- what the named test pins, literally ---\n'
awk '/fn every_candidate_declares_a_name_and_a_ceiling/, /^\}/' ring_bench/tests/bench_test.rs \
  | command grep -E 'assert_eq!\( Candidate' | sed 's/^ */  /'
printf '  candidates pinned : %s of %s in ALL (crossbeam build)\n' \
  "$( awk '/fn every_candidate_declares_a_name_and_a_ceiling/,/^\}/' ring_bench/tests/bench_test.rs \
      | command grep -c 'assert_eq!( Candidate.*producer_ceiling' )" \
  "$( awk '/pub const ALL/{ inb = 1; c = 0 } inb && /Self::/{ c++ } inb && /\];/{ print c; exit }' \
      ring_bench/src/lib.rs )"
printf -- '--- how this crate reaches the crate that imposes the ceiling ---\n'
printf '  ring_handle in ring_bench/Cargo.toml : %s\n' "$( command grep -c 'ring_handle' ring_bench/Cargo.toml )"
printf '  ring_handle in ring_bench/src        : %s (all in prose)\n' "$( command grep -c 'ring_handle' ring_bench/src/lib.rs )"
printf '  fn try_clone, family-wide            : %s\n' "$( command grep -rl 'fn try_clone' ring_*/src/ | sed 's|.*/ring/||' | tr '\n' ' ' )"
printf -- '--- the ceilings themselves ---\n'
awk '/fn producer_ceiling/, /^  \}/' ring_bench/src/lib.rs | command grep -E 'Self::' | sed 's/^ */  /'
```

Live output:

```
--- three statements about C5, by line ---
  line 79   | no test in this crate fails
  line      | so C5 turns into a
  line      | so C5 breaks a test
--- what the named test pins, literally ---
  assert_eq!( Candidate::MutexQueue.producer_ceiling(), None );
  assert_eq!( Candidate::DirectMpsc.producer_ceiling(), None );
  assert_eq!( Candidate::ContractRing.producer_ceiling(), Some( 1 ) );
  assert_eq!( Candidate::TlsOverRing.producer_ceiling(), Some( 1 ) );
  assert_eq!( Candidate::DirectSpsc.producer_ceiling(), Some( 1 ) );
  assert_eq!( Candidate::OffTheShelf.producer_ceiling(), Some( 1 ) );
  candidates pinned : 6 of 6 in ALL (crossbeam build)
--- how this crate reaches the crate that imposes the ceiling ---
  ring_handle in ring_bench/Cargo.toml : 0
  ring_handle in ring_bench/src        : 6 (all in prose)
  fn try_clone, family-wide            : ring_core/src/lib.rs 
--- the ceilings themselves ---
  Self::MutexQueue | Self::DirectMpsc => None,
  Self::ContractRing | Self::TlsOverRing | Self::DirectSpsc => Some( 1 ),
  Self::OffTheShelf => Some( 1 ),
```

`producer_ceiling` is a hand-written `match` returning literals. `ring_handle` is
not a dependency of this crate — it appears zero times in the manifest, and every
mention in `src/` is prose in a doc comment. So the event C5 describes is not
observable here at all: if `ring_handle` gained `try_clone` tomorrow,
`ContractRing.producer_ceiling()` would still return `Some( 1 )`, `run` would
still refuse it at four producers, the report would still print *"refused:
contract_ring admits 1 producer(s)"*, and every test would still pass.

**The assertion offered as the mitigation was what made it worse.**
`every_candidate_declares_a_name_and_a_ceiling` pins the five literals by hand,
`ContractRing → Some( 1 )` among them. That is not a guard against the ceiling
going stale — it is the stale value written down a second time, so correcting
the crate after the door widened would mean editing the test that was named as
the reason the correction could not be forgotten. C5's own row is the accurate
one: *"no test in this crate fails."* The two sentences claiming otherwise were in
the Mitigation list and the Tests table — the empty line numbers above are what
their retraction looks like.

Two smaller cracks in the same place. "Asserts each ceiling value" was five of
six — `OffTheShelf`, the other candidate whose `1` belongs to `ring_handle`, has
no literal assertion, only the generic loop that checks a ceiling is *consistent
with* `admits`. And the loop's `None => assert!( candidate.admits( 64 ) )` branch
means a ceiling that widened from `Some( 1 )` to `None` would pass the loop and
fail only the literal — which is the direction C5 predicts, guarded by the one
assertion the document was describing when it said the opposite.

**The general shape:** a ceiling owned by another crate, transcribed by hand into
this one, cannot be asserted *against its source* by a suite that does not
depend on that source. What the suite can assert is that the transcription still
says what it said, which is a test for local edits and reads as a test for the
remote change.

**Disposition:** applied — as a retraction plus a completion, not as a new guard.
The two sentences that claimed a test catches C5 are gone: the Mitigation item
now says it pins this crate's transcription rather than the ceiling and points
here, and the Tests table row now says the test catches a local edit to the
`match` and explicitly **not** C5. C5's own row was already accurate and is
unchanged. The three-statement census above is the evidence — two of the three
phrases no longer resolve to a line. The smaller crack is closed too:
`every_candidate_declares_a_name_and_a_ceiling` gained the sixth literal, so
`OffTheShelf` — whose `1` has the same off-crate owner — is pinned rather than
left to the generic loop whose `None` branch a widened ceiling would pass; the
test now carries a comment saying, in the file a maintainer edits, that these
lines are a transcription check and why. What this does not buy, and this is the
finding's own point: nothing here can observe C5. `ring_handle` is still not a
dependency, and adding one purely so a test could read its surface would be a
dependency taken for a hypothetical. The honest state is a transcription that
says it is a transcription; making it verifiable against its source needs the
ceiling to be published by the crate that owns it, which is a change to
`ring_handle`, not to this one. Now prints: `candidates pinned : 6 of 6 in ALL (crossbeam build)`

