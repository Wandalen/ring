# decisions

Five fields, and two different answers to "what happens when this value is
wrong". Capacity is `new`'s argument and rejects, producing the crate's only
`Result`. `producers` and `batch` are infallible `const fn` setters that correct
instead, each documented with its own reason. `wait` and `overflow` have no wrong
value to answer for.

Both decisions are defensible on their own terms and both are recorded where they
happen. What no document states is that they are one decision seen twice — the `?`
was concentrated at the head of the chain precisely so that everything after it
could be total — nor that the correcting half puts this crate on the opposite side
of a rule the rest of the family follows.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_capacity_is_the_one_parameter_that_is_not_a_with.md) | Capacity Is the One Parameter That Is Not a `with_` | The constructor/setter split, its rationale, and who constructs a config at all |
| [002](002_clamping_instead_of_a_question_mark_mid_chain.md) | Clamping Instead of a `?` Mid-Chain | Both clamps, the four crates that raise an error instead, and where the clamped value goes |

## One Decision, Documented From the Wrong End

Capacity is the only field whose invalid values have no honest correction — a zero
cannot be raised to anything, and rounding an arbitrary integer to a power of two
resizes the ring by up to a factor of two without saying so. Rejecting is the only
available answer, and putting the only rejection at the head of the chain is what
lets the four setters be infallible `const fn`.

That rationale appears once in the crate, attached to `with_producers`: "clamping
keeps the setter infallible so a builder chain does not need a `?` in its middle."
It is the same decision viewed from a field that had a choice. The half that
explains why `new` is where the `?` lives — because capacity is the one place
clamping would be dishonest — is stated nowhere.

## The Exception to a Rule the Family States

Four crates guard `requested > capacity` and return `RingError::BatchTooLarge`:
`ring_batch`, `ring_claim`, `ring_gating`, `ring_slot`. `ring_gating`'s module
comment names the category outright — "`BatchTooLarge` is a configuration error: a
claim wider than the ring can never fit no matter who moves" — and `ring_config`
is the one crate meeting that predicate that corrects rather than reports.

The exception used to cost nothing, because the clamped value had nowhere to go.
It has somewhere to go now. Eight `.batch()` calls sit outside this crate, all of
them in `ring_bench`, and one is the entire body of `Workload::batch` —
`self.config.batch()`. The other seven are `workload.batch()`, which is that
delegate, so every one of them reads `RingConfig::batch`.

The reader arrived by way of a bug fix in the other crate. `Workload` used to
hold its own `batch` field beside the config, and the two could disagree: a
workload built with capacity 16 and `with_batch( 32 )` reported 32 while its
config held 16, a pair `RingConfig` refuses to produce. The fix deleted the
duplicate and pointed the accessor at the validated copy. The clamp is therefore
no longer protecting an empty path — it is the reason every candidate's publish
loop now sees a batch that fits the ring.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- one fallible constructor, four infallible setters --'
command grep -n 'pub fn new(\|pub const fn with_' ring_config/src/lib.rs | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- the rationale, stated once, at the wrong end --'
command grep -m1 -A2 -F '  /// A count of `0` is clamped to `1`: a ring nothing can publish into has no' ring_config/src/lib.rs
echo '  -- the error four other crates raise for the same predicate --'
command grep -rn 'return Err( RingError::BatchTooLarge' --include=*.rs */src  | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- every construction site outside this crate, and how many are doc comments --'
command grep -rn 'RingConfig::new(' --include=*.rs */src | command grep -vc '^ring_config/' || true
command grep -rn 'RingConfig::new(' --include=*.rs */src | command grep -v '^ring_config/' | command grep -c '/// \|//! ' || true
echo '  -- and the ones that are not, which is where the Result is really discharged --'
command grep -rn 'RingConfig::new(' --include=*.rs */src | command grep -v '^ring_config/' | command grep -v '/// \|//! '  | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
echo '  -- who reads RingConfig::batch, the field RC16 was written about --'
command grep -rn '\.batch()' --include=*.rs */src | command grep -v '^ring_config/'  | sed -E 's/^([[:space:]]*)([^[:space:]:]*:)?[0-9]+[:-]/\1\2/'
```

Live output:

```
  -- one fallible constructor, four infallible setters --
  pub fn new( slots : usize ) -> Result< Self, RingError >
  pub const fn with_wait( mut self, wait : WaitKind ) -> Self
  pub const fn with_overflow( mut self, overflow : OverflowPolicy ) -> Self
  pub const fn with_producers( mut self, producers : usize ) -> Self
  pub const fn with_batch( mut self, batch : usize ) -> Self
  -- the rationale, stated once, at the wrong end --
  /// A count of `0` is clamped to `1`: a ring nothing can publish into has no
  /// use, and clamping keeps the setter infallible so a builder chain does not
  /// need a `?` in its middle.
  -- the error four other crates raise for the same predicate --
ring_batch/src/lib.rs:    return Err( RingError::BatchTooLarge { requested : count, capacity : capacity.get() } );
ring_claim/src/lib.rs:      return Err( RingError::BatchTooLarge
ring_gating/src/lib.rs:      return Err( RingError::BatchTooLarge { requested : count, capacity : self.capacity.get() } );
ring_slot/src/lib.rs:      return Err( RingError::BatchTooLarge { requested : payload.len(), capacity : N } );
  -- every construction site outside this crate, and how many are doc comments --
35
33
  -- and the ones that are not, which is where the Result is really discharged --
smoke_ring_write_path/src/lane.rs:  let config = RingConfig::new( RING_CAPACITY ).map_err( | e | format!( "ring config: {e}" ) )?;
smoke_ring_write_path/src/lane.rs:  let config = RingConfig::new( RING_CAPACITY )
  -- who reads RingConfig::batch, the field RC16 was written about --
ring_bench/src/lib.rs:    self.config.batch()
ring_bench/src/lib.rs:            let mut staged = Vec::with_capacity( workload.batch() );
ring_bench/src/lib.rs:              if staged.len() == workload.batch()
ring_bench/src/lib.rs:    let end = usize::min( next + workload.batch(), workload.records_per_producer() );
ring_bench/src/lib.rs:  let buffer = TlsBuffer::< Record >::with_capacity( workload.batch() );
ring_bench/src/lib.rs:  let mut flusher = Flusher::new( buffer, producer, FlushPolicy::OnBatch( workload.batch() ) )
ring_bench/src/lib.rs:    let end = usize::min( next + workload.batch(), workload.records_per_producer() );
ring_bench/src/lib.rs:      self.workload.batch(),
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RC13 | `ring_config` | n/a — doc gap | Capacity is `new`'s argument and the crate's only `Result` while the other four fields are infallible `const fn` setters, because capacity is the one field whose invalid values have no honest correction — and the rationale for that whole shape appears only once, attached to `with_producers` ("clamping keeps the setter infallible so a builder chain does not need a `?` in its middle"), which explains the field that had a choice and never states why `new` is where the `?` was put |
| RC14 | `ring_config` | n/a — observation | Thirty-five lines across twelve crates mention `RingConfig::new` in a `src/` file, and thirty-three of them are inside doc comments — thirty-two construction expressions discharging the `Result` with `.unwrap()`, plus one line of prose in `ring_bench` — but the remaining two are not doctests: `smoke_ring_write_path` constructs a configuration in two real functions, and both discharge with `.map_err( … )?` rather than `.unwrap()`. The reading this finding once recorded — that the doctests are the family's entire demonstrated answer to a rejected capacity — is therefore no longer true, and what replaced it is the better answer: the one crate that constructs outside a doc comment is also the only one that propagates the error instead of panicking on it |
| RC15 | `ring_config` | n/a — inconsistency | Four crates guard `requested > capacity` and return `RingError::BatchTooLarge` — `ring_batch`, `ring_claim`, `ring_gating` and `ring_slot`, each at its `return Err( RingError::BatchTooLarge` site printed by the census above, which is addressed by content rather than by line because the four line numbers this finding once carried had all drifted between forty and a hundred lines — and `ring_gating`'s module comment names the category — "`BatchTooLarge` is a configuration error: a claim wider than the ring can never fit no matter who moves" — making `ring_config` the one crate meeting that predicate that corrects silently instead, with neither side's documentation recording that the two answers coexist |
| RC16 | `ring_config` | n/a — observation | `with_batch`'s clamp guarantees `1 <= batch <= capacity` for every configuration that exists, and this finding once recorded that nothing read the clamped value: twelve `.batch()` calls outside the crate, every one of them `workload.batch()` on a `ring_bench::Workload` that held its own `batch` field. A bug fix in `ring_bench` — two fields holding the same quantity, only one of them validated, a workload able to report a batch of 32 while its config held 16 — deleted the duplicate and made `Workload::batch` a one-line delegate whose whole body is `self.config.batch()`. That removed four of the twelve calls and turned the other seven into transitive reads of `RingConfig::batch`, so the clamp now has exactly the reader it was written for. The finding is recorded as resolved rather than deleted because nothing in either crate connects the fix to the guarantee it completed |
