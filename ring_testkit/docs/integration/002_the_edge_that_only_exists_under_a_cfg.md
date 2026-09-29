# Integration: The Edge That Only Exists Under A Cfg

### Scope

- **Purpose**: Account for the sixth dependency entry — the one declared under a target section, invisible to an ordinary build, and unmentioned by the workspace's own record of who uses it.
- **Responsibility**: What the entry is, what setting the cfg does to this crate as opposed to the others that declare it, and the two places the family's accounting of the cfg does not include this crate.
- **In Scope**: `[target.'cfg(loom)'.dev-dependencies] loom = "0.7"`; the two whole-file gates in `tests/`; the workspace `check-cfg` entry.
- **Out of Scope**: Why the model is in `tests/` rather than `src/` (→ [`../decisions/002`](../decisions/002_the_model_lives_in_tests.md)); the allocations the model needs (→ [`../workaround/001`](../workaround/001_two_allocations_loom_cannot_avoid.md)); the other five edges (→ [`001`](001_the_three_edges_and_the_one_that_is_missing.md)).

### System Description

Five of this crate's six dependency entries are ordinary: three runtime, two
dev, all of them present in every build. The sixth is declared under
`[target.'cfg(loom)'.dev-dependencies]` and exists only when `--cfg loom` is set
in `RUSTFLAGS`.

That makes it a different kind of edge from the other five, in a way that is
easy to miss because the manifest line looks the same. An ordinary edge is a
crate this one calls. This one is a crate that **replaces the atomics
underneath** `ring_core`, three crates down, and changes which of this crate's
test files exists.

### Integration Points

| Property | The five ordinary edges | The `loom` edge |
|---|---|---|
| Present in a default build | yes | no |
| Named in a signature here | `ring_core` and `ring_tls` are; the rest are not | never |
| Reached by a call from `src/` | `ring_core`, `ring_tls`, `ring_shutdown` | never — `src/lib.rs` has zero `cfg` attributes |
| What using it looks like | a `use` and a call | an inner attribute at the top of a whole file |
| Visible to `cargo +nightly udeps` | yes | no (→ [`001`](001_the_three_edges_and_the_one_that_is_missing.md) TK18) |

**The edge is consumed at file granularity, not expression granularity.** This
crate has no `#[ cfg( loom ) ]` anywhere in `src/`. What `--cfg loom` does here
is swap which of two test files compiles: `tests/testkit_test.rs` opens
`#![ cfg( not( loom ) ) ]` and `tests/exhaustive_test.rs` opens `#![ cfg( loom ) ]`,
so the two are never built together and each is empty on the other's run.

### Three populations, and no two of them are the same set

"Uses the cfg" turns out to name three different things in this family, and a
crate can be in any one of them without being in the others:

| Population | What it means | Size |
|---|---|---|
| Gates a test file **off** | opens a test with `#![ cfg( not( loom ) ) ]` so a loom run skips it | 20 crates |
| Branches inside `src/` | has a `#[ cfg( loom ) ]` arm in its own library | 2 — `ring_atomic`, `ring_cursor` |
| Declares the dependency | has a `[target.'cfg(loom)'.dev-dependencies]` entry, so `loom::` is nameable | 5 — `ring_atomic`, `ring_mpsc`, `ring_publish`, `ring_spsc`, `ring_testkit` |
| Runs **under** the cfg | contains a file opening `#![ cfg( loom ) ]` | **1** — this crate |

The largest population is defensive: twenty crates say *not under loom* and need
no dependency to say it. The smallest is this crate.

`ring_cursor` branches in `src/` and declares nothing, and that is **correct**:
its `#[ cfg( loom ) ]` arms select between shapes `ring_atomic` has already
swapped, so it reads the cfg without ever naming loom. A manifest entry there
would be an unused dependency.

The gap that is not obviously correct is the other one. Four crates —
`ring_atomic`, `ring_mpsc`, `ring_publish`, `ring_spsc` — declare the
dependency and contain no file that runs under the cfg (→ TK19).

### Error Handling

Nothing crosses this edge as a value. `loom` appears in no signature, no return
type and no error type here. The only two names it contributes — `loom::model`
and `loom::thread::spawn` — are called only in `tests/exhaustive_test.rs`;
`src/lib.rs` writes `loom::` four times and every one of them is inside a doc
comment explaining why `leak` and `leak_ends` exist.

The failure mode the edge does have is a build failure rather than a runtime
error: touching a loom atomic outside a `loom::model` closure panics, which is
why the two test files are gated to be mutually exclusive rather than merely
feature-flagged.

### Compatibility Requirements

- **The cfg is set by `RUSTFLAGS`, never by a feature.** A consumer cannot turn
  it on through `--features`, and nothing in this crate's manifest offers it as
  one. That is a workspace-level decision recorded beside the `check-cfg` entry
  in the root `Cargo.toml`.
- **A build that sets the cfg builds the whole workspace with it.** `RUSTFLAGS`
  is not per-crate, so the loom run is a property of the invocation, and every
  crate that branches on the cfg branches together.
- **Adding a loom model to a sixth crate requires both columns above**, not one.
  A manifest entry with no branch is unused; a branch with no entry works only
  while the branch stays inside what `ring_atomic` already swapped.

### Evidence

| # | Claim | How |
|---|---|---|
| G1 | The entry is conditional | `Cargo.toml`, `[target.'cfg(loom)'.dev-dependencies]` |
| G2 | `src/` has no `cfg` attribute | Regenerate below, `cfg attributes in this src/` |
| G3 | The two test files are mutually exclusive gates | Regenerate below, `this crate gates whole files` |
| G4 | Five crates declare; one has a file that runs under the cfg | Regenerate below, the declarer list against the model list |
| G5 | The model runs and explores | `tests/manual/readme.md` M2 — a negative control fails |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'manifests declaring loom:      %s\n' "$( command grep -l '^loom = ' ring_*/Cargo.toml | sed 's|ring/||; s|/Cargo.toml||' | tr '\n' ' ' )"
printf 'crates branching in src/:      %s\n' "$( command grep -lE '#\[ cfg\( ?(not\( )?loom' ring_*/src/*.rs 2>/dev/null | sed 's|ring/||; s|/src/.*||' | sort -u | tr '\n' ' ' )"
printf 'files that run under the cfg:  %s\n' "$( command grep -l '^#!\[ cfg( loom ) \]' ring_*/tests/*.rs 2>/dev/null | sed 's|ring/||' | tr '\n' ' ' )"
printf 'crates gating a test file off: %s\n' "$( command grep -l '^#!\[ cfg( not( loom ) ) \]' ring_*/tests/*.rs 2>/dev/null | sed 's|ring/||; s|/tests/.*||' | sort -u | wc -l )"
printf 'cfg attributes in this src/:   %s\n' "$( command grep -cE '^ *#!?\[ cfg' ring_testkit/src/lib.rs || true )"
printf 'this crate gates whole files:  %s\n' "$( command grep -hm1 '^#!\[ cfg' ring_testkit/tests/*.rs | tr '\n' ' ' )"
printf 'the manifest entry:            %s\n' "$( command grep -A1 "cfg(loom)" ring_testkit/Cargo.toml | tr '\n' ' ' )"
printf 'the root manifest comment:     %s\n' "$( command grep -m1 'read the cfg' Cargo.toml )"
printf 'src files naming loom:: :      %s\n' "$( command grep -c 'loom::' ring_*/src/*.rs 2>/dev/null | command grep -v ':0$' | sed 's|ring/||' | tr '\n' ' ' )"
```

Live output:

```
manifests declaring loom:      ring_atomic ring_mpsc ring_publish ring_spsc ring_testkit 
crates branching in src/:      ring_atomic ring_cursor 
files that run under the cfg:  ring_testkit/tests/exhaustive_test.rs 
crates gating a test file off: 20
cfg attributes in this src/:   0
this crate gates whole files:  #![ cfg( loom ) ] #![ cfg( not( loom ) ) ] 
the manifest entry:            [target.'cfg(loom)'.dev-dependencies] loom = "0.7" 
the root manifest comment:     # its own. Only ring_atomic, ring_cursor and ring_publish read the cfg — see
src files naming loom:: :      ring_atomic/src/lib.rs:1 ring_testkit/src/lib.rs:5 
```

### Integrations

| File | Relationship |
|------|--------------|
| [001_the_three_edges_and_the_one_that_is_missing.md](001_the_three_edges_and_the_one_that_is_missing.md) | The five edges that are present in every build |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_the_model_lives_in_tests.md](../decisions/002_the_model_lives_in_tests.md) | Why the branching is file-level |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_two_allocations_loom_cannot_avoid.md](../workaround/001_two_allocations_loom_cannot_avoid.md) | What the model needs from this crate once the cfg is set |

### Sources

| File | Relationship |
|------|--------------|
| [`Cargo.toml`](../../Cargo.toml) | The conditional entry |
| [`tests/manual/readme.md`](../../tests/manual/readme.md) | M2 and M6, the stages that set the cfg |

### Tests

| File | Relationship |
|------|--------------|
| `tests/exhaustive_test.rs` | The only file that names `loom::` |
| `tests/testkit_test.rs` | The file the cfg switches off |

### TK19 — four of the five crates that declare the model checker have no model

`tests/exhaustive_test.rs` is the **only** file in all thirty-three `ring_*`
crates that opens `#![ cfg( loom ) ]`. Five manifests carry
`[target.'cfg(loom)'.dev-dependencies] loom = "0.7"`; four of them —
`ring_atomic`, `ring_mpsc`, `ring_publish`, `ring_spsc` — contain no file that
compiles under the cfg, and three of those four never write `loom::` anywhere.

`ring_atomic`'s entry earns itself for a different reason: its `src/` is the seam
that swaps the atomic types, and it names `loom::` once. The other three declare
a dependency they do not use, in a form
[`001`](001_the_three_edges_and_the_one_that_is_missing.md) TK18 shows
`cargo +nightly udeps` structurally cannot see.

The consequence for this document is that
[`001`](001_the_three_edges_and_the_one_that_is_missing.md) describes this
crate's entry as *"the same shape `ring_atomic`, `ring_publish`, `ring_spsc` and
`ring_mpsc` already use"*. That sentence is true of the manifest line and reads
as four precedents for a loom model. There is one loom model in the family and
this crate wrote it.

The reading that follows is the opposite of the one the sentence invites: this
crate is not adopting an established pattern, it is the first crate to actually
run under the cfg the family has been declaring for.

### TK20 — the workspace's only record of the cfg's readers matches no measured set

The root `Cargo.toml` justifies putting `check-cfg` in the shared lints table
rather than per crate, and closes with *"Only ring_atomic, ring_cursor and
ring_publish read the cfg — see ring_atomic's module documentation on the
seam."* It is the one place in the workspace that enumerates them, and it is
load-bearing: it is the reason a crate cannot add its own entry.

Three names, and no measured population has three members:

| Population | Size | Contains the named trio? |
|---|---|---|
| Branches inside `src/` | 2 — `ring_atomic`, `ring_cursor` | no — `ring_publish` does not |
| Declares the dependency | 5 | no — `ring_cursor` does not |
| Gates a test file off with `not( loom )` | 20 | no — it is twenty |
| Has a file that runs under the cfg | 1 — `ring_testkit` | no — none of the three |

Under the reading the surrounding sentence supports — *reads the cfg*, meaning
branches on it in library code — the answer is two crates, and `ring_publish` is
not one of them. Under any wider reading the answer is five or twenty.

`ring_testkit` is absent from the list under every reading, which is the part
that matters here: the crate that owns the family's only loom model does not
appear in the workspace's only record of who uses loom's cfg. A reader deciding
whether the cfg is still live would consult this comment and find three crates,
none of which is the one that runs under it.
