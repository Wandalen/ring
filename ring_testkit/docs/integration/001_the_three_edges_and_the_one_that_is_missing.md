# Integration: The Three Edges And The One That Is Missing

### Scope

- **Purpose**: Account for each declared dependency edge, and for the crate this one needs and was not given.
- **Responsibility**: What each edge is used for, how much of it is reachable, and why two more are dev-dependencies.
- **In Scope**: `ring_core`, `ring_tls`, `ring_shutdown`, `ring_config`, `ring_types`, `loom`.
- **Out of Scope**: Whether the missing edge should be added — [`../decisions/readme.md`](../decisions/readme.md) Pending 4.

### System Description

A fixture crate sits in an unusual position: it depends on the crates it drives,
and nothing in the family depends on it at runtime. That makes its dependency
list a statement about *what the fixture can exercise* rather than about what it
needs to compile.

Three runtime edges, two dev-dependencies that are not really edges, and one
absence that is the crate's most-cited limitation.

### Integration Points

This crate's three declared edges are `ring_core`, `ring_tls`, `ring_shutdown`.
Unusually for this family, **all three are real** — the registry needed one of its three, the
debug crate two of two, and this one uses every edge it was given.

| Edge | Used for | How much of it is reachable |
|---|---|---|
| `ring_core` | The ring being driven: `Ring`, `Ends`, `Producer`, `Consumer` | All of the push/receive surface. Not the cursors — there are none |
| `ring_tls` | The staging buffer: `TlsBuffer::{ with_capacity, push, drain, len }` | Everything **except** `flush_into`, the crate's headline operation — → [`pitfall/003`](../pitfall/003_the_amortised_flush_has_no_ring.md) |
| `ring_shutdown` | The closable guard: `Shutdown`, `Guarded`, `Stopped`, `Refusal` | `close`, `guard`, `reopen`, `drain_all`, and both `Refusal` arms. Not `wait_for_close`, `for_space_or_close`, or `Wake` — `Script::run` itself never blocks or waits, though `leak`/`leak_ends` do routinely hand a ring's ends to two threads elsewhere in this crate |

#### The `ring_tls` edge is the interesting one

`TlsBuffer` is used for staging, and `flush_into` — its batch-claiming
operation — **cannot be reached from here**. It claims against a
`SeqCell`, and a `ring_core::Ring` exposes no cursor of any kind. So the staged
records go into the ring one `try_push` at a time.

That leaves `flush_into` with **no caller anywhere in the family**, still,
after the one crate that stages and publishes in the same function tried to use
it. Recorded rather than worked around: a fixture cannot invent the join two
crates do not have.

#### The two dev-dependencies, and why they are not real edges

| Crate | Why it is a dev-dependency |
|---|---|
| `ring_config` | `Script::run` takes a ring the caller already built, so `RingConfig` appears in no signature here. Only the tests and doc examples need to build a ring to drive |
| `ring_types` | `RingConfig::with_overflow` takes an `OverflowPolicy`, and `ring_config` does not re-export it — so a test that varies the policy has to name the crate the vocabulary lives in |

**`ring_types` being needed at all is a small finding.** A caller configuring a
ring touches two crates to set one field, because the setter and its argument
type live apart. It is on the family's export Contract so nothing is hidden, but
it is one import more than the shape suggests.

#### `loom`, and why the model is in `tests/`

`loom` is a `[target.'cfg(loom)'.dev-dependencies]` entry, the same shape
`ring_atomic`, `ring_publish`, `ring_spsc` and `ring_mpsc` already use. Under
`--cfg loom`, `ring_atomic` swaps in loom's atomics, and those panic when
touched outside a `loom::model`.

The model therefore lives in `tests/exhaustive_test.rs` and **not** in `src/`,
for a reason that is about measurement: a coverage run does not set the cfg, and
`cfg`-removed lines in `src/` are counted as uncovered. Keeping the scripted
fixture free of `cfg` gates is what lets it be measured at 100% by an ordinary
`cargo tarpaulin`, with the exhaustive half graded by its own run.

#### The missing edge

This crate needs a model checker that explores interleavings exhaustively.
It does not contain one and should not: `loom` is the family's model
checker and four crates already depend on it. What this crate contributes is the
bridge — `leak` and `leak_ends`, the two allocations a `loom::thread::spawn`
closure needs to reach a borrow-based ring, and `audit_received`, the assertion
worth making once inside the closure.

### Error Handling

Nothing across these three edges is an error in the `Result`-carrying sense that
reaches the fixture's caller. Every refusal is *data* — a counter — because a
fixture that returned early on a refusal could not describe a script whose point
is to provoke one:

| Edge | Failure that can arrive | What the fixture does with it |
|---|---|---|
| `ring_core` | A full ring refuses a push | `refused_full` += 1; the record is dropped, the run continues |
| `ring_tls` | A full staging buffer refuses a push | `refused_staging` += 1; `StageMany` keeps going, `Stage` returns |
| `ring_shutdown` | `Refusal::Closed( record )` | `refused_closed` += 1; the run continues |
| `ring_shutdown` | `Refusal::Full( record )` | Folded into `refused_full` — the arm, not the reason, is what the counter tracks |

**`Script::run` returns `Outcome`, not `Result< Outcome, _ >`, and that is the
design.** The one thing that *can* be wrong is the accounting, and it is checked
afterwards by `Outcome::audit` returning
[`Anomaly`](../type/001_outcome_and_anomaly.md) — a separate call, so a caller
who wants the counters from a deliberately-broken run still gets them.

### Compatibility Requirements

All three edges point downward, into crates this one drives. Nothing in the
family depends on `ring_testkit` at runtime, and nothing should: it is a
dev-facing fixture, and an edge into it from a shipped crate would put a test
harness in a consumer's dependency closure.

Two consequences follow, and both are constraints on *future* work rather than
on this crate:

- **A crate that wants to be exercised by a script must be reachable from these
  three edges**, or the edge list grows. That is the correct trigger for adding
  one — an actual step that cannot be written, not a topical relationship.
- **`ring_tls::flush_into` will not be reachable until the missing edge is
  closed**, so a script cannot exercise the staging crate's headline operation
  today (→ [`pitfall/003`](../pitfall/003_the_amortised_flush_has_no_ring.md)).

### Evidence

| # | Claim | How |
|---|---|---|
| G1 | All three declared edges appear in `src/lib.rs` | `use ring_core::..`, `use ring_shutdown::..`, `use ring_tls::..` |
| G2 | No fourth edge is declared, among the five entries `udeps` can see | `cargo +nightly udeps` clean — `tests/manual/readme.md` M4; `loom`'s `cfg(loom)` target section is outside its reach without the flag |
| G3 | The loom model runs and explores | `tests/manual/readme.md` M2 — a negative control fails |
| G4 | `flush_into` is unreachable | [`pitfall/003`](../pitfall/003_the_amortised_flush_has_no_ring.md) |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
printf 'ring_shutdown names imported:       %s\n' "$( command grep -m1 'use ring_shutdown' src/lib.rs )"
printf 'Guarded anywhere in src/:           %s\n' "$( command grep -cw 'Guarded' src/lib.rs || true )"
printf 'Stopped anywhere in src/:           %s\n' "$( command grep -nw 'Stopped' src/lib.rs | head -1 )"
printf 'guard/close/reopen/drain_all calls: %s\n' "$( command grep -cE '\.(guard|close|reopen|drain_all)\(' src/lib.rs || true )"
printf 'wait_for_close/for_space_or_close:  %s\n' "$( command grep -cE '\.(wait_for_close|for_space_or_close)\(' src/lib.rs || true )"
printf 'thread::spawn calls per file:       %s\n' "$( command grep -c 'thread::spawn(' src/lib.rs tests/testkit_test.rs tests/exhaustive_test.rs | sed 's|.*/||' | tr '\n' ' ' )"
printf 'the M4 command:                     %s\n' "$( command grep -m1 'udeps' tests/manual/readme.md )"
printf 'manifest dependency sections:       %s\n' "$( command grep -cE '^\[.*dependencies\]' Cargo.toml || true )"
printf 'declared dependency entries:        %s\n' "$( command grep -cE '^(ring_[a-z_]+|loom) = ' Cargo.toml || true )"
printf 'RUSTFLAGS on the M4 command:        %s\n' "$( command grep -m1 -B2 -A2 'udeps' tests/manual/readme.md | command grep -c 'RUSTFLAGS' || true )"
printf 'manual stages that do set the cfg:  %s\n' "$( command grep -c 'cfg loom' tests/manual/readme.md || true )"
```

Live output:

```
ring_shutdown names imported:       use ring_shutdown::{ Refusal, Shutdown };
Guarded anywhere in src/:           0
Stopped anywhere in src/:           134:  /// **This closes first.** `Stopped::reopen` consumes the token that proves
guard/close/reopen/drain_all calls: 5
wait_for_close/for_space_or_close:  0
thread::spawn calls per file:       lib.rs:2 testkit_test.rs:2 exhaustive_test.rs:6 
the M4 command:                     cargo +nightly udeps -p ring_testkit --all-targets --all-features
manifest dependency sections:       3
declared dependency entries:        6
RUSTFLAGS on the M4 command:        0
manual stages that do set the cfg:  3
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_the_script_surface.md](../api/001_the_script_surface.md) | `run`'s ring argument, which is what keeps `ring_config` a dev-dependency |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/003_the_amortised_flush_has_no_ring.md](../pitfall/003_the_amortised_flush_has_no_ring.md) | The `ring_tls` gap, measured |

### Sources

| File | Relationship |
|------|--------------|
| [`Cargo.toml`](../../Cargo.toml) | The manifest, with the deviation commented in place |

### TK17 — the reason the waiting surface is unused describes `run`, not the edge

The `ring_shutdown` row explains three unused operations —
`wait_for_close`, `for_space_or_close`, `Wake` — with *"those need a second
thread, and a script has one"*. The second half is true of `Script::run` and is
not true of the crate. `thread::spawn` is called ten times here: twice in
`leak_ends`' own doc example, twice in `tests/testkit_test.rs`, and six times in
`tests/exhaustive_test.rs`.

The second thread is not merely available — it is what half the crate exists
for. `leak` and `leak_ends` are this crate's contribution to the family's
loom-testing effort, per the module documentation, and their entire purpose is
to hand the two ends of one ring to two threads. So the edge does reach a two-thread setting, routinely, in
the half of the crate this row does not consider.

Whether the waiting surface *should* be reached from there is a real question and
this finding does not answer it — a `loom::model` explores interleavings by
scheduling, and a blocking wait is a different thing to model. The finding is
that the row gives a reason which stops being a reason as soon as the reader
turns to `leak_ends`, and offers no other.

The same row lists four type names as what the edge is used for. Two of them —
`Shutdown` and `Refusal` — are the `use` at `src/lib.rs:68`. `Stopped` appears
once, inside a doc comment. `Guarded` appears **zero** times: the crate calls
`guard()` and uses what comes back without ever naming its type.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
command grep -m1 -F "do routinely hand a ring's ends to two threads elsewhere in this crate" docs/integration/001_the_three_edges_and_the_one_that_is_missing.md
```

Live output:

```
| `ring_shutdown` | The closable guard: `Shutdown`, `Guarded`, `Stopped`, `Refusal` | `close`, `guard`, `reopen`, `drain_all`, and both `Refusal` arms. Not `wait_for_close`, `for_space_or_close`, or `Wake` — `Script::run` itself never blocks or waits, though `leak`/`leak_ends` do routinely hand a ring's ends to two threads elsewhere in this crate |
```

**Disposition:** applied — the row's reason now scopes the claim to
`Script::run` itself rather than the crate as a whole, and names `leak`/
`leak_ends` as the routine two-thread setting the original reason overlooked.
Now prints: `do routinely hand a ring's ends to two threads elsewhere in this crate`

### TK18 — the citation for "no fourth edge" runs where the fourth edge is invisible

Evidence row G2 reads *"No fourth edge is declared | `cargo +nightly udeps`
clean"*, citing M4. M4's command is
`cargo +nightly udeps -p ring_testkit --all-targets --all-features`, and there is
no `RUSTFLAGS` on it or within two lines either side of it.

`loom` is declared under `[target.'cfg(loom)'.dev-dependencies]`. With the cfg
unset that target section does not apply, so `loom` is not in the dependency
graph udeps walks. The manifest declares six entries across three sections; a
clean M4 speaks to five of them and is structurally unable to speak to the sixth,
in either direction — it can neither report `loom` unused nor confirm it used.

M4's own Prediction paragraph enumerates five crates by name — `ring_core`,
`ring_tls`, `ring_shutdown`, `ring_config`, `ring_types` — and does not mention
`loom`, so the stage's reasoning agrees with the tool's blind spot rather than
noticing it.

The claim itself is true: `[dependencies]` has exactly three entries and a reader
can see that by opening the file. What the citation adds is the suggestion that a
tool checked, and the tool checked everything except the entry whose correctness
is least obvious. Three of this crate's manual stages do set `--cfg loom`; the
dependency-hygiene stage is not one of them, and nothing prevents it from being
run twice.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit
command grep -m1 -F 'target section is outside its reach without the flag' docs/integration/001_the_three_edges_and_the_one_that_is_missing.md
```

Live output:

```
| G2 | No fourth edge is declared, among the five entries `udeps` can see | `cargo +nightly udeps` clean — `tests/manual/readme.md` M4; `loom`'s `cfg(loom)` target section is outside its reach without the flag |
```

**Disposition:** applied — Evidence row G2 now scopes the claim to the five
entries `udeps` can actually see and discloses that `loom`'s `cfg(loom)` target
section sits outside that reach without the flag, rather than citing the run as
covering all six declared entries.
Now prints: `target section is outside its reach without the flag`
