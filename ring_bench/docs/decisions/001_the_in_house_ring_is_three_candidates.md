# The in-house ring is three benchmark candidates, one per door, each with its own producer ceiling

Status: Accepted

## Context

`ring_bench` compares four write paths: a mutex-guarded queue, an off-the-shelf concurrent queue, the in-house ring,
and thread-local staging over that ring. The requirement asks for all of them to run "under the same producer counts,
batch sizes, and payloads".

The obvious mapping is one `ring_bench::Candidate` variant per path. It fails at more than one producer. Through the
export Contract (`ring_factory::Factory::build`, returning a `ring_handle::Split`) the ring has exactly one producer,
because `ring_handle::Ends::split` yields one producer and `ring_handle` withholds `ring_core::Producer::try_clone`.
"The in-house ring" and "the same producer counts" cannot both be honoured through one door.

## Decision

"The in-house ring" is three `Candidate` variants, each reached through a different door:

- `ContractRing`, through `ring_factory::Factory::build`, ceiling 1. Prices what a Contract-bound consumer gets.
- `DirectSpsc`, through `ring_spsc::Ring::with_config`, ceiling 1 (the backend's own). Prices the dispatch that
  `ring_core` and `ring_handle` add over the bare ring.
- `DirectMpsc`, through `ring_mpsc::Ring::with_config`, no ceiling. The ring's own multi-producer behaviour, which no
  Contract door exposes.

`ContractRing` and `DirectMpsc` differ in door and in backend at once. `DirectSpsc` exists so that the Contract's
cost and the backend difference are not one number nobody can split.

Ceilings are values. `Candidate::producer_ceiling` returns them, `Candidate::admits` checks a workload against them,
and a candidate that cannot run a workload appears in the report as a `RunError::ProducerCeiling` refusal instead of
silently dropping out. The table on `producer_ceiling` names the layer that imposes each ceiling.

`OffTheShelf` stays one variant behind the `crossbeam` feature, because it has one door. It inherits the
`ContractRing` ceiling from `ring_handle`.

To reach the direct doors, the crate depends on `ring_spsc`, `ring_mpsc` and `ring_slot`, below the Contract.

## Alternatives considered

- **One ring variant, run at one producer only.** Every candidate runs and nothing is refused, so the report looks
  complete. The multi-producer question the comparison exists to answer is never asked, and nothing in the stated
  requirement detects that.
- **One ring variant reached directly, bypassing the Contract.** The number is real and describes a path no
  Contract-bound consumer can take.
- **One ring variant that switches door by producer count.** The same report row would be the Contract ring at one
  producer and the raw ring at four. That is two programs under one name, with nothing in the table saying which. The
  family's rule is one name, one program. `ring_factory` follows the same rule when it keeps `Factory::build` and
  `Factory::build_crossbeam` as separate doors instead of one function that routes on the overflow policy.

## Consequences

- The report has more rows than the requirement has candidates, and a reader looking for "the in-house ring" finds
  three. The ceiling table on `Candidate::producer_ceiling` is what makes the rows readable without this record.
- `ring_bench` is not an example of Contract-bound use and should not be read as one. Its direct dependencies on
  `ring_spsc`, `ring_mpsc` and `ring_slot` are deliberate.
- The `ContractRing` against `DirectSpsc` gap is not the wrapper's cost alone. `ring_spsc` has no batch API, so
  `DirectSpsc` makes one `try_push` call per record while `ContractRing` makes one `try_push_batch` call per batch.
  Part of the gap is the batch API's benefit, credited to the wrapper.
- Nothing watches the ceilings. `every_candidate_declares_a_name_and_a_ceiling` checks that each ceiling agrees with
  `admits` and with a copy written in the test. `ring_bench` does not depend on `ring_handle`, so a wider door there
  would leave every test green and a ceiling stale.
- Revisit when `ring_handle` exposes a way to get a second producer. `ContractRing` loses its ceiling, `DirectMpsc`
  narrows to pricing dispatch, which `DirectSpsc` already does, and two in-house variants would be enough.
