# Lifecycle Doc Definition

### Scope

- **Purpose**: Give the arc of a configuration becoming a ring and the arc of the factory itself — both as phases and as states — with the registry-facing name as a second, independent axis.
- **Responsibility**: State the phases, the states, the transitions, the dependencies, the cleanup obligations, and the invariants preserved across them.
- **In Scope**: Config to handle pair; the factory's own lifetime; a configuration's progress to a ring; a name's progress to a registration.
- **Out of Scope**: The ring's teardown, which is `ring_shutdown`'s; the ring's own operational states, which the backend crates own.

### Overview Table

| ID | Name | Purpose | Status |
|----|------|---------|--------|
| 001 | [From a Record to a Handle Pair](001_from_a_record_to_a_handle_pair.md) | The construction arc, and the phase at which a rejection is still possible | 🔄 |
| 002 | [The Factory Outlives Nothing](002_the_factory_outlives_nothing.md) | Why the factory holds no state, and what the registry variant changes about that | 🔄 |
| 003 | [Config State Through a Build](003_config_state_through_a_build.md) | Four states, one of which is reachable only because validation and clamping happen in a different crate | 🔄 |
| 004 | [Name State Through a Registration](004_name_state_through_a_registration.md) | Why a refused registration must not leak a constructed ring | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/lifecycle
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FC[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FC[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC29 | `ring_factory` | n/a — observation | The construction arc crosses four crates and this crate holds two statements of it — a `Ring::new` call and a `Split::new` wrap; every phase a reader would want named happens in a dependency |
| FC30 | `ring_factory` | n/a — observation | Nothing can be retained because there is no retaining machinery in the crate: zero `Drop`, `Default`, `new`, `static`, `Arc`, `Rc`, `Box` and `Vec` outside doc comments |
| FC31 | `ring_factory` | n/a — observation | The requested value has no state — it is overwritten in place at the setter, so the "requested" and "clamped" states this instance distinguishes never coexist in any value |
| FC32 | `ring_factory` | n/a — observation | Registration is atomic and the name's later states are not this crate's to pin: of the registry's eight public methods this crate calls one, and the tests reach six more that no build path touches |
