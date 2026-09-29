# Decision: Five Candidates for Four Named Paths

**Status:** accepted, 2026-08-28. Rules the question of how many `Candidate`
variants the comparison carries, which
[`readme.md`](readme.md) records as Closed 1.

### Scope

- **Purpose**: Rule how many `Candidate` variants stand for the four named write paths, given that one of them cannot be reached at four producers through its own door.
- **Responsibility**: The measurement that forced the question, the four shapes considered, the three-variant answer, and what it costs.
- **In Scope**: Variant count; which door each variant uses; the ceilings; the below-Contract dependencies the decision requires.
- **Out of Scope**: Why the ceilings are where they are (→ [`../pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md)); what the suite may assert about the resulting numbers (→ [`002_no_test_asserts_an_ordering.md`](002_no_test_asserts_an_ordering.md)).

### What forced it

**A measurement, not an argument.** This crate compares four candidate write paths:

> a mutex-guarded queue, an off-the-shelf concurrent queue, the in-house ring,
> and thread-local staging over that ring — under the same producer counts,
> batch sizes, and payloads

The obvious mapping is one variant each. It was written that way first, and the
four-producer workload refused to run: "the in-house ring" reached through
`ring_factory` is capped at **one** producer, because
`ring_handle::Ends::split` yields a single non-clonable producer.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'fn try_clone' ring_core/src/lib.rs
command grep 'fn try_clone' ring_handle/src/lib.rs || true
```

Live output:

```
  pub fn try_clone( &self ) -> Option< Producer< 'a, T > >
```

So the feature's two requirements are in tension: **"the in-house ring" and
"under the same producer counts" cannot both be honoured through one door.**
→ [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md).

### Options

| # | Shape | Consequence |
|---|-------|-------------|
| A | One `Ring` variant, through the Contract, run at one producer | Satisfies "the same producer counts" by choosing the count at which nothing is excluded. The multi-producer question — the one this comparison exists to answer — is never asked |
| B | One `Ring` variant, reached directly, bypassing the Contract | Measures the ring the Contract does not give you. The number is real and describes a path no consumer can take |
| C | One `Ring` variant that switches door by producer count | The same row in the report means two different things depending on the workload, and nothing in the table says which |
| ✅ **D** | **Three variants — `ContractRing`, `DirectSpsc`, `DirectMpsc`** | Three rows, three ceilings, and the difference between them **is** the price of the Contract, stated as a measurement |

**A is what a harness written to pass its own gate would do**, and it is the
option this ADR exists to foreclose. Every candidate runs, the report has no
refusals, the gate goes green, and the question the comparison was commissioned
to settle is quietly not asked. Nothing in the stated requirement detects it.

**C is the subtle one.** Routing by producer count reads as helpful — use the
door that works — and produces a table whose `ring` row is the Contract ring at
one producer and the raw ring at four. Two measurements of different software
under one label. Rejected for the same reason `ring_factory` refused to route
between its two backends on the overflow policy: *the same name must not mean
different code depending on the input*.
→ [`ring_factory/docs/decisions/002`](../../../ring_factory/docs/decisions/002_two_doors_not_one_that_routes.md).

### The decision

**Three in-house variants, and the ceilings are values rather than comments.**

| Variant | Door | Ceiling | What it prices |
|---|---|---|---|
| `ContractRing` | `ring_factory::build` | 1 | What a Contract-bound consumer actually gets |
| `DirectSpsc` | `ring_spsc::Ring::with_config` | 1 | The dispatch `ring_core` and `ring_handle` add over the bare backend |
| `DirectMpsc` | `ring_mpsc::Ring::with_config` | none | The ring's own multi-producer behaviour, which no door exposes |

`DirectSpsc` and `ContractRing` differ in the layers between the caller and the
same data structure, so most of the gap between their timings is the wrapper's
cost — but not all of it: `ring_spsc` has no batch API, so `DirectSpsc`
publishes one record at a time while `ContractRing` publishes a whole batch per
call through the same underlying ring, which credits the wrapper for some of
the batch API's own benefit (→ BN16, and
[`../algorithm/001`](../algorithm/001_one_workload_through_six_runners.md)'s
batching section, which states the asymmetry directly). `DirectMpsc` and
`ContractRing` differ in door *and* backend, which is why `DirectSpsc` has to
exist: without it the Contract's cost and the backend's difference are one
unattributable number.

**`OffTheShelf` stays a single variant behind a cargo feature**, because it has
one door and is optional family-wide. It inherits the
same ceiling as `ContractRing`, from the same place, and the report says so.

### What it costs

**The report has more rows than the feature has candidates**, which is a real
cost: a reader looking for "the in-house ring" finds three. Mitigated by
`Candidate::producer_ceiling`'s own documentation carrying the table of which
layer imposes each ceiling, so the three rows are self-explaining rather than
requiring this ADR to be read alongside the output.

**It also makes the crate depend on `ring_spsc`, `ring_mpsc` and `ring_slot`
directly**, below the export Contract. That is deliberate and permitted — gate
G5 skips `ring_*` manifests precisely so an in-family crate may reach
inside the family — but it means this crate is not an example of Contract-bound
consumption and should not be read as one.
→ [`integration/001`](../integration/001_declared_edges_and_the_three_that_were_missing.md).

### What would reverse it

`ring_handle` exposing a way to obtain a second producer. `ContractRing` would
lose its ceiling, `DirectMpsc`'s reason for existing would narrow to pricing
dispatch (which `DirectSpsc` already does at one producer), and the honest shape
would be two variants rather than three.

`every_candidate_declares_a_name_and_a_ceiling` asserts each ceiling by value,
so that change breaks a test rather than silently making a row redundant.

### BN15 — The Check That Establishes This ADR's Premise Is in a Fence No Checker Opens

The block under *What forced it* is the whole evidentiary base of this decision.
It is fenced ` ```bash `, and the corpus recipe checker only reads ` ```sh `:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- what recipes.py accepts as a recipe --'
command grep -m1 -A8 -F '  lines = pathlib.Path( path ).read_text().splitlines()' bench_harness/gate/corpus/recipes.py | sed 's/^/    /'
echo '  -- bash-fenced blocks in this crate docs, which it therefore never opens --'
command grep -r '^```bash$' ring_bench/docs/ | sed 's/^/    /'
printf '    total : %s blocks across %s files\n' \
  "$( command grep -rho '^```bash$' ring_bench/docs/ | wc -l )" \
  "$( command grep -rl  '^```bash$' ring_bench/docs/ | wc -l )"
echo '  -- and the premise itself, re-run today --'
command grep 'fn try_clone' ring_core/src/lib.rs | sed 's/^/    /'
printf '    matches in ring_handle/src/lib.rs : %s\n' "$( command grep -c 'fn try_clone' ring_handle/src/lib.rs )"
```

Live output:

```
  -- what recipes.py accepts as a recipe --
      lines = pathlib.Path( path ).read_text().splitlines()
      out = []
      i = 0
      while i < len( lines ):
        fence = lines[ i ].rstrip()
        if fence == '```sh':
          ( body, j ) = _fence_body( lines, i + 1 )
          k = j + 1
          while k < len( lines ) and lines[ k ].strip() == '':
  -- bash-fenced blocks in this crate docs, which it therefore never opens --
    ring_bench/docs/non_functional_requirement/001_the_comparison_is_reproducible_and_same_conditions.md:```bash
    total : 1 blocks across 1 files
  -- and the premise itself, re-run today --
      pub fn try_clone( &self ) -> Option< Producer< 'a, T > >
    matches in ring_handle/src/lib.rs : 0
```

`recipes.py` opens a fence in exactly two shapes: a ` ```sh ` recipe, and —
since the probe-recording shape landed — a differently-tagged fence whose
first body line is a probe comment. This block is neither; it opens with a
`cd`, so it is skipped exactly as this finding describes. That is how the
finding originally counted thirteen such blocks across eight of this
crate's documents as outside the gate entirely — since re-gated to one block
in one file, deliberately kept `bash` for a non-deterministic `cargo test`
invocation (the crate's other documents, including `docs/readme.md`, are now
fully open to the gate). They are
not merely unexecuted — they are unseen, so the checker cannot even report
them as missing a `Live output:` fence. A ` ```sh ` block without one is an
`UNQUOTED` finding; a ` ```bash ` block without one is nothing at all.

**The cost is visible in this very block.** Its two results were hand-written
comments — `# exists`, `# nothing` — sitting beside commands that no tool in
the repository had run since they were transcribed. The premise still holds: `ring_core::Producer::try_clone` is at line 464 and `ring_handle`
has no match. But that is now known because this finding re-ran it, not because
the record is self-verifying.

The distinction matters because of what this ADR *is*. Its first line is "**A
measurement, not an argument**", and the discipline it announces — decide from
what the code does, not from what it should do — is exactly the discipline the
fence choice suspends. A measurement recorded as a command whose output was
transcribed by hand is an argument again the moment the code moves.

The general shape, and the reason this is filed here rather than as a gate bug:
**a checker keyed on an exact marker is a checker with a silent opt-out**, and
the opt-out is one character wide and reads as a synonym.

**Disposition:** applied — the *What forced it* block above is now fenced
` ```sh ` with a real `Live output:` fence (the citation was re-run too:
`try_clone` is at line 464 now, not 454). `recipes.py` opens it and confirms
it byte-for-byte. The opt-out this finding describes no longer applies to this
one block, but the general shape it names — a checker keyed on an exact marker
has a silent opt-out — is the actual point of the finding and still stands.

### BN16 — "Differ Only in the Layers" Is Contradicted by This Crate's Own Algorithm Document

`DirectSpsc` exists to isolate the Contract's cost, on the stated grounds that
it and `ContractRing` "differ **only** in the layers between the caller and the
same data structure". They do not:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- how each of the two publishes --'
awk '/^fn run_contract_ring/, /^\}/' src/lib.rs | command grep -nE 'Factory|\.ends\(|\.split\(|try_push|records' | sed 's/^/    contract  /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
awk '/^fn run_direct_spsc/,   /^\}/' src/lib.rs | command grep -nE 'with_config|\.split\(|try_push|records' | sed 's/^/    direct    /' | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- calls into the producer, for the roomy fixture (256 records, batch 32) --'
printf '    contract_ring : %s calls to try_push_batch\n' "$(( 256 / 32 ))"
printf '    direct_spsc   : %s calls to try_push\n' 256
echo '  -- what this crate already wrote down about that --'
awk '/have no batch API/, /differently-shaped/' docs/algorithm/001_one_workload_through_six_runners.md | sed 's/^/    /'
```

Live output:

```
  -- how each of the two publishes --
    contract  3:  let mut split = Factory
    contract  6:  let mut ends = split.ends();
    contract  7:  let ( mut producer, mut consumer ) = ends.split();
    contract  12:  while next < workload.records_per_producer()
    contract  14:    let end = usize::min( next + workload.batch(), workload.records_per_producer() );
    contract  15:    let mut records = next as Record .. end as Record;
    contract  16:    reported += producer.try_push_batch( &mut records );
    direct    4:    ring_spsc::Ring::with_config( &workload.config() );
    direct    5:  let ( mut producer, mut consumer ) = ring.split();
    direct    9:  for record in workload.records_of( 0 )
    direct    11:    if producer.try_push( record ).is_ok()
  -- calls into the producer, for the roomy fixture (256 records, batch 32) --
    contract_ring : 8 calls to try_push_batch
    direct_spsc   : 256 calls to try_push
  -- what this crate already wrote down about that --
    `ring_spsc` and `ring_mpsc` have no batch API, so their runners publish one
    record at a time while the Contract candidates publish `batch` at a time through
    the same underlying ring. Part of the `ContractRing`-versus-`DirectSpsc` gap is
    therefore the batch API's benefit, not only the wrapper's cost — the wrapper is
    being credited for something. Recorded here because the alternative (looping
    `try_push_batch` with a one-element range) would measure a differently-shaped
```

`run_contract_ring` publishes with `try_push_batch( &mut records )` once per
batch; `run_direct_spsc` publishes with `try_push( record )` once per record. On
the roomy fixture that is **eight producer calls against two hundred and
fifty-six**. The two paths also build their record ranges differently —
`next as Record .. end as Record` inline versus `workload.records_of( 0 )` — so
the disjointness guarantee applies to one and not the other
(→ [`algorithm/001`](../algorithm/001_one_workload_through_six_runners.md)'s BN2).

So the gap between the two timings contains the wrapper's cost **plus** the batch
API's benefit, and the two are added together in the direction that makes the
Contract path look better. The sentence "the gap between their timings is the
wrapper's cost with everything else held constant" is the justification for a
variant existing, and everything else is not held constant.

**The contradiction is already in this crate's own documentation.**
`algorithm/001`'s batching section states it plainly: *"Part of the
`ContractRing`-versus-`DirectSpsc` gap is therefore the batch API's benefit, not
only the wrapper's cost — the wrapper is being credited for something."* It even
records why the asymmetry is kept — looping `try_push_batch` with a one-element
range "would measure a differently-shaped lie". That reasoning is sound and this
ADR does not carry it; it carries the opposite claim, in bold, as the reason a
third variant is worth its row. The two statements are one directory apart.

Both documents are dated the same day. Neither cites the other on this point.

The general shape: **a comparison justified by "everything else is held
constant" needs the list of what "everything else" is**, and where the list is
absent the claim is not checkable — which is how one crate ends up asserting a
constant in one document and describing it as a variable in another.

```sh
cd "$(git rev-parse --show-toplevel)"
# -m1 and no -n: this file is its own subject, so an unbounded match
# also finds this command line and every copy of its own output below,
# and -n re-prefixes a fresh line number onto each earlier pass's output
command grep -m1 'credits the wrapper for some' ring_bench/docs/decisions/001_five_candidates_for_four_named_paths.md
```

Live output:

```
call through the same underlying ring, which credits the wrapper for some of
```

**Disposition:** applied — "The decision" section no longer claims the two
variants differ **only** in the layers; it now names the batch-API asymmetry
directly and cross-references `algorithm/001`'s own statement of it, so the two
documents agree instead of one asserting a constant the other calls a
variable.
Now prints: `credits the wrapper for some`
