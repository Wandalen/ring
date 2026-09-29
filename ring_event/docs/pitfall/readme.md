# pitfall

Both traps in this crate came from one field. `BytesSlot` stores a `len` beside
an array that outlives it, and the two are allowed to disagree — so a length of
zero says less than a caller expects in one direction, and the array said more
than a caller expects in the other. Read one way, a payload disappears. Read the
other, two identical slots compared unequal.

Only the first is still live. `BytesSlot`'s `Debug`, `PartialEq` and `Eq` are no
longer derived over the fields: all three are hand-written over `read()` at
`ring_slot/src/lib.rs:248-267`, so the bytes past `len` are no longer
reachable through any public observation, and the assertion this file used to
predict would fail is now written twice and passes both times.
[`002`](002_a_recycled_slot_is_not_equal_to_an_empty_one.md) records that
closure, and what the finding's reasoning kept after losing its subject.

Neither was a defect in this crate, which has no data at all; both were
properties of the shape it publishes into, surfaced by the one path it provides.
What is a defect is where the surviving trap's warning sits: it is documented
thoroughly on a trait method most readers never open and not at all on the
function they call, and its two recommended escapes are one never-instantiated
type and four crates this one has no edge to.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| [001](001_a_zero_length_payload_reads_as_nothing.md) | A Zero-Length Payload Reads as Nothing | The ambiguity, where it is documented, and whether its escapes are reachable |
| [002](002_a_recycled_slot_is_not_equal_to_an_empty_one.md) | A Recycled Slot Is Not Equal to an Empty One | Derived equality against observational equivalence, and the narrowing that closed it |

## One Field, Read Too Little and Too Much

A `BytesSlot< N >` is `[ u8; N ]` plus a `usize`. `write` sets both; `clear`
resets only the length. Everything downstream follows from that split.

Setting the length to the payload's length means a zero-byte payload sets it to
zero, which is what an untouched slot already holds — so the state "somebody
published nothing" has no representation, and `peek` answers `None` to both
questions. Leaving the array untouched on `clear` means a recycled slot carries
its last payload in fields no observation reports — so `capacity`, `len`,
`is_empty`, `read` and `drain_from` all say it is identical to a fresh slot. So
does `==`, since it was narrowed to compare `read()`; it used to be derived
over both fields and to say the opposite.

`TypedSlot` has neither trap. Its occupancy is an `Option`, so nothing is
inferred from a length, and `clear` restores the exact representation `empty()`
builds. The same `recycle`, the same `drain_from`, the same generic bodies —
different answers on the surviving trap, because the two shapes represent
absence differently. On the closed one they now agree, which is what closing
it meant.

## Where the Warnings Are and Are Not

`Peek::peek` spends eleven of the crate's one hundred and fifty-four doc lines
on the first trap: its own heading, the mechanism, why it was accepted, the
price of removing it, and two ways around it. That is the most careful passage
in the file. `drain_from` — the function the module documentation says a ring
calls — has one sentence and a doctest that reads a fresh `BytesSlot` and
asserts `None`, which is the ambiguous shape showing the ambiguous value as
though it were unambiguous.

The second trap needed no warning in the end, because it was removed instead of
documented. `assert_eq!( slot, BytesSlot::< 8 >::empty() )` after a `recycle` —
the natural assertion, and the one this corpus recommended adding — is now
written in two files rather than none. `ring_event/tests/event_test.rs`
line 228 added it deliberately, to pin `recycle`'s claim at the value level,
and `ring_slot/src/lib.rs:233` carries it as a doctest on `clear`
itself. Both pass. Writing the assertion is what showed that the contract, not
the assertion, was the thing to change.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"
echo '  -- the field split both traps come from --'
command grep -m1 -A9 -F '/// assert!( !slot.is_empty() );' ring_slot/src/lib.rs | tail -n 8
command grep -m1 -A8 -F '  pub fn write( &mut self, payload : &[ u8 ] ) -> Result< (), RingError >' ring_slot/src/lib.rs
echo '  -- and the two clears that diverge from it --'
command grep -m1 -A6 -F '    self.0.is_none()' ring_slot/src/lib.rs | tail -n 4
command grep -m1 -A6 -F '    Self::is_empty( self )' ring_slot/src/lib.rs | tail -n 4
echo '  -- doc lines spent on the first trap, against the read functions whole doc --'
command grep -m1 -A11 -F '  /// how a drain observes it.' ring_event/src/lib.rs | tail -n 11
command grep -m1 -B1 -A10 -F '/// Read `slot` — the one read path both shapes take.' ring_event/src/lib.rs
command grep -c '^ *///\|^//!' ring_event/src/lib.rs | awk '{ print "  doc lines in the crate: " $1 }'
echo '  -- the first escape it names, everywhere in the workspace --'
command grep -rn 'TypedSlot< () >\|TypedSlot::< () >' --include=*.rs */ | wc -l
echo '  -- files carrying an equality assertion against a slot value, family-wide --'
command grep -rc 'assert_eq!( *[a-z_]*, *[A-Za-z]*Slot' --include=*.rs */ | awk -F: '$2 > 0' | wc -l
echo '  -- against is_empty assertions in the two suites that would use one --'
command grep -rc 'is_empty()' --include=*.rs ring_event/tests ring_slot/tests | awk -F: '{ s += $2 } END { print "  " s }'
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| EV41 | `ring_event` | n/a — doc gap | `Peek::peek` documents the zero-length ambiguity better than anything else in the crate — its own `#` heading, eleven of the file's then one hundred and twenty-nine doc lines, the mechanism stated, the limitation called deliberate, the rejected alternative priced at "a flag byte per slot", and two escapes named — while `drain_from`, the function the module documentation frames as the ring's read entry point and the one every caller and test actually uses, carries a single sentence and no mention of it; worse, `drain_from`'s only worked example constructs a `BytesSlot< 4 >` and asserts `drain_from( &slot ) == None`, which is exactly the shape and exactly the value where the ambiguity lives, presented as the unambiguous case, so the one example a reader of the public function sees teaches the reading the limitation invalidates, and a cross-reference or a second `assert` publishing an empty slice would put the trap in the example that currently hides it |
| EV42 | `ring_event` | n/a — unadopted | Both remedies the documentation offers are outside what the crate can demonstrate or reach: `TypedSlot< () >`, called "the cheaper way to send a payload-free signal", is constructed zero times anywhere in the workspace — not in this crate's sixteen tests, not in its five doctests, not in any of the other thirty-two ring crates — so the recommended alternative to the crate's one documented trap has never been compiled, though a probe confirms it behaves exactly as claimed (fresh reads `None`, published reads `Some(())`); and the second remedy, "read it there rather than from the slot", points at the published-sequence handshake carried by `ring_publish`, `ring_consume`, `ring_mpsc` and `ring_spsc`, none of which is in this crate's two-crate dependency closure of `ring_types` and `ring_slot`, and none of which the sentence names, so a caller following correct advice has the family to search for an address the doc declines to give |
| EV43 | `ring_slot` | **latent hazard** | `BytesSlot< N >` derives `PartialEq` and `Eq` over both its fields while `Slot::clear` resets only one, so the derived equality is strictly finer than observational equivalence: a probe runs every public observation the type offers — `capacity`, `len`, `is_empty`, `read`, and `peek` through `drain_from` — against a recycled slot and a never-used one, gets agreement on all five, and gets `false` from `==`, which for a type carrying `Eq` (the trait asserting a genuine equivalence on its values) means two values no caller can tell apart are formally unequal; there is consequently no canonical representation of empty, 4,294,967,296 distinct `BytesSlot< 4 >` values sitting in the observationally-empty class, and the asymmetry is sharper still because `TypedSlot::clear` writes `self.0 = None` and restores exactly what `empty()` builds, so the same derive on the same line of the same file and the same generic `recycle` give the two shapes opposite answers with nothing at either declaration saying so |
| EV44 | `ring_slot` | n/a — coverage | The trap is entirely latent — zero equality assertions against a slot value exist in the thirty-three crates, and `ring_slot`'s own suite verifies an unchanged slot with three separate assertions on `read()`, `len()` and `is_empty()` rather than one comparison, with thirty-one `is_empty()` assertions across the two suites — but it stops being latent at the first natural use, since `assert_eq!( slot, BytesSlot::< 8 >::empty() )` after a `recycle` is both the obvious way to say "back to its initial state" and the assertion this corpus recommends adding, and it fails for a true reason with a message naming byte arrays rather than the contract; the adjacent safe form works only by accident of shape, `assert_eq!( slot, before )` against a clone putting the residue on both sides where it cancels, so the derive is at once the sharpest available detector of the `clear` gap and the likeliest way to be confused by it, and one comment on the assertion line converts a puzzle into a diagnosis |
