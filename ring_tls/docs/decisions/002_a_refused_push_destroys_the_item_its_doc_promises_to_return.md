# Decision: A Refused `push` Destroys the Item Its Doc Promises to Return

- **Status**: Open
- **Deciders**: This crate's maintainers, pending a future benchmark verdict
- **Turns on**: Whether `push` adopts the family's payload-carrying error shape, or its documentation is corrected to match what it does

### Context

`TlsBuffer::push` takes `item : T` by value and returns
`Result< (), RingError >`. Its doc comment states:

> The item is returned to the caller by never being taken — `push` consumes `T`
> only on success, so a refused push does not destroy the payload.

The signature cannot do that. `item` is moved into the function; on the refusal
path it is bound, never read, and dropped when the block ends.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
grep -A 8 'pub fn push( &mut self' src/lib.rs
printf 'and the variant it returns:
'
grep -B 2 -A 1 '^  Full,' ../ring_types/src/error.rs
```

Live output:

```
  pub fn push( &mut self, item : T ) -> Result< (), RingError >
  {
    if self.items.len() >= self.limit
    {
      return Err( RingError::Full );
    }
    let reserved = self.items.capacity();
    self.items.push( item );
    debug_assert!
and the variant it returns:
  /// The ring has no free slot and its [`crate::OverflowPolicy`] is
  /// [`crate::OverflowPolicy::Fail`].
  Full,
  /// The ring has no unread item and the caller asked not to wait.
```

`RingError::Full` is a unit variant, so the error cannot carry the item even if
the caller wanted it back.

### The Family Already Solved This

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'Result< (), ( RegistryError' ring_registry/src/lib.rs
grep 'Err( ( RegistryError' ring_registry/src/lib.rs
```

Live output:

```
  -> Result< (), ( RegistryError, Split< T > ) >
        Err( ( RegistryError::NameTaken { name }, ring ) )
```

`ring_registry::register` returns `Result< (), ( RegistryError, Split< T > ) >`
and hands the refused ring back in the error. The idiom exists, in this
family, two crates away.

### Readings

**1 — The documentation is wrong; fix the words.** Cheapest. For `T : Copy` the
distinction is invisible, and every current caller stages `u32` or a POD
`Record`. Under this reading the sentence is deleted and nothing else changes.

**2 — The documentation is right about the requirement; fix the signature.**
`Result< (), ( RingError, T ) >`, matching `ring_registry`. A staging buffer
whose refusal destroys the payload is unusable for any `T` that owns a resource,
and refusal is the *expected* path — `is_full` exists so callers can flush and
retry, and a retry needs the item.

**3 — Neither; the caller should check `is_full` first.** What the current
callers do. It makes the destructive path unreachable by convention rather than
by type, which is the shape that survives exactly until someone writes the
obvious `if buffer.push( x ).is_err() { flush(); buffer.push( x ) }` — where `x`
has already been dropped.

### Decision

Open. Reading 2 is a breaking change to an exported crate's signature, which is
why it is filed rather than applied; reading 1 is cheap and makes the crate
honestly less useful than the doc claimed.

### Consequences

A `TlsBuffer< T >` where `T` owns a heap allocation, a file handle, or a
join handle silently destroys one on every refused push, and the documentation
tells the reader it does not.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | The signature and the claim that contradicts it |
| `../../../ring_types/src/error.rs` | `RingError::Full` as a unit variant |
| `../../../ring_registry/src/lib.rs` | The family's payload-carrying refusal, already in use |

### TL21 — The Doc Comment Promises Something the Signature Cannot Do

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
grep -A 8 'pub fn push( &mut self' src/lib.rs
printf 'and the variant it returns:\n'
grep -B 2 -A 1 '^  Full,' ../ring_types/src/error.rs
```

Live output:

```
  pub fn push( &mut self, item : T ) -> Result< (), RingError >
  {
    if self.items.len() >= self.limit
    {
      return Err( RingError::Full );
    }
    let reserved = self.items.capacity();
    self.items.push( item );
    debug_assert!
and the variant it returns:
  /// The ring has no free slot and its [`crate::OverflowPolicy`] is
  /// [`crate::OverflowPolicy::Fail`].
  Full,
  /// The ring has no unread item and the caller asked not to wait.
```

`RingError::Full` is a unit variant. There is no channel through which the item
could come back even if the body tried.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
grep -A6 '# Errors' src/lib.rs | head -7
```

Live output:

```
  /// # Errors
  ///
  /// [`RingError::Full`] when the buffer already holds `capacity()` items. The
  /// refused item is not returned to the caller: `item` is moved into this
  /// function, and on the refusal path it is bound, never read, and dropped
  /// when the function returns — the same as any other value that goes out of
  /// scope. Callers holding a `T` that owns a resource should check
```

**Disposition:** applied — Reading 1 (`the documentation is wrong; fix the
words`). The `# Errors` doc comment on `push` in `src/lib.rs` no longer claims
the refused item is returned; it states plainly that `item` is dropped on the
refusal path and points callers at `is_full` for the check-then-push idiom.
This settles only Reading 1 — the open part of this decision, whether the
signature itself should carry a payload-returning error shape (Reading 2, a
breaking change to an exported crate), remains open and is a future pass's
to rule on; nothing here forecloses it.
Now prints: `on the refusal path it is bound, never read, and dropped`

### TL22 — The Cost Is Zero for Every Current Consumer and Unbounded for the Next

Refusal is not an error path here — `is_full` is published so callers can
flush and retry, which means refusal is expected. The obvious retry,
`if buffer.push( x ).is_err() { flush(); buffer.push( x ) }`, does not compile
for a non-`Copy` `T`, which is the language catching it. For `T : Copy` it
compiles and is correct.

So the trap fires exactly once: the first time someone stages a `String`, a
`Box`, or a join handle, and reaches for a pattern the doc comment told them
was safe.

**Disposition:** declined — this crate's own `### Decision` section above
already rules on exactly this tradeoff: Reading 2 (carry the payload back in
the error, `Result< (), ( RingError, T ) >`) is a breaking change to an
exported crate's signature and is explicitly "filed rather than applied...
a future pass's to rule on; nothing here forecloses it." TL21's disposition
already closed the honest half of the gap — the doc comment no longer claims
the item is returned. The three current callers already take Reading 3
(`ring_bench/src/lib.rs:1127`, `ring_testkit/src/lib.rs:613`: both check
`.is_err()`/`.is_ok()` before treating a push as landed rather than blindly
retrying), so the residual risk this finding names is real but is the exact
open question the file already tracks, not a gap a local edit should close
ahead of that ruling.

### TL23 — The Family Already Has the Idiom and This Crate Did Not Take It

```sh
cd "$(git rev-parse --show-toplevel)"
grep 'Result< (), ( RegistryError' ring_registry/src/lib.rs
grep 'Err( ( RegistryError' ring_registry/src/lib.rs
```

Live output:

```
  -> Result< (), ( RegistryError, Split< T > ) >
        Err( ( RegistryError::NameTaken { name }, ring ) )
```

The idiom needs no change to the shared `RingError` enum, is in this family,
and is two crates away. Adopting it costs one `.map_err( |( e, _ )| e )` at
three call sites.
