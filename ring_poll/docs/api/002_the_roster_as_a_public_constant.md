# API: The Roster As A Public Constant

### Scope

- **Purpose**: Examine `PARKING_CRATES` as an exported item rather than as a test fixture — what its type lets a consumer do, what its doc comment claims, and how far the guard behind it actually reaches.
- **Responsibility**: The constant's shape, the promise attached to it, the test credited with keeping it honest, and the distance between the claim and the check.
- **In Scope**: The `[ &str; 3 ]` type, the doc comment's reachability claim, and `the_tick_path_cannot_reach_a_parking_operation`.
- **Out of Scope**: The dependency graph itself and which edges create the gap (→ [`../integration/002`](../integration/002_what_actually_reaches_ring_wait.md)); the invariant the roster documents (→ [`../invariant/001`](../invariant/001_no_parking_operation_on_the_tick_path.md)).

### Abstract

`PARKING_CRATES` is the only public item in this crate that is a *claim about
other crates* rather than an operation. Three names are listed. Five crates reach
a parking operation. The array is unchanged and correct — what changed is the
sentence above it, which used to call the three *reachable* and now calls them
what they are.

### The item

```
pub const PARKING_CRATES : [ &str; 3 ] = [ "ring_barrier", "ring_shutdown", "ring_wait" ];
```

The array length is in the type, so adding a name is a breaking change for any
consumer that wrote `[ &str; 3 ]` in a signature — which is the intent. The doc
comment states it outright: the roster is *"part of the surface rather than a
constant inside a test, so that adding a crate to it is a visible API change
rather than a quiet edit to an assertion."*

That reasoning is sound and it is the reason this document exists: an item
promoted to the public surface for the sake of visibility has to be measured
against what it makes visible.

### What the doc comment promises

Three claims, in order. The first and third are the ones the findings below are
about, and both have since been narrowed to what the array actually supports:

| # | Claim, as it stood | Status | Claim, as it stands |
|---|---|---|---|
| 1 | *"The family crates from which a parking operation is reachable"* | was **false by two** → PL7 | *"that declare `ring_wait` as a direct dependency — not every crate a parking operation is transitively reachable from"* |
| 2 | *"Every name here is a crate that depends on `ring_wait`, directly or by re-export"* | true of the three listed | unchanged |
| 3 | *"a crate that gains a `ring_wait` dependency without being listed here fails the suite"* | was true only of a *direct* dependency → PL8 | *"gains a **direct** `ring_wait` dependency"*, plus a paragraph naming the blindness and the test that measures it |

Claim 2 is the one that always described the array correctly, and it is a claim
about membership — everything listed belongs. Claim 1 was the converse, that
everything belonging is listed, and the two are not the same statement. The
roster is sound and incomplete; the doc comment now says so in both places rather
than in neither.

### What a consumer can do with it

| Use | Works? |
|---|---|
| `contains( &"ring_wait" )` in an assertion | yes — this is what the doctest does |
| Compare against a crate's own dependency list | yes, if that list is the *direct* one |
| Decide whether calling into crate X can park | **no** — a name's absence does not establish that |

The third row is a use the array cannot support, and the doc comment's first
sentence used to invite it. A consumer checking
`!PARKING_CRATES.contains( &"ring_consume" )` gets `true`, and `ring_consume`
reaches `ring_wait` through `ring_barrier` — which is now stated in a table
directly beneath the roster rather than left to be discovered.

### Evidence

| # | Claim | Test |
|---|---|---|
| R1 | The roster equals the set of direct manifest declarers | `the_tick_path_cannot_reach_a_parking_operation` |
| R2 | This crate declares no parking dependency | `this_crate_declares_no_parking_dependency` |
| R3 | The array contains `ring_wait` and not `ring_handle` | the doctest on the constant |
| R4 | Reachability is wider than the scan, by exactly `ring_consume` and `ring_testkit` | `the_transitive_reach_is_wider_than_the_manifest_scan_can_see` |

R1 is the guard on the array, and its scan is a substring test over manifest
text. R4 is the guard on the gap that scan leaves: it parses the manifests into a
graph, closes it, and asserts the two sets differ by those two names — so a third
crate slipping into the gap is a test failure rather than a discovery.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'the roster, as declared:      %s\n' "$( command grep -oE '\[ "ring[^]]*\]' ring_poll/src/lib.rs | tr -d '"[]' )"
printf 'its arity, in the type:       %s\n' "$( command grep -oE '\[ &str; [0-9]+ \]' ring_poll/src/lib.rs )"
printf 'crates naming ring_wait:      %s\n' "$( cd ring && for c in ring_*/Cargo.toml; do command grep -q 'ring_wait' "$c" && echo "${c%/Cargo.toml}"; done | tr '\n' ' ' )"
printf 'crates that reach ring_wait:  %s\n' "$( for c in ring_*/; do n=$( basename "$c" ); cargo tree --manifest-path ring/Cargo.toml -e normal -p "$n" 2>/dev/null | command grep -q 'ring_wait' && echo "$n"; done | tr '\n' ' ' )"
printf 'listed but unreachable:       %s\n' "$( for n in $( command grep -oE '\[ "ring[^]]*\]' ring_poll/src/lib.rs | tr -d '"[],' ); do cargo tree --manifest-path ring/Cargo.toml -e normal -p "$n" 2>/dev/null | command grep -q 'ring_wait' || echo "$n"; done | tr '\n' ' ' | command grep . || echo none )"
printf 'reachable but unlisted:       %s\n' "$( for c in ring_*/; do n=$( basename "$c" ); cargo tree --manifest-path ring/Cargo.toml -e normal -p "$n" 2>/dev/null | command grep -q 'ring_wait' && command grep -q "\"$n\"" ring_poll/src/lib.rs || { cargo tree --manifest-path ring/Cargo.toml -e normal -p "$n" 2>/dev/null | command grep -q 'ring_wait' && echo "$n"; }; done | tr '\n' ' ' )"
printf 'the doc word for the set:     %s\n' "$( command grep -m1 -oE 'that declare .ring_wait. as a direct dependency' ring_poll/src/lib.rs | tr -d '\140' )"
printf 'and what it disclaims:        %s\n' "$( command grep -m1 -oE 'every crate a parking operation is transitively reachable from' ring_poll/src/lib.rs )"
printf 'the scan the test performs:   %s\n' "$( command grep -oE 'contains\( "ring_wait" \)' ring_poll/tests/poll_test.rs | head -1 )"
printf 'assertions in the doctest:    %s\n' "$( awk '/^\/\/\/ ```$/{ n++ } n==1 && /^\/\/\/ assert/' ring_poll/src/lib.rs | wc -l )"
printf 'gap crates named in the doc:  %s\n' "$( command grep -oE '^/// \| .ring_(consume|testkit). \|' ring_poll/src/lib.rs | command grep -oE 'ring_(consume|testkit)' | sort | tr '\n' ' ' )"
printf 'and what the doc calls the scan: %s\n' "$( command grep -m1 -oE 'to the two rows above by construction' ring_poll/src/lib.rs )"
printf 'the test that pins the gap:   %s\n' "$( command grep -m1 -oE 'the_transitive_reach_is_wider_than_the_manifest_scan_can_see' ring_poll/tests/poll_test.rs )"
```

Live output:

```
the roster, as declared:       ring_barrier, ring_shutdown, ring_wait 
its arity, in the type:       [ &str; 3 ]
crates naming ring_wait:      ring_barrier ring_shutdown ring_wait 
crates that reach ring_wait:  ring_barrier ring_consume ring_shutdown ring_testkit ring_wait 
listed but unreachable:       none
reachable but unlisted:       ring_consume ring_testkit 
the doc word for the set:     that declare ring_wait as a direct dependency
and what it disclaims:        every crate a parking operation is transitively reachable from
the scan the test performs:   contains( "ring_wait" )
assertions in the doctest:    2
gap crates named in the doc:  ring_consume ring_testkit 
and what the doc calls the scan: to the two rows above by construction
the test that pins the gap:   the_transitive_reach_is_wider_than_the_manifest_scan_can_see
```

### APIs

| File | Relationship |
|------|--------------|
| [001_tick_path_surface.md](001_tick_path_surface.md) | The other seven exported items, all of them operations |

### Integrations

| File | Relationship |
|------|--------------|
| [`../integration/002_what_actually_reaches_ring_wait.md`](../integration/002_what_actually_reaches_ring_wait.md) | The two edges that create the gap, and why the scan cannot see them |

### Invariants

| File | Relationship |
|------|--------------|
| [`../invariant/001_no_parking_operation_on_the_tick_path.md`](../invariant/001_no_parking_operation_on_the_tick_path.md) | The property this roster documents, which is about *this* crate and does hold |

### Patterns

| File | Relationship |
|------|--------------|
| [`../pattern/001_enforcement_by_dependency_graph.md`](../pattern/001_enforcement_by_dependency_graph.md) | Enforcement by absence, and the reach it has |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The constant, its doc comment, and its doctest |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | R1 and R2 |

### PL7 — the roster's own word for what it lists is broader than what it lists

The doc comment's first line names the set: *"The family crates from which a
parking operation is reachable."* Reachable is a transitive word. It is also the
right word for what a reader wants to know — given a crate, can calling into it
end up sleeping.

The array holds the crates that *declare* `ring_wait` in their own manifest.
Those are three. The crates from which `ring_wait` is reachable through normal
dependencies are five: the same three plus `ring_consume`, which depends on
`ring_barrier`, and `ring_testkit`, which depends on `ring_shutdown`.

Both additions are one edge deep. Neither is exotic, neither is a dev-dependency
edge that could be argued away, and both crates are ordinary members of the same
family the roster is about. A reader who takes the sentence at face value and
concludes that `ring_consume` cannot park has been misled by the document that
exists specifically to answer that question.

The narrower sentence two paragraphs later — *"Every name here is a crate that
depends on `ring_wait`, directly or by re-export"* — describes the array
accurately. The two sentences make different claims and only one of them is
true, and the false one is the summary line a reader sees first.

What is *not* affected is this crate's own invariant. `ring_poll` depends on
`ring_core` and nothing else, so nothing on its tick path can park regardless of
how the roster is worded → [`../invariant/001`](../invariant/001_no_parking_operation_on_the_tick_path.md).
The defect is in a claim about the family, not in the guarantee about this crate.

```sh
cd "$(git rev-parse --show-toplevel)"
command grep -F "every crate a parking operation is transitively reachable from" ring_poll/src/lib.rs
```

Live output:

```
/// every crate a parking operation is transitively reachable from; see
```

**Disposition:** applied — `PARKING_CRATES`'s doc comment in `src/lib.rs` no
longer opens with the transitive word `reachable`; its first line now names
the array's actual membership rule (direct dependency) and its second line
states outright that this is narrower than transitive reachability, pointing
at this document for the two crates that fall in the gap. Now prints: `every crate a parking operation is transitively reachable from`

### PL8 — the guard credited with keeping the roster honest cannot see the case that broke it

The doc comment names its own enforcement: *"`tests/poll_test.rs::the_tick_path_cannot_reach_a_parking_operation`
reads the manifests off disk and compares them against this array, so a crate
that gains a `ring_wait` dependency without being listed here fails the suite."*

The test does read every `ring_*/Cargo.toml` and collect the crates whose text
contains `ring_wait`, and it does assert that set equals `PARKING_CRATES`. That
is a real check and it passes.

A manifest scan can only ever find direct declarations. `ring_consume`'s manifest
does not contain the string `ring_wait` — it contains `ring_barrier`. So the
scan found three, the array held three, the assertion held, and the property the
doc comment claimed to be protecting was untested. The test verified that the
array matches the scan; the sentence said the array matches reachability; the
scan and reachability differ by two crates, and the difference was invisible from
inside the test.

This is the specific hypothetical [`../integration/001`](../integration/001_family_dependency_seam.md)
already raises — *"A direct-manifest scan would pass even if `ring_core` grew a
`ring_wait` dependency"* — with the tense corrected. It is not a future risk. Two
crates already sit in the gap, and the corpus recorded the shape of the hole
before anything fell into it.

Two fixes were available and they are not equivalent. Widening the scan to a
transitive closure and growing the array to five changes a public array's length,
which is a decision about the surface rather than a documentation edit — and it
would also make the roster answer a question it was not built to answer, since
neither gap crate is on the tick path. Narrowing the sentence keeps the array
honest at the cost of a reader having to look one document further for
reachability. The narrowing is what was applied, with a third piece added that
neither option originally had: a test that measures the gap itself, so the two
sets are compared rather than assumed equal.

**Disposition:** applied — the sentence in `src/lib.rs` now says a crate fails the
suite when it gains a **direct** `ring_wait` dependency, and a following paragraph
states that the scan is a string search over `Cargo.toml` files and is therefore
blind to the two rows above it. The blindness is no longer an unmeasured
quantity: `the_transitive_reach_is_wider_than_the_manifest_scan_can_see` in
`tests/poll_test.rs` parses every `ring_*/Cargo.toml` into a dependency graph,
closes it to a fixed point, and asserts the transitive set is exactly the five
crates while the string scan finds three — so a third crate entering the gap now
fails the suite that previously could not see it. The array itself stays at
three. Now prints: `the_transitive_reach_is_wider_than_the_manifest_scan_can_see`
