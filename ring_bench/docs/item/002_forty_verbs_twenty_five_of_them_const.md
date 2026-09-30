# Item: Forty Verbs, Twenty-Five of Them `const`

### Scope

- **Purpose**: Catalogue the forty public verbs as a set, and record the two properties only the set shows — how little of the surface does anything, and the one doc sentence that describes all the runners and is true of some of them.
- **Responsibility**: State each verb's shape, and separate the verbs that compute from the verbs that read.
- **In Scope**: The forty `pub fn`; the eight private `fn`; which of them reach a thread and which reach the clock.
- **Out of Scope**: The nouns (→ [`001`](001_seven_nouns_and_the_one_never_compared.md)); the order the verbs are called in (→ [`lifecycle/001`](../lifecycle/001_from_a_description_to_a_verdict.md)); why the ceiling exists (→ [`pitfall/001`](../pitfall/001_the_door_caps_what_the_structure_does_not.md)).

### The Split

Forty public verbs, and the shape that matters is not what they are named
but whether they can run at compile time.

| Kind | Count | What they are |
|------|-------|---------------|
| `pub const fn` | 25 | Accessors and derived predicates on a value that already exists |
| `pub fn` | 15 | Four fallible builders, one range, a buffer accessor, one constructor, five folds over accumulated outcomes, `run`, `report`, `capacity` |
| private `fn` | 8 | Six runners, `commit_batch`, and `destination_of` |

Eight of the forty return `Result`. Thirty-five of the forty carry
`#[ must_use ]`.

**The whole comparison happens in seven functions**, and the surface above them
is a way of reading what those seven produced. That is the correct shape for a
harness — a measurement should be a value, which is
[`pattern/001`](../pattern/001_the_measurement_is_a_value.md) — but it means the
public API is a poor guide to what the crate does, and a reader looking for the
benchmark in a list of method names will not find it.

### What Reaches a Thread, and What Reaches the Clock

All six runners read the clock twice. Two of them spawn threads.

| Runner | Threads | Clock reads | Batches |
|--------|---------|-------------|---------|
| `run_mutex_queue` | yes | 2 | yes |
| `run_contract_ring` | no | 2 | yes |
| `run_tls_over_ring` | no | 2 | yes |
| `run_direct_spsc` | no | 2 | **no** |
| `run_direct_mpsc` | yes | 2 | **no** |
| `run_off_the_shelf` (`crossbeam`) | no | 2 | yes |

The four single-threaded runners are the four whose `producer_ceiling` is
`Some( 1 )` in some build, so the arrangement is consistent — but it means the
multi-producer half of the comparison is answered by exactly two
candidates in the default build, and one of them is the mutex the comparison
exists to beat.

### Sources

| File | Relationship |
|------|-----------------|
| [`src/lib.rs`](../../src/lib.rs) | Every signature, measured rather than described |

### Items

| File | Relationship |
|------|--------------|
| [`001_seven_nouns_and_the_one_never_compared.md`](001_seven_nouns_and_the_one_never_compared.md) | The other half of the catalogue |

### Algorithms

| File | Relationship |
|------|--------------|
| [`../algorithm/001_one_workload_through_six_runners.md`](../algorithm/001_one_workload_through_six_runners.md) | What the six runners do, by role rather than by signature |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`../pitfall/001_the_door_caps_what_the_structure_does_not.md`](../pitfall/001_the_door_caps_what_the_structure_does_not.md) | Why four of six are single-producer |

### Tests

| Test | Relationship |
|------|--------------|
| `every_candidate_declares_a_name_and_a_ceiling` | Walks `Candidate::ALL` and checks the two `const` accessors agree |
| `the_producer_count_sets_the_backend_it_needs` | The producer dimension, asserted against `admits` |
| `a_partial_final_batch_is_published_rather_than_abandoned` | The only test whose fixture makes the batch not divide the record count |
| `a_roomy_run_keeps_everything_and_returns_it` | The lossless path through all five default candidates |

### BN27 — Twenty-Five of Forty Verbs Cannot Run Anything

The public surface is mostly `const`:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
printf '  public verbs        : %s\n' "$( command grep -cE '^ +pub (const )?fn |^pub fn ' src/lib.rs )"
printf '    of them const     : %s\n' "$( command grep -cE '^ +pub const fn ' src/lib.rs )"
printf '    returning Result  : %s\n' "$( command grep -cE '^ *(pub )?(const )?fn .*-> Result<' src/lib.rs )"
printf '    marked must_use   : %s\n' "$( command grep -cE '^ +#\[ must_use \]' src/lib.rs )"
printf '  private fns         : %s\n' "$( command grep -cE '^fn ' src/lib.rs )"
echo '  -- which functions reach a thread, and which the clock --'
awk '/^ *(pub )?(const )?fn /{ f = $0; sub( /\(.*/, "", f ); sub( /^ */, "", f )
       sub( /^pub /, "", f ); sub( /^const /, "", f ); sub( /^fn /, "", f ); cur = f }
     /thread::scope/{ if ( cur != "" ) t[ cur ]++ }
     /Instant::now|\.elapsed\(\)/{ if ( cur != "" ) c[ cur ]++ }
     { if ( cur != "" && !seen[ cur ]++ ) order[ ++k ] = cur }
     END{ for ( i = 1; i <= k; i++ ) { f = order[ i ]
            if ( t[ f ] || c[ f ] ) printf "    %-22s threads=%d  clock=%d\n", f, t[ f ], c[ f ] } }' src/lib.rs
```

Live output:

```
  public verbs        : 40
    of them const     : 25
    returning Result  : 8
    marked must_use   : 35
  private fns         : 8
  -- which functions reach a thread, and which the clock --
    run_mutex_queue        threads=1  clock=2
    run_contract_ring      threads=0  clock=2
    run_tls_over_ring      threads=0  clock=2
    run_direct_spsc        threads=0  clock=2
    run_direct_mpsc        threads=1  clock=2
    run_off_the_shelf      threads=0  clock=2
```

Twenty-five accessors, fifteen verbs that compute, eight private functions, and
inside those eight the entire subject of the crate. Nothing here is wrong — a
harness whose result is a value will have a wide read surface and a narrow write
one — but two consequences follow that the individual rustdoc cannot state.

**First, `#![ deny( missing_docs ) ]` guarantees forty doc comments and
guarantees nothing about proportion.** Twenty-five of them describe a field being
returned. The six functions that make the measurement are private and therefore
outside what the lint checks at all, and they are where every question a reader
of this crate actually has is answered.

**Second, only two functions spawn a thread.** The candidates must be compared
"under the same producer counts"; four of the six runners
are structurally single-threaded, so in the default build the multi-producer
comparison has two arms — `mutex_queue` and `direct_mpsc` — and the first of
those is the baseline the second exists to be measured against. A two-arm
comparison where one arm is the control is a measurement, but it is not the
comparison the feature describes, and no count in the crate reports the
narrowing.

### BN28 — "Every Candidate Honours This" Describes Six Runners and Is True of Four

`Workload::with_batch`'s doc enumerates who reads the batch. The runners
disagree:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_bench
echo '  -- the pre-BN28-fix setter doc ("Every candidate honours this", expect 0 -- see fix below) --'
printf '    hits: %s\n' "$( command grep -c 'Every candidate honours this' src/lib.rs )"
echo '  -- how often each runner reads it --'
awk '/^fn run_/{ f = $0; sub( /\(.*/, "", f ); sub( /^fn /, "", f ); order[ ++k ] = f }
     /workload\.batch\(\)/{ if ( f != "" ) n[ f ]++ }
     END{ for ( i = 1; i <= k; i++ ) printf "    %-22s %d\n", order[ i ], n[ order[ i ] ] + 0 }' src/lib.rs
echo '  -- what the two that read it zero times do instead --'
command grep 'producer.try_push( record )\|producer.push( record )' src/lib.rs
echo '  -- and whether the config carries the batch down to them --'
command grep 'Ring::with_config' src/lib.rs
command grep 'pub fn with_config' ../ring_spsc/src/lib.rs ../ring_mpsc/src/lib.rs
```

Live output:

```
  -- the pre-BN28-fix setter doc ("Every candidate honours this", expect 0 -- see fix below) --
    hits: 0
  -- how often each runner reads it --
    run_mutex_queue        2
    run_contract_ring      1
    run_tls_over_ring      2
    run_direct_spsc        0
    run_direct_mpsc        0
    run_off_the_shelf      2
  -- what the two that read it zero times do instead --
    if producer.try_push( record ).is_ok()
              if producer.push( record ).is_ok()
  -- and whether the config carries the batch down to them --
    ring_spsc::Ring::with_config( &workload.config() );
    ring_mpsc::Ring::with_config( &workload.config() );
../ring_spsc/src/lib.rs:  pub fn with_config( config : &RingConfig ) -> Self
../ring_mpsc/src/lib.rs:  pub fn with_config( config : &RingConfig ) -> Self
```

The doc names three roles and they cover all five default candidates: "the mutex
queue takes its lock once per batch" (one), "the ring candidates publish a batch
at a time" (three), "and the staged candidate binds it as its `FlushPolicy::OnBatch`
trigger" (one). The middle clause is the false one. `run_direct_spsc` and
`run_direct_mpsc` push one record per call and never mention the batch.

**The batch does reach them, which is why this survived.** Both take
`workload.config()`, and `with_batch` writes the batch into that config as well
as into the `Workload` — so the value is present in the argument the direct
runners receive. It is then discarded: `ring_spsc::Ring::with_config` and
`ring_mpsc::Ring::with_config` both read `config.capacity()` and nothing else,
a fact this crate already established from the other direction in
[`pitfall/003`](../pitfall/003_ok_is_not_kept_and_the_verdict_inverts.md), where
the field being ignored was `overflow`.

The doc's own next sentence states the stake exactly: "a batch size that only one
arm observed would measure batching against nothing." Three arms observe it.
Two do not, and the two that do not are the two the comparison uses to price the
Contract dispatch — so the batch dimension is varied against candidates that
cannot see it, and the difference is attributed to dispatch.

The general shape, and the reason this is an `item/` finding rather than an
`api/` one: **a doc comment on one declaration made a claim about six others,
and the lint that guarantees the comment exists cannot read what it says.**

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'Four of six candidates honour this' ring_bench/src/lib.rs
```

Live output:

```
    /// Four of six candidates honour this: the mutex queue takes its lock once
```

**Disposition:** applied — `with_batch`'s doc comment now names the split
directly: four candidates honour the setting, `DirectSpsc` and `DirectMpsc`
receive it through `Workload::config` and never read it, because the backends
they construct from read only capacity. The claim now covers the six runners
it describes rather than three of them.
Now prints: `Four of six candidates honour this`
