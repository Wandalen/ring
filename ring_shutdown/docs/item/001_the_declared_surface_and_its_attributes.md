# Item: The Declared Surface and Its Attributes

### Scope

- **Purpose**: Count what the crate declares and record which declaration-site attributes each item carries, so the gaps are visible as gaps rather than as absences nobody looked for.
- **Responsibility**: The public function inventory, and per item its `#[ must_use ]`, `const`, and `# Errors` status.
- **In Scope**: Every `pub fn` in `src/lib.rs`, across all five types and the three free functions.
- **Out of Scope**: What the surface *means*, which is [`../api/001`](../api/001_shutdown_surface.md)'s by-construction/by-convention table; the derive attributes on the types themselves (→ [`../data_structure/002`](../data_structure/002_five_public_types_and_their_derive_sets.md)).

### Inventory

Twenty-two public functions, on five types plus three free.

| Owner | Items |
|---|---|
| `Shutdown` | `new`, `is_closed`, `close`, `admit`, `guard` |
| `Stopped` | `shutdown`, `drain_all`, `discard_all`, `reopen` |
| `Refusal` | `into_record`, `is_closed`, `reason` |
| `Guarded` | `try_push`, `try_push_batch`, `free_capacity`, `is_blocked`, `shutdown`, `into_inner` |
| `Wake` | `is_ready` |
| free | `wait_for_close`, `for_space_or_close`, `reset` |

### Attributes

Three declaration-site attributes carry contracts a caller can rely on, and each
is applied to a different subset.

**`#[ must_use ]` — twelve, eleven of them on functions.** Eight are accessors
that answer a question — `is_closed` (both), `reason`, `is_ready`, `is_blocked`,
`free_capacity`, `shutdown` (both) — where computing the answer and dropping it
is always a mistake. The ninth is `Shutdown::new`, where dropping the result
discards the flag itself. The last three arrived with SD25 below,
[`../lifecycle/001`](../lifecycle/001_teardown_and_reuse.md)'s SD30 and
[`../type/002`](../type/002_refusal_carries_the_record.md)'s SD48: `close` and
`into_record`, whose return values exist nowhere else, and the `Wake` enum — the
only one of the twelve that sits on a type rather than on a function.

**`const fn` — seven of twenty-three.** The ones that do no work beyond moving a
reference or matching a discriminant. `Shutdown::new` is `const`, which is what
lets a caller put one in a `static`.

**`# Errors` — five.** The five fallible items: `admit`, `Guarded::try_push`,
`wait_for_close`, `for_space_or_close`, `Stopped::drain_all_bounded`. `#![ deny( missing_docs ) ]` at line 34
forces a doc comment on every item; it does not force an `# Errors` section, so
this one is discipline rather than enforcement — and it happens to be complete.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_shutdown
printf 'public functions:              %s\n' "$( command grep -cE '^ *pub (const )?fn ' src/lib.rs )"
printf 'of those, must_use:            %s\n' "$( command grep -c '^ *#\[ must_use' src/lib.rs )"
printf 'of those, const fn:            %s\n' "$( command grep -cE '^ *pub const fn ' src/lib.rs )"
printf 'items with an Errors section:  %s\n' "$( command grep -c '^ */// # Errors' src/lib.rs )"
printf 'items returning Result:        %s\n' "$( command grep -cE '^ *pub (const )?fn .*-> Result' src/lib.rs )"
printf 'plus multi-line signatures:    %s\n' "$( command grep -cE '^-> Result' src/lib.rs )"
printf 'value-returning, no must_use:  %s\n' "$( awk '/^ *#\[ must_use/{ mu=1; next } /^ *pub (const )?fn /{ if ( !mu && $0 ~ /->/ ) { n=$0; sub(/^ */,"",n); sub(/\(.*/,"",n); sub(/^pub (const )?fn /,"",n); printf "%s ", n } mu=0 }' src/lib.rs )"
printf 'is deny missing_docs present:  %s\n' "$( command grep -c 'deny( missing_docs )' src/lib.rs )"
printf 'items with no doc comment:     %s\n' "$( awk '/^ *\/\/\//{ d=1; next } /^ *#\[/{ next } /^ *\/\//{ next } /^ *pub (const )?fn /{ if ( !d ) c++; d=0; next } /^ *$/{ next } { d=0 } END{ print c+0 }' src/lib.rs )"
```

Live output:

```
public functions:              23
of those, must_use:            12
of those, const fn:            7
items with an Errors section:  5
items returning Result:        2
plus multi-line signatures:    2
value-returning, no must_use:  admit guard< 'a, T > drain_all< T : Send > discard_all< T : Send > try_push try_push_batch into_inner reset< T : Send > 
is deny missing_docs present:  1
items with no doc comment:     0
```

### APIs

| File | Relationship |
|------|--------------|
| [`../api/001_shutdown_surface.md`](../api/001_shutdown_surface.md) | The same items, graded by whether their promise holds by construction or by convention |

### Items

| File | Relationship |
|------|--------------|
| [`002_two_checks_and_two_waiters.md`](002_two_checks_and_two_waiters.md) | Four of these items in detail — the pair that duplicates a check, and the pair that disagrees about an error |

### Types

| File | Relationship |
|------|--------------|
| [`../type/002_refusal_carries_the_record.md`](../type/002_refusal_carries_the_record.md) | The argument `into_record` exists to serve, and which SD25 measures the enforcement of |

### Sources

| File | Relationship |
|------|--------------|
| [`src/lib.rs`](../../src/lib.rs) | Every declaration counted here, with its attributes |

### Tests

| File | Relationship |
|------|--------------|
| `tests/shutdown_test.rs` | `a_guarded_producer_refuses_a_closed_ring_and_returns_the_record` — the one test that reads an `into_record` return value |

### SD25 — The Payload-Recovery Method Was the One Value the Compiler Let You Drop

[`type/002`](../type/002_refusal_carries_the_record.md) argues that a guarded
push refuses with a two-armed type carrying the record rather than with a
`RingError`, so that a record that could not be published comes back rather than
being lost. `Refusal::into_record` is the method that performs the hand-back —
it consumes the refusal and yields the `T`.

It was not `#[ must_use ]`. Nine of the crate's public functions carried the
attribute; eight of those return a *question's answer* — `is_closed`,
`is_blocked`, `reason`, `free_capacity` — and the ninth returns a freshly built
flag. Dropping any of the nine wastes a computation. Dropping `into_record`'s
return destroys a record that no longer exists anywhere else, and that was the
one the attribute was missing from, so

```rust
refusal.into_record();
```

compiled without a warning and destroyed the record. That is precisely the loss
the type exists to prevent, reachable in one statement, with the crate's own
`must_use` discipline pointed everywhere except at it.

The pattern behind the gap is legible: the attribute had been applied by asking
*does this compute something?* rather than *is this return value recoverable?* —
two questions that agreed on every other item in the crate and disagreed on the
one where the answer is a payload rather than a fact. The repair therefore
carries a message rather than a bare attribute, because the reason this one
matters is not the reason the other eight do: the other eight waste a
computation, this one destroys the record.

The census line above is the standing check. `value-returning, no must_use`
still lists eight items, and that is not a residual gap — they are the four
`Guarded`/`Stopped` count-returners of SD26, plus `admit`, `guard`, `into_inner`
and `reset`. None of them hands back a payload; SD26 is the finding that argues
about the counts.

**Disposition:** applied —
`#[ must_use = "this is the record itself, not a copy — dropping it loses it" ]`
on `Refusal::into_record`, alongside the same treatment for `Shutdown::close`
(→ SD30) and the `Wake` enum (→ `type/002`'s SD48), taking the crate from nine
attributes to twelve. Now prints: `of those, must_use:            12`

### SD26 — Four Operations Report What They Did in a Return Value Nothing Requires Reading

`Guarded::try_push_batch` returns how many records landed. `Stopped::drain_all`
returns how many were recovered. `Stopped::discard_all` and `reset` return how
many were dropped. In each case the return value is the *only* channel through
which the operation reports what happened — none of the four fails, none takes
an out-parameter that would reveal a shortfall, and a partial result is
indistinguishable from a complete one at the call site.

None of the four is `#[ must_use ]`.

The consequence differs per item and is worst for the first. `try_push_batch`
stops at the first refusal and returns the count that landed; a caller who
ignores it has published an unknown prefix of its iterator and has no way to
learn which. The crate's own test
`a_closed_batch_push_consumes_nothing` exists precisely because that distinction
matters — it asserts the iterator is left intact so a caller *can* resume — and
the attribute that would make a caller notice the count in the first place is
absent.

`discard_all` and `reset` are milder: their counts are informational at teardown.
But the four sit together in the same class and the same attribute would cost
nothing on any of them, so the omission reads as uniform rather than considered.
