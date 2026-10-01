# Non-Functional Requirement: What a Guarded Push Costs

### Scope

- **Purpose**: State the runtime cost the guard adds to a publish, in the units the code actually pays it in, and record that no number in this crate or its family bounds that cost.
- **Responsibility**: The per-record and per-batch cost of consulting the flag, the pre-flight accessor that multiplies it, and the measurement surface available to check either.
- **In Scope**: `Guarded::try_push`, `Guarded::try_push_batch`, `Guarded::is_blocked`, `Shutdown::is_closed`.
- **Out of Scope**: Whether the flag is *correct* under concurrency (→ [`../data_structure/001`](../data_structure/001_the_close_flag_and_its_orderings.md)); the drain side's cost (→ [`002`](002_the_teardown_path_takes_the_slow_one.md)).

### The Requirement

**A guarded producer must cost no more than one `Acquire` flag read per
publish, and a batch must pay that once rather than once per record.**

That is the requirement the code meets. It is stated here because it is stated
nowhere else: not in `api/001`'s surface table, not in any doc comment, not in a
bench, not in a test.

### What Is Actually Paid

| Operation | Flag reads | Per record |
|---|---|---|
| `try_push` | 1 | 1 |
| `try_push_batch` of *n* | 1 | 1/*n* |
| `is_blocked` | 1 | — |

The read is `closed.load( Ordering::Acquire )`. On the workspace's `aarch64`
host that is an `ldar`; it is not free, and it is not a fence.

The batch row is the interesting one. `try_push_batch` checks once, before it
reads the first record from the iterator, and then delegates the whole batch to
the unguarded `Producer::try_push_batch`. So the guard's overhead per record
falls as the batch grows, and a caller who cares about the cost has one
available lever — batching — that the surface never mentions.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'flag reads per single push:    %s\n' "$( awk '/pub fn try_push\(/{f=1} f&&/^  \}$/{exit} f' ring_shutdown/src/lib.rs | command grep -c 'is_closed()' || true )"
printf 'flag reads per batch push:     %s\n' "$( awk '/pub fn try_push_batch/{f=1} f&&/^  \}$/{exit} f' ring_shutdown/src/lib.rs | command grep -c 'is_closed()' || true )"
printf 'the load the flag read emits:  %s\n' "$( command grep -ohE 'closed\.load\( [A-Za-z:]+ \)' ring_shutdown/src/lib.rs )"
printf 'what is_blocked reads:         %s\n' "$( awk '/pub fn is_blocked/{f=1} f&&/^  \}$/{exit} f&&/\|\|/{ sub( /^ */, "" ); print }' ring_shutdown/src/lib.rs )"
printf 'the doc on is_blocked says:    %s\n' "$( command grep -o 'It does not predict a refusal' ring_shutdown/src/lib.rs )"
printf 'benches in the ring family:    %s\n' "$( ls -d ring_*/benches 2>/dev/null | wc -l )"
printf 'manifests naming criterion:    %s\n' "$( command grep -l 'criterion' ring_*/Cargo.toml 2>/dev/null | wc -l )"
printf 'time figures anywhere here:    %s\n' "$( command grep -rhoE '[0-9]+ (ns|us|ms|cycles)' ring_shutdown/docs ring_shutdown/src ring_shutdown/tests 2>/dev/null | sort -u | wc -l )"
printf 'ratios the crate does record:  %s\n' "$( command grep -rhoE '[0-9]+/[0-9]+' ring_shutdown/src/lib.rs ring_shutdown/tests/manual/readme.md | sort -u | tr '\n' ' ' )"
printf 'columns in the api surface:    %s\n' "$( awk -F'\\|' '/^\| Item \|/{ for ( i = 2; i < NF; i++ ) { gsub( /^ +| +$/, "", $i ); printf "%s; ", $i } exit }' ring_shutdown/docs/api/001_shutdown_surface.md )"
printf 'duration units in that file:   %s\n' "$( command grep -cE '[0-9]+ (ns|us|ms|cycles)' ring_shutdown/docs/api/001_shutdown_surface.md || true )"
```

Live output:

```
flag reads per single push:    1
flag reads per batch push:     1
the load the flag read emits:  closed.load( Ordering::Acquire )
what is_blocked reads:         self.shutdown.is_closed() || self.producer.is_full()
the doc on is_blocked says:    It does not predict a refusal
benches in the ring family:    0
manifests naming criterion:    0
time figures anywhere here:    0
ratios the crate does record:  4/4 80/81 81/81 
columns in the api surface:    Item; Signature; Guarantee holds by; 
duration units in that file:   0
```

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [`002_the_teardown_path_takes_the_slow_one.md`](002_the_teardown_path_takes_the_slow_one.md) | The same question asked of the drain rather than the publish, with the same answer about measurement |

### Data Structures

| File | Relationship |
|------|--------------|
| [`../data_structure/001_the_close_flag_and_its_orderings.md`](../data_structure/001_the_close_flag_and_its_orderings.md) | The field this cost is a read of, and why the ordering is `Acquire` |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_shutdown_surface.md`](../api/001_shutdown_surface.md) | The surface table that grades every item by guarantee and none by cost |

### Items

| File | Relationship |
|------|--------------|
| [`../item/002_two_checks_and_two_waiters.md`](../item/002_two_checks_and_two_waiters.md) | The other reading of `is_blocked`'s neighbourhood — what the guard's sites check rather than what they cost |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The three guarded operations and the flag read each performs |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `a_guarded_producer_refuses_a_closed_ring_and_returns_the_record` — the behaviour is asserted; the cost is not |

### SD33 — The Crate's Only Recorded Numbers Are Coverage Ratios

Every number this crate has ever written down is a coverage figure. `72/73`,
`73/73`, `4/4` — the `llvm-cov` line counts from the `loop`-versus-`while`
investigation and the manual probe tally. There is not one duration, not one
cycle count, not one throughput figure anywhere in the crate's source, tests or
docs.

That is not an oversight local to this crate. The ring family has **zero
`benches/` directories** and **zero manifests naming `criterion`**. There is no
place a performance number could go and no tool wired to produce one.

The consequence is specific rather than general. The guard's whole design
argument — that a wrapper consulting a flag is cheap enough to put on every
publish — is a performance claim, and it is the one kind of claim the family has
no machinery to check. `api/001` grades every item by *whether its promise holds
by construction or by convention*; a cost column would have nothing to put in
it. So the crate can tell a reader precisely what is guaranteed and cannot tell
them what it costs, and the asymmetry is invisible because nothing in the corpus
asks for the second column.

The finding is not that the guard is slow. It almost certainly is not. It is
that "almost certainly" is the strongest statement available, for the property
the whole design rests on.

**Disposition:** declined — filling this gap needs new benchmark
infrastructure (a `benches/` harness wired to `criterion` or equivalent),
which does not exist anywhere in the family (confirmed: `ring_*/`
has zero `benches/` directories and zero `Cargo.toml` files naming
`criterion`). That is new tooling, not a doc fix, and out of scope for a
disposition pass over `ring_shutdown/docs/`.

```sh
cd "$(git rev-parse --show-toplevel)"
command find ring_* -maxdepth 1 -type d -name benches | wc -l
command grep -rl 'criterion' ring_*/Cargo.toml 2>/dev/null | wc -l
```

Live output:

```
0
0
```

Now prints: 0 and 0 — no benchmark harness and no `criterion` dependency
anywhere in the family, confirming the gap this finding names has no
existing machinery to route a fix through. `perf/`, the workspace's criterion
suite, sits outside the family and does not time the guard.

### SD34 — The Pre-Flight Accessor Costs More and Promises Less Than Just Pushing

`Guarded::is_blocked` reads `self.shutdown.is_closed() || self.producer.is_full()`.
It exists so a caller can ask whether a push would be impeded before attempting
one, and its name invites exactly the idiom it should not be used for:

```rust
if !guarded.is_blocked()
{
  guarded.try_push( record );
}
```

That costs two flag reads and an occupancy check per record instead of one flag
read, and it buys nothing, because the doc comment says so itself: *"It does
not predict a refusal."* Under the default `OverflowPolicy::DropNewest` a
blocked-by-occupancy push still returns `Ok` having discarded the record, and
`free_capacity`'s advisory contract at MPSC means the occupancy half can be
stale by the time the push runs.

Meanwhile `try_push` already returns the whole answer — refused or not, and if
refused then why, with the record handed back intact. The guarded push *is* the
pre-flight check, performed atomically with the operation it guards.

So the crate ships an accessor whose obvious use is strictly worse than not
using it on both axes at once. The doc comment disclaims the guarantee in its
third paragraph and says nothing about the cost, and there is no measurement
anywhere that would let a reader weigh the difference — which is SD33 arriving
at a concrete call site.
