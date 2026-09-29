# API Doc Definition

### Scope

- **Purpose**: Document the one operation this crate exists to provide, and the registry-facing variant that names what it builds.
- **Responsibility**: State the operations, their error handling, and their compatibility guarantees as an exported crate.
- **In Scope**: `build`; the named variant; what the surface deliberately does not offer.
- **Out of Scope**: `RingConfig`'s own builder surface, which is `ring_config`'s.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [The Build Surface](001_the_build_surface.md) | One function, one input, one output — and the four constructors it exists to prevent | 🔄 |
| 002 | [The Named Build Surface](002_the_named_build_surface.md) | Building into a registry, and why that is a second operation rather than a parameter | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/api
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FC[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FC[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC5 | `ring_factory` | n/a — observation | The five-name export surface has zero consumers outside `ring_*`, so every guarantee this instance states is currently a promise to the family about itself |
| FC6 | `ring_factory` | n/a — doc gap | The re-export rule is derivable and correct — re-export exactly the non-Contract crates a caller is forced to name — and is stated nowhere; the surface test's own doc comment enumerates three of its four `ring_*` imports, omitting `ring_handle` |
| FC7 | `ring_factory` | n/a — inconsistency | The Contract is five crates wide by declaration and six wide in use: `Registry` carries eight public methods through the re-export, and `ring_registry` is not on the surface |
| FC8 | `ring_factory` | **latent hazard** | `build_named` is the one operation with a refusal of its own and it cannot report which half failed — `BuildError` flattens "the ring could not be built" and "the name was taken" into two variants with no shared discriminant a caller can branch on before acting |
