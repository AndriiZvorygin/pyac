# Skills

Guidance documents for whoever is working in this repository — a person, or an
agent under any harness.

These are **plain Markdown files in the repository**. They are not a plugin
format and not tied to any one tool. If your harness reads a `skills/`
directory, it will find them; if it does not, read the file directly, or point an
agent at this index.

| skill | when to read it |
|---|---|
| [pyac-vocabulary](pyac-vocabulary/SKILL.md) | adding or editing a dictionary entry, changing the source grammar, touching the volume format, rendering a volume to source, or debugging a word/quote/vector round-trip |

Each `SKILL.md` opens with YAML frontmatter (`name`, `description`) because some
tools use it for discovery. Nothing in the body depends on it.

## See also

- `AGENTS.md` — orientation for this repository: the three rules, layout, the
  commands you will actually use, and the traps that have already cost time.
- `docs/` — the normative specifications the code implements. Start with
  `docs/27-pyash-vliw-isa.md`.
