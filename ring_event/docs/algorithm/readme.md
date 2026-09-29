# algorithm

There is almost nothing here to describe. Seven bodies — three free functions and
four trait methods — reduce to two executable statements and one branch, and every
one of them forwards to a call in `ring_slot`. The crate computes nothing; it
arranges for two unrelated slot shapes to be reachable through one signature.

So both instances are about the arrangement rather than the computation. One
measures what is actually executed and finds that a third of the surface renames a
guarantee `ring_slot` already provides. The other establishes where the choice
between the shapes is made, and shows by probe that it cannot be moved to runtime
even by a caller who wants it — for two different reasons, one of which reports
itself as something else entirely.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_two_executable_statements_and_one_branch.md) | Two Executable Statements, and One Branch | Every body in the crate, and the one function that renames a `ring_slot` trait method |
| [002](002_the_dispatch_happens_before_the_program_runs.md) | The Dispatch Happens Before the Program Runs | Four probes at erasure, two refusals, and one misdirected diagnostic |

## What Is Actually Executed

Stripping doc comments leaves exactly two semicolon-terminated statements in the
whole crate — `slot.set( self );` at `:75` and `slot.clear();` at `:233` — and
exactly one branch, the `is_empty` test at `:143` that turns a zero-length read
into `None`. Everything else is a signature, a bound, or a tail expression.

That branch is the only place the two shapes are treated differently anywhere in
the crate, and it is the origin of the crate's one documented limitation. The
rest is forwarding: `publish_into` to `Fill::fill`, `drain_from` to `Peek::peek`,
`recycle` to `Slot::clear` — and that last bound belongs to `ring_slot`, not here.

## Why It Cannot Be Deferred

Every public item is generic and the crate contains zero occurrences of `dyn`,
`Box`, `unsafe` or `#[ inline ]`. Both traits refuse erasure, but not in the same
way and not at the same point.

`Peek` refuses at the type: `E0038`, not dyn compatible, because `Out< 'a >` is a
generic associated type. `Fill` accepts the type — `&dyn Fill< TypedSlot< u32 > >`
constructs cleanly — and refuses at the call, `E0277`, because `fill` takes `self`
by value. Written the way a caller would actually write it, through method syntax,
it refuses with neither error: the unbounded blanket impl at `:66` means the trait
object itself implements `Fill`, so resolution finds that impl and reports a type
mismatch against `TypedSlot< &dyn Fill< … > >`.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- executable statements and branches in the whole crate --'
sed 's|//.*||' ring_event/src/lib.rs | command grep -n ';' | command grep -v ':use \|: *fn \|: *type '
sed 's|//.*||' ring_event/src/lib.rs | command grep -nw 'if\|match\|for\|while\|loop' | command grep -v ':impl'
echo '  -- the three free functions and what each bounds on --'
command grep -n -A 3 'pub fn ' ring_event/src/lib.rs | command grep 'pub fn \|  [A-Z] : '
echo '  -- erasure vocabulary, nowhere in the crate --'
for w in 'dyn ' 'Box<' 'unsafe'; do printf '%-8s %s\n' "$w" "$( command grep -c "$w" ring_event/src/lib.rs || true )"; done
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV1 | `ring_event` | n/a — observation | Seven bodies reduce, with doc comments stripped, to exactly two executable statements (`:75`, `:233`) and exactly one branch (`:143`), every other body being a tail expression forwarding to one `ring_slot` call — which the module documentation states outright at `:149-150` ("Deliberately trivial. Its value is not what it does but that there is only one of it") and which fixes what the rest of this corpus can be about, since a crate with two statements has no behaviour to get wrong and every finding must therefore be structural |
| EV2 | `ring_event` | n/a — doc gap | `publish_into` and `drain_from` bound on this crate's own `Fill` and `Peek` and are the mechanism that gives both slot shapes one signature, but `recycle` bounds on `ring_slot::Slot`, whose `clear` is already a trait method with one impl per shape — so any type reachable through `recycle` can reach `Slot::clear` directly with the same result, and the stated rationale at `:216-218` ("a shape-specific reset would be a fourth path") names a risk the trait method already forecloses while failing to prevent the inherent reset that does exist, `TypedSlot::take` at `ring_slot:154` |
| EV3 | `ring_event` | n/a — doc gap | Neither trait can be dispatched dynamically and they refuse differently — `Peek` at the type with `E0038` because `Out< 'a >` is a generic associated type, `Fill` only at the call with `E0277` because `fill` takes `self` by value, its trait object constructing cleanly first — and neither doc comment mentions it, though a consumer wanting a heterogeneous collection of slot shapes is a reasonable thing to want from a crate whose stated purpose is that both shapes take one path |
| EV4 | `ring_event` | n/a — diagnostics | `impl< T > Fill< TypedSlot< T > > for T` at `:66` carries no bounds, so every type including `&dyn Fill< TypedSlot< u32 > >` implements `Fill`, and method syntax on a trait object therefore resolves to the blanket impl rather than attempting dispatch — reporting `E0308` against `&mut TypedSlot< &dyn Fill< … > >`, a type nobody wrote, instead of the real `E0277` obstruction visible only from the fully-qualified form; the blanket impl's benefit is real and tested, its universality is nowhere recorded |
