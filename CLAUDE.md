# CLAUDE.md

@AGENTS.md

## Claude Code specifics

- Skills in `.claude/skills/` are invoked as `/verify`, `/lock-free-review`, `/new-gate`, … (table
  above). `verify`, `lock-free-review`, `concurrency-debug` and `doc-corpus` may also load on
  their own when relevant; the rest run only when asked.
- `.claude/settings.json` (shared) allows the `verb/` scripts and read-only cargo/git commands,
  and runs `.claude/hooks/rustfmt_edited.py` after every edit: nightly rustfmt on the edited
  `.rs` file — only if that file was already formatted at `HEAD` (or is new), so an edit never
  drags a whole unformatted file into its diff. Trybuild inputs and gate fixtures are skipped;
  a missing nightly is silently skipped. `./verb/fmt check::1` remains the check.
- Personal additions go in `CLAUDE.local.md` (loaded automatically after this file) and
  `.claude/settings.local.json` (its `allow` list merges with the shared one). Both are
  gitignored. Put MCP servers, IDE bridges and machine paths there, never here.
- Knowledge that must outlive a session goes into the repository — a crate's `docs/`, a
  decision record, a PR description — so the next contributor sees it, not only your memory.
- Long gate runs: start them with `run_in_background` and keep editing elsewhere — but not in
  the crates G12 mutates.
