# Integration: One Declarer, and It Is a Dev-Dependency

### Scope

**Purpose:** Record every edge into and out of this crate, place it among the
family's unconsumed crates, and record what two other crates' module
documentation says about it while declaring no edge that would let them use it.

**Responsibility:** The manifest in both directions, the one declarer and its
section, every code reference outside the crate, and the family-wide census of
production consumers.

**In Scope:** `ring_event/Cargo.toml`; `ring_tls/Cargo.toml:15`;
`ring_tls/tests/tls_test.rs:22`; `ring_tls/src/lib.rs:33`;
`ring_batch/src/lib.rs:29`.

**Out of Scope:** What the ring does instead is
[`integration/002`](002_the_ring_writes_slots_without_this_crate.md). What a
consumer would gain by arriving is
[`decisions/001`](../decisions/001_the_write_half_on_the_payload_the_read_half_on_the_slot.md).

---

## Every Edge, in Both Directions

```sh
cd "$(git rev-parse --show-toplevel)"
# `command grep` bypasses the shell shim to the ordered GNU binary; the shim
# is a parallel ugrep that emits hits in completion order
echo '  -- what this crate depends on --'
sed -n '/^\[dependencies\]/,/^$/p;/^\[dev-dependencies\]/,/^$/p' ring_event/Cargo.toml
echo '  -- and what declares it, in which section --'
command grep -r 'ring_event' --include=Cargo.toml | command grep -v '^ring_event/'
command grep -m1 -A3 -F '[dev-dependencies]' ring_tls/Cargo.toml
echo '  -- every reference to it in code, anywhere outside the crate --'
command grep -r 'ring_event' --include=*.rs | command grep -v '^ring_event/'
echo '  -- ring crates no manifest declares as a production dependency --'
for c in ring_*/; do
  b=$( basename $c )
  p=0
  for m in $( command grep -rl "^$b = " --include=Cargo.toml 2>/dev/null | command grep -v "^$b/" ); do
    awk -v dep="^$b = " '/^\[/{ d = ( $0 == "[dev-dependencies]" ) } $0 ~ dep && !d { found = 1 } END{ exit !found }' $m && p=$(( p + 1 ))
  done
  [ "$p" = 0 ] && echo "  $b"
done
echo '  -- out of how many --'
ls -d ring_*/ | wc -l
```

Live output:

```
  -- what this crate depends on --
[dependencies]
ring_types = { path = "../ring_types" }
ring_slot = { path = "../ring_slot" }

[dev-dependencies]
ring_store = { path = "../ring_store" }

  -- and what declares it, in which section --
Cargo.toml:  "ring_event",
ring_bench/Cargo.toml:#                through no path measured here — see ring_event's own
ring_tls/Cargo.toml:ring_event = { path = "../ring_event", version = "0.1.0" }
[dev-dependencies]
ring_store = { path = "../ring_store", version = "0.1.0" }
ring_event = { path = "../ring_event", version = "0.1.0" }
ring_slot = { path = "../ring_slot", version = "0.1.0" }
  -- every reference to it in code, anywhere outside the crate --
ring_batch/src/lib.rs://! caller owns; what goes in them is `ring_store`'s and `ring_event`'s
ring_tls/tests/tls_test.rs:use ring_event::{ drain_from, publish_into };
ring_tls/src/lib.rs://! is `ring_store`'s and `ring_event`'s business. A `TlsBuffer` that knew how
  -- ring crates no manifest declares as a production dependency --
  ring_bench
  ring_consume
  ring_debug
  ring_event
  ring_poll
  ring_publish
  ring_testkit
  ring_trace
  -- out of how many --
33
```

---

### EV17 — Nothing in the Family Depends on This Crate to Build

Outbound, the crate is minimal and honest: `ring_types` and `ring_slot` as
dependencies, `ring_store` for tests. Inbound, exactly one manifest in 33 names
it — `ring_tls/Cargo.toml:15` — and that line sits under `[dev-dependencies]`,
between `ring_store` and `ring_slot`. Remove this crate from the workspace and
every other crate still compiles; only `ring_tls`'s test suite breaks.

The code census agrees. Three references exist outside the crate and only one is
an import: `ring_tls/tests/tls_test.rs:22`, `use ring_event::{ drain_from,
publish_into };`. The other two are module-documentation prose.

Eight of the 33 crates have no production consumer, so being unconsumed is not
by itself remarkable in this family. What distinguishes this one is the shape of
its claim: `ring_bench`, `ring_debug`, `ring_testkit` and `ring_trace` are tools,
and being reached only from tests is what tools are for. This crate's module
documentation says something different — that it "is what makes the path
literally one path", and that "the ring's publish and drain call those."

**Finding.** Both halves of that sentence are unrealized. No ring's publish calls
`publish_into` and no ring's drain calls `drain_from`, because no ring declares an
edge that would let it. The crate is not a tool used by tests; it is a load-bearing
component of a design in which nothing yet loads it, and nothing anywhere records
the difference between the two.

**Disposition:** applied at
[`integration/002`](002_the_ring_writes_slots_without_this_crate.md), which owns
the correction. The module documentation quoted above no longer states as fact
that "the ring's publish and drain call those"; it now states the weaker,
accurate claim — the generic path exists and a ring's publish and drain *could*
call those — and names the current reality directly: `ring_core`, the crate that
assembles a ring, calls neither today, reaching `TypedSlot::set` and
`TypedSlot::take` instead. The finding above is kept as the record of what the
documentation said when this instance was written; its closing clause — that
nothing anywhere records the difference — is precisely what that fix retired.

---

### EV18 — Two Crates Delegate to It in Prose and Neither Can Call It

`ring_batch/src/lib.rs:29` and `ring_tls/src/lib.rs:33` both hand the same
responsibility to this crate in almost the same words: what goes in the slots "is
`ring_store`'s and `ring_event`'s business."

Neither crate can act on that. `ring_tls` declares the edge only as a
dev-dependency, so the delegation holds in its tests and nowhere else.
`ring_batch` declares no edge at all — its manifest does not mention this crate,
so the sentence names a boundary its own code has no way to cross.

**Finding.** The statements are defensible as architecture and misleading as
navigation. Read as a layering claim — *slot contents are not our concern* — both
are true and worth saying, and `ring_tls:33` goes on to explain the alternative it
rejected: "A `TlsBuffer` that knew how" would fold slot knowledge into the wrong
crate. Read as a pointer, they send a reader looking for a call that does not
exist and cannot exist without a manifest change neither crate has made.

The fix is a clause, not a redesign. Naming the delegation as intended rather than
current — *slot contents are `ring_event`'s business once a ring adopts it* —
costs four words and turns a reader's dead end into the crate's actual status.
As written, the two sentences are the strongest evidence in the workspace that
this crate is wired in, and they are prose.

---

### Related

| Instance | Relationship |
|----------|--------------|
| [`integration/002`](002_the_ring_writes_slots_without_this_crate.md) | What `ring_core` writes slots with instead |
| [`decisions/001`](../decisions/001_the_write_half_on_the_payload_the_read_half_on_the_slot.md) | The extension points nothing has used either |
| [`api/002`](../api/002_a_result_one_impl_can_never_return.md) | Why the discarded displaced value has no victim yet |
| [`data_structure/002`](../data_structure/002_what_clear_leaves_behind.md) | A hazard whose reach is bounded by the same census |

### Sources

| Fact | Where |
|------|-------|
| Two dependencies, one dev-dependency | `ring_event/Cargo.toml` |
| The single declarer, under `[dev-dependencies]` | `ring_tls/Cargo.toml:13-16` |
| The single import, in a test | `ring_tls/tests/tls_test.rs:22` |
| Eight of 33 crates with no production consumer | Census above |
| "a ring's publish and drain *could* call those" | `ring_event/src/lib.rs:17-18` |
| The two delegating sentences | `ring_batch/src/lib.rs:29`, `ring_tls/src/lib.rs:33` |

### Tests

| Test | Covers |
|------|--------|
| `both_shapes_land_in_storage_through_the_same_two_calls` | The nearest thing to an integration, inside this crate's own suite |
| `a_recycled_storage_slot_stops_returning_the_previous_lap` | A `ring_shutdown` property asserted here rather than there |
| `a_typed_payload_survives_storage_byte_identically` | Storage reached through `ring_store`, the dev-dependency |
