# api

One type, six methods, four names imported, no error type. The surface is small
enough to read in a minute and is documented in two files because two facts about
it need arguing rather than listing: nothing outside this crate calls any of it,
and the one fallible signature returns a `Seq` where all 36 of its siblings
return an error.

Both files are about the same property from opposite ends — the crate is a
primitive that has never been used by a stranger, and its signature is honest
about the fact that a refusal here is not a fault.

### Overview Table

| ID | Name | Covers |
|----|------|--------|
| 001 | [Six Methods and No Caller](001_six_methods_and_no_caller.md) | PB10, PB11 — all seven public items with their annotations and doctests, the internal-only call graph, and the one droppable return value |
| 002 | [A Result Whose Error Is Not an Error](002_a_result_whose_error_is_not_an_error.md) | PB12, and the family's 39 fallible signatures — where the shape comes from, what it buys, its three costs, and the `RingError` variant that does not exist |

### The Whole Surface

| Item | Line | `must_use` | Doctest | Called from `src/` | Called from `tests/` |
|------|------|:----------:|:-------:|--------------------|----------------------|
| `Publisher` | `:85` | — | ✔ | — | both suites |
| `new` | `:100` | ✔ | ✔ | — | both suites |
| `cursor` | `:117` | ✔ | ✔ | — | `publish_test.rs` ×3, barrier wiring |
| `published` | `:133` | ✔ | ✔ | `is_published:231` | throughout |
| `try_publish` | `:161` | — (its `Result` is) | ✔ | `publish:204` | ×9 |
| `publish` | `:200` | **—** | ✔ | — | ×21 |
| `is_published` | `:229` | ✔ | ✔ | — | ×5 |

Seven items, seven doctests, `#![ deny( missing_docs ) ]` at `:55`, and exactly
two intra-library edges — each of them one method calling its own simpler
sibling. The only item on the surface whose result can vanish without a
diagnostic is `publish`, and 001 argues that is correct: its return value is
`start.advanced_by( len )`, which the caller supplied both halves of.

### The Signature, Against the Family

| Error type | Public fallible signatures | Where |
|------------|---------------------------:|-------|
| `RingError` | 21 | including all three Tier 5 siblings |
| a crate-local error | 12 | harness, debug and factory crates |
| a generic `T` | 3 | passthroughs |
| **`Seq`** | **1** | **`try_publish`** |

Thirty-six signatures answer *what went wrong*. One answers *where the frontier
is*. That is not a fault report, and the type says so before the `# Errors`
section does — which is why this crate is also the only one in the family that
names `RingError` without either declaring or importing it, in a sentence
explaining why it is not used. The census command returns two crates, and the
second is `ring_types`, which names it because it declares it.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"

# the whole surface, with its must_use annotations
grep -nE '^\s*(pub (const )?(fn|struct)|#\[ must_use \])' ring_publish/src/lib.rs

# every fallible public signature in the family, grouped by error type
grep -rhE '^\s*pub (const )?fn .*-> *Result<' ring_*/src/*.rs \
  | sed 's/.*-> *//' | sed 's/.*, *//;s/ *>.*//' | sort | uniq -c | sort -rn

# crates that name RingError without a `use` — expect two: its declarer, and this one
for c in ring_*/; do n=$( basename "$c" ); f="$c/src/lib.rs"; [ -f "$f" ] || continue
  if grep -q 'RingError' "$f" && ! grep -qE '^use .*RingError' "$f"; then echo "$n"; fi
done

# where Result< Seq, Seq > exists at all
grep -rn 'Result< Seq, Seq >' */src/*.rs 

# publish's 21 call sites: how many discard the value
grep -rn '\.publish(' ring_publish/src/lib.rs ring_publish/tests/*.rs | grep -v '///'
```

| | Value |
|--|------:|
| Public items | 7 — one type, six methods |
| Modules | 0 — everything at the crate root |
| Public fields | 0 |
| Doctests | 7 — one per item |
| Items carrying `#[ must_use ]` | 4 of 6 methods |
| …whose result can drop silently anyway | **1** — `publish` |
| `use` lines / names imported | 2 / 4 |
| Crates the surface is built from | 2 — `ring_cursor`, `ring_types` |
| Callers of any item outside this crate | **0** |
| Intra-library call edges | 2 |
| Fallible public signatures, family-wide | 37 |
| …returning `RingError` | 21 |
| …returning `Seq` | **1** |
| Crates naming `RingError` without a `use` | 2 — its declarer, and this one |
| …without declaring it either | **1** |
| `Result< Seq, Seq >` sites in the family | 5 |
| …that are a public API rather than a trait impl | **1** |
| `publish` call sites | 21 |
| …discarding the returned `Seq` | 15 |

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PB10 | `ring_publish` | n/a — unadopted | Every call site of every method is a test in this crate; only two items have a caller inside the library, and both are a method calling its own simpler sibling |
| PB11 | `ring_publish` | n/a — observation | The one silently-droppable return value is derivable from its own arguments, so its missing `#[ must_use ]` is correct; the annotation pattern tracks the *error* payload, and a future `publish` that grew a failure mode would need one with nothing to notice its absence |
| PB12 | family | n/a — observation | 21 of the family's 39 fallible signatures return `RingError` and one returns a `Seq`; this crate is the only one that names `RingError` without either declaring or importing it, in the sentence explaining why it does not |
| PB48 | `ring_publish` | n/a — unenforced | Six public functions, five `&self` receivers and no `&mut self` at all, against twelve of thirty-three crates that carry one; the shape is what lets a publisher be shared, and `Sync`, the property it exists to support, is asserted nowhere |
