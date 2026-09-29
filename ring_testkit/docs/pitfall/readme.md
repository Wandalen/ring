# Pitfall Doc Definition

### Scope

- **Purpose**: Record the three traps this crate walked into while being built — a reading of the accounting that fails on the exact case the accounting exists for, a step that reaches its effect only by passing through its opposite, and a headline operation two crates cannot join — and, for each, what was done about it.
- **Responsibility**: The mechanism, why it is not a bug in the crate that owns it, what it costs a caller who does not know, and where the evidence sits.
- **In Scope**: `Outcome`'s count-versus-list ambiguity under a dropping policy; `Step::Reopen`'s close-first path; the `SeqCell` bound between `ring_tls::TlsBuffer` and `ring_core::Ring`.
- **Out of Scope**: A shape forced by a tool rather than discovered in use (→ [`workaround/`](../workaround/readme.md)); a limitation stated as a law the crate upholds (→ [`invariant/`](../invariant/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Neither The Count Nor The List Alone](001_neither_the_count_nor_the_list_alone.md) | Why `accepted` and `received` each read a silent drop wrong, and the accessor that exists because of it | 🔄 |
| 002 | [Reopening Closes First](002_reopening_closes_first.md) | The refusal window a `Reopen` opens on an already-open ring, and why the fixture does not work around it | 🔄 |
| 003 | [The Amortised Flush Has No Ring](003_the_amortised_flush_has_no_ring.md) | The `SeqCell` bound that keeps `flush_into` away from a `ring_core::Ring`, and what the slow join costs | 🔄 |

**The split is by which boundary the trap sits on.** `001` is internal — the
fixture's own accounting, read wrongly by a consumer holding only one of its two
halves. `002` is at the `ring_shutdown` edge: a type whose by-value `self` is
correct in its own crate and expensive in a step that has to reach past it.
`003` is at the `ring_tls` edge: two crates the fixture depends on that have no
fast path between them.

They are apart because their remedies are unrelated. `001` produced a method
(`Outcome::vanished`). `002` produced a decision to change nothing, and a
sentence on the variant. `003` produced neither — the fixture pays the slow
path and records why.

Each is a trap discovered by building, not predicted by design. None is a bug in
the crate that owns the mechanism, which is why all three are documented rather
than filed.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_testkit/docs/pitfall
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### TK[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| TK[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'consequence rows, all three: %s\n' "$( command grep -hcE '^\| (P[0-9]|Q[0-9]|S[0-9]) \| ' [0-9][0-9][0-9]_*.md | paste -sd+ | bc )"
printf 'evidence rows, all three:  %s\n' "$( command grep -hcE '^\| (F[0-9]|R[0-9]|T[0-9]) \| ' [0-9][0-9][0-9]_*.md | paste -sd+ | bc )"
printf 'of those, deferring to the row above: %s\n' "$( command grep -hE '^\| (F[0-9]|R[0-9]|T[0-9]) \| ' [0-9][0-9][0-9]_*.md | awk -F'|' '{ if ( $4 ~ /same test/ ) c++ } END{ print c+0 }' )"
printf 'distinct tests they cite:  %s\n' "$( command grep -hoE '\| .[a-z_]{6,}. \|$' [0-9][0-9][0-9]_*.md | tr -d '| \140' | sort -u | wc -l )"
```

Live output:

```
instances:                3
finding headings inside:  6
rows in the table below:  6
each instance has a recipe: 3
consequence rows, all three: 9
evidence rows, all three:  11
of those, deferring to the row above: 3
distinct tests they cite:  9
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| TK41 | what the audit sums | **latent hazard** | `Outcome::audit` sums `accepted + refused_full + refused_closed + refused_staging + staged_at_end` and names neither `received` nor `in_ring_at_end`, so a record the ring destroyed is still `placed` and the audit returns `Ok( () )` — correct behavior, whose only counter-signal is `vanished()`, a method with zero call sites in `src/` on a struct whose ten fields are all public, so a consumer can read every field, take the packaged verdict and never meet it. |
| TK42 | the constructor the exclusion rests on | **latent hazard** | The Out of Scope line excludes `OverflowPolicy::DropOldest` because `ring_core::Ring::new` refuses it, but `Ring::new_crossbeam` accepts it on the same type and `Script::run` contains zero `backend` or `crossbeam` references, so the fixture cannot tell the two apart — the exclusion holds only because `crossbeam` appears zero times in this crate's manifest, a fact about its dependencies rather than about the fixture. |
| TK43 | where the crate's concurrency tooling points | n/a — coverage | The window's third row has no evidence because it is unreachable from a single-threaded fixture, while the same `tests/` directory holds three `loom::model` closures spawning six threads — the only such file in all 33 `ring_*` crates — plus two real threads in the ordinary suite, and the count of `Shutdown`, `close` and `Refusal` across the loom file is zero in every case. |
| TK44 | where the deferred question is filed | n/a — doc gap | The `ring_shutdown` API question this pitfall correctly declines to answer is parked in this crate's own decision log as Pending 2; `ring_shutdown` has since filed two decisions of its own and neither is this one (*Should `Guarded::into_inner` Exist*, *Should a `Stopped` Token Be Unique* — the second lands next door, asking whether the token can be duplicated rather than whether the ring can open without one), zero of its docs contain the phrase *unconditional open* and zero name `Step::Reopen`; what makes this worth keeping is that the cheap explanation has since been ruled out — when this was written exactly one file there named `ring_testkit` and it was a test comment, whereas ten do now, nine of them documents citing this crate's `Stopped` binding and its guard as the only ones outside `ring_shutdown` itself, so the channel exists and carries detail in both directions and the question still did not travel: **a question filed in a consumer's decision log is addressed to nobody**, and citation traffic moves facts about code rather than open questions about design. |
| TK45 | how far the unadopted path reaches | n/a — unadopted | `.flush_into(` has zero call sites outside `ring_tls` and fifteen inside it, and `ring_tls`'s dependents are exactly `ring_bench`, `ring_flush` and `ring_testkit` — all three of which also depend on `ring_core`, so every crate in the family holding a `TlsBuffer` also holds a `Ring` and all three independently reached for `drain()`, against one shared cause written down in only one of them. |
| TK46 | what "exposes no cursor" establishes | **misleading doc** | The mismatch argument correctly measures that zero of `ring_core`'s sixteen public functions name a cursor and reads that as the material being absent; `SeqCell` is declared in `ring_atomic` with three implementors in the family, `ring_atomic` sits three hops inside `ring_core`'s own dependency tree, and one implementor is the cursor type the SPSC backend already runs on — so the join is unavailable by encapsulation, not unavailable for want of a cursor. |
