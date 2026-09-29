# Type: `Refusal` Carries the Record, Not an Error

### Scope

- **Purpose**: Explain why a guarded push refuses with a two-armed type carrying the record rather than with `RingError`, and record which arm is reachable when.
- **Responsibility**: The type's shape, the information it preserves, and its interaction with the configured overflow policy.
- **In Scope**: `Refusal< T >`, `Refusal::into_record`, `Refusal::is_closed`, `Refusal::reason`, and `Wake`.
- **Out of Scope**: The unreachability of one arm as a caller trap (→ [`pitfall/002`](../pitfall/002_ok_does_not_mean_kept_under_drop_newest.md)).

### Definition

```rust
pub enum Refusal< T >
{
  Full( T ),
  Closed( T ),
}
```

Both arms carry the record. That is the requirement `ring_core`'s producer
surface already imposes — *a refusal never destroys the record* — and a
`RingError` return would break it, because `RingError` is `Copy` and carries
nothing.

#### Why two arms and not a bool

The distinction is the one a producer acts on, and the two actions are
opposite:

| Arm | What cleared it | Correct response |
|---|---|---|
| `Full` | The consumer drains | Retry, with back-pressure |
| `Closed` | Only `Stopped::reopen` | Stop, and stash or drop the record |

`RingError::is_transient` already draws this line — `Full` is transient,
`Closed` is not — and `Refusal::reason` maps onto it, so a caller who wants the
error vocabulary gets it without the type having to choose between carrying the
record and carrying the distinction.

#### `into_record` takes `self`

```rust
pub fn into_record( self ) -> T
```

By value, so the record moves out rather than being cloned — the whole point of
carrying it. A caller who wants both the record and the reason reads `reason()`
first and then unwraps; a caller who wants only one pays for only one.

### Validation

There is no constructor and no validation function — a `Refusal` is built only
by the crate, in exactly two places, and a caller's whole obligation is to
match on it. What *is* worth stating is which arms a given build can actually
produce:

| Arm | Produced when | Reachable under `RingConfig`'s default policy (`DropNewest`)? |
|---|---|---|
| `Refusal::Closed( record )` | The flag was set and `Guarded::try_push` checked it first | ✅ always |
| `Refusal::Full( record )` | The ring is full and the policy is `OverflowPolicy::Fail`, **or** the policy is `OverflowPolicy::DropOldest` on a build without the `crossbeam` feature (`ring_core`'s own default: `default = []`) — `try_push` still resolves `DropOldest` to `Resolution::EvictedOldest`, but only the `crossbeam`-gated arm evicts; without it the same resolution is refused, not evicted | ❌ no — see below |

The single validity rule the type does enforce is that **both arms carry the
record**. There is no arm that reports a refusal without handing the value back,
which is what makes a refusal recoverable rather than a loss.

#### One arm is unreachable only under `DropNewest`

`Refusal::Full` is unreachable only under `OverflowPolicy::DropNewest` —
under that policy a full push returns `Ok` having discarded the record, so a
caller matching both arms exercises one. It is reachable under `Fail`, and
also under `DropOldest` on a build without the `crossbeam` feature (the
eviction that policy promises is a `crossbeam`-gated code path; without it,
`DropOldest` refuses the same way `Fail` does). This is a property of the
configuration and the build's feature set, not of the type, and it is the
subject of [`pitfall/002`](../pitfall/002_ok_does_not_mean_kept_under_drop_newest.md).

The type is still right: making it single-armed would be correct for the
default configuration and wrong for the other one, and a caller cannot tell
which they have from the signature.

### `Wake` Is the Same Idea Without a Payload

```rust
pub enum Wake { Ready, Closed }
```

`for_space_or_close` can end for two reasons and a bare `Ok` would merge them —
a producer that read "the wait succeeded" and went on to publish would publish
into a closed ring. `Wake` makes the two outcomes distinct at the type level,
carrying nothing because there is nothing to carry.

**`Closed` wins when both are true.** Room and a close can hold at the same
instant; the function reports `Closed`, because a producer told to stop must
stop. That precedence is asserted rather than assumed —
`for_space_or_close_names_the_exit_it_took` checks `may_claim()` is still true
at the moment `Closed` is returned.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
D=docs/type/002_refusal_carries_the_record.md
printf 'policies the family declares:  %s\n' "$( cd ..; awk '/pub enum OverflowPolicy/{f=1} f&&/^\}/{exit} f&&/^  [A-Z]/{ sub( /,$/, "" ); sub( /^ */, "" ); printf "%s ", $0 }' ring_types/src/policy.rs )"
printf 'the default among them:        %s\n' "$( cd ..; awk '/pub enum OverflowPolicy/{f=1} f&&/default \]/{ getline; sub( /,$/, "" ); sub( /^ */, "" ); print; exit }' ring_types/src/policy.rs )"
printf 'policies this doc names:       %s\n' "$( awk '/^### Regenerate/{ exit } { print }' $D | command grep -ohE 'DropNewest|DropOldest|Fail' | sort -u | tr '\n' ' ' )"
printf 'what the table says Full needs: %s\n' "$( awk -F' \\| ' '/^### Regenerate/{ exit } /Refusal::Full\( record \)/{ print $2 }' $D )"
printf 'how ring_core decides:         %s\n' "$( cd ..; awk '/Resolution::DroppedIncoming => Ok/{ f=1 } f{ sub( /^ */, "" ); printf "%s ", $0 } f&&/=> Err\( record \)/{ exit }' ring_core/src/lib.rs )"
printf 'what DropOldest maps to:       %s\n' "$( cd ..; command grep -o 'OverflowPolicy::DropOldest => Resolution::[A-Za-z]*' ring_overflow/src/lib.rs )"
printf 'what that Resolution promises: %s\n' "$( cd ..; awk '/pub enum Resolution/{f=1} f&&/^  EvictedOldest,/{ print p2, p1; exit } f{ p2=p1; p1=$0 }' ring_overflow/src/lib.rs | sed 's|///||g; s/^ *//; s/  */ /g' )"
printf 'the only path that evicts:     %s\n' "$( cd ..; awk '/overflow == OverflowPolicy::DropOldest/{ f=1 } f{ sub( /^ */, "" ); printf "%s ", $0 } f&&/return Ok/{ exit }' ring_core/src/lib.rs )"
printf 'and what gates that path:      %s\n' "$( cd ..; command grep -B4 'overflow == OverflowPolicy::DropOldest' ring_core/src/lib.rs | command grep -o 'feature = \"crossbeam\"' )"
printf 'is crossbeam a default feature: %s\n' "$( cd ..; awk '/^\[features\]/{f=1} f&&/^default/{ print; exit }' ring_core/Cargo.toml )"
printf 'must_use attributes in src:    %s\n' "$( command grep -c '#\[ must_use' src/lib.rs )"
printf 'of those, on the Wake enum:    %s\n' "$( command grep -B2 'pub enum Wake' src/lib.rs | command grep -c 'must_use' || true )"
printf 'and on the Refusal enum:       %s\n' "$( command grep -B2 'pub enum Refusal' src/lib.rs | command grep -c 'must_use' || true )"
printf 'what for_space_or_close gives: %s\n' "$( command grep -o '^-> Result< Wake, RingError >' src/lib.rs )"
printf 'crates naming for_space_or_close: %s\n' "$( cd ..; command grep -rl 'for_space_or_close' ring_*/src ring_*/tests --include='*.rs' | cut -d/ -f1 | sort -u | wc -l )"
printf 'of those, other than this one: %s\n' "$( cd ..; command grep -rl 'for_space_or_close' ring_*/src ring_*/tests --include='*.rs' | cut -d/ -f1 | sort -u | command grep -vc ring_shutdown || true )"
```

Live output:

```
policies the family declares:  DropNewest DropOldest Fail 
the default among them:        DropNewest
policies this doc names:       DropNewest DropOldest Fail 
what the table says Full needs: The ring is full and the policy is `OverflowPolicy::Fail`, **or** the policy is `OverflowPolicy::DropOldest` on a build without the `crossbeam` feature (`ring_core`'s own default: `default = []`) — `try_push` still resolves `DropOldest` to `Resolution::EvictedOldest`, but only the `crossbeam`-gated arm evicts; without it the same resolution is refused, not evicted
how ring_core decides:         Resolution::DroppedIncoming => Ok( () ), Resolution::EvictedOldest | Resolution::Refused => Err( record ), 
what DropOldest maps to:       OverflowPolicy::DropOldest => Resolution::EvictedOldest
what that Resolution promises: which [`Resolution::accepted_incoming`] is true, and deleting it would collapse that predicate to a constant `false`.
the only path that evicts:     if self.overflow == OverflowPolicy::DropOldest { let _evicted = queue.force_push( record ); return Ok( () ); 
and what gates that path:      feature = "crossbeam"
is crossbeam a default feature: default = []
must_use attributes in src:    12
of those, on the Wake enum:    1
and on the Refusal enum:       0
what for_space_or_close gives: -> Result< Wake, RingError >
crates naming for_space_or_close: 1
of those, other than this one: 0
```

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_shutdown_surface.md](../api/001_shutdown_surface.md) | Guarantee 1, whose error shape this type is |

### Pitfalls

| File | Relationship |
|------|--------------|
| [../pitfall/002_ok_does_not_mean_kept_under_drop_newest.md](../pitfall/002_ok_does_not_mean_kept_under_drop_newest.md) | The unreachable-arm consequence, as a caller trap |

### Sources

| File | Relationship |
|------|--------------|
| [`ring_core/docs/api/001`](../../../ring_core/docs/api/001_producer_surface.md) | Guarantee 1 there — "a refusal never destroys the record" — which this type extends by one case |
| [`ring_types`](../../../ring_types/readme.md) | `RingError::is_transient`, the same distinction without the payload |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `a_guarded_producer_refuses_a_closed_ring_and_returns_the_record` — the `Closed` arm, record intact |
| `tests/shutdown_test.rs` | `a_full_guarded_producer_refuses_with_the_transient_arm` — the `Full` arm, under the policy that reaches it |
| `tests/shutdown_test.rs` | `for_space_or_close_names_the_exit_it_took` — `Wake`'s precedence when both conditions hold |

### SD47 — The Reachability Table Knows Two Overflow Policies and the Family Declares Three

`OverflowPolicy` has three variants — `DropNewest` (the `#[ default ]`),
`DropOldest`, and `Fail`. This document names two. `DropOldest` appears nowhere
in it, and the omission is not cosmetic, because the reachability table's whole
purpose is to say something the signature cannot.

The table says `Refusal::Full` is produced when *"The ring was full **and** the
policy is `OverflowPolicy::Fail`"*. It is also produced under `DropOldest`.
`ring_core::Producer::try_push` ends:

```rust
Resolution::DroppedIncoming => Ok( () ),
Resolution::EvictedOldest | Resolution::Refused => Err( record ),
```

and `would_resolve( OverflowPolicy::DropOldest ) = Resolution::EvictedOldest`,
which the second arm hands straight back — so the record comes back, wrapped by
`Guarded::try_push` into `Refusal::Full`. That second arm read `_ =>` when this
finding was written, which made the same claim an inference off a catch-all;
`ring_core`'s CO1 has since named both variants, so `EvictedOldest` now appears
literally on the line that refuses it. A caller who configured `DropOldest`
and read this table concludes that arm cannot fire and matches it as dead.

The second half is worse than a missing row. `Resolution::EvictedOldest` is
documented *"The oldest unread item was discarded to make room; the incoming
item was accepted"*, and on this build neither happened: the ring is untouched
and the record is refused. The only code that actually evicts is
`if self.overflow == OverflowPolicy::DropOldest { let _evicted = queue.force_push( record ); return Ok( () ); }`,
inside the arm gated by `feature = "crossbeam"` — and `ring_core`'s manifest
says `default = []`. On the default build, `DropOldest` **is** `Fail` under a
different name, and the refusal it produces is transient
(`RingError::is_transient`), so this document's own arm table tells the caller
to *"Retry, with back-pressure"* for a policy they chose in order not to have to.

The column heading is where the two defaults collide. *"Reachable under the
default config?"* reads as a question about `RingConfig`, and for
`Refusal::Closed` and the `DropNewest` row it is. For `DropOldest` the answer
turns on `ring_core`'s **feature** default, a different file and a different
kind of default, and one column cannot hold both. The type remains right —
every arm carries the record, which is the validity rule this document exists
to state — and what is wrong is the part it added because reachability is
invisible from the signature.

**Disposition:** applied — the reachability table now names all three
policies: the `Full` row states it is produced under `Fail` **or** under
`DropOldest` on a build without the `crossbeam` feature (naming the
`crossbeam`-gated eviction path and `ring_core`'s own `default = []`), and
the column heading now reads "Reachable under `RingConfig`'s default policy
(`DropNewest`)?" rather than the ambiguous "default config?", separating
`RingConfig`'s default from `ring_core`'s feature default. The "One arm is
unreachable" prose subsection below the table was corrected to match. The
same false claim — "`Full` is only reachable under `OverflowPolicy::Fail`" —
was also present on `Guarded::try_push`'s own doc comment in `src/lib.rs`
(the rustdoc a caller actually reads) and has been corrected there too, since
leaving it would have shipped the identical error one location over. Now
prints: `policies this doc names:       DropNewest DropOldest Fail`

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
D=docs/type/002_refusal_carries_the_record.md
awk '/^\| Arm \| Produced when/{f=1} f&&/^$/{exit} f' $D | command grep -c 'DropOldest'
command grep -c 'only reachable under' src/lib.rs
command grep -o 'Full. is unreachable only under .OverflowPolicy::DropNewest' src/lib.rs
```

Live output:

```
1
0
Full` is unreachable only under `OverflowPolicy::DropNewest
```

### SD48 — The Enum That Exists to Stop a Publish Was One `?` Away From Being Discarded

`Wake` is here because *"a producer that read 'the wait succeeded' and went on
to publish would publish into a closed ring"* — the type makes the two exits
distinct so that outcome is unspellable. It is returned as
`Result< Wake, RingError >`, and `Result` is `#[ must_use ]`, so
`for_space_or_close( … );` on its own warns.

`for_space_or_close( … )?;` did not. The `?` consumes the `Result`, satisfying
its `must_use`, and yields a `Wake` in expression-statement position — where
nothing objected, because neither `Wake` nor `Refusal` carried `#[ must_use ]`.
The file had nine of them and all nine sat on accessors and predicates: `new`,
two `is_closed`, `reason`, `free_capacity`, `is_blocked`, two `shutdown`, and
`is_ready`. So the crate marked `Wake::is_ready` — reading the answer — and not
`Wake` — having the answer. One line, written by reflex inside any
`Result`-returning function, compiled clean under `-D warnings` and restored
exactly the merge the type was introduced to prevent.

The fix was an attribute:
`#[ must_use = "a Wake::Closed means stop, not publish" ]` on the enum makes
`expr?;` warn while leaving every caller who reads the value untouched. Nothing
argued against it anywhere; the question was unanswered because it was never
asked.

There was no witness yet and that was the argument for doing it immediately.
`for_space_or_close` is named in exactly one crate in the family — this one —
so no caller had written the `?;` line, no test had to be edited to silence a
new warning, and the cost of the attribute was zero. It would not have stayed
zero, and the companion finding proves it: at
[`../lifecycle/001`](../lifecycle/001_teardown_and_reuse.md)'s SD30 the same
omission on `close` had already grown seven call sites, every one of which had
to be rewritten when the attribute went on. Here the same repair cost nothing
because it was made before the callers existed — which is the whole argument for
paying an attribute's price at the moment it is free rather than at the moment
it is needed.

The pattern across both is that this crate had spent `must_use` on values that
are merely *informational* and withheld it from the two whose discard is a
correctness failure — the proof token, and the reason a wait ended. Both now
carry it, and the attribute sits on the `Wake` enum rather than only on the
`Result`, because it is `?` and not the bare call that defeats `Result`'s own.

**Disposition:** applied — the enum declaration now carries
`#[ must_use = "a Wake::Closed means stop, not publish" ]`, so
`for_space_or_close( .. )?;` in statement position warns under `-D warnings`
while `let wake = ..?;` stays silent; zero call sites needed editing, as
predicted. Now prints: `of those, on the Wake enum:    1`
