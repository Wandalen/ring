# integration

Three dependencies out, two dependents in, and the two dependents disagree about
what this crate is for. One links it into a production ring; the other links it
only under `cargo test`, to prove that the half of the feature it *does* ship
composes with the half it does not.

The second file is about the edge that matters most and is used least: this
crate is `ring_gating`'s principal caller, and it calls one of that crate's four
public predicates. The other three have no library caller anywhere in the 33
crates, and the most caller-shaped of them cannot be called from here without
reintroducing the race the crate exists to prevent.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Two Dependents That Split One Feature](001_two_dependents_that_split_one_feature.md) | CL1, CL2, CL3 — the graph, `ring_mpsc` adopting the claim half after rejecting the publish half, and the `ring_seqno` edge that was removed from both halves |
| 002 | [Four Predicates and the One That Is Called](002_four_predicates_and_the_one_that_is_called.md) | CL4, CL5, CL6 — `headroom` against `admits`/`check`/`limit`, why `check` is `claim`'s guards and calling it would be the bug, and three different reasons for three uncalled predicates |

### The Graph, in Full

| Edge | Section | What crosses it |
|------|---------|-----------------|
| → `ring_types` | `[dependencies]` | `Seq`, `RingError` |
| → `ring_cursor` | `[dependencies]` | `PaddedCursor`, `SeqCell`, `GATING` |
| → `ring_gating` | `[dependencies]` | `GatingSet` — one method of four |
| ← `ring_mpsc` | `[dependencies]` | `Claimer`, and the borrow that shaped its `Ends` type |
| ← `ring_publish` | `[dev-dependencies]` | `Claim`, in the claim/publish handshake's reached-test only |

Five edges. The two that are least like each other are the two incoming ones:
one crate builds a ring on this primitive, the other links it to test that a
handshake it does not participate in still works.

### Why the Gating Edge Is Thin

`GatingSet` is a four-predicate type and this crate touches one predicate. The
distribution is not neglect — it follows from where each predicate can be
evaluated:

| Question | Answered by | Usable inside a CAS retry? |
|----------|-------------|----------------------------|
| how many slots are free at sequence *s*? | `headroom` | **yes** — it is a pure read |
| may I take *n* at *s*? | `admits` | yes, but it is `headroom` plus `<=` |
| may I take *n* at *s*, with a reason? | `check` | **no** — a verdict computed before the exchange is stale by the time it lands |
| where is the ceiling? | `limit` | not a control-flow question at all |

Only the predicate that returns a raw number survives being re-evaluated on
every retry, because only a number can be compared against a cursor value that
has already moved.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order

# out — declared dependencies
awk '/^\[dependencies\]/{f=1;next} /^\[/{f=0} f' ring_claim/Cargo.toml \
  | grep -oE '^ring_[a-z_]+'

# in — every manifest naming this crate, with its section
for m in */Cargo.toml; do
  awk -v f="$m" '/^\[/{s=$0} /ring_claim/{printf "%s  %s  %s\n", f, s, $0}' "$m"
done

# the four predicates and every call site of each
grep -E '^  pub fn (headroom|admits|check|limit)' ring_gating/src/lib.rs
for p in headroom admits check limit; do
  printf "%-10s " "$p"
  grep -r "\.$p(" ring_*/src/*.rs | grep -v '///' | wc -l
done

# prose references that are not edges
command grep -rl 'ring_claim' */src/*.rs | command grep -v ring_claim/
```

Live output:

```
ring_types
ring_cursor
ring_gating
ring_claim/Cargo.toml  [package]  name = "ring_claim"
ring_mpsc/Cargo.toml  [dependencies]  ring_claim = { path = "../ring_claim" }
ring_publish/Cargo.toml  [dev-dependencies]  ring_claim = { path = "../ring_claim" }
  pub fn headroom( &self, producer : Seq ) -> usize
  pub fn admits( &self, producer : Seq, count : usize ) -> bool
  pub fn check( &self, producer : Seq, count : usize ) -> Result< (), RingError >
  pub fn limit( &self ) -> Option< Seq >
headroom   6
admits     1
check      0
limit      0
ring_batch/src/lib.rs
ring_consume/src/lib.rs
ring_cursor/src/lib.rs
ring_gating/src/lib.rs
ring_index/src/lib.rs
ring_mpsc/src/lib.rs
ring_publish/src/lib.rs
ring_spsc/src/lib.rs
/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/spike/demo_orbital_rail/src/beat_s11.rs
```

| | Value |
|--|------:|
| Declared dependencies | 3 |
| …removed before implementation | 1 — `ring_seqno` |
| Dependents, `[dependencies]` | 1 — `ring_mpsc` |
| Dependents, `[dev-dependencies]` | 1 — `ring_publish` |
| Tier 5 crates with dependents in both sections | **1** — this one |
| `GatingSet` public predicates | 4 |
| …with a library caller anywhere in the family | **1** |
| …whose only outside caller is a test in this crate | 1 — `limit` |
| `GatingSet` call sites in this crate's library | 3 — all `headroom` |
| Guards shared verbatim between `check` and `claim` | 2 |
| …that could be replaced by a call | **1** |
| Crates naming this one in prose | 3 |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CL1 | `ring_claim` | n/a — observation | The only Tier 5 primitive whose two dependents sit in different manifest sections; the two halves of the claim/publish handshake are joined by an edge that exists only under `cargo test` and have never been linked in a release build |
| CL2 | `ring_mpsc` | n/a — drift | Both halves of the handshake were scaffolded into `ring_mpsc` and one survived: a per-slot stamp replaces a published cursor, but nothing replaces range exclusivity, which can only be established at the cursor |
| CL3 | `ring_claim` | n/a — observation | `ring_seqno` was removed from this crate and from `ring_publish` by the same check, because `GatingSet::headroom` reaches Tier 1 arithmetic on this crate's behalf |
| CL4 | `ring_gating` | n/a — observation | `ring_gating`'s principal caller uses one of its four public predicates; the one that returns a number is used, the three that return a verdict are not |
| CL5 | `ring_gating` | **latent hazard** | `GatingSet::check` is `claim`'s two guards in the same order with the same payloads, and calling it would move the gate outside the retry — the exact `fetch_add` design the module doc rejects |
| CL6 | `ring_gating` | n/a — observation | Three uncalled predicates for three different reasons: `admits` is sugar over the called one, `check`'s shape is wrong here specifically, `limit` answers a diagnostic question and its only outside caller is a test in this crate |

Supporting observations recorded alongside those findings, carrying no ID of their own:

| Note | Where |
|------|-------|
| `ring_spsc` states this crate's entire justification as a negation — "two producers may race for the same sequence" — and then declines the crate, because one producer cannot race itself | [001](001_two_dependents_that_split_one_feature.md) |
| The first guard is genuine, removable duplication; the second cannot be removed, because `check` asks *is there room now* and `claim` needs *is there room at the value I am exchanging against* | [002](002_four_predicates_and_the_one_that_is_called.md) |
| `admits` is also the family's one name collision between two public predicates — `ring_barrier:238` has a different signature under the same name | [002](002_four_predicates_and_the_one_that_is_called.md) |

