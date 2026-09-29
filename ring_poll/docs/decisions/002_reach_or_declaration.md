# Decision: Should The Roster Name What Declares `ring_wait` Or What Reaches It

**Status:** open. Unowned — the evidence needed to settle it already exists, so
this is a choice waiting to be made rather than a question waiting on work.

### Scope

- **Purpose**: Record the choice forced by `PARKING_CRATES` documenting one set and containing another — either narrow the sentence to match the array, or widen the array to match the sentence.
- **Responsibility**: The two options, what each costs, what each would break, and why the cheaper one is not obviously the right one.
- **In Scope**: The roster's membership rule and the scan that enforces it.
- **Out of Scope**: Whether the roster should be generated at all (→ [`001`](001_should_the_roster_be_generated.md)); the measurement that found the gap (→ [`../api/002`](../api/002_the_roster_as_a_public_constant.md)).

### The Question

`PARKING_CRATES`'s doc comment says it holds *"the family crates from which a
parking operation is reachable"*. The array holds the crates that name
`ring_wait` in their own manifest. Those are different sets — three names against
five crates — and the test enforces the second while the sentence promises the
first.

One of the two has to move. Which?

### The Two Options

| | **A — narrow the sentence** | **B — widen the array** |
|---|---|---|
| The roster becomes | "crates that declare `ring_wait` directly" | "crates from which `ring_wait` is reachable" |
| Array length | stays `3` | becomes `5` |
| The test | unchanged — the scan already matches | must switch to a `cargo tree` closure |
| Breaking change? | no | **yes** — `[ &str; 3 ]` is in the type |
| What a consumer can then conclude | very little | the thing they wanted to conclude |
| Effort | one sentence | a test rewrite plus a public type change |

### Why A Is Not Simply Right

A is a documentation edit and B is a breaking change, so the cost argument points
one way hard. Three things push back.

**The narrow set answers a question nobody asks.** "Which crates name `ring_wait`
in their manifest" is a fact about manifests. "Which crates can end up sleeping"
is a fact about behaviour, and it is the one a scheduler author needs. Option A
keeps a public constant that is correct and not useful.

**The array's own justification is about visibility.** The doc comment defends
promoting the roster to the public surface on the grounds that *"adding a crate
to it is a visible API change rather than a quiet edit to an assertion"*. Under
A, a crate acquiring a transitive parking path is not a visible API change and
not an edit either — it is nothing at all. The mechanism the constant exists for
stops covering the case that motivated it.

**The breaking change is cheap right now.** Nothing outside `ring_poll`
*compiles* against `PARKING_CRATES`. Five files in the family name it and only
two are code — this crate's own `src/lib.rs` and its own test — while the other
three are prose: a doc comment in `ring_handle`'s suite and two manual test
plans. Widening the array costs a public-type edit and no downstream churn, and
that will be less true every month.

### Why B Is Not Simply Right Either

The `cargo tree` closure is a heavier check than a manifest scan: it needs a
resolved dependency graph, so the test stops being a file read and starts being a
subprocess against `cargo`, in a suite that currently shells out nowhere. That is
a real cost paid on every run of the suite, for a guard that fires rarely.

There is also a scope question B forces and A does not. Reachable *how* — through
normal dependencies only, or through dev-dependencies too? `ring_publish` reaches
`ring_barrier` through a dev-dependency, so the answer changes the count again.
This document measures the normal-only closure because that is what a consumer
links against; a suite that also cares about test builds would count six.

### The Measurement

Both counts, and the delta, are already computable. There is no missing evidence.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'option A set, size %s: ' "$( cd ring && for c in ring_*/Cargo.toml; do command grep -q 'ring_wait' "$c" && echo x; done | wc -l )"
( cd ring && for c in ring_*/Cargo.toml; do command grep -q 'ring_wait' "$c" && echo "${c%/Cargo.toml}"; done | tr '\n' ' ' ); echo
printf 'option B set, size %s: ' "$( for c in ring_*/; do n=$( basename "$c" ); cargo tree --manifest-path ring/Cargo.toml -e normal -p "$n" 2>/dev/null | command grep -q 'ring_wait' && echo x; done | wc -l )"
for c in ring_*/; do n=$( basename "$c" ); cargo tree --manifest-path ring/Cargo.toml -e normal -p "$n" 2>/dev/null | command grep -q 'ring_wait' && printf '%s ' "$n"; done; echo
printf 'with dev edges included:  %s\n' "$( for c in ring_*/; do n=$( basename "$c" ); cargo tree --manifest-path ring/Cargo.toml -p "$n" 2>/dev/null | command grep -q 'ring_wait' && echo x; done | wc -l )"
printf 'files naming the const:   %s\n' "$( command grep -rl 'PARKING_CRATES' */src */tests 2>/dev/null | wc -l )"
printf 'the declaration:          %s\n' "$( command grep -rl 'pub const PARKING_CRATES' */src 2>/dev/null | tr '\n' ' ' )"
printf 'code references:          %s\n' "$( command grep -rl 'PARKING_CRATES' */src */tests 2>/dev/null | while read -r f; do command grep -qE '^ *(use [^/]*|for [a-z]+ in |[a-z]+, )PARKING_CRATES' "$f" && echo "$f"; done | tr '\n' ' ' )"
printf 'prose-only references:    %s\n' "$( command grep -rl 'PARKING_CRATES' */src */tests 2>/dev/null | while read -r f; do command grep -qE '^ *(use [^/]*|for [a-z]+ in |[a-z]+, |pub const )PARKING_CRATES' "$f" || echo "$f"; done | tr '\n' ' ' )"
printf 'crates compiling against: %s\n' "$( command grep -rl 'PARKING_CRATES' */src */tests 2>/dev/null | while read -r f; do command grep -qE '^ *(use [^/]*|for [a-z]+ in |[a-z]+, |pub const )PARKING_CRATES' "$f" && echo "$f"; done | cut -d/ -f1 | sort -u | tr '\n' ' ' )"
printf 'subprocess calls in suite: %s\n' "$( command grep -c 'Command::new' ring_poll/tests/poll_test.rs || true )"
```

Live output:

```
option A set, size 3: ring_barrier ring_shutdown ring_wait 
option B set, size 5: ring_barrier ring_consume ring_shutdown ring_testkit ring_wait 
with dev edges included:  6
files naming the const:   5
the declaration:          ring_poll/src/lib.rs 
code references:          ring_poll/tests/poll_test.rs 
prose-only references:    ring_handle/tests/manual/readme.md ring_handle/tests/handle_test.rs ring_poll/tests/manual/readme.md 
crates compiling against: ring_poll 
subprocess calls in suite: 0
```

### Decisions

| File | Relationship |
|------|--------------|
| [001_should_the_roster_be_generated.md](001_should_the_roster_be_generated.md) | The other open question about the same constant, and the one whose threshold this one's answer changes |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/002_the_roster_as_a_public_constant.md`](../api/002_the_roster_as_a_public_constant.md) | PL7 and PL8 — the measurement that made this a decision rather than a bug |

### Integrations

| File | Relationship |
|------|--------------|
| [`../integration/002_what_actually_reaches_ring_wait.md`](../integration/002_what_actually_reaches_ring_wait.md) | The two edges, and the dev-edge case that makes "reachable" ambiguous |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | The constant, the sentence, and the doctest that is its only consumer |

### Tests

| File | Relationship |
|------|--------------|
| [`tests/poll_test.rs`](../../tests/poll_test.rs) | `the_tick_path_cannot_reach_a_parking_operation`, which option B would rewrite |

### PL15 — the two decisions about this constant have thresholds that move each other

[`001`](001_should_the_roster_be_generated.md) proposes generating the roster
once it reaches six entries. This decision's option B would take it to five in
one edit.

Neither document mentions the other's number. Read together they say: hand-write
the list while it is small, and here is a change that nearly doubles it. Taking
option B would put the roster one entry below the generate-it threshold, so a
single further crate acquiring a parking path would trip both decisions at once —
and the second one is filed as owned by work that has not started
([`001`](001_should_the_roster_be_generated.md) PL14).

The interaction runs the other way too. Option B replaces the manifest scan with
a `cargo tree` closure, which is most of the machinery generation would need
anyway. A crate that can compute the reachable set in a test can compute it in a
build script, so choosing B makes 001's expensive option substantially cheaper —
and 001 was deferred precisely because that option looked expensive.

Two decisions, one constant, and each one's cost estimate depends on an answer
the other has not given. Recording the coupling is what keeps either from being
settled in isolation on a number that the other invalidates.

### PL16 — an open decision with all its evidence in hand and no owner

The convention in this corpus is that an open decision names the measurement that
would settle it and the work that will produce it. [`001`](001_should_the_roster_be_generated.md)
does both — a count and an owner — which is what makes it a legitimate deferral.

This one has the measurement and it already returns an answer. Both sets are
computable today, the delta is two named crates, and the consumer count that
prices the breaking change is one. Nothing here is waiting on evidence.

That makes it a different kind of open than 001, and the corpus has no vocabulary
for the difference. Both render as `❓` in an Overview Table and both say
"Status: open", but one is blocked on a future measurement and the other is
blocked on somebody choosing. A reader scanning statuses cannot tell which is
which, and the second kind is the kind that quietly never resolves — there is no
event scheduled that will force it.

The honest label would be something like *decidable, undecided*. Filing it under
the same marker as a genuinely blocked question is the closest the schema allows,
and this paragraph is the difference written out longhand because the schema
cannot carry it.
