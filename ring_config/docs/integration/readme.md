# integration

Eleven crates declare `ring_config` and four build on it. The split between
`[dependencies]` and `[dev-dependencies]` is exactly right in all eleven
manifests, and it separates the two groups cleanly: the four with a real
dependency are the four whose `src/` carries a `RingConfig` in a signature, and
the seven with a dev dependency construct one only to have a ring to test against.

Two crates sit outside that accounting. `ring_bench` uses the type in three
signature positions and declares no edge here at all, reaching it through
`ring_factory`'s `pub use`. `ring_wait` names it in prose while depending only on
`ring_types` and `ring_cursor`.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_eleven_declarers_and_four_that_build_on_it.md) | Eleven Declarers, and Four That Build On It | The manifest census, the four/seven split, and the two crates outside it |
| [002](002_a_re_export_that_carries_the_record_but_not_its_vocabulary.md) | A Re-Export That Carries the Record but Not Its Vocabulary | `ring_factory`'s `pub use`, the undeclared edge, and the three types that do not travel with it |

## A Fan-Out Three Times Smaller Than It Looks

Counting manifests, `ring_config` has eleven dependents. Counting signatures, it
has four: `ring_core`, `ring_factory`, `ring_mpsc` and `ring_spsc`. The remaining
seven — `ring_debug`, `ring_flush`, `ring_handle`, `ring_poll`, `ring_registry`,
`ring_shutdown`, `ring_testkit` — construct a configuration, hand it to something
that builds a ring, and never name the type in their own surface.

That every one of those seven declares it under `[dev-dependencies]` is worth
stating as a pass rather than assumed: `ring_atomic` declares `ring_seqno` and
`ring_align` declares `ring_types`, and in both cases the only mention in the
crate is the module comment's own "Depends on" line — a prose claim with no `use`
behind it, in either `src/` or `tests/`.

## The Edge That Is Not in Any Manifest

`ring_factory` re-exports two names, `RingConfig` and `Registry`. `ring_bench`
takes the first and declares nothing, so the coupling is real, the build resolves,
and the dependency graph does not show it. A reader tracing the type from
`ring_bench` meets `ring_factory` first, and `ring_factory` re-exports it in one
line that does not say where it came from.

The re-export also stops short of the record's own vocabulary. `Capacity`,
`WaitKind` and `OverflowPolicy` are not re-exported, so `with_wait` and
`with_overflow` cannot be called through the factory's surface alone —
demonstrated by `ring_factory`'s own doctest, which imports `RingConfig` from
itself and `OverflowPolicy` from `ring_types` on consecutive lines in order to
call one setter.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- every crate declaring ring_config, and the section it declares it in --'
for c in ring_*/Cargo.toml; do awk -v n="$( basename "$( dirname "$c" )" )" '/^\[/ { s = $0 } /^ring_config/ { printf "  %-13s %s\n", n, s }' "$c"; done
echo '  -- every crate naming RingConfig in a src/ signature or field --'
command grep -rn ': *&\?RingConfig' --include=*.rs */src | command grep -v '//\|use ring_config' | sed -E 's#^(/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/module/division/[^/]+|/home/user1/pro/lib/yrd_gamedev/codename_space_sandbox/substrate/[^/]+|module|ring|spike)/([a-z_]*)/.*#  \2#' | sort -u
echo '  -- everything ring_factory re-exports --'
command grep -n 'pub use' ring_factory/src/lib.rs
echo '  -- and the two family manifests declaring an edge with no use behind it --'
command grep -n 'ring_seqno' ring_atomic/src/lib.rs
command grep -n 'ring_types' ring_align/src/lib.rs
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| RC17 | `ring_config` | n/a — observation | Four crates declare `ring_config` under `[dependencies]` (`ring_core`, `ring_factory`, `ring_mpsc`, `ring_spsc`) and seven under `[dev-dependencies]`, and the split matches usage exactly — the same four are the only ones carrying the type in a `src/` signature — which is a hygiene pass worth recording against a family where `ring_atomic` declares `ring_seqno` and `ring_align` declares `ring_types` with no `use` behind either, in `src/` or `tests/` |
| RC18 | `ring_config` | n/a — observation | Eleven declaring manifests overstate the coupling by roughly three times: only four crates accept or store a `RingConfig` in production code, the other seven construct one and pass it through on the way to a ring they need for a test, and two more blur the count in opposite directions — `ring_bench` uses the type in three signature positions with no declared edge, and `ring_wait` names it in its module comment while depending only on `ring_types` and `ring_cursor` |
| RC19 | `ring_bench` | n/a — doc gap | `ring_bench` imports `RingConfig` from `ring_factory`'s `pub use` and declares no `ring_config` edge, so a breaking change here breaks it with nothing in either manifest to explain why, and its module comment and doctests teach `use ring_factory::RingConfig` — making a tier-11 crate the type's apparent home while `ring_factory`'s one-line re-export says nothing about where it came from |
| RC20 | `ring_factory` | n/a — observation | `ring_factory` re-exports `RingConfig` and `Registry` and not `Capacity`, `WaitKind` or `OverflowPolicy`, so `with_wait` and `with_overflow` cannot be called through its surface alone — its own doctest at `:134-137` imports `RingConfig` from itself and `OverflowPolicy` from `ring_types` on consecutive lines to call one setter, and `ring_bench` needed `ring_types` anyway for an unrelated reason, so nothing currently pays for the gap |
