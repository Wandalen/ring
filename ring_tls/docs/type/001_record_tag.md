# Type: Record Tag

### Scope

- **Purpose**: Define the per-record discriminator that makes an otherwise opaque byte region walkable, and state the per-record overhead it costs.
- **Responsibility**: Fix what the tag is, what it is not, its validation rules, and the width trade — while keeping the tag *values* out, since those belong to the consumer.
- **In Scope**: The tag as a value type: its role, its width, its relationship to record length, its validation.
- **Out of Scope**: The tag's vocabulary — which values mean what — which is the consumer's and deliberately not this crate's (→ [`../readme.md`](../readme.md)); the region layout storing it, undecided; the payload's own structure.

### Definition

A **record tag** is a small integer written immediately before each record's
payload, identifying what the following bytes are. It is the mechanism that
turns the buffer from an opaque byte region into a **walkable sequence**: the
consolidator reads a tag, learns the record's shape, advances past it, and
reads the next tag.

Without it the region is unreadable by anyone but its writer, since a
consolidator receiving a bump-allocated byte range has no way to know where
one record ends and the next begins.

| Property | Value |
|----------|-------|
| Written by | The appending thread, as part of the append (→ [Tagged Record Bump Append](../algorithm/001_tagged_record_bump_append.md)) |
| Read by | The consolidator, once per record during drain |
| Meaning of a value | **Consumer's.** This crate assigns none |
| Relationship to length | Either the tag implies a fixed length, or a length field accompanies it — undecided, see below |

**What the tag is not: a type identifier this crate understands.** This crate
is payload-agnostic by design — its factored-out contribution is "the append
discipline alone, never the payload vocabulary either consumer encodes into
it" (`src/lib.rs`). So the tag is an opaque `u8` or `u16` from this crate's
perspective, and the temptation to give it a meaning here — an enum of known
record kinds — must be resisted, because doing so would pull one consumer's
vocabulary into a crate deliberately shared between consumers with different
ones.

**Tag and length are one design question, not two.** Two shapes:

- **Tag implies length.** Each tag value maps to a fixed record size the
  consumer knows. Cheapest possible framing — 1 byte of overhead per record.
  But it forbids variable-length records entirely, and this crate cannot
  validate it because it does not know the mapping.
- **Tag plus explicit length.** Every record carries `(tag, len, payload)`.
  Costs 2–4 bytes more per record. Makes the region walkable *by this crate*
  with no consumer knowledge at all, which means the consolidator can skip an
  unrecognized record instead of failing.

**Undecided, and the second is what the crate's own agnosticism argues for.**
A crate that declines to know the payload vocabulary cannot walk a
tag-implies-length region without asking the consumer for the mapping —
which reintroduces exactly the coupling the agnosticism avoids. Recording
this here rather than in the region layout is deliberate: the layout is
undecided pending a future benchmark verdict, but *this* half of it follows
from a design position already taken.

### Validation

| Rule | Statement | Checked where |
|------|-----------|---------------|
| V1 | Every record in a buffer is preceded by exactly one tag | Structural — the append path writes both or neither |
| V2 | A tag's value is never interpreted by this crate | Enforced by having no interpretation to apply |
| V3 | Tag plus payload does not straddle the region's end | The append's bounds check, before the write |
| V4 | Under the tag-plus-length shape, `len` does not exceed the bytes remaining in the occupied prefix | Drain-time check |
| V5 | The tag's alignment does not force padding that breaks the payload's own alignment | **Open** — see below |

**V4 is the rule that distinguishes the two shapes' failure behaviour.**
Under tag-plus-length, a corrupt length is *detectable* at drain time — it
would point past the watermark, and the consolidator can stop rather than
read garbage. Under tag-implies-length there is nothing to check: a wrong tag
silently advances the walk by the wrong stride and every subsequent record in
that buffer is misread. One shape degrades loudly and the other silently,
which is a stronger argument for the explicit length than the byte cost is
against it.

**V5 is the interaction that gets discovered late.** A `u8` tag followed by a
payload requiring 8-byte alignment means 7 bytes of padding per record — the
tag's 1-byte "cheapness" costing 8. Aligning the tag itself to the payload's
requirement, or grouping tags separately from payloads, both change the
region layout, which is still an open question. This crate's alignment
preconditions are stated in
[POD and Pointer-Free Payloads](../non_functional_requirement/002_pod_pointer_free_payloads.md);
the tag is the thing most likely to violate them by accident, and naming that
here is the point of V5 existing as a rule rather than a note.

**Open at this grain.** The tag's width. `u8` gives 256 record kinds, which
is generous for a single consumer's intent vocabulary and restrictive if
several consumers share a buffer format. `u16` doubles the per-record
overhead for records that may themselves be only a few bytes. The right
answer depends on the consumer's vocabulary size, which this crate does not
know by construction — so it is a configuration point rather than a constant,
and possibly a type parameter.

### Algorithms

| File | Relationship |
|------|--------------|
| [../algorithm/001_tagged_record_bump_append.md](../algorithm/001_tagged_record_bump_append.md) | Writes the tag as part of each append; V1 and V3 are its obligations |

### APIs

| File | Relationship |
|------|--------------|
| [../api/001_writer_append_surface.md](../api/001_writer_append_surface.md) | Takes the tag as a parameter; its width is this type's open question |
| [../api/002_consolidator_read_surface.md](../api/002_consolidator_read_surface.md) | Decodes tags during drain; V4's detectability decides whether it can fail loudly |

### Data Structures

| File | Relationship |
|------|--------------|
| [../data_structure/001_thread_local_append_log.md](../data_structure/001_thread_local_append_log.md) | The region tags are interleaved into; the tag-versus-length shape is part of its undecided layout |

### Non-Functional Requirements

| File | Relationship |
|------|--------------|
| [../non_functional_requirement/002_pod_pointer_free_payloads.md](../non_functional_requirement/002_pod_pointer_free_payloads.md) | The alignment preconditions V5 says the tag most easily violates |

### Types

| File | Relationship |
|------|--------------|
| [002_epoch.md](002_epoch.md) | The other value type; a per-record epoch would sit alongside the tag and compound its overhead |

### Sources

| File | Relationship |
|------|--------------|
| `src/lib.rs` | States the payload-agnosticism V2 formalizes — "never the payload vocabulary either consumer encodes into it" |

### Tests

| File | Relationship |
|------|--------------|
| `tests/record_tag_test.rs` (to create) | A buffer of mixed-length records walks correctly end to end under the chosen framing — V1 and V4 |
| `tests/record_tag_test.rs` (to create) | A corrupt length is detected at drain rather than misreading subsequent records — the loud-versus-silent degradation argument, asserted |

### TL48 — The Record Tag Type Was Never Declared

A typed `Vec< T >` needs no discriminant — `T` is the type. The tag exists in
the specified design because a byte region has to be walked by something that
does not know what it is reading.

**Disposition:** declined — the Record Tag value type, its width trade-off,
and V1-V5 validation rules this instance specifies govern a byte-region
framing the crate never built; the typed `Vec<T>` needs no discriminant at
all. One of the nineteen pre-implementation instances
`../decisions/001_the_corpus_specifies_an_api_the_crate_did_not_build.md`
catalogs, whose rewrite-vs-supersede resolution is still open.
