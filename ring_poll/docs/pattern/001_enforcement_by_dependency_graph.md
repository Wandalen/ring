# Pattern: Enforcement by Dependency Graph

### Scope

- **Purpose**: Extract the shape used when a rule says "X must not be reachable from Y" and the language offers no way to say it: assert it against the dependency graph and put the roster in the public surface.
- **Responsibility**: The three parts, the conditions under which the pattern earns its cost, the part people skip, and where else in the family it fits.
- **In Scope**: Reachability rules spanning crate boundaries, and the difference between enforcing them and merely declaring them.
- **Out of Scope**: The specific parking claim this crate makes (→ [`../invariant/001`](../invariant/001_no_parking_operation_on_the_tick_path.md)).

### Problem

A rule of the form *"X must not be reachable from Y"* has no expression in Rust.
Privacy governs items within a crate; nothing governs which crates a manifest
may name. So the rule exists only in someone's head, and the first violation
compiles cleanly and passes every test it can see.

The compounding problem is *where* the violation would be caught. A crate that
newly depends on a dangerous operation does not call it yet — that is what makes
the dependency look harmless — so the crate's own suite stays green. The check
has to fail somewhere other than the crate that changed.

### Solution

**1. Do not take the dependency.** The crate that must not reach the dangerous
operation does not depend on the crate that offers it — even when it would use
only the harmless parts. Depending on `ring_wait` to call `pause( Spin, .. )`
would put `pause( Park, .. )` one autocomplete away, and autocomplete is the
attack this defends against.

**2. Declare the exception roster as a public constant.** The crates that *are*
allowed to reach it are named in the surface, not in a test:

```rust
pub const PARKING_CRATES : [ &str; 3 ] = [ "ring_barrier", "ring_shutdown", "ring_wait" ];
```

Public, because editing it should be an API change. A roster inside a test file
is a line someone changes to make a red build green; a roster in the surface is
a line someone has to justify.

**3. Assert the roster against the graph.** A test reads the manifests off disk
and compares. The declaration and the reality cannot drift, because the drift is
what fails.

### Applicability

| Condition | Why it matters |
|---|---|
| The rule is about *reachability*, not behaviour | If you could call the bad thing and check what it does, write an ordinary test |
| The dangerous operation lives in a separate crate | Within one crate, privacy already does this job better |
| The rule constrains crates other than the one enforcing it | Otherwise put the check in the constrained crate, where it is closer to what it governs |
| A violation would compile and pass its own crate's tests | This is the whole reason the pattern earns its cost. `ring_handle` gaining a `ring_wait` dependency breaks nothing in `ring_handle` |

The last row is the one to check first. If a violation would already fail
something, this is ceremony.

#### Where else this fits in the family

| Situation | Applies? |
|---|---|
| No parking on the tick path | Yes — this crate |
| `unsafe` confined to three declared crates | Yes, and it is already done — `gate/g6_unsafe.sh`, at the gate level rather than in a crate |
| External dependencies confined to a declared set | Yes, and likewise — `gate/g5_export_surface.sh` |
| One liveness flag in the family | **No** — that is a grep for a *pattern in source*, not an edge in the graph. Recorded in `ring_shutdown/docs/invariant/001` as a manual check for exactly that reason |

Two of the four are already gates rather than tests, which suggests the honest
generalization: this pattern is what you use when the rule is narrower than a
gate deserves and wider than one crate's suite can see.

### Consequences

**Part 3 is the load-bearing part, and it is the one people skip.** A roster
declared without an assertion is a comment with a `pub` on it — it goes stale
the first time the graph changes, and it goes stale *silently*, which is worse
than not having it, because the next reader trusts it.

The generalizable check: *if the roster were wrong, what would fail?* If the
answer is "nothing", the pattern has not been applied; only its first two parts
have, and those are the decorative ones.

A second, subtler consequence: **the assertion is a proxy.** Reading a manifest
for a substring tests a *name*, while the claim is about *reachability* — a crate
that reaches `ring_wait` transitively, through a dependency that has it, passes
a direct-manifest scan. Such paths exist. `ring_consume` reaches it through
`ring_barrier` and `ring_testkit` through `ring_shutdown`, both one edge deep in
`[dependencies]`; `ring_publish` reaches it through `[dev-dependencies]`, which is
a weaker edge and still an edge `cargo tree` reports. The roster names three
crates and six reach it → PL37.

`tests/manual/readme.md` P2 runs the transitive form, and what it runs is
`cargo tree -p ring_poll | grep -c ring_wait` — **this crate's own closure**,
which is genuinely `0`. That result is correct and is the guarantee this crate
actually makes. It is not a family-wide result, and reading it as one is the
mistake this paragraph used to make.

The pattern also buys enforcement by *noise* rather than by *impossibility*. A
violating manifest still compiles; what changes is that making the suite green
again requires editing a public constant, which is an API change someone has to
justify rather than a quiet one.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'part 1, ring_wait as a dep:  %s\n' "$( awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_poll/Cargo.toml | command grep -c 'ring_wait' || true )"
printf 'part 2, the public roster:   %s\n' "$( command grep -oE 'pub const PARKING_CRATES : \[ &str; [0-9]+ \]' ring_poll/src/lib.rs )"
printf 'part 3, the asserting test:  %s\n' "$( command grep -oE 'fn the_tick_path_cannot_reach_a_parking_operation' ring_poll/tests/poll_test.rs )"
printf 'the roster names:            %s\n' "$( command grep '^pub const PARKING_CRATES' ring_poll/src/lib.rs | command grep -oE '"ring_[a-z_]+"' | tr -d '"' | tr '\n' ' ' )"
printf 'proxy — manifests naming it: %s\n' "$( command grep -l 'ring_wait' ring_*/Cargo.toml 2>/dev/null | wc -l )"
printf 'target — reaching, lib only: %s\n' "$( for c in $( ls -d ring_*/ | sed 's|ring/||;s|/||' ); do cargo tree --manifest-path Cargo.toml -p "$c" -e normal 2>/dev/null | command grep -q 'ring_wait' && printf '%s ' "$c"; done )"
printf 'target — with dev edges too: %s\n' "$( for c in $( ls -d ring_*/ | sed 's|ring/||;s|/||' ); do cargo tree --manifest-path Cargo.toml -p "$c" 2>/dev/null | command grep -q 'ring_wait' && printf '%s ' "$c"; done )"
printf 'ring_poll own closure:       %s\n' "$( cargo tree --manifest-path Cargo.toml -p ring_poll 2>/dev/null | command grep -c 'ring_wait' || true )"
printf 'what P2 measured:            %s\n' "$( command grep -oE 'cargo tree -p ring_poll \| grep -c ring_wait' ring_poll/tests/manual/readme.md )"
printf 'cargo_metadata in family:    %s\n' "$( command grep -l 'cargo_metadata' ring_*/Cargo.toml 2>/dev/null | wc -l )"
printf 'gate scripts over the tree:  %s\n' "$( ls bench_harness/gate/g*.sh | wc -l )"
```

Live output:

```
part 1, ring_wait as a dep:  0
part 2, the public roster:   pub const PARKING_CRATES : [ &str; 3 ]
part 3, the asserting test:  fn the_tick_path_cannot_reach_a_parking_operation
the roster names:            ring_barrier ring_shutdown ring_wait 
proxy — manifests naming it: 3
target — reaching, lib only: ring_barrier ring_consume ring_shutdown ring_testkit ring_wait 
target — with dev edges too: ring_barrier ring_consume ring_publish ring_shutdown ring_testkit ring_wait 
ring_poll own closure:       0
what P2 measured:            cargo tree -p ring_poll | grep -c ring_wait
cargo_metadata in family:    0
gate scripts over the tree:  22
```

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_tick_path_surface.md`](../api/001_tick_path_surface.md) | Lists `PARKING_CRATES` as an exported item — part 2 of the pattern, visible on the surface |

### Integrations

| File | Relationship |
|------|--------------|
| [`../integration/001_family_dependency_seam.md`](../integration/001_family_dependency_seam.md) | The graph this pattern asserts against |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/001_no_parking_operation_on_the_tick_path.md`](../invariant/001_no_parking_operation_on_the_tick_path.md) | The instance this pattern was extracted from |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | `PARKING_CRATES` — parts 1 and 2, and the absence of a `ring_wait` dependency that is part 1 |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `the_tick_path_cannot_reach_a_parking_operation` — part 3, the load-bearing one |
| [`tests/manual/readme.md`](../../tests/manual/readme.md) | P2 runs the transitive form the automated check only proxies |

### PL37 — the pattern's own load-bearing question, asked of its own instance, answers "nothing"

The Consequences section states the check that decides whether this pattern has
actually been applied: *"if the roster were wrong, what would fail?"* If the
answer is "nothing", only the decorative first two parts are in place.

Asked of the *naming* claim, the answer is good. `PARKING_CRATES` lists three
crates, three manifests contain the string `ring_wait`, and
`the_tick_path_cannot_reach_a_parking_operation` asserts those two sets equal. A
fourth manifest gaining the string turns the suite red. Part 3 is real.

Asked of the *reachability* claim — the one the roster's own doc comment makes —
the answer is nothing. Five crates reach `ring_wait` through `[dependencies]`:
the three listed, plus `ring_consume` through `ring_barrier` and `ring_testkit`
through `ring_shutdown`. A sixth, `ring_publish`, reaches it through
`[dev-dependencies]`. The roster is wrong by two names on the strict reading and
three on the permissive one, and nothing fails, because the scan and the roster
are wrong in the same direction.

This document previously said *"Today the family has no such path"* and cited P2
as having established it. P2 establishes something narrower and correct: that
`cargo tree -p ring_poll` finds zero, which is this crate's own guarantee and
does hold. Generalizing one crate's closure to the family was the error, and it
is the same error the pattern warns about one paragraph earlier — trusting a
declaration whose failure mode is silence.

The instance is still worth keeping and the guarantee it protects is still
intact. What the finding records is that the pattern's parts hold in a different
proportion than the document claimed: parts 1 and 2 hold for reachability, part 3
holds only for naming, and the gap between them has three crates in it.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -m1 -F "nothing fails, because the scan and the roster" ring_poll/docs/pattern/001_enforcement_by_dependency_graph.md
```

Live output:

```
three on the permissive one, and nothing fails, because the scan and the roster
```

**Disposition:** applied — the Consequences section above no longer
generalises `tests/manual/readme.md` P2's zero-reachability result to the
family; it now states the reachability count directly (five on the strict
reading, six with dev edges) and that the scan and the roster are wrong in
the same direction, which is why nothing fails. Now prints: `nothing fails, because the scan and the roster`

### PL38 — the placement that makes the pattern affordable is what makes it a proxy

The *Where else this fits* table places four family rules and marks two as
already enforced — `unsafe` confinement by `gate/g6_unsafe.sh`, external
dependencies by `gate/g5_export_surface.sh` — both at the gate level, both over
the whole tree. This crate's rule is the one enforced from inside a crate's own
suite, and the document reads the difference as scope: *"narrower than a gate
deserves and wider than one crate's suite can see."*

The difference is also capability, and that is the part not stated. A gate script
is a shell script over the tree; it can run `cargo tree` and get reachability
directly. A `#[ test ]` inside `ring_poll` gets reachability only by linking
something that reads cargo metadata, and no crate in the family depends on
`cargo_metadata` — so the test does the one thing a test cheaply can, which is
read manifest text. The proxy is not a shortcut somebody took; it is what the
placement leaves available.

That reframes the choice. Twenty-one gate scripts already run over this tree, two
of them enforcing rules of exactly this shape, and moving the check there would
make the target measurable rather than proxied. The cost is that it leaves the
crate's own suite, so the rule stops being visible next to the constant it
governs — which is the real tradeoff, and a different one from the tradeoff the
document currently describes.

Recorded rather than moved, because relocating an enforcement point is a decision
about where the family's rules live, and this crate is not the right place to
make it unilaterally → [`../workaround/001`](../workaround/001_reimplementing_the_pause_hint.md).
