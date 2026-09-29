# workaround

External constraints `ring_core` absorbs on behalf of its consumers, each with the
cost it imposes and the condition under which it can be deleted.

### Scope

- **Purpose**: Record every external constraint this crate compensates for, so each one carries a stated cost and a deletion condition rather than becoming permanent by default.
- **Responsibility**: Document this crate's workarounds, and keep each one's deletion condition falsifiable.
- **In Scope**: Constraints originating outside this repository — for this crate, that means `crossbeam-queue`, its only non-sibling dependency.
- **Out of Scope**: This crate's own design decisions, which are not workarounds however unusual they look (→ [`decisions/`](../decisions/readme.md)); constraints compensated in shared tooling outside this crate.

### Overview Table

| ID | Status | Name | Constraint Source | Removal Trigger |
|----|--------|------|-------------------|-----------------|
| 001 | 🔄 | [`crossbeam-queue` as an Interim Backend](001_crossbeam_queue_as_interim_backend.md) | `crossbeam-queue` — an external MPMC queue standing in for the in-house multi-producer ring | The in-house rings reach the operating history consumers were waiting for (three stated conditions) |

**This is the family's only crate with a non-sibling dependency**, which is why
it is the only one whose `workaround/` is not empty. Verify:

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core
cargo tree --depth 1 --features crossbeam
```

Live output:

```
ring_core v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_core)
├── crossbeam-queue v0.3.14
├── ring_config v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_config)
├── ring_mpsc v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_mpsc)
├── ring_overflow v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_overflow)
├── ring_slot v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_slot)
├── ring_spsc v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_spsc)
└── ring_types v0.1.0 (/home/user1/pro/lib/yrd_gamedev/substrate/ring/ring_types)
```

Expect six `ring_*` siblings and exactly one external crate.

### Regenerate

```sh
cd "$(git rev-parse --show-toplevel)"/ring_core/docs/workaround
printf 'instances:                '; ls [0-9][0-9][0-9]_*.md | wc -l
printf 'finding headings inside:  '; grep -hoE '^### CO[0-9]+ ' [0-9][0-9][0-9]_*.md | wc -l
printf 'rows in the table below:  '; grep -coE '^\| CO[0-9]+ ' readme.md
# instances:                2
# finding headings inside:  1
# rows in the table below:  1
```
### Findings Recorded Here

| ID | Subject | Tier | Finding |
|----|---------|------|---------|
| CO52 | `src/lib.rs` | **latent hazard** | The `expect` is gone; the crate's one surviving runtime obligation is a `debug_assert` a release build compiles out. |
