# API: Seven Items and the One With a Caller

### Scope

- **Purpose**: Present the crate's entire public surface in one table, and record which parts of it are reached from anywhere other than this crate's own tests.
- **Responsibility**: Give every item's signature, its tier, its doctest, and its caller count — each backed by the command that produces it.
- **In Scope**: The seven public items and their reachability.
- **Out of Scope**: What each item does internally — see [`item/001`](../item/001_the_four_arms_of_the_pause.md) and [`item/002`](../item/002_the_loop_the_wrapper_and_the_two_questions.md).

### The Whole Surface

```sh
cd "$(git rev-parse --show-toplevel)"
grep -E '^\s*pub (const |fn |const fn )' ring_wait/src/lib.rs
```

Live output:

```
pub const DEFAULT_SPINS : usize = 1024;
pub const fn escalation_hint( kind : WaitKind ) -> Option< WaitKind >
pub fn pause( kind : WaitKind, attempt : usize ) -> bool
pub fn wait_until< F >( kind : WaitKind, spins : usize, mut ready : F ) -> Result< usize, RingError >
pub fn wait< F >( kind : WaitKind, ready : F ) -> Result< usize, RingError >
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
```

| Line | Item | Signature | Tier |
|------|------|-----------|------|
| `:63` | `DEFAULT_SPINS` | `pub const DEFAULT_SPINS : usize = 1024` | **constant** |
| `:83` | `escalation_hint` | `( WaitKind ) -> Option< WaitKind >` | **advice** |
| `:112` | `pause` | `( WaitKind, usize ) -> bool` | **strategy** |
| `:179` | `wait_until` | `( WaitKind, usize, F ) -> Result< usize, RingError >` | **loop** |
| `:210` | `wait` | `( WaitKind, F ) -> Result< usize, RingError >` | loop, default budget |
| `:239` | `for_space` | `( &CursorPair, WaitKind, usize ) -> Result< usize, RingError >` | **question** |
| `:265` | `for_data` | `( &CursorPair, u64, WaitKind, usize ) -> Result< usize, RingError >` | question |

Six functions and one constant. Every one carries a doctest — 14 fence lines,
7 blocks, one per item — which is the crate's only mechanism for asserting the
constant, since `DEFAULT_SPINS` is not a function and cannot have a test of its
own without one.

### WT1 — One Item of Seven Is Reached From Outside

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r --include=*.rs "ring_wait::" . \
  | grep -vE ':\s*(///|//!|//)' | grep -v '^ring_wait/'
```

Live output:

```
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
```

Three lines of output, all of them `wait_until`:

| Site | Caller | Predicate |
|------|--------|-----------|
| `ring_barrier/src/lib.rs:285` | `Barrier::wait_for` | `\|\| self.admits( from, count )` |
| `ring_shutdown/src/lib.rs:578` | `wait_for_close` | `\|\| shutdown.is_closed()` |
| `ring_shutdown/src/lib.rs:613` | `for_space_or_close` | a two-condition closure |

The other six items — `DEFAULT_SPINS`, `escalation_hint`, `pause`, `wait`,
`for_space`, `for_data` — have **zero** callers in any `src/` in the family.
Their only callers are `ring_wait/tests/wait_test.rs` and their own
doctests. Counting code lines that mention each name, excluding the single
import at `tests/wait_test.rs:47` that names all seven:

| Item | `src/` callers | `wait_test.rs` lines |
|------|---------------:|---------------------:|
| `wait_until` | **3** | 8 |
| `DEFAULT_SPINS` | 0 | 7 |
| `for_data` | 0 | 7 |
| `pause` | 0 | 6 |
| `escalation_hint` | 0 | 5 |
| `for_space` | 0 | 5 |
| `wait` | 0 | 1 |

The shape is the same one `ring_barrier`'s corpus records as BR20 from the other
side: the generic bottom of the ladder is used, and every convenience built on
top of it is not. Here it is starker, because the convenience is 4 of the 7
items and 5 of the crate's 6 functions.

### WT2 — Nobody Writes `use ring_wait`

```sh
cd "$(git rev-parse --show-toplevel)"
grep -r --include=*.rs "use ring_wait" . | grep -v '^ring_wait/' \
  || echo '(nobody imports the name)'
# control: the identical expression for a sibling that is imported by name
grep -rc --include=*.rs "use ring_cursor" . | grep -v ':0$' | head -3
```

Live output:

```
(nobody imports the name)
ring_mpsc/src/lib.rs:1
ring_claim/tests/claim_test.rs:1
ring_claim/src/lib.rs:4
```

No match, against a control — the identical expression for `ring_cursor` —
that returns three importing files. All three call sites spell the path in full —
`ring_wait::wait_until( … )` — rather than importing the name.

That is not a style preference; it is what a single-call dependency looks like.
An import earns its keep at the second use, and no crate has a second use. The
consequence is that `ring_wait` appears in exactly two kinds of place in the
family's source: a `Cargo.toml` (two crates) and a fully-qualified call (three
sites). It never appears in a `use` list, which is where a reader normally
learns what a module depends on.

### WT18 — Every `wait(` Call in the Family, None of Them This Crate's

`wait` is the crate's least-used item and its name is also the family's most
overloaded:

```sh
cd "$(git rev-parse --show-toplevel)"
# scoped to the family, because the claim is about the family's vocabulary:
# unrelated crates elsewhere in the workspace spell `wait(` for process exit,
# which is a different word that happens to be the same word
grep -r --include=*.rs -F "wait(" ring_*/ \
  | grep -vE ':\s*(///|//!|//)' | grep -v '^ring_wait/'
```

Live output:

```
ring_config/tests/config_test.rs:    .with_wait( WaitKind::Park )
ring_config/tests/config_test.rs:  assert_eq!( cfg.wait(), WaitKind::Park );
ring_config/tests/config_test.rs:  assert_eq!( cfg.wait(), WaitKind::Spin );
ring_config/tests/config_test.rs:  let waited = base.with_wait( WaitKind::Yield );
ring_config/tests/config_test.rs:  assert_eq!( waited.wait(), WaitKind::Yield );
ring_config/tests/config_test.rs:  assert_eq!( overflowed.wait(), base.wait() );
ring_config/tests/config_test.rs:    .with_wait( WaitKind::None )
ring_config/tests/config_test.rs:    .with_wait( WaitKind::None );
ring_config/tests/config_test.rs:      Step::Wait => cfg.with_wait( WaitKind::Park ),
ring_config/tests/config_test.rs:    .with_wait( WaitKind::Park )
ring_config/tests/config_test.rs:      cfg.with_wait( kind ).is_tick_safe(),
ring_config/tests/config_test.rs:  assert!( cfg.with_wait( WaitKind::None ).is_tick_safe() );
ring_config/tests/config_test.rs:  assert!( !cfg.with_wait( WaitKind::Spin ).is_tick_safe() );
ring_config/src/lib.rs:  pub const fn with_wait( mut self, wait : WaitKind ) -> Self
ring_config/src/lib.rs:  pub const fn wait( &self ) -> WaitKind
ring_factory/tests/factory_test.rs:    observable_profile( base.with_wait( WaitKind::Spin ) ),
ring_factory/tests/factory_test.rs:    observable_profile( base.with_wait( WaitKind::Park ) ),
ring_spsc/tests/spsc_test.rs:      .with_wait( WaitKind::Spin )
ring_stats/tests/stats_test.rs:  stats.record_wait( 700 );
ring_stats/tests/stats_test.rs:  stats.record_wait( 0 );
ring_stats/tests/stats_test.rs:  stats.record_wait( 6 );
ring_stats/tests/stats_test.rs:  stats.record_wait( 77 );
ring_stats/tests/stats_test.rs:        stats.record_wait( FILL );
ring_stats/src/lib.rs:  pub fn record_wait( &self, nanos : u64 )
```

Twenty-four hits across six files — twenty when this was written, then
twenty-two once `ring_stats` grew two more tests that record a wait, then
twenty-four once `ring_config` grew two more that set one — and **not one of
them is `ring_wait::wait`**:

| Hits | File | What it actually is |
|-----:|------|---------------------|
| 13 | `ring_config/tests/config_test.rs` | `RingConfig::with_wait` / `RingConfig::wait` |
| 5 | `ring_stats/tests/stats_test.rs` | `RingStats::record_wait` |
| 2 | `ring_config/src/lib.rs` | `RingConfig::with_wait` at `:89`, `RingConfig::wait` at `:167` |
| 2 | `ring_factory/tests/factory_test.rs` | `RingConfig::with_wait` |
| 1 | `ring_spsc/tests/spsc_test.rs` | `RingConfig::with_wait` |
| 1 | `ring_stats/src/lib.rs` | `RingStats::record_wait`'s own declaration |

`ring_wait::wait`'s only caller anywhere is `tests/wait_test.rs:243`. A
reachability question asked with a bare `grep wait(` therefore returns every one
of them as a false positive and misses the one true answer, which is why every census in
this corpus greps the qualified path `ring_wait::` instead.

### The Three Tiers, and Where They End

| Tier | Item | Question it answers | Reached |
|------|------|--------------------|:-------:|
| Configuration | `DEFAULT_SPINS` | how many looks, absent a decision | no |
| Strategy | `pause` | what happens between two looks | no |
| Loop | `wait_until` | ask until ready or give up | **yes** |
| Loop, defaulted | `wait` | the same, at the default budget | no |
| Question | `for_space`, `for_data` | ask the two ring questions by name | no |
| Advice | `escalation_hint` | what to try when this strategy is wrong | no |

The tier that is reached is the one that fixes nothing. Everything above and
below it supplies a decision the caller turned out to want to make itself —
the budget, the predicate, or the strategy.


### WT26 — Seven Items, Seven Examples, One Caller

Every public item carries exactly one executable example. The distribution is
perfectly flat across a surface WT1 measures as anything but.

```sh
cd "$(git rev-parse --show-toplevel)"
# doctest blocks per public item, attributed to the item each precedes
awk '/^pub (const )?fn / { name = $0; sub( /^pub (const )?fn /, "", name );
       sub( /[(<].*/, "", name ); printf "%-16s %d\n", name, n / 2; n = 0; next }
     /^pub const [A-Z]/ { printf "%-16s %d\n", "DEFAULT_SPINS", n / 2; n = 0; next }
     /^\/\/\/ ```/ { n++ }' ring_wait/src/lib.rs
# what the crate's lints actually require
grep '#!\[ *\(deny\|forbid\|warn\)' ring_wait/src/lib.rs
```

Live output:

```
DEFAULT_SPINS    1
escalation_hint  1
pause            1
wait_until       1
wait             1
for_space        1
for_data         1
#![ deny( missing_docs ) ]
```

One each, for all seven. The enabled lint is `deny( missing_docs )`, which
requires a doc comment and says nothing about examples — so the uniformity is
convention, not enforcement, and it held across every item anyway.

That is a real property and it is worth naming, because it means documentation
effort here is allocated by *position on the surface* rather than by reach. Six
of the seven examples demonstrate items no other crate calls; `escalation_hint`'s
shows a ladder nothing climbs, `for_space`'s shows a composition nobody has
written (WT32). The one item three external call sites use gets the same single
example as the rest, and that example passes a counter closure — nothing like the
cursor-reading predicates the real callers pass (WT33).

The upside is that the flat rule needs no judgement to apply and no maintenance
when reach changes. The cost is that a reader using example density to find the
load-bearing part of the surface learns nothing at all from it.

### APIs

| File | Relationship |
|------|--------------|
| [002_the_predicate_is_the_parameter.md](002_the_predicate_is_the_parameter.md) | The closure that makes `wait_until` the reachable one |

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_one_loop_and_the_two_ways_out.md](../algorithm/001_one_loop_and_the_two_ways_out.md) | The body behind the one reached signature |
| [../algorithm/002_two_wrappers_over_a_predicate_they_fix.md](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md) | WT3 — why the questions tier has no caller |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | The two crates the three call sites live in |

### Items

| File | Relationship |
|------|--------------|
| [../item/001_the_four_arms_of_the_pause.md](../item/001_the_four_arms_of_the_pause.md) | `pause` and `escalation_hint`, in detail |
| [../item/002_the_loop_the_wrapper_and_the_two_questions.md](../item/002_the_loop_the_wrapper_and_the_two_questions.md) | The four `Result`-returning functions |

### Lifecycles

| File | Relationship |
|------|--------------|
| [../lifecycle/002_the_escalation_ladder_nobody_climbs.md](../lifecycle/002_the_escalation_ladder_nobody_climbs.md) | The advice tier, and what would call it |

### Types

| File | Relationship |
|------|--------------|
| [../type/002_one_return_type_and_the_one_must_use.md](../type/002_one_return_type_and_the_one_must_use.md) | WT8 — where the crate's single `#[ must_use ]` went |

### Sources

| File | Relationship |
|------|--------------|
| `ring_barrier/src/lib.rs:285` | The first of the three call sites |
| `ring_shutdown/src/lib.rs:578,613` | The other two |
| `ring_config/src/lib.rs:89,167` | `RingConfig::with_wait` and `RingConfig::wait` — the name collision |
| `ring_stats/src/lib.rs:312` | `Stats::record_wait` — the other half of it |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:44-47` | The import list — the crate's whole surface in one `use` |
| `tests/wait_test.rs:239-251` | The only call to `wait` in the family |
