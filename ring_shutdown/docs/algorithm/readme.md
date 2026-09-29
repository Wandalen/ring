# Algorithm Doc Definition

### Scope

- **Purpose**: Record the two procedures this crate owns — draining a closed ring to empty, and deciding which of two exits a close-aware wait took — with the argument each one rests on.
- **Responsibility**: The drain loop, its bound and the condition under which the bound fails; the dual-condition spin, its precedence rule, and what each of its three answers claims.
- **In Scope**: `Stopped::drain_all`, `Stopped::discard_all`, `for_space_or_close`, `wait_for_close`.
- **Out of Scope**: The per-backend drain shapes underneath (→ [`ring_core/docs/algorithm/002`](../../../ring_core/docs/algorithm/002_uniform_drain_over_three_shapes.md)); the bounded spin itself, which is `ring_wait`'s (→ [`ring_wait/docs/algorithm/readme.md`](../../../ring_wait/docs/algorithm/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Drain to Empty](001_drain_to_empty.md) | Why one `try_recv_batch` is not a drain, and what makes the loop stop | 🔄 |
| 002 | [Deciding Which Exit a Wait Took](002_deciding_which_exit_a_wait_took.md) | One spin, two conditions, and a side channel that carries the reason out of the predicate | 🔄 |


**Both documents are about a loop whose bound comes from outside it**, which is
what makes them one definition rather than two unrelated ones. The drain stops
because publication stopped — a fact about a different object entirely. The
close-aware wait stops because a spin budget ran out or because a flag flipped
— neither of which the loop controls. In both cases reading the loop tells a
reader almost nothing about when it ends.

The four findings split by *what went unrecorded*. `001`'s two are about a
decision that was made carefully and then under-propagated: the `while`-over-
`loop` spelling is explained in six lines of comment on `drain_all` and nowhere
on `discard_all`, which the same document calls "the same shape" (SD1), and the
`llvm-cov` artifact behind it is written up in the crates that already adopted
the fix and in neither of the two that still contain a bare `loop` (SD2).
`002`'s two are about information the code had and discarded: the spin count
`wait_until` returns is passed through by the teardown waiter and thrown away by
the publish-path one, which is the one where budget tuning matters (SD3), and
the staleness caveat is attached to the `Wake` variant that does not need it
rather than the one that does (SD4).

Read together: this crate's procedures are correct and its record of *why* they
are shaped as they are stops one step short each time.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/algorithm
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'loops in the crate source:    %s\n' "$( command grep -vE '^ *//' ../../src/lib.rs | command grep -cE '^ *(while|loop)' || true )"
printf 'of those, bare loop:          %s\n' "$( command grep -c '^ *loop$' ../../src/lib.rs || true )"
printf 'procedures with a bound:      %s\n' "$( command grep -c 'spins' ../../src/lib.rs || true )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
loops in the crate source:    2
of those, bare loop:          0
procedures with a bound:      5
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD1 | the pseudocode covers one of the two procedures it claims to | **misleading doc** | [`001`](001_drain_to_empty.md) states the drain loop once and adds *"`discard_all` is the same shape over `try_recv` instead of `try_recv_batch`"*, but the two spellings differ in exactly the dimension the document spends a section on — `drain_all` is a `while taken > 0` with a duplicated first read, chosen over a bare `loop` to avoid an `llvm-cov` artifact and carrying six lines of comment saying so, while `discard_all` is a `while let Some( … )` carrying none; a maintainer rewriting `discard_all` therefore meets no reason not to reach for the `loop`, and the loop with no explanation is the one [`non_functional_requirement/002`](../non_functional_requirement/002_the_teardown_path_takes_the_slow_one.md) shows `reset` actually routes through. |
| SD2 | the workaround is recorded in the crates that adopted it and in neither that needs it | n/a — unadopted | The `llvm-cov`-opens-a-region-on-`loop` artifact is written into two source files (`ring_core`, `ring_shutdown`) and three doc corpora (`ring_core`, `ring_poll`, `ring_shutdown`), each with a matching manual-probe entry — and the only two crates in the family still containing a bare `loop`, `ring_bench` with five and `ring_publish` with one, mention `llvm-cov` nowhere; so either those six lines cost a coverage point nobody has attributed, or the artifact is narrower than three write-ups claim and the workaround is carried on stale grounds, and the two possibilities are separated by one measurement nobody has run because the artifact is described in prose everywhere and asserted nowhere. |
| SD3 | one waiter returns the spin count and the other throws it away | n/a — inconsistency | `ring_wait::wait_until` returns `Ok( attempt )`, the only feedback a caller has on whether a spin budget is generously or barely sized; `wait_for_close` passes it through as `Result< usize, RingError >` while `for_space_or_close` matches `Ok( _ )` twice and replaces it with a `Result< Wake, RingError >`, though `Result< ( Wake, usize ), RingError >` would cost nothing at runtime — and the side that lost the number is the publish-path one, run in a loop under a budget the caller is expected to choose, in a crate whose surface promises to be reachable from inside a tick. |
| SD4 | the staleness caveat is on the variant that does not need it | **misleading doc** | `Wake::Closed` is documented *"The ring closed while waiting. The condition may still be false"* and stays true for as long as a caller could act on it, since reopening needs a `Stopped` token the waiting producer does not hold; `Wake::Ready` is documented *"The condition the caller was waiting for became true"* with no caveat at all, though it reports that `pair.may_claim()` was true at some past attempt and another producer may have taken the slot since — and the crate caveats exactly this elsewhere three times over (`is_blocked`'s *"does not predict a refusal"*, `free_capacity`'s *"advisory elsewhere"*, `admit`'s *"deliberately not transient"*), which is what makes the one omission legible rather than ordinary. |
