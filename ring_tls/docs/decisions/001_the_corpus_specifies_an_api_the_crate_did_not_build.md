# Decision: The Corpus Specifies an API the Crate Did Not Build

- **Status**: Open
- **Deciders**: This crate's maintainers, pending a future benchmark verdict
- **Turns on**: Whether the nineteen pre-implementation instances are rewritten against the built crate, superseded in place, or kept as the record of a design that lost

### Context

This crate's nineteen existing doc instances specify a per-thread byte region:
records appended as a tag byte plus little-endian operands behind a monotonic
bump pointer, a `reset` that rewinds it, an epoch counter dating each
consolidation cycle, a thread registry the consolidator walks, and a
`seal`/`drain`/`reset` read surface.

`src/lib.rs` declares none of it.

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls
printf 'names the corpus specifies, and how many src/lib.rs declares:\n'
for n in append append_with reset seal remaining free_capacity register consolidate_all; do
  printf '  %-16s %s\n' "$n" "$( grep -vE '^\s*(//|///|//!)' src/lib.rs | grep -cE "fn $n\b" )"
done
printf 'names src/lib.rs does declare:\n  '
grep -vE '^\s*(//|///|//!)' src/lib.rs | grep -oE 'fn [a-z_]+' | sed 's/^fn //' | sort -u | tr '\n' ' '
echo
```

Live output:

```
names the corpus specifies, and how many src/lib.rs declares:
  append           0
  append_with      0
  reset            0
  seal             0
  remaining        0
  free_capacity    0
  register         0
  consolidate_all  0
names src/lib.rs does declare:
  capacity claim discard drain flush_into is_empty is_full len next push size_hint with_capacity 
```

Eight specified, none declared. Twelve declared, and the nineteen instances
name none of them.

### It Was Noticed, and Written Down Somewhere Else

```sh
cd "$(git rev-parse --show-toplevel)"
grep -A 5 'pre-implementation surface specified' ring_flush/src/lib.rs
```

Live output:

```
//! `ring_tls`'s pre-implementation surface specified `seal`/`drain`/`reset` as
//! three calls, and what was built is `flush_into` — claim and drain fused,
//! emptying the buffer whether or not the records land. That shape cannot
//! satisfy this crate's O3/O4: a rejected batch would already be gone.
//! `TlsBuffer::drain` was added there so the check can happen before the buffer
//! is touched (→ `docs/algorithm/002_sequencing_seal_drain_reset.md`).
```

A consumer crate's module documentation states the divergence exactly, names
both shapes, explains why the built one cannot satisfy its own requirement, and
records that `TlsBuffer::drain` was added to `ring_tls` to close the gap.

**That paragraph is the only place in the repository where the two designs are
compared, and it is in the crate that had to work around the result.** The
crate that changed has nineteen instances that do not mention it.

### Readings

**1 — Rewrite the nineteen against the built crate.** Honest, and expensive:
the byte-region material is real design work that was correct for the consumer
it was written for. Rewriting deletes it rather than relocating it.

**2 — Supersede in place.** Mark each affected instance as specifying the
pre-implementation surface, and add the built-crate instances alongside. Cheap,
and leaves a corpus where a reader must check every heading against a status
line to know which crate it is about.

**3 — Keep them as the design record they are, and move them.** The byte-region
design is still live for a prospective consumer's own append-path replacement
(→ [`../integration/002`](../integration/002_prospective_consumer_adoption.md)),
which is where the tags and epochs are actually needed. Under this reading the
nineteen are not stale — they are filed under the wrong crate.

### Decision

Open. What is filed here is the measurement and the three readings. The choice
is a future pass's because reading 3 moves material into a crate outside this
family, and reading 1 deletes design work that a second consumer may still need.

### Consequences

Until it is ruled, twelve of this crate's doc instances describe types that do
not exist, and nothing in the corpus says so except this instance. The five
corpus checkers cannot detect it: none of them compares a doc claim to a
declaration, and an instance with no `sh` block at all passes `recipes.py`
unexamined.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | Declares the twelve functions the corpus does not name |
| `../../../ring_flush/src/lib.rs` | Records the divergence, in the consumer rather than here |
| `../../readme.md` | States the built shape, contradicting `../data_structure/001` |

### TL17 — Eight Specified Names, None Declared; Twelve Declared, None Specified

The disjointness is the finding. A partial overlap would be ordinary drift —
a renamed argument, a dropped variant. Two vocabularies with an empty
intersection means no instance was revisited after the code landed, not one.

**Disposition:** declined — this finding restates the Context section's own
live count accurately; there is no false claim in this document to correct.
Resolving the underlying disjointness — rewrite the nineteen, supersede them
in place, or relocate them toward a prospective consumer's own append-path
need — is this very document's own open Decision, reserved for a future pass
per its Deciders field, not an action a documentation-disposition pass can
take on its behalf.

### TL18 — The Recipe Gate Cannot See This Because the Nineteen Publish No Recipe

```sh
cd "$(git rev-parse --show-toplevel)"/ring_tls/docs
printf 'pre-implementation instances:                %s\n' \
  "$( ls algorithm/001*.md api/00*.md data_structure/001*.md integration/00*.md \
       invariant/00*.md lifecycle/00*.md non_functional_requirement/00*.md \
       pattern/00*.md pitfall/001*.md type/00*.md 2>/dev/null | wc -l )"
printf 'of those, bodies publishing a sh block:      %s\n' \
  "$( for f in algorithm/001*.md api/00*.md data_structure/001*.md \
                integration/00*.md invariant/00*.md lifecycle/00*.md \
                non_functional_requirement/00*.md pattern/00*.md \
                pitfall/001*.md type/00*.md; do
        sed '/^### TL[0-9]/,$d' "$f" | grep -q '^```sh' && echo "$f"
      done | wc -l )"
printf 'sh blocks now present, all from TL findings: %s\n' \
  "$( grep -l '^```sh' algorithm/001*.md api/00*.md data_structure/001*.md \
       integration/00*.md invariant/00*.md lifecycle/00*.md \
       non_functional_requirement/00*.md pattern/00*.md pitfall/001*.md \
       type/00*.md 2>/dev/null | wc -l )"
```

Live output:

```
pre-implementation instances:                19
of those, bodies publishing a sh block:      3
sh blocks now present, all from TL findings: 9
```

The scan is deliberately body-scoped — `sed '/^### TL[0-9]/,$d'` cuts each file
at its first finding heading. Without that cut the same scan reports eight, and
all eight are recipes this pass appended: measuring after the fix and reporting
it as the state before is how a finding falsifies its own claim.

An instance whose body has no `sh` block is not reported by any of the five
checkers — it is simply never executed against anything. So a corpus can be
entirely fictional and read as clean: `vocabulary.py` and `citations.py` both
returned zero problems on these nineteen.

**The gate measures whether published claims are checkable, not whether claims
were published.** That is the right scope for it and the wrong thing to rely on
alone.

### TL19 — The Divergence Is Recorded in the Crate That Had to Work Around It

A reader of `ring_tls/docs/` has no path to that paragraph — nothing in the
nineteen instances links to `ring_flush`, and the divergence is not the kind of
thing a reader thinks to search a consumer for.

The knowledge was not lost. It was written down, accurately, in the one place
where the person who discovered it needed it.

### TL20 — One Function Was Added to This Crate for a Consumer and Documented There

`drain` is the twelfth function, and the reason for it is a requirement
belonging to another crate: `ring_flush` must decide whether a batch is
acceptable *before* the buffer is emptied, which `flush_into` makes impossible.

The built surface therefore grew from consumer pressure rather than from the
corpus — and the corpus, which specified `drain` as part of a triple, is
accidentally right about the name and wrong about everything around it.
