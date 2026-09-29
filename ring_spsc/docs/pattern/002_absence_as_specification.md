# Pattern: Absence as Specification

### Scope

- **Purpose**: Record the crate's own documentation technique — stating what it depends on by listing what it does not — and the conditions under which that technique is honest.
- **Responsibility**: The pattern, its one worked instance, and what makes it fail.
- **In Scope**: `src/lib.rs`'s module documentation and the four absent crates it enumerates.
- **Out of Scope**: The absent stamp array, which is storage rather than dependency (→ [`../data_structure/002`](../data_structure/002_the_absent_stamp_array.md)).

### The Pattern

The crate's module documentation opens with a heading — *What single-producer
buys, stated as a dependency list* — and then names four sibling crates that are
**not** dependencies, with a one-line reduction argument for each:

| Absent crate | Its question | Why it collapses at cardinality one |
|--------------|--------------|--------------------------------------|
| `ring_gating` | Minimum over a set of consumer cursors | The minimum over one value is that value |
| `ring_claim` | Compare-exchange the producer cursor | Nothing races; a plain store suffices |
| `ring_publish` | Spin until the frontier reaches this range | The frontier is always already there |
| `ring_consume` | Barrier over the cursors a batch depended on | The barrier has one member, and it is this end's own cursor |

```sh
cd "$(git rev-parse --show-toplevel)"/ring_spsc
printf 'named as absent:   '; grep -oE 'ring_(gating|claim|publish|consume)' src/lib.rs | sort -u | tr '\n' ' '; echo
printf 'actually absent:   '
for x in ring_gating ring_claim ring_publish ring_consume; do
  grep -q "$x" Cargo.toml && echo -n "$x=PRESENT " || echo -n "$x=absent "
done; echo
```

Live output:

```
named as absent:   ring_claim ring_consume ring_gating ring_publish 
actually absent:   ring_gating=absent ring_claim=absent ring_publish=absent ring_consume=absent 
```

### The Rules

1. **Every named absence must be a real crate**, so the claim is falsifiable by
   a one-line manifest check rather than by reading the argument.
2. **Each must carry its own reduction**, not a shared "not needed here" — the
   four arguments above are four different arguments, and a reader who accepts
   one has not thereby accepted the others.
3. **The reduction must name the parameter it reduces on.** All four here reduce
   on producer or consumer cardinality being one. That is what makes the list a
   specification of the crate's thesis rather than a list of things it happens
   not to import.

### When It Fails

When the absence is contingent rather than structural. A crate that omits a
dependency because it has not needed it *yet* documents nothing by saying so —
the list becomes a snapshot that goes stale the first time the omission ends,
and unlike a present dependency, nothing breaks to signal it.

### Sources

| File | Relationship |
|------|-----------------|
| `src/lib.rs` | The module documentation this pattern is abstracted from |
| `Cargo.toml` | The five dependencies against which the four absences are checkable |
| `001_validate_simple_before_general.md` | The sibling pattern — this crate's role in the family, of which the absence list is the evidence |

### SP42 — All Four Named Absences Check Out

```sh
cd "$(git rev-parse --show-toplevel)"
for x in ring_gating ring_claim ring_publish ring_consume; do
  printf '%-14s exists=%s  dep_of_spsc=%s\n' "$x" \
    "$( [ -d $x ] && echo yes || echo no )" \
    "$( grep -c "$x" ring_spsc/Cargo.toml )"
done
```

Live output:

```
ring_gating    exists=yes  dep_of_spsc=0
ring_claim     exists=yes  dep_of_spsc=0
ring_publish   exists=yes  dep_of_spsc=0
ring_consume   exists=yes  dep_of_spsc=0
```

Four for four. That is rule 1 of the pattern satisfied, and it is the rule that
makes the list falsifiable — a named absence that is not a real crate cannot be
checked and cannot be wrong.

### SP43 — The Sibling Does Not Carry the Mirror List

The absence list works because each entry names the parameter it reduces on. The
mirror — *what multi-producer costs, stated as a dependency list* — is exactly as
writable and is not written: `ring_mpsc`'s head lists its eight dependencies
without saying which three exist because producers can race.

Recorded as a gap rather than proposed as a task: the technique is this crate's
and its value in the sibling is a judgement the sibling's own docs should
make.
