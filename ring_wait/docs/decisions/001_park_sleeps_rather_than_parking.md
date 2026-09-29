# Decision: `Park` Sleeps Rather Than Parking

### Scope

- **Purpose**: Record the crate's one deliberate deviation from a variant's own name, the alternative that was not taken, what it costs, and what would let it be deleted.
- **Responsibility**: State the decision, its argument, its measured price, and the mechanism that keeps the argument attached to the code.
- **In Scope**: `pause`'s `WaitKind::Park` arm.
- **Out of Scope**: The absorbed constraint framed as an external one — see [`workaround/001`](../workaround/001_a_sleep_where_a_park_belongs.md).

### The Decision

```rust
// ring_wait/src/lib.rs:132-143
WaitKind::Park =>
{
  // Sleeping rather than `thread::park` on purpose. Parking requires the
  // publisher to hold the waiter's handle and unpark it, which is a
  // registration relationship this crate deliberately does not have —
  // `ring_handle` owns who-knows-whom. A short sleep is the same
  // cost profile (idle rather than spinning) without inventing that
  // relationship here, and the sleep length is what a real unpark would
  // make unnecessary.
  std::thread::sleep( std::time::Duration::from_micros( 50 ) );
  true
}
```

`WaitKind::Park`'s own definition in `ring_types` is what a caller is promised,
and it used to promise a mechanism — *"Block until a publisher signals. Costs
nothing while idle, pays a wakeup"* (`ring_types/src/policy.rs:30`). This
arm delivers the cost profile and not the mechanism: it is idle rather than
spinning, and no publisher signals it. WT30 below is that mismatch; the variant
doc now states the cost profile alone, which is the half this arm does deliver.

### Why Not `thread::park`

`std::thread::park` needs a `Thread` handle held by whoever will unpark it. That
is a registration — *this waiter is waiting on that ring* — and it is a
relationship with an owner already:

| Question | Owner |
|----------|-------|
| Which threads hold a handle to this ring? | `ring_handle` |
| Which of them should be woken on a publish? | `ring_handle` |
| What happens to a registration when a thread exits? | `ring_handle` |

Putting a registry here would give the crate a type, a lifetime, and a teardown
story ([`data_structure/001`](../data_structure/001_a_crate_with_no_type_of_its_own.md)).
Putting one in `ring_handle` and reaching for it is worse. `ring_wait` declares
itself Tier 4 (`:3`) and depends on `ring_types` and `ring_cursor` only;
`ring_handle`'s one dependency is `ring_core`, which in turn depends on six
crates including both ring implementations. An edge from here to there is not a cycle
today — nothing in `ring_handle`'s closure reaches this crate — but it would
drag `ring_core`, `ring_mpsc`, and `ring_spsc` into `ring_barrier`'s dependency
closure, which today is `ring_types`, `ring_cursor`, and this crate and nothing
else. A wait strategy would have become downstream of the rings it waits on.

There is a sharper reason too. `ring_poll` forbids the *tick path*
from reaching any parking operation, and enforces it by banning the dependency
edge on this crate. A real `thread::park` here would make that ban load-bearing
in a way it currently is not — today the worst a stray `ring_wait` edge costs a
tick is a 50 µs sleep per attempt, which is a blown frame; with a genuine park it
is a thread that never wakes, which is a hang.

### The Alternatives, and What Each Would Cost

| Alternative | Gets | Costs |
|-------------|------|-------|
| **A sleep** (chosen) | idle-not-spinning, no new relationship | a fixed 50 µs floor per attempt, and a name that lies |
| `thread::park` + registry here | a real wakeup, no polling | a type, a teardown story, and an inverted dependency |
| `thread::park` + registry in `ring_handle` | the same, correctly placed | the edge points the wrong way — see below |
| Rename the variant to `Sleep` | honesty | `WaitKind` is `ring_types`', serialized into configs, and three other crates read it |
| Delete the variant | one fewer lie | deleting one would break callers matching on all four variants |

The fourth row is the one worth pausing on. Renaming looks like the cheap fix and
is not: the discriminant order is asserted in
`tests/wait_test.rs:56-65` with the note *"a reordering changes what a serialized
config means"*, and `WaitKind` is a configuration value that travels through
`RingConfig` (`ring_config/src/lib.rs:89,167`). The name is on the wire.

### WT20 — A Manual Check That Requires a Comment to Exist

W4 in `tests/manual/readme.md` is two commands, and the second is unusual:

```sh
cd "$(git rev-parse --show-toplevel)"
grep -vE "^[[:space:]]*//" ring_wait/src/lib.rs \
  | grep -E "thread::(park|sleep|yield_now)|spin_loop"
grep -c "thread::park" ring_wait/src/lib.rs
```

Live output:

```
        core::hint::spin_loop();
      std::thread::yield_now();
      std::thread::sleep( std::time::Duration::from_micros( 50 ) );
1
```

The first must print three hits — `spin_loop`, `yield_now`, `sleep` — with **no**
`thread::park` among them. The second must return non-zero: `thread::park` must
appear in the file, in the comment explaining why it is not called.

That is a check on prose, and the family has no other. Every other manual check
in every other crate asserts something about *code* — a call is present, a symbol
is absent, a dependency is declared. W4's second half asserts that an
**explanation still exists**, because the failure it guards against is not a
wrong line of code but a right line of code that lost its reason:

> The risk is not the sleep; it is the sleep losing its explanation and looking
> like an oversight to the next reader, who fixes it by reaching for
> `thread::park` and produces a thread nobody will ever unpark.

Both halves currently pass. The first prints `:28`, `:34`, `:39` of the
comment-stripped stream — `spin_loop`, `yield_now`, `sleep` — and the second
returns 1.

The check has a known blind spot: `grep -c` counts *lines*, so it cannot tell
the explanatory comment from a `thread::park` that a future edit adds in code,
as long as the code line is not also stripped by the first command's filter.
In practice the first command catches that case — a real `thread::park` call
would appear in its output, where W4 requires it not to — so the two halves
together are sound, and neither is alone.

### The Deletion Condition

This decision can be reversed, cleanly, when one thing exists: a wakeup
relationship owned by a crate this one may depend on. Concretely, `ring_handle`
gaining a *"register a waiter against this ring, and signal it on publish"* API
would let the `Park` arm become a wait on that signal, and the 50 µs sleep — the
only number in the crate that is a guess — would go with it.

Until then the arm is correct as written, the comment is the record of why, and
W4 is what keeps the comment from being tidied away.


### WT30 — The Deviation Is Recorded Where It Happens, Not Where It Is Chosen

This instance records why `Park` sleeps: parking needs a registration
relationship the crate declines to invent, and `src/lib.rs:134-140` says so in a
comment W4 requires to keep existing (WT20). That explanation is complete and it
is in the wrong crate to be read.

```sh
cd "$(git rev-parse --show-toplevel)"
# what a caller choosing the variant reads
grep -B1 -A3 '  Park,' ring_types/src/policy.rs
# every signalling primitive the family's source mentions. Line numbers are
# dropped and the result sorted: this scans the whole workspace, so an absolute
# address stales on an edit in any crate, and the claim is which files name a
# signalling primitive, never where in one.
grep -riE 'condvar|futex|unpark|thread::park|notify_one|Waker' --include=*.rs /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ . /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/ \
  | grep -v '/docs/' | LC_ALL=C sort
```

Live output:

```
  /// Idle between reads; no publisher wakes it. Cheapest, highest latency.
  Park,
  /// Return immediately with whatever is available, possibly nothing. The only
  /// variant reachable from inside a tick.
  None,
ring_handle/tests/handle_test.rs:  [ "thread::sleep", "yield_now", "::park", "park(", "Condvar", "Duration", "Waker" ];
ring_wait/src/lib.rs:      // Sleeping rather than `thread::park` on purpose. Parking requires the
ring_wait/src/lib.rs:      // publisher to hold the waiter's handle and unpark it, which is a
ring_wait/src/lib.rs:      // relationship here, and the sleep length is what a real unpark would
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/render/rhi_vulkan/src/buffer.rs:    let mut cx = std::task::Context::from_waker( waker );
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/render/rhi_vulkan/src/buffer.rs:    let waker = std::task::Waker::noop();
```

`ring_types::WaitKind::Park` documented the variant as *"Block until a publisher
signals. Costs nothing while idle, pays a wakeup."* Three claims: it blocks until
signalled, it is free while idle, and it pays a one-time wakeup.

The implementation sleeps 50 µs on a timer. There is no signal — the second
command returns six lines and not one of them is a signalling call: three are
this crate's comment explaining why it declines to park, one is `ring_handle`'s
list of parking-shaped names it forbids in its own source (WT13), and the last
two are `rhi_vulkan` constructing a no-op `Waker` to poll a GPU buffer mapping
— a crate outside this family, added after this instance was filed, whose
`Waker` has nothing to do with waking a ring consumer.
So a `Park` wait was not woken by a publisher; it wakes on a schedule and looks
again. It was not free while idle; it pays a syscall per attempt, and WT7
measures the arm at 112–119 µs delivered against the 50 µs requested. There was
no wakeup to pay for.

The variant doc is the text a caller reads at the moment they choose, in the
crate they must already depend on to name the strategy at all (WT28). This
correction sits in a crate two-thirds of `WaitKind`'s users never take a
dependency on, which is why the wording had to be fixed there and could not be
fixed here. The two crates are otherwise unconnected: `ring_types` has no reason
to know how `ring_wait` implements the arm, which is the same independence that
makes the discriminants/handlers split worth having. That independence is also
what the replacement wording has to respect — an accurate variant doc can state
the *cost profile* a caller is choosing between, but not the mechanism a handler
crate happens to use to deliver it.

**Disposition:** applied — in `ring_types/src/policy.rs:30`, the doc's
own crate, which this instance had already named as the owner rather than
change from a corpus pass in `ring_wait`. The line now reads *"Idle between
reads; no publisher wakes it. Cheapest, highest latency."* — the first line of
the Live output above, quoted from the source it corrects. It names no
signalling primitive, so it survives § The Deletion Condition: if `ring_handle`
ever grows the registration API described there and the arm becomes a real
wait, the variant is still the cheapest and still the highest-latency, and only
the 50 µs guess goes. Now prints:
`Idle between reads; no publisher wakes it. Cheapest, highest latency.`

### Decisions

| File | Relationship |
|------|--------------|
| [002_the_discriminants_live_in_ring_types.md](002_the_discriminants_live_in_ring_types.md) | Why the variant's name is not this crate's to change |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | The arm in the context of the other three |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | The tick-path ban this arm is the reason for |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md](../non_functional_requirement/002_what_each_strategy_costs_per_attempt.md) | WT7 — what the 50 µs actually delivers |

### Patterns

| File | Relationship |
|------|--------------|
| [../pattern/002_discriminants_here_handlers_there.md](../pattern/002_discriminants_here_handlers_there.md) | The split that leaves the name in another crate |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_a_sleep_where_a_park_belongs.md](../workaround/001_a_sleep_where_a_park_belongs.md) | The same arm, as an absorbed external constraint with a deletion condition |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/policy.rs:30` | What `Park` promises a caller |
| `ring_poll/src/lib.rs:77-80` | `PARKING_CRATES`, the ban this arm justifies |
| `ring_config/src/lib.rs:89,167` | `WaitKind` travelling through a config |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:56-65` | The discriminant set and its order, which the name is part of |
| `tests/wait_test.rs:159-175` | `Park` loops like the other two blocking kinds |
| `tests/manual/readme.md` § W4 | The sleep is present, `thread::park` is not called, and the explanation still exists |
