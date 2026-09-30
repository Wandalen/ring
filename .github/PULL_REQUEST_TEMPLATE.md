## Summary

<!-- What changed and why, in two to five lines. Capstone topic, if any: docs/capstone/0N_*.md -->

## How it is proven

<!-- Commands and their result. For a new gate: the planted violation and the red → green run. -->

- [ ] `./verb/test level::3` green (`level::5` if gates, lints or `unsafe` changed)
- [ ] `./verb/fmt check::1` clean
- [ ] A new gate or check was shown failing on a planted violation before it passed

## Checklist

- [ ] New `pub` items documented; affected crate `docs/` and `readme.md` updated
- [ ] No new `#[allow]` and no workspace lint weakened
- [ ] `unsafe`: allowlist and `docs/workaround/readme.md` updated, every block has `// SAFETY:`
- [ ] New or changed atomics: orderings justified in comments, `loom` model added or extended
- [ ] Trybuild `.stderr` changes explained (toolchain wording vs. a real change in what is rejected)

## Notes for the reviewer

<!-- Trade-offs, follow-ups, anything you are unsure about. Delete if empty. -->
