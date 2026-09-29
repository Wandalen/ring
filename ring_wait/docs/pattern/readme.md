# pattern

Two patterns, and this crate is the reference instance of both. The first is
about its loop: predicate, pause, budget — three parts, each independently
substitutable, and the family substitutes each of them somewhere. The second is
about its enum: discriminants in `ring_types` where everything can reach them,
handlers in a leaf crate three others are forbidden to depend on.

Both are stated here as patterns rather than as this crate's design because both
have instances elsewhere — and in each case the instances are where the
interesting divergence shows up.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [The Predicate, the Pause, and the Budget](001_the_predicate_the_pause_and_the_budget.md) | The three-part shape, its four instances, `ring_publish`'s deliberate omission of the budget, and WT24 — the pause after the final look |
| 002 | [Discriminants Here, Handlers There](002_discriminants_here_handlers_there.md) | The policy-enum split, a comment-stripped census of who really matches on what, and WT22 — the one-handler assumption already broken for `OverflowPolicy` |

### The Two Patterns

| | Loop shape (001) | Enum shape (002) |
|--|------------------|------------------|
| This crate's role | the only parameterised instance | the sole handler for `WaitKind` |
| Other instances | 3 open-coded in `ring_poll` | 2 handlers for `OverflowPolicy` |
| Near-miss | `ring_publish` — no budget | `ring_core` — one variant, one branch |
| Why not shared | the low crate *is* the forbidden one | the enum crosses a tier boundary the handler must not |
| Cost | one rule written twice ([WT17](../algorithm/001_one_loop_and_the_two_ways_out.md)) | an N-crate edit per new variant, N stated nowhere |

### Regenerate Both Censuses

```sh
cd "$(git rev-parse --show-toplevel)"

# instances of the loop shape — every bounded retry with a pause hint. The roots
# are named explicitly and the result sorted: a bare `` glob
# reaches neither `ring/` nor the `/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/`-nested crates, and `grep` here is
# a `ugrep` shim whose unsorted output order is not stable between runs
command grep -r --include=*.rs "spin_loop" /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ . /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/ | command grep "/src/" | sort

# instances of the enum shape — which files name every variant in *code*
for e in WaitKind OverflowPolicy; do
  echo "## $e"
  for f in $( command grep -rl --include=*.rs "$e::" /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/ . /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/ /home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/ | command grep "/src/" | sort ); do
    v=$( command grep -vE "^[[:space:]]*(//|///|//!)" "$f" | command grep -o "$e::[A-Za-z]*" \
         | sort -u | tr '\n' ' ' )
    # the trailing `true` keeps a final empty `$v` from failing the whole block
    [ -n "$v" ] && printf '%-40s %s\n' "$f" "$v"
    true
  done
done
```

Live output:

```
ring_poll/src/lib.rs:          core::hint::spin_loop();
ring_poll/src/lib.rs:      core::hint::spin_loop();
ring_poll/src/lib.rs:      core::hint::spin_loop();
ring_publish/src/lib.rs:      core::hint::spin_loop();
ring_publish/src/lib.rs://! [`Publisher::publish`] loops on a `spin_loop` hint with no wait strategy and
ring_wait/src/lib.rs:        core::hint::spin_loop();
## WaitKind
ring_config/src/lib.rs              WaitKind::default 
ring_wait/src/lib.rs                WaitKind::None WaitKind::Park WaitKind::Spin WaitKind::Yield 
## OverflowPolicy
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/smoke_ring_write_path/src/lane.rs OverflowPolicy::default 
ring_config/src/lib.rs              OverflowPolicy::default 
ring_core/src/lib.rs                OverflowPolicy::DropOldest 
ring_overflow/src/lib.rs            OverflowPolicy::DropNewest OverflowPolicy::DropOldest OverflowPolicy::Fail 
ring_stats/src/lib.rs               OverflowPolicy::ALL OverflowPolicy::DropNewest OverflowPolicy::DropOldest OverflowPolicy::Fail 
```

| | Count |
|--|------:|
| Sites with a `spin_loop` hint | 5 — `ring_wait` 1, `ring_poll` 3, `ring_publish` 1 |
| Of those, bounded by a budget | 4 |
| Files *mentioning* `WaitKind::` under `src/` | 6 |
| Files naming a `WaitKind` variant **in code** | **2** — `ring_wait` (all four), `ring_config` (`default` only) |
| Files *mentioning* `OverflowPolicy::` under `src/` | 12 |
| Files naming every `OverflowPolicy` variant in code | **2** — `ring_overflow`, `ring_stats` |

The gap between the mention count and the code count is the reason both censuses
strip comments first: six-versus-two and twelve-versus-two are the difference
between "this enum is matched on everywhere" and "this enum has one handler".

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| WT22 | family | n/a — drift | `WaitKind` has exactly one handler and `OverflowPolicy`, ruled identically, already has two, so adding a variant to the latter is an edit in two crates that neither compiler nor test connects |
| WT24 | `ring_wait` | **measured cost** | `wait_until` pauses after the final look, for a look that never happens: a `Park` wait at a budget of 1 sleeps 133 µs and that sleep is 100% of the call's cost; `ring_poll`'s copies guard against it with one comparison and nothing tests it in either crate |
| WT44 | family | n/a — inconsistency | The shared three-part shape disagrees about whether its budget is a domain type: `ring_poll` wraps it in a `Budget` newtype that clamps in its constructor, this crate passes a bare `usize` and clamps at every loop header (WT17) — and the crates may not depend on each other, so the disagreement is unresolvable |
| WT45 | `ring_wait` | n/a — drift | `escalation_hint` folds `Park` with `WaitKind::None` into one arm, the only place in the handler crate the enum is not matched variant by variant; a fifth variant would break `pause`'s compile and be silently absorbed as terminal here, in the one function whose correctness rests on which variants are terminal |
