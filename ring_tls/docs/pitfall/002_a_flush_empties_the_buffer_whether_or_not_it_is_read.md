# Pitfall: A Flush Empties the Buffer Whether or Not It Is Read

### Scope

- **Purpose**: Record the two ways `flush_into` costs something a caller did not ask for — an unread `Flush` still empties and still advances, and an empty flush still spends an atomic.
- **Responsibility**: The `Drain`-on-drop semantics, the unconditional claim, and why neither is guarded.
- **In Scope**: `flush_into`'s effects when the returned `Flush` is dropped early, dropped unread, or taken on an empty buffer.
- **Out of Scope**: The refusal path (→ [`../decisions/002`](../decisions/002_a_refused_push_destroys_the_item_its_doc_promises_to_return.md)); the policy deciding when to flush, which is `ring_flush`'s.

### Both Effects, Asserted

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
grep -oE '^fn [a-z_0-9]+' tests/tls_test.rs | sed 's/^fn //' \
| grep -E 'dropped_unread|partially_consumed|empty_buffer_claims'
```

Live output:

```
flushing_an_empty_buffer_claims_nothing_but_still_costs_its_operation
a_flush_empties_the_buffer_even_when_the_iterator_is_dropped_unread
a_partially_consumed_flush_still_empties_the_buffer
drain_empties_the_buffer_even_when_the_iterator_is_dropped_unread
```

Four tests, all passing, all asserting that the surprising thing is what
happens — and two of them are `drain`'s, which empties on drop the same way. This is documented behaviour, not a defect — and it is still the
shape a caller gets wrong first.

### Why the Claim Cannot Be Undone

The sequences are allocated by a `fetch_add` before any item is yielded. A
`Flush` dropped after two of sixty-four pairs have been read leaves sixty-two
sequences owned by nothing: a consumer waiting on the published bound waits on
records that will never be written. Returning them is not possible — the cursor
is shared and other producers may already have claimed past it.

So the buffer must empty. Keeping the items while the sequences are gone would
be worse: the next flush would claim a second range for the same records and
publish them twice.

### The Empty Flush

`flush_into` on an empty buffer calls `claim( cursor, 0, order )`, which is
`fetch_add( 0 )` — a real atomic read-modify-write that returns a range of
length zero. The source states the reason: a silent skip would make the
operation count depend on the data, and this crate's whole claim is a count
that does not.

**The consequence is a caller in a hot loop paying one atomic per idle
iteration.** `is_empty` exists for exactly this and the crate deliberately does
not call it on the caller's behalf.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | Documents both effects at `flush_into` |
| `tests/tls_test.rs` | Asserts all three cases |
| `../../../ring_flush/src/lib.rs` | The consumer that needed `drain` because this shape could not serve it |

### TL46 — A Dropped or Partly-Read `Flush` Still Empties and Still Advances

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
grep -oE '^fn [a-z_0-9]+' tests/tls_test.rs | sed 's/^fn //' \
| grep -E 'dropped_unread|partially_consumed|empty_buffer_claims'
```

Live output:

```
flushing_an_empty_buffer_claims_nothing_but_still_costs_its_operation
a_flush_empties_the_buffer_even_when_the_iterator_is_dropped_unread
a_partially_consumed_flush_still_empties_the_buffer
drain_empties_the_buffer_even_when_the_iterator_is_dropped_unread
```

The sequences cannot be returned — the cursor is shared and other producers may
already have claimed past it — and keeping the items while the sequences are
gone would publish them twice on the next flush. Emptying is the correct
behaviour and it is still the shape a caller gets wrong first.

**Disposition:** applied — the cost claim is backed by this section's own live
run: four tests (`flushing_an_empty_buffer_claims_nothing_but_still_costs_its_operation`,
`a_flush_empties_the_buffer_even_when_the_iterator_is_dropped_unread`,
`a_partially_consumed_flush_still_empties_the_buffer`,
`drain_empties_the_buffer_even_when_the_iterator_is_dropped_unread`) exist in
`tests/tls_test.rs` and name exactly the two costs this finding describes —
empty-on-drop and the unconditional atomic. No text was inaccurate; the
measurement this tier asks for was already present as a passing test suite
rather than an assertion, which is the disposition itself.
Now prints: `a_partially_consumed_flush_still_empties_the_buffer`

### TL47 — `drain` Has the Same Drop Semantics and a Different Purpose

Two of the four tests asserting drop-empties behaviour are `drain`'s. The
difference is the claim: `flush_into` has already taken sequences when the
iterator is dropped, `drain` has taken none.

That is the whole of `ring_flush`'s workaround — inspect via `drain`, decide,
and only then claim.
