# Lifecycle Doc Definition

### Scope

- **Purpose**: The ring's teardown and reuse cycle as this crate defines it, and the two states, with the edges between them, that the cycle traverses.
- **Responsibility**: The open→closed→drained→open path, what `reset` composes, what 'the same allocation' does and does not promise, the state diagram, the per-state operation table, and which edge is idempotent.
- **In Scope**: `close`, `drain_all`, `discard_all`, `reopen`, `reset`; Open and Closed, and the `reset` edge that traverses both.
- **Out of Scope**: Ring construction (→ [`ring_core/docs/lifecycle/001`](../../../ring_core/docs/lifecycle/001_construction_and_backend_selection.md)); slot-level states (→ `ring_mpsc/docs/lifecycle/003`).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Teardown and Reuse](001_teardown_and_reuse.md) | What a reset ring is indistinguishable from, and how that is asserted rather than claimed | 🔄 |
| 002 | [Open and Closed](002_open_and_closed.md) | Two states, three edges, and the one operation reachable in only one of them | 🔄 |

**The split is path versus machine.** `001` walks the teardown once, end to end,
and says what the ring is afterwards. `002` draws the same territory as states
and edges and says what is permitted where. Neither subsumes the other: a path
document cannot answer "what can I call right now", and a state table cannot
carry a promise about two rings being indistinguishable.

They are separate because the two go stale on different edits. `001` changes
when the composition changes — a fourth step, a different middle. `002` changes
when a method's availability changes, which in this crate is a function of the
type system rather than of the flag, and is therefore where the type-level
claims accumulate. Every finding below that is about a *guarantee* landed in
`002`; every one about a *test* landed in `001`.

The four findings are two pairs. `001`'s are about **the gap between the feature
and its evidence**: the reference-ring apparatus that proves indistinguishability
never calls `reset`, the function the feature is named for (SD29), and the token
that carries the crate's central guarantee was not `#[ must_use ]` while nine
accessors were, so the suite dropped it seven times without a diagnostic (SD30).
`002`'s are about **a machine described more strongly than it is**: a Transitions
row that refutes a Behavioral Invariant twenty lines below it (SD31), and a
three-edge machine drawn over a type that can only take two of the edges (SD32).

SD31 is the one to read twice. The same fact — `close( &self )` mints without
limit — now appears as an open question, an asserted impossibility, and an
intended feature, in three current documents that do not cite each other on it.
Nothing in the corpus toolchain compares two statements inside one file.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown/docs/lifecycle
printf 'instances:                    %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:      %s\n' "$( command grep -hoE '^### SD[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:      %s\n' "$( command grep -coE '^\| SD[0-9]+ ' readme.md )"
printf 'each instance has a recipe:   %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'states the table has columns for: %s\n' "$( awk -F'\\|' '/^\| Operation \|/{ print NF - 3 }' 002_open_and_closed.md )"
printf 'stores into the flag:         %s\n' "$( cd ../..; command grep -c 'closed\.store' src/lib.rs )"
printf 'edges the tables list:        %s\n' "$( awk '/^### Transitions/{f=1} f&&/^`close/{exit} f&&/^\| .[a-z]/{ n++ } END{ print n-1 }' 002_open_and_closed.md )"
printf 'of those, on Shutdown itself: %s\n' "$( cd ../..; command grep -cE '^  pub fn (close|reopen)\(' src/lib.rs )"
printf 'what reset composes:          %s\n' "$( cd ../..; awk '/^pub fn reset</{f=1} f&&/^\}$/{exit} f' src/lib.rs | command grep -ohE 'close\(\)|drain_all|discard_all|reopen\(\)' | tr '\n' ' ' )"
printf 'what the reached-test does:   %s\n' "$( cd ../..; awk '/^fn the_three_operations/{f=1} f&&/^\}$/{exit} f' tests/shutdown_test.rs | command grep -ohE 'close\(\)|drain_all|discard_all|reopen\(\)|reset\(' | sort -u | tr '\n' ' ' )"
printf 'must_use on the proof token:  %s\n' "$( cd ../..; awk '/#\[ must_use/{ mu=1; next } /pub (const )?fn /{ if(mu){ sub( /.*fn /, "" ); sub( /\(.*/, "" ); print } mu=0 }' src/lib.rs | command grep -cx 'close' || true )"
printf 'bare closes in the suite:     %s\n' "$( cd ../..; command grep -c '^ *shutdown\.close();' tests/shutdown_test.rs || true )"
```

Live output:

```
instances:                    2
finding headings inside:      4
rows in the table below:      4
each instance has a recipe:   2
states the table has columns for: 2
stores into the flag:         2
edges the tables list:        3
of those, on Shutdown itself: 2
what reset composes:          close() discard_all reopen() 
what the reached-test does:   close() drain_all reopen() 
must_use on the proof token:  1
bare closes in the suite:     0
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| SD29 | the reached-test for the feature never calls the function the feature names | n/a — coverage | [`001`](001_teardown_and_reuse.md) cites `the_three_operations_hand_back_a_ring_fit_for_the_next_run` as *"the reached-test, with the reference ring"* — the only test that builds two rings and drives both through one script, which is the entire apparatus its "How the promise is asserted" section describes — and that test makes zero calls to `reset`, composing `close()`, **`drain_all`**, `reopen()` by hand while `reset` composes `close()`, **`discard_all`**, `reopen()`; the middle step is exactly what this document's own "Why `reset` discards rather than drains" paragraph argues is a different operation for a different caller, so behavioural indistinguishability — the promise the document exists to state — is asserted for a path that is not `reset`, while `reset`'s own test builds one ring, checks a count and a flag, and compares nothing; a regression in `discard_all` leaving the ring subtly unfit passes both. |
| SD30 | the proof token was not `must_use` while nine accessors were; it is now, and the seven discards state themselves | **latent hazard** | `src/lib.rs` carried nine `#[ must_use ]` attributes and every one sat on an accessor or predicate whose discarded value costs nothing (`new`, `Shutdown::is_closed`, `Stopped::shutdown`, `Refusal::is_closed`, `Refusal::reason`, `free_capacity`, `is_blocked`, `Guarded::shutdown`, `Wake::is_ready`), while `Shutdown::close` — which mints the `Stopped` whose consumption [`002`](002_open_and_closed.md)'s second Behavioral Invariant calls structural and [`../type/001`](../type/001_stopped_proof_token.md) exists to explain — carried none; `shutdown.close();` therefore compiled clean under `RUSTFLAGS="-D warnings"` and the suite wrote it seven times, which was legitimate in every one of those tests and is also exactly the statement a caller writes when they meant to close *and* drain, with nothing to say so — a type whose whole purpose is to be carried to a second call site could be dropped at the first without diagnostic, while a `Copy` enum accessor could not; `close` now carries a `#[ must_use ]` whose message names binding `_` as the way to close and nothing else, and the seven suite sites, five doctest lines and one `ring_testkit` site — the eighth, which the crate-scoped census never saw and the compiler found in two seconds — were rewritten to that explicit form, so a bare call written by reflex anywhere under `ring_*` is a warning rather than a ninth indistinguishable line. |
| SD31 | a Transitions row refutes a Behavioral Invariant twenty lines below it, in the same document | **wrong doc** | [`002`](002_open_and_closed.md) states as Behavioral Invariant 2 that *"a drain in the Open state is not an error a caller could hit, it is an expression that does not compile"*, and states in the Transitions table above it that `close` is idempotent because *"Closing a closed shutdown returns a fresh, usable token"*; both describe the code correctly and together refute the first, since `close( &self )` mints without limit while `reopen( self )` consumes only its receiver, so two tokens minted and one spent leaves the machine Open with a live `Stopped` in scope where `drain_all` compiles — making the permission table's two **not reachable** rows reachable and the paragraph beneath them true of the current *state* but not the current *scope*; the idempotence is deliberate and well-argued, and the defect is that one document holds both statements, unread against each other, while the same fact appears as an open question in [`decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md) and an asserted impossibility in [`invariant/002`](../invariant/002_drain_terminates_because_close_preceded_it.md), three current stances that do not cite one another. |
| SD32 | the state machine has three edges and the type it is drawn over can take only two | n/a — inconsistency | [`002`](002_open_and_closed.md) draws its machine over `Shutdown` and says so — *"the machine is that small because the flag is one `AtomicBool`"* — yet of the Transitions table's three edges, only `close` and `reopen` are methods on that flag: `reset` is `pub fn reset< T : Send >( shutdown : &Shutdown, consumer : &mut Consumer< '_, T > ) -> usize`, a free function needing a `Consumer` borrowed from a ring this crate does not own, while `Shutdown::new()` takes no arguments and five of the crate's twenty tests drive the machine with no ring in scope at all, so for every one of them Behavioral Invariant 3's *"every state is reachable from every other in one edge"* holds only through two of the three rows; the same conflation runs through the permission table, whose nine rows mix operations on the flag with operations on values it hands out and one that needs a `CursorPair` from a third crate — a useful table, but not the state machine over `Shutdown` the section heading claims. |
