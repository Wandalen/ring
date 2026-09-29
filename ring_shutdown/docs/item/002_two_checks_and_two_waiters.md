# Item: Two Checks and Two Waiters

### Scope

- **Purpose**: Take four of the crate's twenty-three items in detail — the pair that spells one predicate twice, and the pair that reports one event as two different errors — and record what holds them together, which in both cases is nothing.
- **Responsibility**: `Shutdown::admit` against `Guarded`'s three consulting sites; `wait_for_close` against `for_space_or_close`, and both against the one `ring_wait` call they share.
- **In Scope**: The four items' bodies, their doc comments' claims about each other, and the tests that reach them.
- **Out of Scope**: The full inventory and its attributes (→ [`001`](001_the_declared_surface_and_its_attributes.md)); why the crate calls `ring_wait` rather than re-exporting it (→ [`../integration/001`](../integration/001_family_dependency_seam.md)).

### The Two Checks

`Shutdown::admit` and `Guarded::try_push` answer the same question — *may a
publish proceed?* — and `admit`'s doc comment says so:

> The check [`Guarded`] performs, exposed for a caller who holds a raw producer
> and wants to perform it explicitly.

| | `admit` | `Guarded`'s sites |
|---|---|---|
| Reads | `self.is_closed()` | `self.shutdown.is_closed()` |
| Returns | `Result< (), RingError >` | `Result< (), Refusal< T > >`, `usize`, `bool` |
| Called by | nothing in the crate | — |

The relationship the doc asserts is *the same check*. What exists is two
readings of one flag, written out separately, in different types.

### The Two Waiters

`wait_for_close` and `for_space_or_close` both call `ring_wait::wait_until`,
whose sole error is `RingError::Empty`. They disagree about what to do with it.

| | `wait_for_close` | `for_space_or_close` |
|---|---|---|
| Budget exhausted | `Err( RingError::Empty )` | `Err( RingError::Full )` |
| Treatment | forwarded unchanged | rewritten |
| Doc's justification | "`ring_wait`'s own budget-exhausted error, unchanged" | "matching `ring_wait::for_space`" |

Both justifications are accurate. `ring_wait::for_space` does map the same
outcome to `Full` — `for_space_or_close` reproduces that mapping exactly. The
two items are individually correct and collectively contradictory: one crate,
one underlying call, one physical event, two errors, and no rule stating when a
wrapper forwards its callee's error and when it translates it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
printf 'admit declared:                   %s\n' "$( command grep -c 'pub fn admit' ring_shutdown/src/lib.rs )"
printf 'admit call sites in the crate:    %s\n' "$( command grep -vE '^ *///' ring_shutdown/src/lib.rs | command grep -c '\.admit(' || true )"
printf 'guard sites reading the flag:     %s\n' "$( command grep -vE '^ *///' ring_shutdown/src/lib.rs | command grep -c 'self\.shutdown\.is_closed()' || true )"
printf 'tests naming both admit and push: %s\n' "$( awk '/^#\[ test \]/{ b=""; f=1 } f{ b = b "\n" $0 } f && /^\}$/{ if ( b ~ /admit/ && b ~ /try_push/ ) c++ ; f=0 } END{ print c+0 }' ring_shutdown/tests/shutdown_test.rs )"
printf 'wait_until returns on exhaustion: %s\n' "$( awk '/pub fn wait_until/{f=1} f&&/^  Err\(/{ gsub( /^ *Err\( | \)$/, "" ); print; exit }' ring_wait/src/lib.rs )"
printf 'wait_for_close remaps it:         %s\n' "$( awk '/pub fn wait_for_close/{f=1} f&&/^\}$/{exit} f' ring_shutdown/src/lib.rs | command grep -c 'map_err\|RingError::' || true )"
printf 'for_space_or_close remaps it to:  %s\n' "$( awk '/pub fn for_space_or_close/{f=1} f&&/^\}$/{exit} f&&/Err\( _ \) =>/{ sub( /.*=> *Err\( /, "" ); sub( / \).*/, "" ); print }' ring_shutdown/src/lib.rs )"
printf 'ring_wait::for_space maps it to:  %s\n' "$( awk '/pub fn for_space\(/{f=1} f&&/^\}$/{exit} f&&/map_err/{ sub( /.*RingError::/, "RingError::" ); sub( / *\).*/, "" ); print }' ring_wait/src/lib.rs )"
printf 'for_space exists in ring_wait:    %s\n' "$( command grep -c 'pub fn for_space(' ring_wait/src/lib.rs )"
printf 'ring_wait fns this crate calls:   %s\n' "$( command grep -vE '^ *///' ring_shutdown/src/lib.rs | command grep -ohE 'ring_wait::[a-z_]+' | sort -u | tr '\n' ' ' )"
printf 'ring types in wait_for_close arg: %s\n' "$( awk '/pub fn wait_for_close/{f=1} f&&/^-> /{exit} f' ring_shutdown/src/lib.rs | command grep -c 'Ring\|Consumer\|Producer\|CursorPair' || true )"
printf 'errors the family calls transient: %s\n' "$( awk '/pub const fn is_transient/{f=1} f&&/=> true,/{ sub( /^ */, "" ); sub( / *=> true,.*/, "" ); print; exit }' ring_types/src/error.rs )"
printf 'what Empty means in ring_types:   %s\n' "$( command grep -B1 '^  Empty,' ring_types/src/error.rs | head -1 | sed 's|^ */// ||' )"
```

Live output:

```
admit declared:                   1
admit call sites in the crate:    0
guard sites reading the flag:     3
tests naming both admit and push: 0
wait_until returns on exhaustion: RingError::Empty
wait_for_close remaps it:         0
for_space_or_close remaps it to:  RingError::Full
ring_wait::for_space maps it to:  RingError::Full
for_space exists in ring_wait:    1
ring_wait fns this crate calls:   ring_wait::wait_until 
ring types in wait_for_close arg: 0
errors the family calls transient: Self::Full | Self::Empty
what Empty means in ring_types:   The ring has no unread item and the caller asked not to wait.
```

### Items

| File | Relationship |
|------|--------------|
| [`001_the_declared_surface_and_its_attributes.md`](001_the_declared_surface_and_its_attributes.md) | The full twenty-three, of which these are four |

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_shutdown_surface.md`](../api/001_shutdown_surface.md) | The surface these four sit on, and its by-construction/by-convention grading |

### Integrations

| File | Relationship |
|------|--------------|
| [`../integration/001_family_dependency_seam.md`](../integration/001_family_dependency_seam.md) | The `ring_wait` edge both waiters cross, and what the crate takes across it |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | All four items, their bodies and their doc comments |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `close_is_idempotent_and_admit_reports_it`, `wait_for_close_reports_the_budget_running_out`, `for_space_or_close_reports_back_pressure_as_full` — each item alone, none of them together |

### SD27 — A Documented Equivalence Between Two Code Paths That Never Meet

`Shutdown::admit`'s doc comment describes it as *"the check [`Guarded`]
performs, exposed for a caller who holds a raw producer and wants to perform it
explicitly."* That is a statement of equivalence: use `admit` by hand and you
get what the guard would have done for you.

`Guarded` does not call `admit`. Its three consulting sites — `try_push`,
`try_push_batch`, `is_blocked` — each read `self.shutdown.is_closed()`
directly, and `admit` has **zero call sites in the crate**. It is a public
function the crate declares for callers and never uses itself.

That would be unremarkable if the two were the same expression, and today they
are. What makes it a hazard is that they are not the same *type*: `admit`
returns `Result< (), RingError >` and `try_push` returns
`Result< (), Refusal< T > >`, so the compiler cannot be asked whether they
agree, and the suite is not asked either — no test in the crate names `admit`
and a guarded push in the same body. The one test that exercises `admit`
(`close_is_idempotent_and_admit_reports_it`) drives the flag alone, with no
producer in sight.

So the equivalence is stated in a doc comment, held by two independent
expressions, and checked nowhere. Adding a second admission condition — a
drain-in-progress flag, a capacity rule — to either side leaves the other on
the old behavior, the doc still claiming they match, and the suite green.

### SD28 — One Event, Two Errors, and the Function With No Ring Returns "Empty"

Both free waiters call `ring_wait::wait_until`. It has one failure mode: the
spin budget runs out, and it returns `Err( RingError::Empty )`.

`wait_for_close` forwards that unchanged, and its doc says so explicitly —
*"`ring_wait`'s own budget-exhausted error, unchanged."* `for_space_or_close`
rewrites it to `Err( RingError::Full )`, and its doc says so too, citing
`ring_wait::for_space`, which performs the identical mapping. Neither is
wrong. Both are tested. The crate contains both policies for the same event
and states no rule for choosing between them.

The forwarding side is the one that reads badly. `ring_types` defines
`RingError::Empty` as *"the ring has no unread item and the caller asked not to
wait"* — a statement about a ring's occupancy. `wait_for_close` takes no ring:
its parameters are a `&Shutdown`, a `WaitKind` and a spin count, and there is
no `Ring`, `Consumer`, `Producer` or `CursorPair` in its signature. The one
item in the crate that cannot observe a ring at all is the one that reports a
ring-state error, because forwarding a callee's error verbatim also forwards
the callee's vocabulary.

The reason this has gone unnoticed is legible in `ring_types` itself:
`is_transient` returns true for `Full` and `Empty` alike, so a caller using the
family's own coarse dispatch sees one answer — *retry* — from both waiters and
never learns they disagree. Only a caller that matches the variant meets the
difference, and it will read one timeout as back-pressure and the other as a
starved consumer.
