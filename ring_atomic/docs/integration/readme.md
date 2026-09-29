# integration

Two edges in, five out, and neither number is what it looks like. Of the two
declared dependencies one is imported nowhere — confirmed by `cargo udeps` across
every target, and faithfully repeated in three prose documents that all describe
the manifest rather than the code. Of the five dependants, the trait they exist for
is in scope in twelve, because `ring_cursor` republishes it in one line.

Both errors run in the same direction as the family's usual drift and for the same
reason: a number or a list written once, correct once, and never recomputed —
while every one of them is a two-line shell loop away from being checked.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_two_declared_one_used.md) | Two Declared, One Used | The unused `ring_seqno` edge, the three documents that record it, and where it came from |
| [002](002_five_crates_downstream.md) | Five Crates Downstream, Twelve Reached | The re-export that triples the trait's reach, its argument, and its own stale count |

## The Only Edge That Matters Is the One Nobody Can See

`ring_types` is the crate's single real dependency and it supplies exactly one
item: `Seq`, a `u64` newtype. That one import is the whole difference between a
crate that is part of a family and a crate that is `AtomicU64` under another name
— it is what lets `ring_batch::claim` return `BatchClaim` of `Seq` with no cast and
`ring_cursor::PaddedCursor` be a drop-in.

The unused `ring_seqno` sits beside it in the manifest, in the module comment, in the
readme, and in the task file, indistinguishable from it in all four.

## Two Names, One Rule, Nobody Told

A crate imports `SeqCell` from `ring_atomic` if and only if it declares
`ring_atomic`; otherwise from `ring_cursor`. Twelve crates, no exceptions, and one
of the twelve has a comment saying the two paths exist. `ring_cursor` argues for
the re-export on its own side; nothing on this side records that the crate's
central abstraction has a second public home.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- in --'
command grep -E '^ring_|^loom' ring_atomic/Cargo.toml
command grep -nE '^use ring_' ring_atomic/src/lib.rs
echo '  -- out --'
command grep -l '^ring_atomic' ring_*/Cargo.toml | sed 's|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/||;s|/Cargo.toml||' | tr '\n' ' '; echo
echo '  -- and the re-export that widens it --'
command grep -n 'pub use ring_atomic' */src/lib.rs | sed -E 's#^(/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/[^/]+|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/[^/]+|module|ring|spike)/##'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| AT17 | `ring_atomic` | **wrong doc** | `ring_seqno` is declared, named in the module comment, the readme, and the task file, and imported nowhere — `cargo udeps --all-targets` confirms it, and it is one of only two unused dependencies in all 33 crates |
| AT18 | `ring_atomic` | n/a — observation | The one real edge supplies a single newtype, and is indistinguishable from the dead one in the manifest, the readme, the module comment, and the task file — only the compiler can tell them apart |
| AT19 | `ring_atomic` | n/a — observation | Five crates declare `ring_atomic`; `SeqCell` is in scope in twelve via `ring_cursor`'s one-line re-export, by an exact rule with no exceptions — so a trait-level change is a twelve-crate change that every tool shows as five |
| AT20 | `ring_cursor` | **wrong doc** | The re-export's own argument says it saves "six manifests" where the count is now eight, and `ring_mpsc` is the only crate in the family whose source acknowledges the trait has two import paths |
