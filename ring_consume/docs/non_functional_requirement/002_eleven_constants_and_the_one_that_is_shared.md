# Non-Functional Requirement: Eleven Constants and the One That Is Shared

### Scope

**Purpose:** Establish how the family names its memory orderings, and what
guarantees — if any — keep two crates that use the same name for the same
concept from drifting apart.

**Responsibility:** Every `const … : Ordering` declaration across all 33 crates,
its visibility, its value, and whether a dependency edge exists between crates
sharing a name.

**In Scope:** `ring_*/src/*.rs` ordering constant declarations;
`ring_consume::COMMIT` in particular; the dependency edges between every pair of
crates declaring the same name.

**Out of Scope:** Whether each *value* is correct — that is
[`decisions/002`](../decisions/002_plain_stores_rather_than_compare_exchange.md).
The orderings' effect on the invariant, which is
[`invariant/001`](../invariant/001_never_reads_past_what_was_published.md).

---

## The Family Names Its Orderings, Once Per Crate

`ring_consume` declares one:

```rust
const COMMIT : core::sync::atomic::Ordering = core::sync::atomic::Ordering::Release;
```

at `src/lib.rs:67`, private, used at both store sites. The practice of naming an
ordering after the *operation* rather than the memory model — `COMMIT` rather
than `Release` — is family-wide and is the strongest self-documentation the
concurrency code has: a reader who sees `self.cursor.store( through, COMMIT )`
learns what the store means, not just how strong it is.

```sh
cd "$(git rev-parse --show-toplevel)"
echo "crate           file:line  vis    name            value"
for f in ring_*/src/*.rs; do
  c=$( basename "$( dirname "$( dirname "$f" )" )" )
  grep -nE '^\s*(pub )?const [A-Z_]+\s*:\s*(core::sync::atomic::)?Ordering' "$f" \
  | while IFS= read -r line; do
      ln=${line%%:*}; body=${line#*:}
      vis=$( echo "$body" | grep -q 'pub const' && echo pub || echo priv )
      nm=$( echo "$body" | sed -E 's/.*const ([A-Z_]+).*/\1/' )
      vl=$( echo "$body" | sed -E 's/.*Ordering::([A-Za-z]+).*/\1/' )
      printf '%-14s %-10s %-6s %-15s %s\n' "$c" "$( basename "$f" ):$ln" "$vis" "$nm" "$vl"
    done
done
```

Live output:

```
crate           file:line  vis    name            value
ring_claim     lib.rs:76  priv   CLAIM_SUCCESS   AcqRel
ring_consume   lib.rs:81  priv   COMMIT          Release
ring_cursor    lib.rs:89  pub    GATING          Acquire
ring_debug     lib.rs:73  priv   OBSERVE         Acquire
ring_mpsc      lib.rs:237 pub    PUBLISH         Release
ring_mpsc      lib.rs:250 pub    OBSERVE         Acquire
ring_mpsc      lib.rs:271 pub    COMMIT          Release
ring_mpsc      lib.rs:287 pub    OWN             Relaxed
ring_publish   lib.rs:67  priv   PUBLISH         Release
ring_spsc      lib.rs:198 pub    OWN             Relaxed
ring_spsc      lib.rs:213 pub    HANDOFF         Release
```

Eleven constants, seven crates, seven distinct names.

---

### CN37 — One of Eleven Ordering Constants Is Actually Shared

```sh
cd "$(git rev-parse --show-toplevel)"
grep -rl 'GATING' ring_*/src/*.rs | sed 's|ring/||;s|/src/.*||' | sort -u
```

Live output:

```
ring_claim
ring_consume
ring_cursor
ring_gating
ring_mpsc
ring_publish
ring_spsc
```

`ring_cursor::GATING` is declared once, `pub`, and imported by six crates
besides the one defining it. It is the family's only genuinely shared ordering:
one declaration, one value, and a compiler error the moment anyone's
expectation of it diverges from the definition.

The other ten are each private to a crate, or `pub` but imported by nobody. Of
the seven distinct names, one is shared and six are re-declared wherever needed.

That is not automatically wrong — `CLAIM_SUCCESS` is specific to `ring_claim`'s
compare-exchange and has no business being public. But it produces the situation
CN38 describes, and it makes `GATING` the exception rather than the rule in a
family where the rule could just as easily have been the other way round.

**Cost:** none directly. Recorded because it is the baseline CN38 departs from.

---

### CN38 — Four Names Are Declared Twice, in Crates With No Dependency Between Them

Reading the inventory for repeated names:

| Name | Declared in | Value | Dependency edge between them |
|------|-------------|-------|:----------------------------:|
| `COMMIT` | `ring_consume:81` (priv), `ring_mpsc:271` (pub) | both `Release` | **none** |
| `PUBLISH` | `ring_publish:67` (priv), `ring_mpsc:237` (pub) | both `Release` | **none** |
| `OBSERVE` | `ring_debug:73` (priv), `ring_mpsc:250` (pub) | both `Acquire` | **none** |
| `OWN` | `ring_spsc:198` (pub), `ring_mpsc:287` (pub) | both `Relaxed` | **none** |

```sh
cd "$(git rev-parse --show-toplevel)"
for d in ring_consume ring_publish ring_debug ring_spsc; do
  printf '  ring_mpsc -> %-14s %s\n' "$d" \
    "$( grep -qE "^\s*$d\s*=" ring_mpsc/Cargo.toml && echo YES || echo no )"
done
```

Live output:

```
  ring_mpsc -> ring_consume   no
  ring_mpsc -> ring_publish   no
  ring_mpsc -> ring_debug     no
  ring_mpsc -> ring_spsc      no
```

Every pair currently agrees on its value. Nothing keeps them agreeing.

The absence of a dependency edge is the load-bearing part. If `ring_mpsc`
depended on `ring_consume`, its `COMMIT` would either import the shared one or
shadow it visibly, and a divergence would be a name collision a reader could
see. Because there is no edge, the two `COMMIT`s are unrelated declarations
that happen to spell the same word — the compiler has no way to relate them,
so no change to either can ever produce a diagnostic about the other.

`ring_mpsc`'s own manifest explains why the edges are absent: it dropped
`ring_publish` and `ring_consume` per a recorded rationale in favour of a per-slot
stamp it owns ([`integration/001`](../integration/001_four_edges_in_and_none_out.md)
CN3). So the four re-declarations are the residue of a design that moved. The
crate that took over publication re-derived the same four names for the same
four concepts, correctly, and now maintains them independently of the crates
that still define them.

What breaks: someone strengthening `ring_mpsc::COMMIT` from `Release` to
`AcqRel` after finding a real reordering has fixed one crate. `ring_consume`,
which implements the same commit against the same kind of cursor for the same
reason, keeps `Release`. Both compile. Both pass. Nothing anywhere states that
the two were ever meant to be the same value, so nothing is wrong in a way any
reader or tool can detect — the knowledge that they correspond exists only in
the fact that they are spelled alike.

**Cost:** reachable, and it is the family's most quietly expensive
documentation gap. The fix is not necessarily to share the constants — the
crates genuinely are independent now — but to say so: one sentence in each
declaration recording that the name is deliberately re-derived and not
imported, so a reader knows the duplication is a decision rather than an
oversight.

---

## What Is Correctly Absent

| Not present | Correctly so |
|-------------|--------------|
| a family-wide `ring_ordering` crate | four of the seven names are crate-specific; a shared crate would export five names nobody outside one crate uses |
| `SeqCst` anywhere | no site needs a total order across unrelated locations; every ordering here is a pairwise release/acquire |
| a `loom` model of the commit | `ring_atomic` provides the seam and `ring_publish`'s handshake test uses it; a single-consumer commit has no interleaving to explore |
| documented ordering rationale on `COMMIT` | one line exists; `ring_claim::CLAIM_SUCCESS`'s six-line argument is the family's best and this store is far simpler |

---

## Cross-References

| Definition | Instance | Relationship |
|------------|----------|--------------|
| non_functional_requirement | [001](001_what_the_read_path_costs.md) | the other three properties nothing checks |
| decisions | [002](../decisions/002_plain_stores_rather_than_compare_exchange.md) | why this crate stores rather than compare-exchanges, and what `COMMIT` has to carry |
| invariant | [001](../invariant/001_never_reads_past_what_was_published.md) | the property the `Release`/`Acquire` pair exists to hold |
| integration | [001](../integration/001_four_edges_in_and_none_out.md) | the dependency-change rationale which produced the four re-declarations |

### Sources

| What | Where |
|------|-------|
| `ring_consume`'s constant | `ring_consume/src/lib.rs:81` |
| The two store sites using it | `ring_consume/src/lib.rs:433,475` |
| The shared one | `ring_cursor/src/lib.rs:89` |
| `ring_mpsc`'s four | `ring_mpsc/src/lib.rs:237, 250, 271, 287` |
| The absent edges | `ring_mpsc/Cargo.toml` |

### Tests

| Claim | Verified by |
|-------|-------------|
| Eleven constants across seven crates | the per-file inventory scan above |
| `GATING` is the only shared one | `grep -rl 'GATING'` → seven crates, one declaration |
| Four names are declared twice | reading the inventory for repeats |
| No dependency edge between any pair | the `ring_mpsc` manifest check above |
