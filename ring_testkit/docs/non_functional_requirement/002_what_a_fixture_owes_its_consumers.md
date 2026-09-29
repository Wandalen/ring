# Non-Functional Requirement: What A Fixture Owes Its Consumers

### Scope

- **Purpose**: State the obligations a crate takes on by being a testkit rather than a library, and check each against what this crate actually provides.
- **Responsibility**: Four obligations, their measurement, and which are met, unmet, or met without being promised.
- **In Scope**: Panic behavior, cfg neutrality, import cost, and matching stability, as properties a consumer would depend on.
- **Out of Scope**: Determinism, which is [`001`](001_two_runs_compare_equal.md)'s whole subject; what each signature guarantees (→ [`../api/001`](../api/001_the_script_surface.md)).

### Why a fixture's obligations are different

A library is depended on because a consumer wants what it computes. A fixture is
depended on because a consumer wants to know whether *their own* code is right,
which puts the fixture inside the consumer's failure diagnosis. Four properties
follow from that position and from nothing else:

| # | Obligation | Why it is a fixture obligation specifically |
|---|---|---|
| F1 | A failure inside the fixture is distinguishable from a failure in the code under test | A panicking fixture reports itself as the bug, in a test the consumer wrote to find their own |
| F2 | The fixture behaves the same under whatever cfg the consumer's own build sets | A consumer sets cfgs for their reasons; a fixture that changes underneath is unusable as a control |
| F3 | Taking the fixture on costs no more of the consumer's manifest than it must | A fixture is reached for by crates outside the family, which is the only reason it exists as a crate |
| F4 | The types a consumer matches on can grow without breaking them | A fixture's whole output is values a consumer inspects |

### How this crate stands

| # | Obligation | Verdict | Measurement |
|---|---|---|---|
| F1 | Panic-free | **met, unpromised** | Zero `unwrap`, `expect` or `panic!` in `src/lib.rs` outside doc comments → TK35 |
| F2 | cfg-neutral | **unmet** | Zero `#[ cfg ]` in `src/`, and the atomics four crates down switch under `--cfg loom` → TK36 |
| F3 | Cheap to take on | **unmet** | Zero `pub use`; three manifest entries to call one function (→ [`../item/002`](../item/002_what_the_crate_does_not_declare.md) TK28) |
| F4 | Stable to match on | **met** | `Anomaly` carries `#[ non_exhaustive ]`, so a fourth variant is additive — and one has already arrived that way (→ [`../item/002`](../item/002_what_the_crate_does_not_declare.md) TK27) |

Two of four — and the first of them is the one nothing asked for.

### F1 in detail — what makes a fixture failure legible

`Script::run` executes ten step kinds against a caller's ring and returns a
value. Every fallible operation inside it is matched rather than unwrapped:
`guard.try_push` at two sites feeding three counters, `staging.push` behind an
`is_err()` test, `consumer.try_recv` behind an `if let`. There is no path on
which the fixture aborts the consumer's process.

That is exactly the property F1 asks for, and it is worth more here than the
same property would be in a library: a panic from four crates below a
`#[ test ]` the consumer wrote is a diagnosis problem, not just a failure.

### F2 in detail — the seam the fixture sits above

`ring_testkit` declares no cfgs at all. The atomics it ultimately drives do:

```text
ring_testkit → ring_core → ring_spsc → ring_cursor → ring_atomic
                                                          │
                                          #[ cfg( loom ) ] use loom::sync::atomic
                                          #[ cfg( not( loom ) ) ] use core::sync::atomic
```

`ring_atomic`'s module documentation states the design and its intended
consequence: *"This is the family's only such switch"*, and *"no other crate
needs to know the seam exists."* For the thirty-two crates whose own tests set
the cfg deliberately, that is true and is the point of putting the switch in one
place.

`ring_testkit` is the exception, and the crate's own test suite already knows it
— `tests/exhaustive_test.rs`'s helper carries the comment *"the cursors are loom
atomics under this cfg and panic if touched with no model running"*, which is
why it constructs its ring inside the model closure rather than outside. That
knowledge lives in a test file and in no part of the public surface. → TK36.

### Evidence

| # | Claim | Test |
|---|---|---|
| F-E1 | A ring that refuses every push returns an `Outcome` rather than panicking | `a_closed_ring_refuses_with_a_reason_of_its_own` |
| F-E2 | A zero-slot staging buffer is a run, not a crash | `a_zero_slot_staging_buffer_refuses_every_record` |
| F-E3 | The crate builds and its models run under `--cfg loom` | `tests/manual/readme.md` M6, three configurations |
| F-E4 | Nothing automated covers F2 | no test sets `--cfg loom` and calls `Script::run` |

F-E1 and F-E2 are the closest the suite comes to testing F1, and neither is
written as a panic-freedom test — both assert an `Outcome`'s contents and
establish panic-freedom only as a side effect of having returned one.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'panic surface in src, all lines:  %s\n' "$( command grep -cE 'unwrap\(|expect\(|panic!' ring_testkit/src/lib.rs || true )"
printf 'of those, outside doc comments:   %s\n' "$( command grep -vE '^\s*//' ring_testkit/src/lib.rs | command grep -cE 'unwrap\(|expect\(|panic!' || true )"
printf 'cfg attributes in ring_testkit:   %s\n' "$( command grep -cE '^ *#!?\[ cfg' ring_testkit/src/lib.rs || true )"
printf 'cfg mentioned in its doc prose:   %s\n' "$( command grep -cE '^ *//[/!].*cfg' ring_testkit/src/lib.rs || true )"
printf 'the chain to the seam:\n'
for c in ring_testkit ring_core ring_spsc ring_cursor ring_atomic ; do
  d=$( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f && /^ring_/{ sub( / =.*/, "" ); printf "%s ", $0 }' $c/Cargo.toml )
  printf '  %-13s -> %s\n' "$c" "${d:-<no ring_ deps>}"
done
printf 'the switch itself:                %s\n' "$( command grep -A1 '#\[ cfg( loom ) \]' ring_atomic/src/lib.rs | command grep -m1 'use loom' | sed 's/^ *//' )"
joined=$( awk '/^\/\/!/{ sub( /^\/\/! ?/, "" ); printf "%s ", $0 }' ring_atomic/src/lib.rs )
printf 'ring_atomic on the switch:        %s\n' "$( printf '%s' "$joined" | command grep -m1 -oE "This is the family's only such switch" )"
printf 'ring_atomic on who must know:     %s\n' "$( printf '%s' "$joined" | command grep -m1 -oE 'no other crate needs to know the seam exists' )"
printf 'what the loom test knows:         %s\n' "$( command grep -m1 -oE 'panic if touched with no model running' ring_testkit/tests/exhaustive_test.rs )"
printf 'src files stating it publicly:    %s\n' "$( command grep -lE 'no model running|outside a model' ring_testkit/src/*.rs 2>/dev/null | wc -l )"
printf 'F-E1 and F-E2 exist:              %s\n' "$( for t in a_closed_ring_refuses_with_a_reason_of_its_own a_zero_slot_staging_buffer_refuses_every_record ; do printf '%s=%s ' "$t" "$( command grep -c "fn $t" ring_testkit/tests/testkit_test.rs || true )" ; done )"
```

Live output:

```
panic surface in src, all lines:  7
of those, outside doc comments:   0
cfg attributes in ring_testkit:   0
cfg mentioned in its doc prose:   10
the chain to the seam:
  ring_testkit  -> ring_core ring_tls ring_shutdown 
  ring_core     -> ring_config ring_mpsc ring_overflow ring_slot ring_spsc ring_types 
  ring_spsc     -> ring_store ring_config ring_cursor ring_slot ring_types 
  ring_cursor   -> ring_types ring_seqno ring_atomic ring_align 
  ring_atomic   -> ring_types 
the switch itself:                use loom::sync::atomic::{ AtomicU64, AtomicUsize };
ring_atomic on the switch:        This is the family's only such switch
ring_atomic on who must know:     no other crate needs to know the seam exists
what the loom test knows:         panic if touched with no model running
src files stating it publicly:    1
F-E1 and F-E2 exist:              a_closed_ring_refuses_with_a_reason_of_its_own=1 a_zero_slot_staging_buffer_refuses_every_record=1 
```

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [001_two_runs_compare_equal.md](001_two_runs_compare_equal.md) | The one obligation this crate does state, and its evidence |

### Items

| File | Relationship |
|------|--------------|
| [../item/002_what_the_crate_does_not_declare.md](../item/002_what_the_crate_does_not_declare.md) | F3 and F4 as absences — TK27 and TK28 |

### APIs

| File | Relationship |
|------|--------------|
| [../api/002_the_surface_no_crate_has_taken.md](../api/002_the_surface_no_crate_has_taken.md) | The zero consumers that make all four verdicts currently costless |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/002_the_edge_that_only_exists_under_a_cfg.md](../integration/002_the_edge_that_only_exists_under_a_cfg.md) | The cfg F2 is about, and who else in the family reads it |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_object_whose_lifecycle_has_no_end.md](../lifecycle/002_the_object_whose_lifecycle_has_no_end.md) | The allocations a consumer inherits by calling the bridge |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | F1's panic count and F2's zero cfgs |
| [`Cargo.toml`](../../Cargo.toml) | The first link of the chain |

### Tests

| File | Relationship |
|------|--------------|
| `tests/testkit_test.rs` | F-E1, F-E2 |
| `tests/exhaustive_test.rs` | The comment that records F2 privately |
| `tests/manual/readme.md` | F-E3 |

### TK35 — the one obligation the fixture meets is the one it never promises

`src/lib.rs` contains no `unwrap`, no `expect` and no `panic!` outside doc
comments. Every fallible call is handled: `guard.try_push` at both accounting
sites, `staging.push` behind `is_err()`, `consumer.try_recv` behind `if let`.
`Script::run` cannot abort a caller's process, whatever ring, staging limit or
step sequence it is handed.

For a fixture that is the load-bearing property. A consumer writes a `#[ test ]`
to find a bug in *their* code; a panic raised inside the fixture instead points
four crates away from where they are looking, and the first thing they must do
is establish which of the two failed. Panic-freedom is what makes the fixture's
verdict readable.

Nothing states it. There is no `# Panics` section on `Script::run`, no sentence
in the module documentation, no entry in [`001`](001_two_runs_compare_equal.md)'s
requirement table, and no test named for it. The two tests that come closest —
`a_closed_ring_refuses_with_a_reason_of_its_own` and
`a_zero_slot_staging_buffer_refuses_every_record` — assert the contents of an
`Outcome` and establish panic-freedom only incidentally, by virtue of having
returned.

So a consumer learns the property by reading 741 lines, and having learned it
has no guarantee it survives the next change: an `expect` added inside `run` for
what looks like an impossible case would break nothing in the suite and violate
nothing written down. The crate documents its determinism at length and its
panic behavior nowhere, and of the two it is panic behavior a consumer meets
first when something goes wrong.

### TK36 — the crate that most needs to know about the seam is the one told it does not

`ring_atomic` owns the family's `--cfg loom` switch and says so plainly: *"This
is the family's only such switch"*, and the design that follows from it, *"no
other crate needs to know the seam exists."* Thirty-two crates reach their
atomics through `AtomicSeq` and are correct to be ignorant of it.

`ring_testkit` reaches them too — `ring_testkit → ring_core → ring_spsc →
ring_cursor → ring_atomic` — and declares **zero** `cfg` attributes of its own.
`Script::run` therefore compiles identically with and without the cfg, while the
cursors underneath it become loom atomics that carry model state and, as the
crate's own loom test records, *"panic if touched with no model running."*

The consequence is a consumer-visible hazard the public surface does not
mention. A crate that sets `--cfg loom` for its own reasons and calls
`Script::run` outside a `loom::model` closure gets a panic from four crates
down, in a function whose signature, documentation and every doc example are
silent about the cfg — and this is a fixture, which by construction is reached
for by crates that are not part of the family and have no reason to have read
`ring_atomic`'s module docs.

The knowledge existed in the crate. It was a comment on a private helper in
`tests/exhaustive_test.rs`, explaining why the helper builds its ring inside the
model closure. **Zero** files in `src/` said anything about it.

The ignorance `ring_atomic` grants is correct for a crate whose loom exposure is
its own test suite's business. It is wrong for the one crate in the family whose
purpose is to be called by strangers, and this crate inherited the grant without
having the property that justifies it.

**Disposition:** applied — to the module documentation, and the `cfg` attribute
is **declined**. The crate still declares zero `cfg` of its own, because a `cfg`
here would be a fifth copy of a switch `ring_atomic` explicitly says no other
crate should need, and it would not stop the panic anyway — the atomics are
loom's whichever way this crate is compiled. What the crate now carries is a
`# Under --cfg loom` module section that states the whole chain in public: that
declaring no `cfg` is a decision rather than an omission, that the seam arrives
four crates down through `ring_core` → `ring_mpsc`/`ring_spsc` → `ring_atomic`,
that under the cfg loom's atomics panic when touched outside a `loom::model`, and
that `Script::run` therefore panics on a ring built anywhere else — from a crate
that mentions neither loom nor the cfg. The census that found this needed two
repairs to keep finding it: the `cfg` count was unanchored, so the new prose
mentioning `#![ cfg( not( loom ) ) ]` read as a declaration and turned a true
`0` into a false `1`; and both quotes from `ring_atomic` were matched with a
one-line `grep`, which returned empty the moment that paragraph was rewrapped —
empty being indistinguishable, in the rendered output, from the sentence having
been deleted. What this does not buy: a consumer still gets a panic rather than
a compile error, the doc section is only reachable by someone who reads the
module page, and nothing in the build checks any of it. Now prints:
`src files stating it publicly:    1`
