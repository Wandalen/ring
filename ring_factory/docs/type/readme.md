# Type Doc Definition

### Scope

- **Purpose**: Define the two types this crate contributes — the factory itself and the error a build can return.
- **Responsibility**: Give each type's definition, its validation rules, and its exported status.
- **In Scope**: `Factory`; the build error.
- **Out of Scope**: `RingConfig`, `WaitKind`, `OverflowPolicy` and `Capacity`, which are `ring_config`'s and `ring_types`'.

### Overview Table

| ID | Name | Purpose | domain | ddd | Status |
|----|------|---------|--------|-----|--------|
| 001 | [Factory](001_factory.md) | A type with no fields, exported, and the reason it is a type rather than a free function | ring | service | 🔄 |
| 002 | [Build Error](002_build_error.md) | What a build can refuse and why the list is shorter than it should be | ring | value object | 🔄 |

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_factory/docs/type
printf 'instances:                %s\n' "$( ls [0-9][0-9][0-9]_*.md | wc -l )"
printf 'finding headings inside:  %s\n' "$( command grep -hoE '^### FC[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l )"
printf 'rows in the table below:  %s\n' "$( command grep -coE '^\| FC[0-9]+ ' readme.md )"
```

### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| FC45 | `ring_factory` | n/a — doc gap | The only way to obtain a `Factory` is the unit literal and that is stated nowhere; the absent `Default` is argued at length and the absent `new` — the one a Rust reader looks for first — is not mentioned |
| FC46 | `ring_factory` | n/a — observation | Three methods, all `&self`, zero fields: the type is a namespace with a receiver, and replacing it with three free functions would change every call site's spelling and nothing else |
| FC47 | `ring_factory` | n/a — doc gap | `Copy` is the family convention rather than this type's choice — seven of eight family error types derive it, `RegistryError` is the lone exception because it holds a `String` — and nothing records that |
| FC48 | `ring_factory` | n/a — inconsistency | The enum and its payload make opposite exhaustiveness choices: `BuildError` is deliberately exhaustive and wraps `RingError`, which is `#[ non_exhaustive ]` — the opposite choice, one `match` apart |
