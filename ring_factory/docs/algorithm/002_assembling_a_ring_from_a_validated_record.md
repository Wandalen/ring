# Algorithm: Assembling a Ring From a Validated Record

### Scope

- **Purpose**: Walk the five fields in the order construction consumes them, and mark the two the crate cannot honour to the precision the record expresses — one because its implementing crate is unreachable, one because it is a count reduced to a boolean.
- **Responsibility**: State the abstract and the algorithm.
- **In Scope**: Field consumption order; what each field reaches; where a failure is still possible.
- **Out of Scope**: The backend branch itself (→ [`algorithm/001`](001_selecting_a_backend_from_one_boolean.md)); registration (→ [`api/002`](../api/002_the_named_build_surface.md)).

### Abstract

**By the time `build` runs, nothing is left to validate.** `Capacity` refused
non-powers-of-two at `RingConfig::new`; `with_batch` and `with_producers`
clamped rather than rejected. The record arriving here is, by construction,
legal. So this algorithm has no validation phase — it is five reads and an
assembly, and its only interesting property is that two of the five reads
cannot produce what the field describes.

| Field | Implementing crate | In closure | Honoured |
|-------|--------------------|-----------|----------|
| `capacity` | `ring_store`, `ring_cursor` | yes | **Fully** — the value passes through unchanged |
| `producers` | `ring_spsc` / `ring_mpsc` | yes | **As a boolean.** The count above 1 is not representable in the result |
| `overflow` | `ring_overflow` | yes | **Two of three.** `ring_core` stores the policy and refuses `DropOldest` outright (→ F3 below) |
| `batch` | `ring_batch` | **no** | **Not at all.** Nothing on the build arc reads it, and the crate is no longer reachable to read it with |
| `wait` | `ring_wait` | **no** | **Not at all** (→ [`pitfall/002`](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md)) |

Regenerate the third column rather than trusting it — the family's closures have
moved three times during this crate's authoring, and `batch`'s row is the
evidence. `ring_batch` was reachable through `ring_core`; `ring_core` was
implemented without it; the crate then arrived only through `ring_tls`, and that
last route has since gone too:

```sh
cd "$(git rev-parse --show-toplevel)"
# `2>/dev/null` is load-bearing, not tidiness. When any other cargo process
# holds the package-cache lock, `cargo tree` writes `Blocking waiting for file
# lock on package cache` to stderr — once per invocation, so seven times here —
# and a recipe that lets those through publishes output that depends on whether
# a concurrent build happened to be running. The counts on stdout do not.
tree=$( cargo tree -p ring_factory --prefix none --no-dedupe 2>/dev/null )
for c in ring_store ring_cursor ring_spsc ring_mpsc ring_overflow ring_batch ring_wait; do
  printf '%-14s %s\n' "$c" "$( printf '%s\n' "$tree" | command grep -c "^$c " )"
done
```

Live output:

```
ring_store     6
ring_cursor    15
ring_spsc      3
ring_mpsc      3
ring_overflow  3
ring_batch     0
ring_wait      0
```

It prints occurrence counts, not a yes/no — `--no-dedupe` repeats a crate once
per path that reaches it, so `ring_cursor 15` means fifteen routes and
`ring_wait 0` means none. **Non-zero is the only reading that matters here;**
the magnitudes are an artefact of the graph's shape, not a measure of anything.

**`ring_batch 0` is the row to watch, and an earlier draft of this file watched
it reading `1`.** The prediction attached to that `1` was written here verbatim:
it had exactly one route, through a crate the build arc never calls
(→ [`integration/001`](../integration/001_declared_edges_and_the_reached_closure.md)'s
E7), and *a single route through an unused dependency is one manifest edit away
from zero.* It was one manifest edit, and the edit landed.

**It landed at the other end of the route than the guess.** `ring_tls` still
declares `ring_batch`; what changed is that `ring_factory` stopped declaring
`ring_tls`, and its own manifest states the reason — `RingConfig` has no field
that would configure staging or counters, so there was nothing for a build to do
with the dependency, and inventing surface to justify a manifest line is
backwards. The route did not break where it was thin. It broke where someone had
a reason to touch it.

**A count of 1 said the route was fragile; it did not say which of the two
manifests along it would move.** That is the transferable part. The magnitude
was read correctly and the mechanism was read correctly, and the specific edge
named in the inference was still the wrong one — so the value of the count was
in prompting the regeneration, never in the story attached to it.

**The two fields with no honouring code on this arc are `batch` and `wait`, and
they arrived there by opposite routes.** `wait` was never reachable. `batch`
was, twice, and lost both — first the route that mattered, when the crate that
would have used it was written without it, then the leftover one, when the arc
dropped the dependency carrying it. Neither loss is visible as a compile error,
because neither field is read at all.

### Algorithm

**Step 1 — branch on `producers`.** Covered in
[`algorithm/001`](001_selecting_a_backend_from_one_boolean.md). Total, no
failure edge, and the *only* step at which `producers` is read.

**Step 2 — construct the backend with `capacity`.** Both backends already
accept exactly this:

```rust
ring_spsc::Ring::< S >::new( cfg.capacity() )
ring_mpsc::Ring::< S >::new( cfg.capacity() )
```

`Capacity` is a validated newtype, so the argument cannot be wrong. This is the
one field with no gap of any kind between what the caller asked, what the
record holds, and what the ring does.

**Step 3 — hand `overflow` to `ring_core`, which may refuse it.** This step was
specified as "wrap with `overflow`", on the reading that `OverflowPolicy`'s three
variants are total and the design guarantees no variant overwrites unread data
— so every policy is a choice about *which* record is lost on a full ring, never
about corrupting one that was accepted. **That reading was right about the
policy and wrong about the backends.** `ring_core` stores the policy and
rejects one of the three before constructing anything:

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -A5 -F '  pub fn new( config : &RingConfig ) -> Result< Self, RingError >' ring_core/src/lib.rs
```

Live output:

```
  pub fn new( config : &RingConfig ) -> Result< Self, RingError >
  {
    if config.overflow() == OverflowPolicy::DropOldest
    {
      return Err( RingError::PolicyUnsupported );
    }
```

`DropOldest` asks for eviction of an unread record, which contradicts the
exactly-once delivery both in-house backends provide. The refusal is correct;
what it does to this algorithm is give step 3 a failure edge that steps 1, 2, 4
and 5 do not have (→ F3).

**Step 4 — `batch`. Nothing happens, and now nothing can.** This step was
specified as "wrap with `batch`", on the strength of `ring_batch` being in the
closure and `batch` being pre-clamped into `1..=capacity` so the wrapping could
not be asked for more than the ring holds. **The clamp half is still true; the
closure half is not.** `ring_core` never reads `config.batch()`, and `ring_batch`
is no longer in `ring_factory`'s closure at all — the step is empty, and its
implementing crate is now a manifest edit away rather than a call away.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep 'config\.' ring_core/src/lib.rs | command grep -v '^ *///'
```

Live output:

```
    if config.overflow() == OverflowPolicy::DropOldest
    let storage = match config.is_multi_producer()
    Ok( Self { storage, overflow : config.overflow() } )
          crossbeam_queue::ArrayQueue::new( config.capacity().get() ),
          config.capacity(),
        overflow : config.overflow(),
```

**Three of the five fields are read on this arc, and `batch` is not one of
them.** That is worth stating as a measured fact rather than an expectation,
because the step is easy to mark done: the clamp is in place, the crate is in
the closure, and nothing fails.

**Step 5 — `wait`. Nothing happens**, and the question of whether the step
should exist narrowed rather than closing. `ring_wait` shipped as free functions
taking a `WaitKind` per call, so there is **no waiter object to construct** — the
factory-resolves-it branch has nothing to resolve into. What remains open is
smaller: who carries the kind from the config to the call site, given that a
`Split` does not carry its config and `RingConfig` is `Copy` so the caller still
holds theirs. Pending 5 in [`decisions/`](../decisions/readme.md).

**Step 6 — hand back the owner, not the pair.** `Split::new( ring )`, returned by
value. `ring_handle` is now a directly declared dependency rather than being
reachable only through `ring_registry`
(→ [`integration/001`](../integration/001_declared_edges_and_the_reached_closure.md)'s
requirement 1), which it had to become before this step could name its own
return type.

#### The two imprecise fields, stated plainly

**`wait` is not honoured and the gap is visible.** Its pitfall instance covers
it. It is at least *loud* in the sense that no code exists to do the wrong
thing — the failure is omission.

**`producers` is honoured imprecisely and the gap is invisible**, which makes
it the worse of the two. Above 1, the count does not reach the ring:

| `cfg.producers()` | Backend | Ring produced |
|-------------------|---------|---------------|
| 1 | SPSC | The SPSC ring |
| 2 | MPSC | The MPSC ring |
| 8 | MPSC | **The same MPSC ring** |
| 64 | MPSC | **The same MPSC ring** |

This is correct behaviour and not a defect: `ring_mpsc`'s claim path is a
compare-exchange, so it admits any number of producers without being told one.
The consequence is for the *criterion*, not the ring — this crate's own acceptance criterion asks that
observable behaviour match every field, and for `producers` the observable
difference between 2 and 64 is entirely in how many threads the test spawns,
not in anything `build` returned.

**So a sweep varying `producers` across 2, 4, 8, 16 is measuring contention,
not construction**, and that is the right thing to measure. What must not
happen is a `build`-level assertion claiming to have verified the field, when
the strongest available claim is "above one selected MPSC"
(→ [`non_functional_requirement/001`](../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md)).

#### Where a failure is still possible

Steps 1, 2, 4 and 5 cannot fail — the record is pre-validated and every read is
infallible. Step 3 can, and two other things could refuse:

| # | Refusal | Status |
|---|---------|--------|
| F1 | Allocation failure for the backing buffer | Rust aborts rather than returning `Err`. Not this crate's to report |
| F2 | A duplicate name on the registering path | Real, and the only error edge this crate raises itself (→ [`lifecycle/004`](../lifecycle/004_name_state_through_a_registration.md)) |
| F3 | `overflow == DropOldest` at step 3 | **Real, on both paths, and relayed rather than raised.** `ring_core::Ring::new` returns `Err( RingError::PolicyUnsupported )` from a config `ring_config` accepted without complaint |

**F3 did not exist when this section was written and its arrival changed the
crate's signature.** The section previously concluded: "F2 being the only one is
why [`type/002`](../type/002_build_error.md) has so little to hold. A `build`
that cannot fail should not return `Result`." The premise was true of every line
of code then in the repository, and stopped being true while this crate's
documentation was still being written, when `ring_core` was implemented.

**The lesson is about which absences are evidence.** F1 and F2 were derived from
things that exist — Rust's allocation behaviour, the registry's semantics. The
conclusion that nothing else could fail was derived from a *skeleton*: a
`ring_core` with zero items refuses nothing, and reading that as "construction
is infallible" mistook unwritten for decided. **When that conclusion was drawn,
nine of the family's thirty-three crates were empty, so an absent failure mode
here was an unmeasured one rather than an established absence. None are empty
now:**

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim is
# a parallel ugrep that emits hits in completion order. The name list is joined
# with `paste` and suppressed entirely when empty, so no line ever ends in
# whitespace — the markdown file has had its own trailing whitespace stripped,
# so a recipe emitting a trailing space could never match a quoted block
empty=$( for c in ring_*/; do
  [ "$( cat "$c"src/*.rs 2>/dev/null | command grep -cE '^\s*(pub )?(fn|struct|enum|trait|type|const) ' )" = 0 ] \
    && basename "$c"
done | LC_ALL=C sort | paste -sd' ' - )
printf '  crates in the family : %s\n' "$( ls -d ring_*/ | wc -l )"
printf '  of those, empty      : %s%s\n' "$( printf '%s' "$empty" | wc -w )" "${empty:+ -- $empty}"
```

Live output:

```
  crates in the family : 33
  of those, empty      : 0
```

**Reaching zero confirms the lesson rather than retiring it.** Every one of those
nine crates has since gained the items that make its refusals real, and F3 is
what one of them turned out to contain — the failure mode that arrived while this
document was being written is exactly the kind the count was warning about. The
count is worth keeping at zero for the same reason it was worth stating at nine:
it is the difference between "nothing else can fail" and "nothing else has been
written yet." This instance's step-3 and step-4 recipes print what is actually
read rather than asserting what will be, for that reason.

**F3 also breaks F2's exclusivity in a way worth naming.** `build_named` now has
two variants to distinguish, and they need different handling: `NameTaken` means
retry with another name, `Unsupported` means the config can never work. A caller
that matches only on `is_err()` will retry a build that cannot succeed.

### Algorithms

| File | Relationship |
|------|--------------|
| [001_selecting_a_backend_from_one_boolean.md](001_selecting_a_backend_from_one_boolean.md) | Step 1, in full |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_build_surface.md](../api/001_the_build_surface.md) | F1 and F2's consequence for the signature |
| [../api/002_the_named_build_surface.md](../api/002_the_named_build_surface.md) | F2's surface |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_the_configuration_record_as_input.md](../data_structure/001_the_configuration_record_as_input.md) | The five fields, and which arrive already corrected |
| [../data_structure/002_the_handle_pair_as_output.md](../data_structure/002_the_handle_pair_as_output.md) | Step 6's product |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_declared_edges_and_the_reached_closure.md](../integration/001_declared_edges_and_the_reached_closure.md) | The closure the third column of the Abstract table is read from |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/001_from_a_record_to_a_handle_pair.md](../lifecycle/001_from_a_record_to_a_handle_pair.md) | These six steps as phases, with F2 as the one phase that can end early |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md](../non_functional_requirement/001_five_fields_asserted_one_at_a_time.md) | What "one field at a time" can honestly claim for `producers` and `wait` |
| [../non_functional_requirement/002_construction_cost_is_paid_once.md](../non_functional_requirement/002_construction_cost_is_paid_once.md) | Steps 2–4 allocate; the requirement is that nothing they build costs a tick |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md](../pitfall/002_a_wait_strategy_it_can_read_and_cannot_honour.md) | Step 5, in full |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_build_error.md](../type/002_build_error.md) | F1 and F2 — why the error type has one variant and not five |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_config/src/lib.rs`](../../../ring_config/src/lib.rs) | The five accessors this algorithm reads |
| [`ring_types/src/policy.rs`](../../../ring_types/src/policy.rs) | `OverflowPolicy`'s three variants and `WaitKind`'s four |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/factory_test.rs`](../../tests/factory_test.rs) | ✅ Written to that instruction, and it changed the finding. Steps 2–4 were expected to be assertable one field at a time; only **two of the three** are — `only_two_of_five_config_fields_are_observable_through_the_factory` names capacity and overflow as observable and producers, wait and batch as not. The test says which of the three it is doing, per this row, and the saying is what exposed the gap |

### FC3 — The Assembly Has No Control Flow At All

Five reads and an assembly, said the abstract. The assembly turns out to be a
constructor call and a wrap, and the crate contains no branch that decides
anything about a ring:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- control flow in the whole crate, doc comments excluded --'
for k in 'if ' 'return ' 'match ' 'while ' 'loop'; do
  printf '  %-8s %s\n' "$k" \
    "$( command grep -vE '^\s*(///|//!)' ring_factory/src/lib.rs | command grep -c "$k" )"
done
printf '  %-8s %s\n' 'for..in' \
  "$( command grep -vE '^\s*(///|//!)' ring_factory/src/lib.rs | command grep -cE 'for .+ in ' )"
echo '  -- and where the two matches are --'
command grep -E '^\s+match ' ring_factory/src/lib.rs
```

Live output:

```
  -- control flow in the whole crate, doc comments excluded --
  if       0
  return   0
  match    2
  while    0
  loop     0
  for..in  0
  -- and where the two matches are --
    match registry.register( name, split )
    match self
```

Zero `if`, zero `return`, zero loops. Two `match` expressions: one dispatches a
registration result, one formats an error message. Neither reads a config field.

Every step in the walk above is therefore a step taken by `ring_core`, reported
here. This crate's contribution to "assembling a ring from a validated record"
is `Ring::new( &cfg ).map_err( BuildError::Unsupported )?` — one call, one error
translation, one `Split::new`. The value of writing the walk out is not that it
describes work this crate does; it is that the five-field consumption order is
not documented anywhere else at all, and a reader of `ring_core` sees the fields
arriving already extracted.

That also fixes what a change to this instance costs: any field whose handling
moves inside `ring_core` invalidates the walk without touching a line of
`ring_factory`, so the table above is a claim about a crate this file cannot
watch. The `### Regenerate` block in [`readme.md`](readme.md) is the mitigation,
and it is a weak one — it re-derives the closure, not the honouring.

### FC4 — Two Fields Reach Nothing, and the Only Crate That Reads Them Builds No Rings

`wait` and `batch` are set through infallible builders, survive into the record,
and are then read by nobody on the arc from `RingConfig` to a live ring:

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- readers of the two unhonoured fields, on the build arc --'
for c in ring_factory ring_core; do
  printf '  %-13s wait(): %s  batch(): %s\n' "$c" \
    "$( command grep -vE '^\s*(///|//!)' $c/src/lib.rs | command grep -c 'wait()' )" \
    "$( command grep -vE '^\s*(///|//!)' $c/src/lib.rs | command grep -c 'batch()' )"
done
echo '  -- and every crate in the family that does read either --'
command grep -rl '\.wait()\|\.batch()' --include=*.rs ring_*/src/ \
  | sed 's|ring/||;s|/src/.*||' | sort -u
```

Live output:

```
  -- readers of the two unhonoured fields, on the build arc --
  ring_factory  wait(): 0  batch(): 0
  ring_core     wait(): 0  batch(): 0
  -- and every crate in the family that does read either --
ring_bench
ring_config
```

Two crates family-wide. One of them is `ring_config`, which defines the
accessors, and the other is `ring_bench`, which is a measurement harness and
constructs its own waiting rather than asking a ring to do it. Across the
remaining thirty-one crates, `.wait()` and `.batch()` are never called.

So the record carries two fields whose only consumer is a benchmark. Setting
either changes nothing a caller can observe and produces no warning, no `Err`,
and no `#[ deprecated ]` — the builders are `const fn` returning `Self`, so even
`#[ must_use ]` would not fire on the value being discarded into a build that
ignores it. The honest reading is that `RingConfig` is one record serving two
audiences: the fields a ring is built from, and the fields a benchmark is
configured by. Nothing in the type says which is which, and this instance's
table is the only place the split is written down.
