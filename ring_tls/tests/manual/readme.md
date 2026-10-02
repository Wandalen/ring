# ring_tls manual testing plan

`tests/tls_test.rs` asserts the crate's two numbers: zero atomic operations to
accumulate, one to land. Both are measured through
`ring_atomic::CountingSeq`, which counts operations **on the cursor it was
handed**. That is the right instrument for the flush and a blunt one for the
push. A `push` that took a lock, or allocated, or touched some *other* atomic
would still leave the cursor's count at zero.

So this plan reads the push path directly. "Zero atomic operations" is a claim
about a short body; the cheapest way to check it is to read it.

Run from the workspace root.

## M1. The push path, in full

```bash
sed -n '/  pub fn push(/,/^  }/p' ring_tls/src/lib.rs
```

**Expected:** a bounds check, a `Vec::push` with a `debug_assert!` that the
capacity did not grow, an `Ok`. Nothing else.

## M2. No atomic, lock or allocation appears in it

```bash
sed -n '/  pub fn push(/,/^  }/p' ring_tls/src/lib.rs \
  | grep -nE "Ordering|atomic|lock|Mutex|with_capacity|reserve|Box::|Vec::"
```

**Expected:** only the reservation check, the `let reserved` binding and the
`debug_assert!` that compares against it, whose message names `with_capacity`.
No atomic, lock or allocation call. This rules out an explicit allocation, not a growth inside
`Vec::push`, because grep sees a method call, not a growth. M3 covers that.

## M3. `Vec::push` cannot reallocate, because the two bounds are the same number

M2 shows no explicit allocation, but `Vec::push` grows when it is full. What
rules that out is that the buffer refuses at the same count it reserved for.
That is a property of two numbers agreeing, which grep can show and cannot prove.

```bash
grep -nE "Vec::with_capacity\( limit \)|>= self\.limit" ring_tls/src/lib.rs \
  | grep -vE "^[0-9]+:[[:space:]]*(///|//!)"
```

**Expected:** `Vec::with_capacity( limit )` in the constructor, and `>=
self.limit` guarding `push`. Since `Vec::with_capacity( n )` reserves at least
`n`, and `push` runs only while `len < limit`, the vector never grows. Read the
two lines together and confirm the same `limit` is on both sides. A
constructor reserving `limit * 2`, or a guard at `> self.limit`, would break it
silently and no test would notice.

## M4. The flush claims once and the buffer empties unconditionally

```bash
sed -n '/  pub fn flush_into</,/^  }/p' ring_tls/src/lib.rs
```

**Expected:** one `claim(...)` call, then `self.items.drain( .. )`. The drain
must not be conditional on the iterator being consumed, because the sequences
are claimed by then, and items left staged could be claimed a second time.

## M5. The discard path claims nothing, and says why

A thread going away with staged items must not advance the cursor. Sequences
claimed for items nobody writes leave a hole a consumer waits on forever.

```bash
grep -n -B 10 "pub fn discard" ring_tls/src/lib.rs
```

**Expected:** the body clears without touching a cursor, and the doc contrasts
it with the flush rather than leaving the reader to infer the difference.

## M6. The empty-flush cost is documented as deliberate

Flushing an empty buffer still costs one atomic. A reader will see that as a
missed optimisation unless the reason is written down.

```bash
grep -n -A 12 "Flushing an empty buffer" ring_tls/src/lib.rs
```

**Expected:** the doc says a silent skip would make the operation count depend
on the data, which would undermine every assertion the counting shim makes.

## M7. The doc examples are the API's first reader

```bash
cargo test -p ring_tls --doc
```

**Expected:** every example passes, and the type-level example shows the zero
and the one together. Those two numbers *are* the crate's claim.

## Run Record

| Date | Check | Result | Note |
| ---- | ----- | ------ | ---- |
| 2026-08-28 | M1 | ✅ | Exactly as expected: `if self.items.len() >= self.limit { return Err( RingError::Full ); }`, `self.items.push( item );`, `Ok( () )`. |
| 2026-08-28 | M2 | ✅ | No output: no atomic, no lock, no allocation call in the body. |
| 2026-08-28 | M3 | ✅ | `Vec::with_capacity( limit )` at line 96; the guard `>= self.limit` at line 117. Same `limit` on both sides, so `push` runs only below the reserved capacity. |
| 2026-08-28 | M4 | ✅ | One `claim( cursor, self.items.len(), order )`, then `self.items.drain( .. )` in the returned struct. The drain is unconditional, so a dropped `Flush` still empties. `a_flush_empties_the_buffer_even_when_the_iterator_is_dropped_unread` asserts it. |
| 2026-08-28 | M5 | ✅ | Body is `self.items.clear()`; the doc says "Distinct from a flush precisely because it advances no cursor" and names the consequence. |
| 2026-08-28 | M6 | ✅ | `flush_into` documents that a silent skip would make the operation count depend on the data. |
| 2026-08-28 | M7 | ✅ | 6 doc tests pass; the `TlsBuffer` example asserts `total == 0` after 64 pushes and `total == 1` after the flush. |
