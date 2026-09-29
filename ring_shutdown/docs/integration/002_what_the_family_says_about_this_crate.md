# Integration: What the Family Says About This Crate

### Scope

- **Purpose**: Read the seam from the other side — every place another `ring_*` crate names this one — and check each claim against what this crate's code actually does.
- **Responsibility**: The one inbound dependency edge, the five crates that name this crate in source without one, and the three claims among them that the code contradicts.
- **In Scope**: `ring_testkit`'s edge; `ring_poll::PARKING_CRATES`; `ring_stats::RingStats::reset`; `ring_store::Buffer::reset`; `ring_bench` and `ring_handle`'s references.
- **Out of Scope**: This crate's own outbound edges and their justification (→ [`001`](001_family_dependency_seam.md)); what the surface promises (→ [`../api/001`](../api/001_shutdown_surface.md)).

### The Two Kinds of Coupling

[`001`](001_family_dependency_seam.md) reads the seam outward: four edges, each
justified by a named item. Read inward, the picture is different in kind.

| Coupling | Count | Visible to `cargo` |
|---|---|---|
| Manifests depending on `ring_shutdown` | 1 — `ring_testkit` | yes |
| Crates with a compiled `use ring_shutdown` | 1 — `ring_testkit` | yes |
| Crates naming it in `src/` with no edge | 5 — `ring_bench`, `ring_store`, `ring_handle`, `ring_poll`, `ring_stats` | no |
| Doc corpora naming it | 28 crates | no |

The first two rows are the same crate. Every other mention in the family is a
string: a doc comment, a table row, a `const [ &str; 3 ]`. That is not itself a
defect — a family whose crates explain themselves in terms of each other is doing
the right thing. It becomes one because the prose makes *claims about this
crate's behaviour*, and nothing compiles, links, or tests those claims. Three of
them are wrong.

### The Three Claims the Code Contradicts

**1. `ring_poll` lists this crate as one that parks.**

`ring_poll::PARKING_CRATES` is a public `const [ &str; 3 ]` naming
`ring_barrier`, `ring_shutdown` and `ring_wait`, asserted in `ring_poll`'s own
doctest. Its table row reads *"`ring_shutdown` | Waits for a close, or for space
before one | Outside the tick"*.

This crate says the opposite, twice. [`api/001`](../api/001_shutdown_surface.md)'s
fourth surface claim is *"Nothing on this surface allocates or parks"*, tied in
the same sentence to being reachable from inside a tick.
[`001`](001_family_dependency_seam.md) explains `ring_poll`'s absence from the
dependency list by saying *"the constraint is discharged by **not** parking"*.

**2. `ring_stats::RingStats::reset` says it is used by this crate's `reset`.**

Its doc comment: *"Used by `ring_shutdown`'s reset, so a recycled ring does not
carry the previous world's numbers."* `ring_stats` is not a dependency of this
crate — [`001`](001_family_dependency_seam.md) lists it as *pointedly absent*,
with an argument for why the edge would be wrong.

**3. `ring_store::Buffer::reset` says it exists for this crate's reset.**

Its doc comment: *"The reset `ring_shutdown` needs: a recycled ring must not
hand a consumer the previous world's payloads."* `ring_store` is not a direct
dependency, and `ring_shutdown::reset` is five lines that close, discard through
the consumer, and reopen — it clears no slots.

Two of the three are load-bearing for the *other* crate: a method whose doc
justifies its existence by naming a consumer that never arrived.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'manifests depending on this:   %s\n' "$( command grep -rl 'ring_shutdown' ring_*/Cargo.toml | cut -d/ -f1 | command grep -v '^ring_shutdown$' | tr '\n' ' ' )"
printf 'crates naming it in src:       %s\n' "$( command grep -rl 'ring_shutdown' ring_*/src --include='*.rs' | cut -d/ -f1 | command grep -v '^ring_shutdown$' | sort -u | tr '\n' ' ' )"
printf 'of those, a compiled use stmt: %s\n' "$( command grep -rh '^ *use ring_shutdown' ring_*/src --include='*.rs' | sort -u | tr '\n' ' ' )"
printf 'crates with an edge, not prose: %s\n' "$( command grep -rl '^ *use ring_shutdown' ring_*/src --include='*.rs' | cut -d/ -f1 | command grep -v '^ring_shutdown$' | sort -u | tr '\n' ' ' )"
printf 'doc corpora naming it:         %s\n' "$( command grep -rl 'ring_shutdown' ring_*/docs --include='*.md' | cut -d/ -f1 | command grep -v '^ring_shutdown$' | sort -u | wc -l )"
printf 'what ring_poll declares:       %s\n' "$( command grep -o 'pub const PARKING_CRATES.*' ring_poll/src/lib.rs )"
printf 'its row for this crate:        %s\n' "$( command grep -o '. .ring_shutdown. | [^|]*| [A-Za-z ]*' ring_poll/src/lib.rs )"
printf 'what this surface claims:      %s\n' "$( command grep -ohE 'Nothing on this surface [a-z ]+\.' ring_shutdown/docs/api/001_shutdown_surface.md )"
printf 'why 001 needs no poll edge:    %s\n' "$( command grep -o 'the constraint is discharged by .not. parking' ring_shutdown/docs/integration/001_family_dependency_seam.md )"
printf 'what ring_stats claims:        %s\n' "$( command grep -o 'Used by .ring_shutdown..s reset' ring_stats/src/lib.rs )"
printf 'what ring_store claims:       %s\n' "$( command grep -o 'The reset .ring_shutdown. needs' ring_store/src/lib.rs )"
printf 'what reset actually calls:     %s\n' "$( awk '/^pub fn reset</{f=1} f&&/^\}$/{exit} f&&/stopped\.|shutdown\./{ sub( /^ */, "" ); printf "%s ", $0 }' ring_shutdown/src/lib.rs )"
printf 'stats or buffer resets called: %s\n' "$( command grep -cE 'RingStats|Buffer::|\.reset\(\)' ring_shutdown/src/lib.rs || true )"
printf 'tests joining any two crates:  %s\n' "$( command grep -rlE 'ring_poll|ring_stats|ring_store' ring_shutdown/tests --include='*.rs' | wc -l )"
```

Live output:

```
manifests depending on this:   ring_testkit 
crates naming it in src:       ring_bench ring_store ring_handle ring_poll ring_stats ring_testkit 
of those, a compiled use stmt: use ring_shutdown::{ Refusal, Shutdown }; 
crates with an edge, not prose: ring_testkit 
doc corpora naming it:         28
what ring_poll declares:       pub const PARKING_CRATES : [ &str; 3 ] = [ "ring_barrier", "ring_shutdown", "ring_wait" ];
its row for this crate:        | `ring_shutdown` | Waits for a close, or for space before one | Outside the tick 
what this surface claims:      Nothing on this surface allocates or parks.
why 001 needs no poll edge:    the constraint is discharged by *not* parking
what ring_stats claims:        
what ring_store claims:       
what reset actually calls:     let stopped = shutdown.close(); let discarded = stopped.discard_all( consumer ); stopped.reopen(); 
stats or buffer resets called: 0
tests joining any two crates:  0
```

### Integration

| File | Relationship |
|------|--------------|
| [`001_family_dependency_seam.md`](001_family_dependency_seam.md) | The same seam read outward, with the argument for each absence these claims contradict |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_shutdown_surface.md`](../api/001_shutdown_surface.md) | The no-parking claim `ring_poll` contradicts |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [`../non_functional_requirement/002_the_teardown_path_takes_the_slow_one.md`](../non_functional_requirement/002_the_teardown_path_takes_the_slow_one.md) | What `reset` actually does, against what two crates say it does |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/001_exactly_one_liveness_flag.md`](../invariant/001_exactly_one_liveness_flag.md) | The one-flag argument, which is the reason `ring_core` has nothing here to depend on |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `reset`, whose five lines are the counter-evidence to two of the three claims |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | No test in this crate names `ring_poll`, `ring_stats` or `ring_store`; there is nowhere for the contradiction to surface |

### SD19 — Three Crates Document a Relationship With This One That the Code Does Not Have

`ring_poll` declares a public `const PARKING_CRATES : [ &str; 3 ]` containing
`"ring_shutdown"`, and asserts its contents in a doctest — so the claim is
compiled and tested, as a claim about a *string*. This crate states the negation
twice: the surface *"allocates or parks"* nothing, and `ring_poll` needs no edge
here precisely because *"the constraint is discharged by not parking"*. Both
crates are in-house, both statements are deliberate, and one of them is wrong.

`ring_stats::RingStats::reset` is documented as *"Used by `ring_shutdown`'s
reset"*. `ring_store::Buffer::reset` is documented as *"The reset
`ring_shutdown` needs"*. `ring_shutdown::reset` is five lines:

```rust
let stopped = shutdown.close();
let discarded = stopped.discard_all( consumer );
stopped.reopen();
discarded
```

It calls neither. `ring_stats` is not a dependency at all — this crate's own
seam document lists it as *pointedly absent* and argues the edge would be wrong.
`ring_store` is transitive through `ring_core` and is never named in `src/`.

The shape is worth separating from the individual errors. Two of these are doc
comments that justify a method's *existence* by naming a consumer: delete the
sentence and `RingStats::reset` and `Buffer::reset` are two functions with no
stated caller. They may well be right that a recycled ring should clear its
counters and its slots — that is what a reset is for — but what they
assert is that this crate does it, and this crate does not.

Nothing can catch any of the three. The claims live in doc comments and a string
array; `cargo` sees no edge between the crates; no test in this crate names any
of them; and the corpus checkers are crate-scoped — `citations.py` resolves links
within one `docs/` tree and never leaves it.

**Disposition:** declined — the false text lives in three other crates'
own sources, not this crate's: `ring_poll`'s `PARKING_CRATES` constant
(`ring_poll/src/lib.rs`), `ring_stats::RingStats::reset`'s doc comment
(`ring_stats/src/lib.rs`), and `ring_store::Buffer::reset`'s doc
comment (`ring_store/src/lib.rs`). This disposition pass is scoped to
`ring_shutdown/docs/`; fixing SD19 means editing those three crates'
own doc corpora, which is out of scope here and belongs to their own
disposition passes instead.

### SD20 — The Only Real Inbound Edge Went Unrecorded While Five Prose Ones Accumulated

Until this pass, [`001`](001_family_dependency_seam.md) said *"nothing in the
family depends on it yet"* and predicted the first consumer would be
`ring_factory`. `ring_testkit` has already depended on it — a manifest
entry and `use ring_shutdown::{ Refusal, Shutdown };` — and `ring_factory` still
does not.

So the accounting is wrong in both directions at once. This crate under-reported
its consumers, claiming zero where there was one. Three other crates
over-reported theirs, claiming one where there was none (→ SD19). Every one of
those four statements is prose about the dependency graph, and the graph is
sitting in five `Cargo.toml` files that no document reads.

The asymmetry with the outbound direction is the point.
[`001`](001_family_dependency_seam.md) justifies each outgoing edge with a named
item and a use site, and pairs the closure claim with a `cargo tree` command — it
is checkable, and it stayed correct. The one sentence in it that was not derived
from a command is the one that went stale, and it went stale in the same release
that added the edge it denies.

The generalizable form: **a dependency claim should be spelled as a command over
`Cargo.toml`, not as a sentence.** Nothing here needs a new checker — this
instance's own recipe reads the manifests in one line, and the same line
belongs in whichever crates make claims about their consumers.
