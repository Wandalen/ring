# pitfall

Both hazards live in `with_batch`, the one setter that reads a field it does not
write. The first is what it does to a caller today: it corrects a batch larger
than the ring and reports nothing, in a family whose shared error type has a
variant named for exactly that condition, carrying both numbers, returned by four
other crates. The second is what it would do if the record ever gained the setter
it is missing.

Neither is a bug. Both are documented behaviours whose documentation stops one
step short of the consequence — the clamp's rationale is about hand-written
chains and the record exists for manifests, and the commutation test's comment
promises a property over "any order" that its body checks by naming four
functions.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_clamp_with_no_way_to_detect_it.md) | A Clamp With No Way to Detect It | `BatchTooLarge`, the family's stated position on silent correction, and three zeros with three answers |
| [002](002_the_setter_that_would_break_commutation.md) | The Setter That Would Break Commutation | The two failure shapes an added `with_capacity` produces, and why the suite stays green |

## The Family Argued Against This, in Writing

`ring_types::RingError` declares `BatchTooLarge { requested, capacity }` and
documents it as a batch the ring's whole capacity is smaller than — the same
sentence `with_batch` uses to explain why it clamps instead. Two variants further
down, `PolicyUnsupported`'s doc states the general position: a ring refuses to be
built rather than silently degrade a policy, because "a caller who never learns
their policy was not applied is worse off than one whose construction failed."

Four crates return `BatchTooLarge` rather than correcting. `RingConfig::with_batch`
corrects, and the correction is unreportable — recoverable only by a caller who
kept the value they passed and compares it back, which nothing in the family does.

## Three Zeros, Three Policies

A zero capacity is an error. A zero producer count is a documented clamp with a
doctest. A zero spin budget in `ring_wait::wait_until` is an undocumented clamp
whose function summary says "at most `spins` attempts" and which performs one
attempt and one pause — measured at 132µs and 146µs under `WaitKind::Park`, on a
call the caller wrote to mean *do not wait*.

No rule anywhere says which of the three a new zero-valued parameter should get.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the error variant named for the condition with_batch corrects --'
command grep -m1 -A8 -F '  /// A batch of the requested length cannot be served — the ring'"'"'s whole' ring_types/src/error.rs
echo '  -- crates that return it rather than correcting --'
command grep -rlc 'return Err( RingError::BatchTooLarge' --include=*.rs */src | sed -E 's|/src/lib.rs||'
echo '  -- the three zeros --'
command grep -rn 'return Err( RingError::CapacityZero )\|if [a-z_.]* == 0 { 1 }\|\.max( 1 )' --include=*.rs ring_*/src | command grep -v '///\|//!' | sed 's|/src/lib.rs||;s|/src/capacity.rs||'
echo '  -- the one cross-field read, and the setter that would make it mutable --'
command grep -m1 -F '    let capped = if batch > self.capacity.get() { self.capacity.get() } else { batch };' ring_config/src/lib.rs
command grep -c 'with_capacity' ring_config/src/lib.rs || true
echo '  -- and how the commutation test enumerates orderings --'
command grep -c 'permut\|Step::' ring_config/tests/config_test.rs || true
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RC41 | `ring_config` | **latent hazard** | `with_batch` clamps a batch larger than the capacity and explains it in the same terms `RingError::BatchTooLarge` uses to name the condition — a variant carrying both `requested` and `capacity`, returned rather than corrected by `ring_batch:318`, `ring_claim:425`, `ring_gating:269` and `ring_slot:351` — while `PolicyUnsupported`'s doc two variants below states the family's position outright, that "a caller who never learns their policy was not applied is worse off than one whose construction failed"; the correction is recoverable only by comparing against a remembered argument, and the setter's rationale covers hand-written chains rather than the manifest future this crate's own design gives the record |
| RC42 | `ring_wait` | **misleading doc** | `wait_until`'s summary says "for at most `spins` attempts" while the loop is `0..spins.max( 1 )`, so a budget of zero performs one attempt and one pause — measured across two runs at 146µs and 132µs under `WaitKind::Park` against a nominal 50µs sleep, and 0–5µs under the other three strategies — which makes the stated bound wrong exactly where a caller passing a computed zero meant *do not wait*, and completes the family's set of three different answers to a zero alongside `CapacityZero`'s error and `with_producers`' documented clamp |
| RC43 | `ring_config` | n/a — unenforced | Adding the missing `with_capacity` breaks commutation in two shapes and the dangerous one is silent: a probe over a verbatim copy of `with_batch` gives `batch = 32` for `with_capacity( 64 ).with_batch( 32 )` and `batch = 16` for the reverse, both satisfying the `1..=capacity` range the getter declares, so no invariant check could distinguish them — unlike `invariant/002`'s shrinking example, which violates the range and is therefore catchable — and the only distinguishing operation is an equality between orderings that exists solely for the test suite |
| RC44 | `ring_config` | n/a — coverage | `setters_commute`'s comment claims "the same five values in any order produce the same record" while its body names four setters in two spelled-out chains, and the suite contains zero permutation machinery, so a fifth setter would leave the test green, uncalled and unchanged while making its own comment false — the mechanism that would catch it is the twenty-four-ordering `Step`-enum sweep recorded under `invariant/002`, which lives in this corpus as a probe because nothing asked for it in `tests/` |
