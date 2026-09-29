# workaround

External constraints `ring_cursor` absorbs on behalf of its consumers, each with the
cost it imposes and the condition under which it can be deleted.

### Overview Table

| ID | Name | Constraint | Deletable when |
|----|------|------------|----------------|
| 001 | [The `loom` Constructor Cannot Be `const`](001_the_loom_constructor_cannot_be_const.md) | `const`ness is part of a signature and cannot be `cfg`-conditional | Rust gains conditional `const`, or loom's atomics become `const`-constructible |
| 002 | [The Trait Must Travel With the Type](002_the_trait_must_travel_with_the_type.md) | A trait method is callable only where the trait is in scope | Never, without giving `PaddedCursor` inherent methods — and `ring_batch`'s generic caller needs the trait anyway |

### Both Are Language Constraints, Not Library Ones

Neither workaround exists because a dependency is awkward. Each is the minimum
construction Rust permits for a thing the crate has to do:

| | 001 | 002 |
|---|---|---|
| Rule | `const fn` is a signature property | Trait methods need the trait in scope |
| Cost | Two copies of two constructor bodies | A second public path to one trait |
| Caught by the compiler if broken? | **No** — the `cfg( loom )` copy is not built by default | Yes, immediately |

**001's row is the one to watch.** Its duplicated bodies compile only under
`RUSTFLAGS="--cfg loom"`, which no verification level runs, so a divergence
between the two copies lands and stays until someone next runs a loom model.

The third language rule this crate absorbs — the orphan rule — is not filed here,
because its consequence is a design shape rather than a concession:
[`pattern/001`](../pattern/001_the_forwarding_newtype.md).

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- constructors, where one apiece would do --'
command grep -cE '^  pub (const )?fn new' ring_cursor/src/lib.rs
echo '  -- the trait that must travel with the type --'
command grep '^pub use ring_atomic::SeqCell' ring_cursor/src/lib.rs
echo '  -- who took the re-export, in library code --'
command grep -r '^use ring_cursor::' --include=*.rs */src/ | command grep 'SeqCell' | sed 's|ring/||;s|/src/lib.rs||'
echo '  -- control: who took it directly from ring_atomic instead --'
command grep -r '^use ring_atomic::' --include=*.rs */src/ | command grep 'SeqCell' | sed 's|ring/||;s|/src/lib.rs||'
```

Live output:

```
  -- constructors, where one apiece would do --
4
  -- the trait that must travel with the type --
pub use ring_atomic::SeqCell;
  -- who took the re-export, in library code --
ring_claim:use ring_cursor::{ PaddedCursor, SeqCell, GATING };
ring_consume:use ring_cursor::{ PaddedCursor, SeqCell, GATING };
ring_publish:use ring_cursor::{ PaddedCursor, SeqCell, GATING };
ring_spsc:use ring_cursor::{ CursorPair, SeqCell, GATING };
  -- control: who took it directly from ring_atomic instead --
ring_batch:use ring_atomic::SeqCell;
ring_debug:use ring_atomic::SeqCell;
ring_mpsc:use ring_atomic::{ AtomicSeq, SeqCell };
ring_tls:use ring_atomic::SeqCell;
```

**The control is the arm that makes the count mean something.** Crates import
the same trait straight from `ring_atomic`, so the re-export is not the family's
only route to it, and what it saves is the first list rather than the six
manifests the doc comment claims (→ [`002`](002_the_trait_must_travel_with_the_type.md) CU51).

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CU49 | The `loom` constructors | n/a — unenforced | Four constructor bodies exist so that two can be `const` in ordinary builds. Nothing compiles the `loom` half except a manual `RUSTFLAGS="--cfg loom"` invocation that appears in no verification level, so the workaround's other arm is unbuilt on every run that matters |
| CU50 | The deletion condition | n/a — unenforced | The workaround goes when `loom`'s cell constructor becomes `const`. That is decidable by compiling the `loom` build and reading one error, and nothing runs it — so the day the constraint lifts, the crate keeps four constructors and nobody is told |
| CU51 | The re-export | **measured cost** | It saves exactly four manifests a `ring_atomic` edge — `ring_claim`, `ring_consume`, `ring_publish`, `ring_spsc`. The doc comment claimed six. Four is still worth the line, and the difference between the two numbers was the difference between a measurement and an estimate written once and never re-run. **Disposition: applied** — doc comment now says "four manifests", matching CU5's fix |
| CU52 | The direct-import exception | n/a — observation | `src/lib.rs:60-67` names "independent business with the atomic layer" as the legitimate reason to skip the re-export. Four crates do — `ring_batch`, `ring_debug`, `ring_mpsc`, `ring_tls` — and two of them do not depend on this crate at all, so the exception as written covers cases it was not aimed at |
