# Workaround: A Counter-Bounded `while` for the Coverage Tool

### Scope

- **Purpose**: Record that every loop in this crate is a counter-bounded `while` rather than a `loop` with a break, that the reason is `llvm-cov`'s region attribution rather than style, and that the reason appears nowhere in this crate's source.
- **Responsibility**: The external constraint, the shape it forces, where the evidence for it lives, and why this one is invisible at the site it governs.
- **In Scope**: The four loops, the measurement that motivated the shape, and the provenance of the rule.
- **Out of Scope**: What the retry loops do (→ [`../algorithm/001`](../algorithm/001_bounded_retry.md)); the pause hint inside them (→ [`001_reimplementing_the_pause_hint.md`](001_reimplementing_the_pause_hint.md)).

### Constraint

`llvm-cov` opens a coverage region on a bare `loop` line and never attributes a
hit to it. The line reads as uncovered no matter how thoroughly the tests
exercise the body, so a crate that uses `loop` cannot reach 100% line coverage
however good its suite is.

This is an external constraint in the strict sense the readme demands: it comes
from a tool, it is not this crate's to fix, and it imposes a shape on code that
would otherwise be written differently.

### Replacement

Four loops, all counter-bounded, no `loop` keyword anywhere in `src`:

```rust
while attempt < budget.attempts()   // push_within, push_batch_within, recv_within
while taken < max                   // drain_up_to
```

### Cost

**The shape costs almost nothing. The invisibility costs everything the
workaround is for.**

`ring_shutdown` measured this — 72/73 lines with `loop`, 73/73 with a `while`,
identical suite — and wrote the finding into `src/lib.rs`, six lines at the loop
it governs, naming the tool, the mechanism, both numbers and the manual probe
that produced them. Anyone editing that function meets the reason before they
can change the shape.

`ring_poll` inherited the shape and did not inherit the note. The string
`llvm-cov` does not appear in this crate's source at all. It appears only under
`docs/` — in `algorithm/001`, which is about bounded retry and mentions it in
passing; in this directory's readme, which until now cited it in order to argue
this was not a workaround; and in this file. A maintainer rewriting
`while attempt < budget.attempts()` as a
`loop` with an inner `break` — a change that reads as a pure simplification —
has nothing in the file to stop them, and the cost surfaces as a coverage
regression days later, attributed to whatever else moved → PL51.

### Deletion condition

When `llvm-cov` attributes hits to a bare `loop` line. No version is named
anywhere, no issue is linked, nothing checks whether the behaviour still holds,
and the measurement that established it was taken once, in a different crate, on
a version nobody recorded. Two crates in the family still use `loop` today, so
the rule is not applied consistently enough for a family-wide check to be the
thing that catches its obsolescence either → PL52.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'while loops in ring_poll:       %s\n' "$( command grep -cE '^\s*while ' ring_poll/src/lib.rs || true )"
printf 'their conditions:               %s\n' "$( command grep -ohE '^\s*while .*' ring_poll/src/lib.rs | sed 's/^ *while //' | sort -u | tr '\n' ' ' )"
printf 'bare loop in ring_poll:         %s\n' "$( command grep -cE '^\s*loop$' ring_poll/src/lib.rs || true )"
printf 'llvm-cov named in ring_poll src:%s\n' "$( command grep -c 'llvm-cov' ring_poll/src/lib.rs || true )"
printf 'and in its docs, excl. index:   %s\n' "$( command grep -rl 'llvm-cov' ring_poll/docs/ 2>/dev/null | command grep -v '/definition/' | sed 's|ring_poll/docs/||' | tr '\n' ' ' )"
printf 'llvm-cov in ring_shutdown src:  %s\n' "$( command grep -c 'llvm-cov' ring_shutdown/src/lib.rs || true )"
printf 'the numbers it recorded there:  %s\n' "$( command grep -ohE '[0-9]+/[0-9]+ with [^,]*' ring_shutdown/src/lib.rs | tr -d '\140' | tr '\n' ' ' )"
printf 'crates still using bare loop:   %s\n' "$( command grep -rlE '^\s*loop$' ring_*/src/*.rs 2>/dev/null | sed 's|/src/lib.rs||' | tr '\n' ' ' )"
printf 'a tool version pinned anywhere: %s\n' "$( command grep -rhoE 'llvm-cov [0-9][0-9.]*' ring_poll ring_shutdown 2>/dev/null | wc -l )"
```

Live output:

```
while loops in ring_poll:       4
their conditions:               attempt < budget.attempts() taken < max 
bare loop in ring_poll:         0
llvm-cov named in ring_poll src:2
and in its docs, excl. index:   workaround/002_a_counter_bounded_while.md workaround/readme.md algorithm/001_bounded_retry.md 
llvm-cov in ring_shutdown src:  1
the numbers it recorded there:  80/81 with loop 81/81 with this 
crates still using bare loop:   ring_bench ring_publish 
a tool version pinned anywhere: 0
```

### Algorithms

| File | Relationship |
|------|--------------|
| [`../algorithm/001_bounded_retry.md`](../algorithm/001_bounded_retry.md) | The three retry loops this shapes, and the only place in this crate the reason is written |

### Workarounds

| File | Relationship |
|------|--------------|
| [`001_reimplementing_the_pause_hint.md`](001_reimplementing_the_pause_hint.md) | The other one, and the contrast: that constraint is chosen, this one is imposed |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [`../non_functional_requirement/002_the_cost_nobody_has_measured.md`](../non_functional_requirement/002_the_cost_nobody_has_measured.md) | The other property with no standing measurement — that one was never measured at all, this one was measured once in another crate and never re-checked |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Four counter-bounded `while` loops, zero `loop` keywords, and zero mention of why |

### PL51 — the crate that inherited the rule did not inherit the note that protects it

`ring_shutdown` established this. Its `drain_all` carries a six-line comment
saying what the tool does, why the line reads as uncovered, that the measurement
was 72/73 with `loop` against 73/73 with a `while` on the identical suite, and
where the probe that produced those numbers is recorded. The comment is at the
loop, so the reason is unavoidable for anyone who edits the shape.

`ring_poll` took the shape and left the note behind. `llvm-cov` appeared zero
times in this crate's `src`, and only in documents written afterwards about
something else: `docs/algorithm/001`, which mentions it in passing while
explaining bounded retry, and this directory's readme, which cited it in order
to argue the item did not belong here.

The asymmetry is the finding, and it was worse than a missing comment. A
workaround whose shape is indistinguishable from ordinary code has exactly one
defence — a note at the site — and this was the crate where that defence was
absent. Every other constraint here is visible in some structural way: the
absent `ring_wait` dependency is visible in the manifest and asserted by a test,
the clamp is visible in the constructor. This one was visible nowhere. The fix
was four lines of comment, or one line and three cross-references, and it was the
cheapest item in this crate's findings.

**Disposition:** applied — a comment now sits directly above `push_within`'s
`while attempt < budget.attempts()` in `src/lib.rs`, naming `llvm-cov` as the
tool, stating that a bare `loop` reads as uncovered under it, citing
`ring_shutdown`'s measured 72/73 against 73/73 on the identical suite as the
origin of the rule, and naming `cargo llvm-cov -p ring_poll` as the probe that
re-checks it here. The note is at one of the four loops rather than all four,
because the four are adjacent in one file and repeating it would make the shape
noisier than the constraint it defends. The crate's `src` is no longer silent
about the tool — Now prints: `llvm-cov named in ring_poll src:2`

### PL52 — the workaround has no deletion condition and nothing that would notice it had expired

The readme opening this directory defines a workaround as a constraint absorbed
*"with the cost it imposes and the condition under which it can be deleted."*
For this one the deletion condition is real and simple — `llvm-cov` attributing
hits to a bare `loop` line — and nothing about it is written down.

No version of the tool is named in either crate. No issue is linked. The
measurement behind the rule was taken once, in `ring_shutdown`, against a
toolchain nobody recorded, and no check re-runs it. If the tool were fixed
tomorrow, the way this crate would find out is that somebody happened to try a
`loop` and noticed the coverage did not drop.

The family is not a safety net for this either. Two crates — `ring_bench` and `ring_publish` — still use a bare
`loop` in `src`, so the rule is not uniform, and a family-wide check asserting
its absence would fail today for reasons unrelated to the tool — meaning nobody
can add one without first deciding whether those two crates are wrong or
exempt.

What that produces is a permanent workaround by default: a constraint absorbed
into the shape of four loops, with a stated cost of nearly zero, an unstated and
unmeasured expiry, and no mechanism that would ever raise it again. Recorded
rather than fixed because pinning it properly means re-running `ring_shutdown`'s
D2 probe on the current toolchain and writing the version down, which is a
measurement task rather than a documentation one, and belongs with the crate
that owns the probe.
