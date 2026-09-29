# Non-Functional Requirement: A Record Sized for a Design Not Yet Built

### Scope

**Purpose:** Record what the crate's shape is justified by — a manifest language
that does not exist yet — and which parts of the justification have and have not
been delivered.

**Responsibility:** The module comment's own Definition- and If-Missing-style
argument, the state of the consumer crate, and the one stated payoff that is
still missing.

**In Scope:** `ring_config/src/lib.rs:7-17`;
`lang_channel/src/lib.rs`.

**Out of Scope:** The non-functional properties the crate actually has are
[`non_functional_requirement/001`](001_a_record_that_allocates_nothing_and_is_read_once.md).
Why a built ring cannot report its configuration is
[`lifecycle/002`](../lifecycle/002_nothing_can_ask_a_built_ring_what_it_was_configured_with.md).

---

## What Justifies the Shape

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- the consumer this crate says it is sized for --'
command grep -m1 -A2 -F '//! set of legal configurations enumerable instead of "whatever someone wrote a' ring_config/src/lib.rs
echo '  -- lines in that consumer crate, then mentions of the record inside it --'
command grep -c '' lang_channel/src/lib.rs
command grep -c 'RingConfig' lang_channel/src/lib.rs || true
```

Live output:

```
  -- the consumer this crate says it is sized for --
//! set of legal configurations enumerable instead of "whatever someone wrote a
//! constructor for", and it means the manifest language that eventually
//! describes channels describes exactly this record with no translation layer.
  -- lines in that consumer crate, then mentions of the record inside it --
41
0
```

---

### RC35 — The Justification for the Shape Is a Consumer That Does Not Exist Yet

The crate's module comment gives the record two reasons to be a record rather than
a set of constructors. The first is enumerability. The second is that "the
manifest language that eventually describes channels describes exactly this record
with no translation layer." It names all five fields — capacity, wait strategy,
overflow policy, expected producer count, batch size — so the shape was specified
before any consumer for it existed.

That consumer is `lang_channel`. Its `src/lib.rs` is forty-one lines and mentions
`RingConfig` zero times.

**Finding.** So three of the record's five fields have no reader
([`item/001`](../item/001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md))
and the reason they are carried anyway is written down in the module comment
itself, in future tense. That is unusually good practice — the crate is not
pretending the fields are used — and it is worth recording precisely because it
reframes every unused-field finding in this corpus.

`wait`, `producers` and `batch` are not oversights. They are the specified
vocabulary of a language that has not been written, held in a shape chosen so the
language will need no translation layer when it arrives. What the corpus can
record is that the bet has been outstanding long enough for the crate to be
finished, tested, and depended on by eleven manifests while its justifying
consumer is a forty-one-line skeleton.

---

### RC36 — Reading Back a Built Ring's Configuration Is Only Partly Possible

Without this record, constructors would sprout everywhere, one per combination
someone needed, with no single place to add a knob or to read what a given ring
was built with. Two of those symptoms are cured: there is one constructor and
four setters, and the set of legal configurations is exactly what the type
admits. The third is in two halves, and only one half is cured. Adding a knob has
a single place — a sixth field. Reading what a given ring was built with does
not.

A built `ring_core::Ring` reports `capacity()`, `overflow()` and `backend()`;
`wait` and `batch` are recoverable from nothing, and `backend()` is a tag rather
than the configured producer count
([`lifecycle/002`](../lifecycle/002_nothing_can_ask_a_built_ring_what_it_was_configured_with.md)).

**Finding.** Read-back of a built ring's configuration is only partly delivered,
and nothing records that. That is worth having written down for two reasons.

The first is that the missing half is the cheap one. `ring_core::Ring` already
stores `overflow`; storing the whole thirty-two-byte record instead of one byte of
it would make `Ring::config()` possible, and the type is `Copy`, so the change is
a field swap and one accessor. Nothing in the family has asked for it, which is
why it is recorded here as an observation rather than proposed.

The second is that the crate's own module comment is the closest thing to an
acceptance test for this shape, and reading it against the implementation is the
only way to notice the gap: the comment argues for one enumerable record in place
of scattered constructors, and delivers that, but neither the crate nor its tests
note that reading a built ring's configuration back out is still only partial.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`non_functional_requirement/001`](001_a_record_that_allocates_nothing_and_is_read_once.md) | The properties the crate does have, and what enforces them |
| [`lifecycle/002`](../lifecycle/002_nothing_can_ask_a_built_ring_what_it_was_configured_with.md) | Why the third symptom survives |
| [`item/001`](../item/001_five_fields_and_the_one_another_crate_keeps_a_copy_of.md) | The three fields the bet is held in |
| [`decisions/001`](../decisions/001_capacity_is_the_one_parameter_that_is_not_a_with.md) | The record-versus-constructors choice 180 asked for |

### Sources

| Fact | Where |
|------|-------|
| The module comment's two reasons, and its "eventually" | `ring_config/src/lib.rs:7-17` |
| `lang_channel` at forty-one lines, naming the record zero times | `lang_channel/src/lib.rs` |
| The three readings a built ring exposes | `ring_core/src/lib.rs:214`, `:227`, `:244` |

### Tests

| Test | Covers |
|------|--------|
| `every_named_field_is_carried` | That all five fields 180 named survive construction |
| `defaults_are_the_documented_ones` | The values the unread fields hold in practice |
| `the_record_is_copy_and_compares_by_value` | The property that would make a `Ring::config()` a field swap |
