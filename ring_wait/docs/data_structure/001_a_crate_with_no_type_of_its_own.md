# Data Structure: A Crate With No Type of Its Own

### Scope

- **Purpose**: Record that this crate declares no `struct`, no `enum`, and no `trait`, place it among the two others in the family that also declare none, and say what distinguishes it from them.
- **Responsibility**: Give the census, name the three type-free crates, and account for the state this crate holds between two looks — which is none, and not for the reason the other two hold none.
- **In Scope**: What `ring_wait` declares and what it borrows.
- **Out of Scope**: The two integers that flow through its signatures — see [`data_structure/002`](002_the_budget_and_the_attempt_index.md).

### The Census

```sh
cd "$(git rev-parse --show-toplevel)"
for d in ring_*/; do
  code=$( cat "$d"src/*.rs | grep -vE '^\s*(///|//!|//)' )
  s=$( printf '%s' "$code" | grep -cE '^\s*pub struct ' )
  e=$( printf '%s' "$code" | grep -cE '^\s*pub enum ' )
  t=$( printf '%s' "$code" | grep -cE '^\s*pub trait ' )
  [ "$s$e$t" = "000" ] && echo "$( basename "$d" )"
done
```

Live output:

```
ring_index
ring_seqno
ring_wait
```

Three of the family's 33 crates print:

| Crate | Public functions | What they compute |
|-------|-----------------:|-------------------|
| `ring_index` | 3 | a slot index from a sequence and a capacity |
| `ring_seqno` | 5 | laps, free slots, pending, and the slowest of a slice |
| `ring_wait` | 6 | a pause, a loop, and what to escalate to |

Everything else in the family declares at least one type. `ring_types` declares
six; `ring_mpsc` declares six structs.

### WT14 — Two of the Three Are Pure, and This One Is Not

The distinction is not visible in the census and is the whole point of it.

| | `ring_index` | `ring_seqno` | `ring_wait` |
|--|-------------|-----------|-------------|
| Reads memory it was not handed | no | no | **yes** — through the caller's closure |
| Touches the scheduler | no | no | **yes** — `yield_now`, `sleep` |
| Reads a clock | no | no | **yes** — `sleep` waits on one |
| Same inputs, same output | always | always | **no** |
| Declared `const fn` | 0 of 3 | 0 of 5 | 1 of 6 |

`ring_index` and `ring_seqno` declare no type because arithmetic needs none. This
crate declares no type because **the state it operates on belongs to somebody
else** — the cursors are `ring_cursor`'s, the strategy discriminants are
`ring_types`', and the predicate is the caller's. What is left is control flow,
and control flow is not a data structure.

The last row is the one that reads backwards until the reason is stated.
`ring_index` and `ring_seqno` declare nothing `const fn` **by choice** — their
bodies are arithmetic and could be. This crate declares one, and the other five
are barred: `core::hint::spin_loop`, `std::thread::yield_now`,
`std::thread::sleep`, and every atomic load reachable through a caller's
predicate are all non-`const`. So the one `const fn` here is not a smaller
version of the other two crates' purity — it is the only function in the crate
that *could* have been one.

### What It Holds Between Two Looks

Nothing, and that is a design commitment rather than an accident of size:

| Candidate state | Where it lives instead |
|-----------------|------------------------|
| Which cursor is being watched | the caller's closure |
| How many attempts have happened | a `for` loop's induction variable, discarded at return |
| Which strategy is running | `kind`, passed by value — `WaitKind` is `Copy` and one byte |
| Whether to escalate | nowhere; `escalation_hint` is advice a caller may ignore |
| Who to wake | nowhere at all — see [`workaround/001`](../workaround/001_a_sleep_where_a_park_belongs.md) |

The last row is the one with consequences. A crate that held a registration —
*this thread is waiting on that ring* — would need a type, and would need
`ring_handle`'s ownership relationships. It holds none, which is precisely why
`Park` sleeps rather than parks: there is no registry to unpark from, because
there is no type to keep one in.

### The Types It Borrows

```rust
// ring_wait/src/lib.rs:50-51
use ring_cursor::CursorPair;
use ring_types::{ RingError, WaitKind };
```

Three imported types, two crates, and each appears in a signature:

| Type | From | Appears in | Role |
|------|------|-----------|------|
| `WaitKind` | `ring_types` | all 6 functions | the strategy, by value |
| `RingError` | `ring_types` | 4 return types | the give-up value |
| `CursorPair` | `ring_cursor` | `for_space`, `for_data` | by shared reference |

`CursorPair` is the one that could go. It is named by exactly two functions,
neither of which has a production caller
([`api/001`](../api/001_seven_items_and_the_one_with_a_caller.md) § WT1), so the
dependency it justifies rests entirely on two convenience wrappers. W6 checks
that the dependency is used at all, and its own note records that before those
two functions existed it was declared and unused.

### `#![ deny( missing_docs ) ]` on Seven Items

```rust
// ring_wait/src/lib.rs:48
#![ deny( missing_docs ) ]
```

Deny rather than the workspace's `warn` (`Cargo.toml:220`). With no type, the
lint has nothing to reach except the seven items and the module itself, so the
whole of what it can enforce is the 179 doc-comment lines that make up two
thirds of the file. In a crate with a public struct the same lint also reaches
fields and variants; here it reaches functions, and that is all there is.


### WT27 — A Manifest Edge Held Up Entirely by Unreached Code

`ring_cursor` is one of this crate's two dependencies. Everything that uses it is
unreachable from outside the crate.

```sh
cd "$(git rev-parse --show-toplevel)"
# what the ring_cursor dependency is for
grep 'CursorPair' ring_wait/src/lib.rs
# and who calls the two functions that take one
grep -r 'for_space(\|for_data(' --include=*.rs . --exclude-dir=docs \
  | grep -v '^ring_wait/' \
  || echo '(no call site outside ring_wait)'
# control: the identical expression for the item that does have callers
grep -r 'wait_until(' --include=*.rs . --exclude-dir=docs | grep -v '^ring_wait/'
```

Live output:

```
use ring_cursor::CursorPair;
/// use ring_cursor::CursorPair;
/// let pair = CursorPair::new( Capacity::new( 4 ).unwrap() );
pub fn for_space( pair : &CursorPair, kind : WaitKind, spins : usize ) -> Result< usize, RingError >
/// use ring_cursor::CursorPair;
/// let pair = CursorPair::new( Capacity::new( 8 ).unwrap() );
pub fn for_data( pair : &CursorPair, count : u64, kind : WaitKind, spins : usize )
(no call site outside ring_wait)
ring_shutdown/src/lib.rs:  ring_wait::wait_until( kind, spins, || shutdown.is_closed() )
ring_shutdown/src/lib.rs:  let outcome = ring_wait::wait_until( kind, spins, ||
ring_barrier/src/lib.rs:    ring_wait::wait_until( kind, spins, || self.admits( from, count ) )?;
```

`CursorPair` appears in the `use` line and in the two signatures. `for_space` and
`for_data` are the only functions that take one, and the census finds no call to
either outside this crate — while the control, the same expression run for
`wait_until`, returns three. The empty result is a measured absence rather than a
pattern that silently matched nothing. `ring_shutdown::for_space_or_close` is a
same-named rewrite over `wait_until`, not a call (WT3).

The crate's own manual plan has a check for this shape and it passes:
`tests/manual/readme.md § W6` asks whether every declared dependency is actually
used, and `ring_cursor` is used — by code that compiles, is tested, and is
reached by nobody. "Used" and "reached" are different questions, and W6 asks the
one a manifest can answer.

Nothing here is broken. The two helpers are the crate's stated reason to exist
([`algorithm/002`](../algorithm/002_two_wrappers_over_a_predicate_they_fix.md)),
and a dependency held up by an unadopted API is what an unadopted API looks like
from the manifest side.


### WT28 — The Vocabulary Travels Four Times Further Than the Code

This crate declares no type. The enum it exists to interpret is declared in
`ring_types`, and the two travel through the family at very different rates.

```sh
cd "$(git rev-parse --show-toplevel)"
# crates whose source names the enum
grep -rl 'WaitKind' --include=*.rs */src/ | sed 's#ring/##;s#/src.*##' | sort -u
echo '--- crates whose manifest takes the dependency ---'
grep -rl 'ring_wait' --include=Cargo.toml . | command grep -v '^ring/Cargo.toml$' | sed 's#ring/##;s#/Cargo.toml##' | sort
```

Live output:

```
ring_barrier
ring_claim
ring_config
ring_factory
ring_publish
ring_shutdown
ring_types
ring_wait
--- crates whose manifest takes the dependency ---
ring_barrier
ring_shutdown
ring_wait
```

Eight crates name `WaitKind` in their own source. Three manifests name
`ring_wait`, and one of those three is `ring_wait`'s own `name =` field (WT11),
leaving two real dependents.

So the type is a family-wide vocabulary and the crate implementing it is a
two-consumer leaf. That gap is the design working: `WaitKind` lives in
`ring_types` precisely so a crate can accept a strategy as configuration without
taking a dependency on the crate that performs it — a ruling recorded in
[`decisions/002`](../decisions/002_the_discriminants_live_in_ring_types.md). A
crate storing a `WaitKind` in a config struct never needs `pause`.

It is worth measuring because the same gap is what makes the parking ban
enforceable at all. If naming the strategy required depending on the
implementation, `ring_poll::PARKING_CRATES` would have to list most of the family
and would stop meaning anything.

### Data Structures

| File | Relationship |
|------|--------------|
| [002_the_budget_and_the_attempt_index.md](002_the_budget_and_the_attempt_index.md) | The two integers that stand in for the state |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_seven_items_and_the_one_with_a_caller.md](../api/001_seven_items_and_the_one_with_a_caller.md) | The seven items the lint reaches |
| [../api/002_the_predicate_is_the_parameter.md](../api/002_the_predicate_is_the_parameter.md) | Where the state actually lives — the caller's closure |

### Decisions

| File | Relationship |
|------|--------------|
| [../decisions/002_the_discriminants_live_in_ring_types.md](../decisions/002_the_discriminants_live_in_ring_types.md) | Why `WaitKind` is not declared here |

### Integrations

| File | Relationship |
|------|--------------|
| [../integration/001_two_dependencies_two_dependents_and_a_roster.md](../integration/001_two_dependencies_two_dependents_and_a_roster.md) | The two crates these three types come from |

### Types

| File | Relationship |
|------|--------------|
| [../type/001_a_usize_budget_and_a_u64_count.md](../type/001_a_usize_budget_and_a_u64_count.md) | The widths of the borrowed types' fields |
| [../type/002_one_return_type_and_the_one_must_use.md](../type/002_one_return_type_and_the_one_must_use.md) | `RingError`, and which two of its nine variants appear |

### Workarounds

| File | Relationship |
|------|--------------|
| [../workaround/001_a_sleep_where_a_park_belongs.md](../workaround/001_a_sleep_where_a_park_belongs.md) | The registration this crate would need a type to hold |

### Sources

| File | Relationship |
|------|--------------|
| `ring_types/src/policy.rs:21-34` | `WaitKind`, declared elsewhere |
| `ring_cursor/src/lib.rs:247-252` | `CursorPair`, the one borrowed structure |
| `ring_index/src/lib.rs` | The family's other type-free crate, and a pure one |
| `ring_seqno/src/lib.rs` | The third, also pure |
| `Cargo.toml:220` | The workspace's `missing_docs = "warn"`, which this crate raises to deny |

### Tests

| File | Relationship |
|------|--------------|
| `tests/wait_test.rs:49-52` | The only helper in the suite — a `Capacity`, not a type of this crate's |
| `tests/manual/readme.md` § W6 | Every declared dependency is actually used |
