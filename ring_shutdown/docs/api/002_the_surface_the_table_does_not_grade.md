# API: The Surface the Table Does Not Grade

### Scope

- **Purpose**: Read the ten public items [`001`](001_shutdown_surface.md)'s guarantee table leaves out, and ask what each one does to the guarantee the table grades.
- **Responsibility**: The producer-side methods and the accessors — what they return, what a holder can reach through them, and where two of them share a name without sharing a meaning.
- **In Scope**: `Guarded::try_push`, `try_push_batch`, `free_capacity`, `is_blocked`, `shutdown`, `into_inner`; `Refusal::into_record`, `is_closed`, `reason`; `Wake::is_ready`; `Stopped::shutdown`, `drain_all_bounded`.
- **Out of Scope**: The graded surface itself (→ [`001`](001_shutdown_surface.md)); attribute-by-attribute census (→ [`../item/001`](../item/001_the_declared_surface_and_its_attributes.md)); whether `into_inner` should exist at all (→ [`../decisions/001`](../decisions/001_should_into_inner_exist.md)).

### What Is Left Out, and Why It Groups Cleanly

[`001`](001_shutdown_surface.md) names eleven of the crate's twenty-one
distinct public function names. Nine of the ten it does not name are not
scattered: they are the **producer side** and the **accessors**. The tenth,
`Stopped::drain_all_bounded`, is neither — it is a consumer-side drain standing
beside `drain_all`, which the table does grade, and it is the one omission the
grouping does not explain.

| Owner | Ungraded items |
|---|---|
| `Guarded` | `try_push`, `try_push_batch`, `free_capacity`, `is_blocked`, `shutdown`, `into_inner` |
| `Refusal` | `into_record`, `is_closed`, `reason` |
| `Wake` | `is_ready` |
| `Stopped` | `drain_all_bounded` |
| `Stopped` | `shutdown` |

That is a defensible selection rule — `001` grades *operations performed on a
shutdown*, and these are operations performed on the things a shutdown hands
back. The rule is nowhere written down, and following it costs the table the two
items that decide whether its own **construction** column means anything.

### Two Ways Out of a Guarantee

`Shutdown::guard` states the crate's strongest claim: *"A caller holding a
`Guarded` cannot publish into a closed ring, because the only push it has
performs the check."* The claim is about **publishing**, and it is true.

`Guarded` has two methods that return something other than a push result:

| | `into_inner` | `shutdown` |
|---|---|---|
| Receiver | `self` — consumes the guard | `&self` — guard stays live |
| Returns | `Producer< 'a, T >` | `&'a Shutdown` |
| Doc comment | *"Give up the guarantee and take the raw producer back."* | *"The shutdown this producer consults."* |
| `const fn` | no | **yes** |
| Has a decision record | [yes](../decisions/001_should_into_inner_exist.md) | no |

`into_inner` is the documented exit. It announces itself, it costs the guard,
and the crate argued about it in writing.

`shutdown` is framed as an accessor — the least alarming thing an API can be
called. But `Shutdown::close` takes `&self`, so what a `&'a Shutdown` grants is
not read access. It grants `close()`, which yields a `Stopped`, which grants
`drain_all`, `discard_all` and `reopen`. A holder of a `Guarded` can therefore
close the ring it is guarding and drain it, from behind a shared reference,
while still holding the guard and still pushing into it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
printf 'names the surface table grades: %s\n' "$( awk -F'\\|' '/^\| /{ print $3 }' docs/api/001_shutdown_surface.md | command grep -ohE '[a-z_]+\(' | tr -d '(' | sort -u | wc -l )"
printf 'public names it does not:      %s\n' "$( comm -13 <( awk -F'\\|' '/^\| /{ print $3 }' docs/api/001_shutdown_surface.md | command grep -ohE '[a-z_]+\(' | tr -d '(' | sort -u ) <( command grep -ohE '^ *pub (const )?fn [a-z_]+' src/lib.rs | awk '{ print $NF }' | sort -u ) | tr '\n' ' ' )"
printf 'the guarantee guard promises:  %s\n' "$( awk '/^ *\/\/\//{ b = b " " $0; next } /^  pub const fn guard/{ print b; exit } { b="" }' src/lib.rs | sed 's|///||g; s/  */ /g' | command grep -o 'cannot publish into a closed ring[^.]*\.' )"
printf 'how Guarded gives the ring up: %s\n' "$( awk '/^  pub fn into_inner/{ sub( /^ */, "" ); print }' src/lib.rs )"
printf 'its doc line:                  %s\n' "$( awk '/^ *\/\/\//{ if ( !d ) { d=1; f=$0; sub( /^ *\/\/\/ ?/, "", f ) } next } /^ *#\[/{ next } /^  pub fn into_inner/{ print f; exit } { d=0 }' src/lib.rs )"
printf 'how Guarded gives the flag up: %s\n' "$( awk '/^  pub const fn shutdown/{ if ( ++n == 2 ) { sub( /^ */, "" ); print } }' src/lib.rs )"
printf 'its doc line:                  %s\n' "$( awk '/^ *\/\/\//{ if ( !d ) { d=1; f=$0; sub( /^ *\/\/\/ ?/, "", f ) } next } /^ *#\[/{ next } /^  pub const fn shutdown/{ if ( ++n == 2 ) { print f; exit } } { d=0 }' src/lib.rs )"
printf 'what close needs and returns:  %s\n' "$( awk '/^  pub fn close/{ sub( /^ */, "" ); print }' src/lib.rs )"
printf 'what a Stopped then grants:    %s\n' "$( awk '/^impl< .a > Stopped/{f=1} f&&/^\}$/{exit} f&&/^  pub (const )?fn /{ n=$0; sub( /.*fn /, "", n ); sub( /[(<].*/, "", n ); printf "%s ", n }' src/lib.rs )"
printf 'tests closing via the accessor: %s\n' "$( command grep -c 'guarded\.shutdown()\.close()' tests/shutdown_test.rs || true )"
printf 'decision records on into_inner: %s\n' "$( ls docs/decisions/ | command grep -c into_inner || true )"
printf 'decision records on shutdown(): %s\n' "$( ls docs/decisions/ | command grep -c shutdown || true )"
printf 'predicates about a close:      %s\n' "$( awk '/^impl/{ o=$0; sub( /^impl(< [^>]*> )? */, "", o ); sub( / *$/, "", o ) } /^  pub (const )?fn (is_closed|is_blocked|is_ready)\(/{ n=$0; sub( /.*fn /, "", n ); sub( /\(.*/, "", n ); printf "%s::%s ", o, n }' src/lib.rs )"
printf 'of those, on the graded table: %s\n' "$( awk -F'\\|' '/^\| /{ print $3 }' docs/api/001_shutdown_surface.md | command grep -ohE '(is_closed|is_blocked|is_ready)' | sort -u | tr '\n' ' ' )"
printf 'the two names declared twice:  %s\n' "$( command grep -ohE '^ *pub (const )?fn [a-z_]+' src/lib.rs | awk '{ print $NF }' | sort | uniq -d | tr '\n' ' ' )"
printf 'what Refusal::is_closed asks:  %s\n' "$( awk '/^ *\/\/\//{ if ( !d ) { d=1; f=$0; sub( /^ *\/\/\/ ?/, "", f ) } next } /^ *#\[/{ next } /^  pub const fn is_closed/{ print f; exit } { d=0 }' src/lib.rs )"
printf 'what Shutdown::is_closed asks: %s\n' "$( awk '/^ *\/\/\//{ if ( !d ) { d=1; f=$0; sub( /^ *\/\/\/ ?/, "", f ) } next } /^ *#\[/{ next } /^  pub fn is_closed/{ print f; exit } { d=0 }' src/lib.rs )"
printf 'the doctest reading the second: %s\n' "$( command grep -o 'guarded.try_push( 8 ).unwrap_err().is_closed()' src/lib.rs )"
```

Live output:

```
names the surface table grades: 11
public names it does not:      drain_all_bounded free_capacity into_inner into_record is_blocked is_ready reason shutdown try_push try_push_batch 
the guarantee guard promises:  cannot publish into a closed ring, because the only push it has performs the check.
how Guarded gives the ring up: pub fn into_inner( self ) -> Producer< 'a, T >
its doc line:                  Give up the guarantee and take the raw producer back.
how Guarded gives the flag up: pub const fn shutdown( &self ) -> &'a Shutdown
its doc line:                  The shutdown this producer consults.
what close needs and returns:  pub fn close( &self ) -> Stopped< '_ >
what a Stopped then grants:    shutdown drain_all drain_all_bounded discard_all reopen 
tests closing via the accessor: 1
decision records on into_inner: 1
decision records on shutdown(): 0
predicates about a close:      Shutdown::is_closed Refusal< T >::is_closed Guarded< 'a, T >::is_blocked Wake::is_ready 
of those, on the graded table: is_closed 
the two names declared twice:  is_closed shutdown 
what Refusal::is_closed asks:  Whether the refusal was a close rather than back-pressure.
what Shutdown::is_closed asks: Whether the ring has been closed to further publication.
the doctest reading the second: guarded.try_push( 8 ).unwrap_err().is_closed()
```

### APIs

| File | Relationship |
|------|--------------|
| [`001_shutdown_surface.md`](001_shutdown_surface.md) | The graded eleven; this document is the complement |

### Decisions

| File | Relationship |
|------|--------------|
| [`../decisions/001_should_into_inner_exist.md`](../decisions/001_should_into_inner_exist.md) | The one exit from the guarantee that was argued about in writing |

### Types

| File | Relationship |
|------|--------------|
| [`../type/001_stopped_proof_token.md`](../type/001_stopped_proof_token.md) | What a `&Shutdown` can be turned into, and why the token was supposed to be scarce |
| [`../type/002_refusal_carries_the_record.md`](../type/002_refusal_carries_the_record.md) | The three ungraded `Refusal` methods, read as a type |

### Items

| File | Relationship |
|------|--------------|
| [`../item/001_the_declared_surface_and_its_attributes.md`](../item/001_the_declared_surface_and_its_attributes.md) | The same ten items with their attributes rather than their reach |

### Pitfalls

| File | Relationship |
|------|--------------|
| [`../pitfall/001_close_is_advisory_to_an_unguarded_producer.md`](../pitfall/001_close_is_advisory_to_an_unguarded_producer.md) | The unguarded producer, which is what both exits here produce |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Every declaration read here |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | No test names `Guarded::shutdown`; the accessor's reach is unexercised |

### SD7 — The Guard Handed Out, Through a `const` Accessor, More Power Than the Method Documented as Giving Up the Guarantee

`Guarded` has two ways to hand something back, and the crate treats them very
differently.

`into_inner` consumes `self`, returns the raw `Producer`, and says so: *"Give up
the guarantee and take the raw producer back."* It has a decision instance to
itself, [`decisions/001`](../decisions/001_should_into_inner_exist.md), arguing
whether it should exist. That is the documented exit, and it behaves like one —
after it, there is no guard.

`shutdown` is a `#[ must_use ] const fn` taking `&self`, documented as *"The
shutdown this producer consults."* It read as the most innocuous shape an API
has: a borrowing accessor for a field the caller already provided.

It is not one, because `Shutdown::close` also takes `&self`. So what
`guarded.shutdown()` returns is not read access to a flag; it is the capability
to `close()` the ring, which returns a `Stopped`, which grants `drain_all`,
`discard_all` and `reopen`. The guard is not consumed, not borrowed mutably, and
not aware. A caller can close and drain the ring it is mid-way through
publishing into, and keep pushing afterwards.

Compare the two exits on every axis and the framing is backwards:

- `into_inner` costs the guard and yields **less** than the guard had — a
  producer with no flag.
- `shutdown` costs nothing, keeps the guard, and yields **more** than the guard
  had — the flag *plus* the consumer-side drain surface, from a shared
  reference, in a `const fn`.

Neither the surface table nor any test mentions it. `Shutdown::guard`'s promise
is narrowly worded enough to survive — it says a `Guarded` holder cannot
*publish into a closed ring*, and this does not let them — but the sentence a
reader takes away is that holding a `Guarded` is the constrained position, and
on this path it is the unconstrained one.

The root cause is one signature: `close( &self )`, which makes `Stopped` tokens
unlimited rather than unique. That is a family-level decision and not a doc
edit — [`decisions/002`](../decisions/002_should_a_stopped_token_be_unique.md)
is where it is open — and it stays open. What belonged here is that the crate's
own accessor is the shortest route to exploiting it, and that route was
ungraded, undocumented and untested.

Two of those three were repairable without touching the signature. Both
`shutdown` accessors now carry a `# What this hands out` section naming what the
return value can reach rather than only what it points at, and the `Guarded` one
states the inversion in the same words as the comparison above — that the
documented exit yields less than the guard had and the undocumented one yields
more. A test now walks the route end to end: it closes through
`guarded.shutdown()`, with nothing else in scope, and asserts the guard that
handed the capability over is the guard that gets refused.

The third — grading — is left where it was on purpose. The surface table is
`api/001`'s, and adding a row for a path that exists only because of an open
decision would grade a shape the family has not settled on. The rustdoc says
what the accessor does; the decision says what it should be.

**Disposition:** applied — `# What this hands out` rustdoc on both
`Stopped::shutdown` and `Guarded::shutdown`, the second naming the `into_inner`
inversion and citing
[`pattern/002`](../pattern/002_a_proof_token_must_be_scarce.md), plus
`closing_through_the_guards_own_accessor_stops_the_guard` in
`tests/shutdown_test.rs`. Grading in `api/001` deliberately not added, pending
`decisions/002`. Now prints: `tests closing via the accessor: 1`

### SD8 — Four Predicates Share One Verb and Answer About Four Different Times

The crate declares twenty-one distinct public function names across
twenty-three items. Exactly two names are declared twice, and they behave in opposite ways.

`shutdown` is a true synonym — `Stopped::shutdown` and `Guarded::shutdown` have
the same signature and the same meaning, the flag this thing consults.

`is_closed` is a homonym. `Shutdown::is_closed` asks *"Whether the ring has
been closed to further publication"* — a live `Acquire` load whose answer can
differ between two consecutive calls. `Refusal::is_closed` asks *"Whether the
refusal was a close rather than back-pressure"* — a `matches!` over an owned
enum, fixed at the instant the push was refused, and incapable of ever
changing. Same name, same `&self -> bool`
shape, opposite temporal status: one is a reading of the present, the other a
record of the past.

Widen it by one letter and the pattern is a family of four:

| Predicate | Asks about | Can its answer change? |
|---|---|---|
| `Shutdown::is_closed` | now | yes, on any other thread's `close` |
| `Guarded::is_blocked` | now, for either reason | yes, and *"does not predict a refusal"* |
| `Refusal::is_closed` | the instant of one refusal | no — owned, frozen |
| `Wake::is_ready` | some instant inside a completed wait | no — owned, frozen, and possibly already false |

Each doc comment is individually accurate. Nothing anywhere states the shape,
and the surface table grades one of the four — the `Read it` row, for
`Shutdown::is_closed` — leaving the reader to discover from three separate
declaration sites that the same verb is being used for a live reading and for
two frozen records.

The crate's own doctest walks straight into the ambiguity. Three lines after
`shutdown.close();` it writes
`guarded.try_push( 8 ).unwrap_err().is_closed()` — which reads the `Refusal`,
not the flag, in a snippet whose subject one line earlier *was* the flag. The
assertion is correct and the reader has no cue that the receiver changed.
