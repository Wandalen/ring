# Item Doc Definition

### Scope

- **Purpose**: The crate's declarations as a set — what is declared, how the declarations relate to each other, and which kinds are absent.
- **Responsibility**: Eight top-level declarations, five `impl` blocks, eighteen public functions, the forwarding relation between the free layer and the `Tick` layer, and nine categories the crate does not use at all.
- **In Scope**: Names, kinds, counts, and the mirror between the two call layers.
- **Out of Scope**: What each signature guarantees a caller (→ [`../api/`](../api/readme.md)); the shape of the values themselves (→ [`../data_structure/`](../data_structure/readme.md)).

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [Four Operations, Eight Entry Points](001_four_operations_eight_entry_points.md) | Every declaration, and the free-function/`Tick` mirror that organises them | 🔄 |
| 002 | [What The Crate Does Not Declare](002_what_the_crate_does_not_declare.md) | Nine zeros, and the two whose absence reaches a caller's code | 🔄 |

**Presence and absence.** `001` is the inventory: what exists, in what kind, and
how the items reach each other. `002` is its complement, and it is a separate
document rather than a section because an absence has no line to hang off — you
find it by asking, not by reading. The two questions also fail differently:
`001` goes stale when a declaration is added, `002` when one is added *of a kind
that was previously zero*, which is the change nobody thinks to check.

Both findings in `001` are about the mirror — one where its documentation
describes the wrong parameter, one where the two layers share a ring and only
one counts. Both in `002` are about a zero having a cost: the missing error type
at every seam with the rest of the family, and the missing `#[ inline ]` on ten
non-generic one-liners.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_poll/docs/item
printf 'instances:                  %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:    %s\n' "$( command grep -hoE '^### PL[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:    %s\n' "$( command grep -coE '^\| PL[0-9]+ ' readme.md )"
printf 'each instance has a recipe: %s\n' "$( command grep -lc '^### Regenerate' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'declarations catalogued:    %s\n' "$( command grep -cE '^pub (fn|struct|enum|const|type|mod|use|trait) ' ../../src/lib.rs || true )"
printf 'public fns and methods:     %s\n' "$( command grep -cE '^(pub|    pub) (const )?fn ' ../../src/lib.rs || true )"
printf 'impl blocks:                %s\n' "$( command grep -cE '^impl ' ../../src/lib.rs || true )"
printf 'declaration kinds present:  %s\n' "$( command grep -oE '^pub (fn|struct|enum|const|type|mod|use|trait) ' ../../src/lib.rs | sed 's/^pub //' | sort -u | tr '\n' ' ' )"
printf 'declaration kinds absent:   %s\n' "$( for k in fn struct enum const type mod use trait; do command grep -qE "^pub $k " ../../src/lib.rs || printf '%s ' "$k"; done )"
printf 'items doc-commented:        %s\n' "$( command grep -B1 -E '^(pub|    pub) (const )?fn |^pub (struct|enum|const) ' ../../src/lib.rs | command grep -c '///' || true )"
```

Live output:

```
instances:                  2
finding headings inside:    4
rows in the table below:    4
each instance has a recipe: 2
declarations catalogued:    8
public fns and methods:     20
impl blocks:                5
declaration kinds present:  const  enum  fn  struct  
declaration kinds absent:   type mod use trait 
items doc-commented:        10
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| PL25 | a summary line naming a parameter the type does not own | **misleading doc** | `Tick::drain`'s first doc line reads *"[`drain_up_to`] with this tick's own ceiling"* while the ceiling is `max`, supplied by the caller on every call, and `Tick`'s only two fields are `budget` and `moved` — the paragraph directly beneath it states the distinction correctly, so the doc contradicts itself across consecutive lines and the wrong half is the summary, which is what `cargo doc` shows in the method list beside three siblings whose matching summaries are true. |
| PL26 | two public entry points per operation, accounting on one | **latent hazard** | `push_within` and `Tick::push` act on the same producer and only the second updates `moved`, so a tick whose caller reaches past the convenience layer reports a `Progress` that undercounts by exactly the records that went through the free function — the free layer is the one the module docs introduce first and every doctest uses, none of the twenty-three tests mixes the layers against one producer, and no doc comment on either side mentions the other. |
| PL27 | a crate with no error vocabulary inside a family that has one | n/a — observation | `ring_poll` names `ring_types::RingError` zero times where twenty of the family's thirty-three crates use it, expressing its one failure as `Result< (), T >` carrying the refused record — which is the right call, since an error would discard what the caller most needs back, but it means `?` and `Box< dyn Error >` do not compose across the seam and every boundary between this crate and the other twenty needs a `match`, a contract difference no document in the crate previously stated. |
| PL28 | ten cross-crate one-liners with no inlining hint and no LTO | n/a — unenforced | Eight of the eighteen public functions are generic and monomorphise into the caller, while the ten non-generic ones — `Budget`'s three, `Progress`'s four, and `Tick::new`/`budget`/`progress`, each a single expression — carry no `#[ inline ]`, and the workspace manifest sets `lto` in no profile, so a per-tick hot path pays a call for a field read; the whole family is consistent at zero `#[ inline ]` attributes, which makes this a family-wide default nobody has measured rather than a local oversight. |
