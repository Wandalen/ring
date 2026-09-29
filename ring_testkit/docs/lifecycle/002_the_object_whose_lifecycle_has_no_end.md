# Lifecycle: The Object Whose Lifecycle Has No End

### Scope

- **Purpose**: Trace the lifecycle of the ring the loom bridge produces — which has a construction, a split, a use, and no terminal state at all — and count how many of them a test run leaves behind.
- **Responsibility**: The states a leaked ring occupies, the state it never reaches, where each instance is created, and how the count scales.
- **In Scope**: `leak`, `leak_ends`, and the allocations they hand to `'static`.
- **Out of Scope**: Why the leak is necessary at all (→ [`../workaround/001`](../workaround/001_two_allocations_loom_cannot_avoid.md)); the shutdown states a *script* moves through (→ [`001`](001_the_states_a_script_moves_through.md)), which is a different machine over a different object.

### Two lifecycles, one crate

[`001`](001_the_states_a_script_moves_through.md) traces a script's shutdown
state and closes with a determinate answer: the machine ends Open or Closed, and
`Outcome::closed_at_end` reports which. This file traces the other object the
crate creates, and the interesting property is the opposite one.

| | A script's `Shutdown` | A leaked `Ring` |
|---|---|---|
| States | 2 — Open, Closed | 3 — allocated, split, borrowed |
| Terminal state | either one; the script ends in it | **none** |
| Reported by | `Outcome::closed_at_end` | nothing |
| Instances per run | one per `Script::run` | one per `loom::model` execution, ×2 allocations |
| Freed | with the caller's `Ring` | never |

### The states

| # | State | Entered by | Left by |
|---|---|---|---|
| L1 | **Allocated** — a `Ring< T >` on the heap with no owner | `Box::leak( Box::new( ring ) )` inside `leak` | `Ring::ends`, which borrows it for `'static` |
| L2 | **Split** — an `Ends< 'static, T >`, itself leaked | `Box::leak( Box::new( … .ends() ) )` inside `leak_ends` | `Ends::split` |
| L3 | **Borrowed** — a `Producer` and a `Consumer`, both `'static` | `split` | nothing |

There is no L4. `leak_ends` returns the two ends and the crate declares no
`impl Drop`, no destructor, and no function that takes either end back. Once a
value reaches L3 it stays there for the life of the process.

That is not a defect — it is the definition of the function, and its doc comment
says so in bold: **"Nothing frees this."** What the state table adds is that the
absence is structural rather than an oversight. A terminal state would need an
owner, and the whole purpose of the leak is that there is no owner for a
`loom::thread::spawn` closure to outlive.

### Where instances come from

Both loom and the scripted suite create them, for different reasons:

| Site | Call | Per invocation | Setting |
|---|---|---|---|
| `tests/exhaustive_test.rs` `ends()` | `leak_ends` | 2 allocations | inside the `loom::model` closure — three models |
| `tests/testkit_test.rs:474` | `leak` | 1 allocation | single-threaded, to check the ends outlive their block |
| `tests/testkit_test.rs:491` | `leak` | 1 allocation | single-threaded, to check a script runs the same either way |
| `tests/testkit_test.rs:509` | `leak_ends` | 2 allocations | single-threaded, spawning a `std::thread` |

The loom row is the one that scales. `ends()` is called **inside** the model
closure, and its own comment explains why it must be: *"the cursors are loom
atomics under this cfg and panic if touched with no model running."* loom
re-executes that closure once per interleaving it explores, so the leak count is
loom's execution count times two — a number the test never states and the
allocator never reclaims.

### What the count actually is

Two `Box::leak` calls per execution, and neither allocation is a bare struct.
`ring_core::Ring< T >` holds a `Storage< T >` and an `OverflowPolicy`, and the
storage owns the slot buffer, so the ring's leak is at minimum two heap blocks —
the boxed `Ring` and whatever `Storage` allocated for `CAPACITY` slots. The
`Ends` leak is the third.

The models run a two-slot ring by design, so each individual leak is tiny. The
argument the crate makes is exactly that: *"loom's own per-execution bookkeeping
dwarfs it."* That argument is about size, and it is sound. It is not about
count, and the count is the thing that scales with the model.

### Evidence

| # | Claim | Test |
|---|---|---|
| L-E1 | Ends from a leaked ring outlive the block that built the ring | `a_leaked_ring_gives_ends_that_outlive_their_scope` |
| L-E2 | A script behaves identically on a leaked ring and a borrowed one | `a_script_runs_the_same_against_a_leaked_ring` |
| L-E3 | `leak_ends` produces ends a spawned thread can take by value | `leak_ends_produces_ends_that_can_be_moved_onto_spawned_threads` |
| L-E4 | The bridge works under the cfg it exists for | `tests/exhaustive_test.rs`, all three models |

No test asserts anything about L3 being terminal, because there is nothing to
assert: the absence of a transition is not observable from inside the process.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'what leak does:              %s\n' "$( awk '/^pub fn leak</{f=1} f && /Box::leak/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'what leak_ends does:         %s\n' "$( awk '/^pub fn leak_ends</{f=1} f && /Box::leak/{ sub( /^ */, "" ); print; exit }' src/lib.rs )"
printf 'Box::leak calls in src:      %s\n' "$( command grep -c 'Box::leak' src/lib.rs || true )"
printf 'impl Drop in src:            %s\n' "$( command grep -c 'impl Drop' src/lib.rs || true )"
printf 'what a Ring owns:            %s\n' "$( awk '/^pub struct Ring</{f=1} f && /^  [a-z]/{ sub( /^  /, "" ); printf "%s ", $0 } f && /^}$/{exit}' ../ring_core/src/lib.rs )"
printf 'loom models in the suite:    %s\n' "$( command grep -c 'loom::model( ||' tests/exhaustive_test.rs || true )"
printf 'where ends() is called:      %s\n' "$( awk '/loom::model/{f=1} f && /= ends\(\)/{ sub( /^ */, "" ); print; exit }' tests/exhaustive_test.rs )"
printf 'why it must be inside:       %s\n' "$( command grep -m1 -oE 'panic if touched with no model running' tests/exhaustive_test.rs )"
printf 'leak_ends call sites:        %s\n' "$( for f in tests/*.rs ; do printf '%s=%s ' "${f#tests/}" "$( command grep -c 'leak_ends(' "$f" || true )" ; done )"
printf 'leak call sites, scripted:   %s\n' "$( command grep -c '[^_]leak( ' tests/testkit_test.rs || true )"
printf 'the cfg the scripted suite carries: %s\n' "$( command grep -m1 -oE '#!\[ cfg\( not\( loom \) \) \]' tests/testkit_test.rs )"
docs=$( command grep -h '^///' src/lib.rs | sed 's|^/// \?||' | tr '\n' ' ' )
printf 'leak doc, the cost sentence: %s\n' "$( printf '%s' "$docs" | command grep -oE 'Leaking one ring per model execution' )"
printf 'leak_ends doc, the correction: %s\n' "$( printf '%s' "$docs" | command grep -oE 'Two leaks are needed, not one' )"
```

Live output:

```
what leak does:              Box::leak( Box::new( ring ) )
what leak_ends does:         let ends : &'static mut Ends< 'static, T > = Box::leak( Box::new( leak( ring ).ends() ) );
Box::leak calls in src:      2
impl Drop in src:            0
what a Ring owns:            storage : Storage< T >, overflow : OverflowPolicy, 
loom models in the suite:    3
where ends() is called:      let ( mut producer, mut consumer ) = ends();
why it must be inside:       panic if touched with no model running
leak_ends call sites:        exhaustive_test.rs=1 testkit_test.rs=1 
leak call sites, scripted:   2
the cfg the scripted suite carries: #![ cfg( not( loom ) ) ]
leak doc, the cost sentence: Leaking one ring per model execution
leak_ends doc, the correction: Two leaks are needed, not one
```

### Lifecycles

| File | Relationship |
|------|--------------|
| [001_the_states_a_script_moves_through.md](001_the_states_a_script_moves_through.md) | The other machine — the one that does have a terminal state |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_two_allocations_loom_cannot_avoid.md](../workaround/001_two_allocations_loom_cannot_avoid.md) | Why two leaks and not one |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_two_components_that_meet_in_no_line_of_src.md](../item/001_two_components_that_meet_in_no_line_of_src.md) | `leak` and `leak_ends` as the crate's second component |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_edge_that_only_exists_under_a_cfg.md](../integration/002_the_edge_that_only_exists_under_a_cfg.md) | The cfg the loom row of the sites table runs under |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `leak`, `leak_ends`, and their doc comments |
| [`tests/exhaustive_test.rs`](../../tests/exhaustive_test.rs) | The `ends()` helper and the three models |
| [`tests/testkit_test.rs`](../../tests/testkit_test.rs) | L-E1 through L-E3 |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | L-E1, L-E2, L-E3 |
| `tests/exhaustive_test.rs` | L-E4, and the per-execution multiplier |

### TK31 — the sentence that sizes the leak is attached to the function loom does not call

`leak`'s doc comment justifies the workaround by its cost: *"Leaking one ring
per model execution is the least contrived way to get the lifetime."* One ring,
per execution.

No loom test calls `leak`. All three models reach the bridge through
`leak_ends`, whose own doc comment states the correction in bold three
paragraphs later — **"Two leaks are needed, not one"** — and repeats the caveat
*"twice over: nothing frees either allocation."*

So the crate contains both numbers and puts the smaller one where a reader
sizing the cost will find it. `leak`'s doc is the one that argues the tradeoff;
`leak_ends`' doc is the one that corrects the count, and it corrects it while
explaining a *borrow error*, not a cost. A reader who wants to know what a model
run leaves behind reads the argument, not the borrow-checker note.

The real figure is higher than either. `Box::leak` appears twice in `src/lib.rs`
and a `ring_core::Ring` is not a leaf — it holds a `Storage< T >` that owns the
slot buffer — so a single `leak_ends` strands the boxed `Ring`, the boxed
`Ends`, and the storage's own allocation. Three blocks per execution, described
as one.

None of this makes the design wrong. The rings are two slots wide and loom's
bookkeeping genuinely dwarfs them, which is the argument's actual claim and it
survives the correction. What does not survive is the arithmetic a reader
performs from the sentence they were given.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
command grep -F 'strands three heap blocks, not one' src/lib.rs
```

Live output:

```
/// calls.** Each call strands three heap blocks, not one: the boxed `Ring`,
```

**Disposition:** applied — `leak_ends`' own doc comment now states it, not
`leak`, is what every loom model in this crate calls, and that each call
strands three heap blocks (the boxed `Ring`, its `Storage`, and the boxed
`Ends`), correcting `leak`'s undercounted "one ring per model execution"
sentence at the point a reader sizing the cost actually looks.
Now prints: `strands three heap blocks, not one`

### TK32 — the workaround for a constraint that does not apply runs three times in the suite that does not have it

`leak` and `leak_ends` exist because `loom::thread::spawn` takes `'static`
closures and loom has no scoped threads. That is the stated reason, and under
the loom cfg it is exactly right.

`tests/testkit_test.rs` opens with `#![ cfg( not( loom ) ) ]` and calls the
bridge three times: `leak` twice, at lines 474 and 491, and `leak_ends` once at
509. The file is single-threaded except for two `std::thread::spawn` calls, and
`std::thread` has had scoped threads since 1.63 — so in every one of those three
sites the constraint the workaround answers is absent.

The tests are correct and worth having. L-E1 pins that ends from a leaked ring
outlive their block; L-E2 pins that a script produces the same `Outcome` against
a leaked ring as a borrowed one, which is the property that lets the two halves
of the crate be compared at all; L-E3 pins that the ends can be *moved* onto a
spawned thread, which is the shape loom needs and a scope would not demonstrate.
Testing the bridge requires using the bridge.

What is worth recording is the accounting. Every `cargo test -p ring_testkit`
run — the default one, with no cfg set and no model in sight — permanently
strands four allocations: one ring at 474, one at 491, and a ring plus an `Ends`
at 509. The crate's justification for the leak is a property of a setting those
runs are compiled out of, and nothing in the scripted suite says so.

It is small, it is bounded at four, and it does not grow. It is also the only
place in all thirty-three `ring_*` crates where a test deliberately leaks under
the default configuration, which makes it worth a sentence somewhere — and there
is currently no sentence.
