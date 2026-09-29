# Non-Functional Requirement: Two Runs Compare Equal

### Scope

- **Purpose**: State the property that makes this a fixture rather than a demo, and be honest about what it is and is not evidence of.
- **Responsibility**: The requirement, how it is enforced, how it is measured, and its limits.
- **In Scope**: `Script::run`'s determinism.
- **Out of Scope**: Concurrent determinism, which no single-threaded run can establish — that is what the loom model in `tests/exhaustive_test.rs` covers.

### The requirement

| # | Requirement | Measurement |
|---|---|---|
| C1 | One script, run against two equivalent rings, produces equal `Outcome`s | `assert_eq!` on the whole value, not a chosen field |
| C2 | The above holds across repeats, not just once | Ten runs compared against a reference |
| C3 | No field of `Outcome` varies with scheduling, address, or hash seed | Enforced by the type: ten fields, none a duration or a rate |

**C3 is the one that does the work.** C1 and C2 are assertions; C3 is why they
can hold. A fixture that recorded elapsed time would fail C1 on the first run,
and one that recorded a thread id would fail it on some machines and not others.
The `Outcome` type is small and boring on purpose.

### How it is enforced

| # | Mechanism | What it removes |
|---|---|---|
| D1 | The script mints its own records — consecutive `u32`s from `0` | A caller-supplied generator, which is a way for two runs to differ |
| D2 | `run` takes a ring rather than building one | Any construction-time variability the fixture would otherwise own |
| D3 | No step spins or retries | A busy-wait whose result depends on nothing the script controls |
| D4 | `RecvMany` stops at the first empty read | An unbounded loop whose iteration count would depend on timing |
| D5 | No field is a timing | C3, structurally |

### What the measurement is worth

**Ten equal runs is the cheap half of the claim, and it is not proof.** A
single-threaded script over a ring with one producer and one consumer has no
interleaving to get wrong — the sequence is the sequence. What C1 and C2
actually establish is that the *fixture* introduces no variability of its own:
no iteration order leaked in, no address, no hash seed.

The expensive half is `tests/exhaustive_test.rs`, which explores every
interleaving of a two-slot ring under a memory model weaker than any real
hardware. Neither subsumes the other, and the split is deliberate:

| | Scale | Coverage |
|---|---|---|
| `testkit_test.rs` | Scripts of any length, ordinary atomics | One interleaving — the one that happened |
| `exhaustive_test.rs` | One push, two slots | Every interleaving |

**A fixture that claimed determinism from C1 and C2 alone would be claiming
something it had not tested.** This file exists to say so.

### Cost

The guard costs one `Acquire` flag read per push, whether or not the script ever
closes. That is the price of `Step::Close` meaning anything, and it is paid on
every push in every script.

`leak` and `leak_ends` allocate and never free. One small ring per loom
execution, which loom's own per-execution bookkeeping dwarfs — and a real cost
for anything that calls them outside a test, which is why both say so.

### Evidence

| # | Claim | Test |
|---|---|---|
| E1 | C1 holds on a five-step script mixing pushes, receives, staging and a drain | `one_script_run_twice_produces_equal_outcomes` |
| E2 | C2 holds across ten runs | `ten_runs_of_one_script_all_agree` |
| E3 | A leaked ring gives the same outcome as a borrowed one | `a_script_runs_the_same_against_a_leaked_ring` |
| E4 | The loom model explores rather than trivially passing | `tests/manual/readme.md` M2 — a negative control fails |

**E4 is the one that was not assumed.** A `loom::model` that passes in 0.02s
looks the same whether it explored 200 interleavings or one. M2 breaks an
assertion deliberately and confirms loom finds the interleaving that violates
it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'the flag read the Cost cites:  %s\n' "$( awk '/pub fn is_closed/{f=1} f && /load/{ sub( /^ */, "" ); print; exit }' ../ring_shutdown/src/lib.rs )"
printf 'C1 script, step variants:      %s\n' "$( awk '/fn one_script_run_twice/{f=1} f && /^}$/{exit} f' tests/testkit_test.rs | command grep -oE 'Step::[A-Za-z]+' | sort -u | tr '\n' ' ' )"
printf 'C2 script, step variants:      %s\n' "$( awk '/fn ten_runs_of_one_script/{f=1} f && /^}$/{exit} f' tests/testkit_test.rs | command grep -oE 'Step::[A-Za-z]+' | sort -u | tr '\n' ' ' )"
printf 'Step variants the enum has:    %s\n' "$( awk '/^pub enum Step/{f=1} f && /^  [A-Z]/{n++} f && /^}$/{exit} END{print n+0}' src/lib.rs )"
printf 'Push steps in the C2 script:   %s\n' "$( awk '/fn ten_runs_of_one_script/{f=1} f && /^}$/{exit} f' tests/testkit_test.rs | command grep -cE 'Step::Push' || true )"
printf 'the C2 ring and stage limit:   %s\n' "$( awk '/fn ten_runs_of_one_script/{f=1} f && /Script::new/{ sub( /^ */, "" ); print; exit }' tests/testkit_test.rs )"
for e in one_script_run_twice_produces_equal_outcomes ten_runs_of_one_script_all_agree a_script_runs_the_same_against_a_leaked_ring ; do
  printf 'E-row is a #[ test ] fn:       %-52s %s\n' "$e" "$( command grep -c "fn $e" tests/testkit_test.rs || true )"
done
printf 'E4 is a #[ test ] fn:          %-52s %s\n' 'M2' "$( cat tests/*.rs | command grep -c 'fn .*negative_control\|fn .*m2' || true )"
printf 'where M2 lives:                %s\n' "$( command grep -m1 -n '^## M2' tests/manual/readme.md )"
printf 'M2 in the Run Record:          %s\n' "$( command grep -m1 '^| M2 |' tests/manual/readme.md )"
printf 'what the plan says about M2:   %s\n' "$( command grep -m1 -oE 'M2 is the stage this plan would be weakest without' tests/manual/readme.md )"
```

Live output:

```
the flag read the Cost cites:  self.closed.load( Ordering::Acquire )
C1 script, step variants:      Step::DrainAll Step::Flush Step::PushMany Step::RecvMany Step::StageMany 
C2 script, step variants:      Step::DrainAll Step::Flush Step::StageMany 
Step variants the enum has:    10
Push steps in the C2 script:   0
the C2 ring and stage limit:   let script = Script::new( 3 ).then( Step::StageMany( 3 ) ).then( Step::Flush ).then( Step::DrainAll );
E-row is a #[ test ] fn:       one_script_run_twice_produces_equal_outcomes         1
E-row is a #[ test ] fn:       ten_runs_of_one_script_all_agree                     1
E-row is a #[ test ] fn:       a_script_runs_the_same_against_a_leaked_ring         1
E4 is a #[ test ] fn:          M2                                                   0
where M2 lives:                54:## M2 — Does the loom model actually explore, or does it trivially pass?
M2 in the Run Record:          | M2 | Does the loom model explore | ✅ negative control fails, restored run passes |
what the plan says about M2:   M2 is the stage this plan would be weakest without
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_script_surface.md](../api/001_the_script_surface.md) | A1 and A2, the guarantees C1 and C2 correspond to |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_outcome_and_anomaly.md](../type/001_outcome_and_anomaly.md) | The ten fields, and why none is a timing |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_from_a_step_to_an_outcome.md](../algorithm/001_from_a_step_to_an_outcome.md) | D3 and D4 in the step loop |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | E1–E3 |
| `tests/exhaustive_test.rs` | The exhaustive half |
| `tests/manual/readme.md` | M2, the negative control |

### TK33 — the repeats claim is measured on the script with the fewest live counters

C1 and C2 are stated as the same property at two scales — one script run twice,
then the same idea across ten runs. They are not run on the same script.

C1's script is five steps over five distinct `Step` variants: `PushMany`,
`RecvMany`, `StageMany`, `Flush`, `DrainAll`, against a four-slot ring with a
four-slot staging buffer. Six pushes into four slots refuse two; five stagings
into four slots refuse one. Both refusal counters carry a non-zero value, and
the `Outcome` compared is a value with most of its fields doing work.

C2's script is three steps over three variants — `StageMany( 3 )`, `Flush`,
`DrainAll` — with `Script::new( 3 )` and an eight-slot ring. Three records into
a three-slot buffer refuse none; three records into eight free slots refuse
none. There is no `Push` step at all: **zero** occurrences of `Step::Push` in
the function. Every refusal counter is structurally zero, and so are both
end-state counts.

So the run that is repeated ten times is the run in which the fewest fields can
disagree. That is not nothing — an address, an iteration order or a hash seed
leaking into `minted`, `accepted`, `received` or `closed_at_end` would still be
caught. It is less than the section claims, because *"C2 holds across ten runs"*
reads as the C1 script repeated, and the C1 script is the one where three more
counters are live.

The enum has ten variants. C1 reaches five of them, C2 reaches three, and their
union is five. Determinism across repeats is currently asserted over three.

### TK34 — the evidence row the section calls load-bearing is the only one nothing re-runs

The Evidence table has four rows and the section names the fourth as the one
that matters: *"E4 is the one that was not assumed."* The reasoning is right — a
`loom::model` that passes in 0.02s looks identical whether it explored two
hundred interleavings or one, so something has to confirm the model can fail.

E1, E2 and E3 are `#[ test ]` functions in `tests/testkit_test.rs`, one match
each, re-run on every `cargo test`. E4 is `tests/manual/readme.md` M2 — a
heading at line 54 of a human procedure, with a Run Record row recording that it
passed once. Searching both test files for any automated form of it returns
**zero**.

The manual plan reaches the same conclusion about itself, in its own closing:
*"M2 is the stage this plan would be weakest without. Every other stage checks
something the automated suite also touches."* Both documents independently
identify M2 as the irreplaceable check, and neither of them arranges for it to
run again.

This is a real constraint, not an oversight to fix by writing a test. The
negative control works by *breaking* an assertion and confirming loom finds the
interleaving that violates it — a mutation, which an ordinary test cannot
perform on itself. `cargo-mutants` is the tool that automates exactly this shape
and the workspace has run it elsewhere.

What is missing is the sentence saying so. As written, a reader counts four
evidence rows, sees three test names and one file path, and has no way to tell
that the row the section vouches for hardest is the row that expires.
