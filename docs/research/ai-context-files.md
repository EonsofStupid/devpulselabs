# AI context files: what each AI tool reads, and what is never read

R313 research, 2026-11-27. The source for `crates/ai/core/catalog/context-files.json` (internal build §3.8). Each row was checked against the vendor's own documentation on 2026-11-27; rows marked UNVERIFIED say why. Re-check a vendor before changing its rows.

Rows marked **no** in the verified column are paths I could not confirm on a vendor page; each says why.

**Read this first: four products have moved since mid-2026.**
- **Codex:** `developers.openai.com/codex/*` now redirects (308) to `learn.chatgpt.com/docs/*`.
- **Windsurf:** `docs.windsurf.com` now redirects to `docs.devin.ai/desktop`. The editor is now "Devin Desktop", and the `.windsurf/` paths still load as legacy locations.
- **Kimi:** Kimi CLI (`~/.kimi`) is archived and replaced by Kimi Code CLI (`~/.kimi-code`).
- **Gemini CLI:** a banner on its docs says it "was replaced by Antigravity CLI on June 18th, 2026" for unpaid-tier and Google One users.

**Reading the tables.** I read the pages through a fetch tool that summarises them, so quotes are close paraphrases. `~` is `$HOME` on macOS and Linux and `%USERPROFILE%` on Windows, unless a row says otherwise. Each row's Doc column gives a key; the full URLs are in the URL legend at the end.

---

## 1. Claude Code (Anthropic)
`CLAUDE_CONFIG_DIR` relocates `~/.claude` and `~/.claude.json`. On Windows, `~/.claude` is `%USERPROFILE%\.claude`.

| Glob | Scope | Kind | Format and precedence | Doc |
|---|---|---|---|---|
| `CLAUDE.md`, `.claude/CLAUDE.md` | project, committed; loaded from cwd and every ancestor | instructions | Imports with `@path`, relative or absolute, recursive to 4 hops; skipped inside code spans. Ancestor files are concatenated root first, so the closest is read last. | CC-mem |
| `CLAUDE.local.md` | project, local (you add it to `.gitignore`) | instructions | Appended after `CLAUDE.md` in the same directory. | CC-mem |
| `<subdir>/CLAUDE.md`, `<subdir>/CLAUDE.local.md` | nested | instructions | Loaded on demand when Claude touches a file in that subdirectory. | CC-mem |
| `AGENTS.md`, `.claude/AGENTS.md` (cwd, ancestors and subdirectories) | project, nested | instructions | Default `claude-md-or-agents-md`: read only when no `CLAUDE.md`, `.claude/CLAUDE.md` or `CLAUDE.local.md` exists at cwd or above. Built-in plugin `cc-plugin-agents-md@builtin`; needs v2.1.277 or later. | CC-mem |
| `.claude/rules/**/*.md` | project, committed; also nested `.claude/rules` | rules | Frontmatter `paths:` (globs). Without `paths` it loads at start; with `paths` it loads when a matching file is touched. Symlinks allowed. | CC-mem, CC-dir |
| `~/.claude/CLAUDE.md`, `~/.claude/rules/**/*.md` | user | instructions, rules | Same format. | CC-mem |
| `/Library/Application Support/ClaudeCode/CLAUDE.md` (macOS), `/etc/claude-code/CLAUDE.md` (Linux, WSL), `C:\Program Files\ClaudeCode\CLAUDE.md` (Windows) | managed | instructions | Cannot be excluded with `claudeMdExcludes`. The managed-settings `claudeMd` key is the inline alternative. | CC-mem |
| `~/.claude/projects/<project>/memory/MEMORY.md` and topic `*.md` files | user, per repository | memory (auto) | First 200 lines or 25 KB loaded. | CC-mem, CC-dir |
| `.claude/agent-memory/<agent>/MEMORY.md`, `~/.claude/agent-memory/` | project, user | memory (subagent) | | CC-dir |
| `.claude/skills/<name>/SKILL.md`, `~/.claude/skills/...`, `<subdir>/.claude/skills/...`, `<managed dir>/.claude/skills/...`, `<plugin>/skills/...`, `~/.claude/skills/synced/` | project, user, nested, managed, plugin | skill | Keys: `name`, `description`, `when_to_use`, `argument-hint`, `arguments`, `disable-model-invocation`, `user-invocable`, `allowed-tools`, `disallowed-tools`, `model`, `effort`, `context`, `agent`, `background`, `hooks`, `paths`, `shell`, `metadata`, `license`, `compatibility`. | CC-skills |
| `.claude/commands/**/*.md`, `~/.claude/commands/` | project, user | command | Skill keys except `name` and `paths`. A subfolder adds a namespace (`/a:b`). A skill wins over a command of the same name. | CC-skills |
| `.claude/agents/**/*.md`, `~/.claude/agents/`, `<managed dir>/.claude/agents/`, `<plugin>/agents/` | project (walks up to repo root), user, managed, plugin | agent | Required: `name`, `description`. Optional: `tools`, `disallowedTools`, `model`, `permissionMode`, `maxTurns`, `autoCompactWindow`, `skills`, `mcpServers`, `hooks`, `memory`, `background`, `omitClaudeMd`, `effort`, `isolation`, `color`, `initialPrompt`, `experimental`. Priority: managed > `--agents` flag > project > user > plugin. | CC-agents |
| `.claude/output-styles/*.md`, `~/.claude/output-styles/` | project, user | instructions (output style) | Keys: `name`, `description`, `keep-coding-instructions`, `force-for-plugin`. | CC-dir |
| `.claude/workflows/*.js`, `~/.claude/workflows/` | project, user | command (workflow) | | CC-dir |
| `.claude/settings.json` | project, committed | settings, hooks | Priority: managed > `--settings` > `.claude/settings.local.json` > `.claude/settings.json` > `~/.claude/settings.json`. Lists merge across files. | CC-set |
| `.claude/settings.local.json` | project, local (auto-added to global git excludes) | settings, hooks | | CC-set |
| `~/.claude/settings.json` | user | settings, hooks | | CC-set |
| `managed-settings.json` and `managed-settings.d/*.json` in `/Library/Application Support/ClaudeCode/`, `/etc/claude-code/` or `C:\Program Files\ClaudeCode\`; plist domain `com.anthropic.claudecode`; registry `HKLM\SOFTWARE\Policies\ClaudeCode` (value `Settings`) and `HKCU\...`; cache `~/.claude/remote-settings.json` | managed | settings | The legacy Windows path `C:\ProgramData\ClaudeCode\` is not read. | CC-managed |
| `.mcp.json` | project, committed | MCP | | CC-dir |
| `~/.claude.json` | user | MCP, settings, state | Holds account details, personal MCP servers and per-project trust. Sensitive. | CC-dir, CC-set |
| `managed-mcp.json` (same system directories as `managed-settings.json`) | managed | MCP | | CC-managed |
| `<plugin>/hooks/hooks.json`; `~/.claude/plugins/` | plugin | hook, plugin | | CC-hooks, CC-dir |
| `.worktreeinclude` | project | settings | | CC-dir |
| `~/.claude/keybindings.json`, `~/.claude/themes/*.json` | user | settings | | CC-dir |

**Credentials (Claude Code):**
- `~/.claude/.credentials.json`, `%USERPROFILE%\.claude\.credentials.json` on Windows. macOS uses the Keychain and falls back to this file. Follows `CLAUDE_CONFIG_DIR`.
- Anthropic profiles in `~/.config/anthropic/` (macOS, Linux) or `%APPDATA%\Anthropic\` (Windows), including `configs/` and `active_config`.
- `~/.claude/policy-limits.json` with its `.stamp.json` sidecar (records which account the cache belongs to).
- Plaintext transcripts may hold secrets: `~/.claude/projects/**/*.jsonl`, `~/.claude/history.jsonl`, `~/.claude/paste-cache/`, `~/.claude/file-history/`.
- Sources: CC-auth, CC-dir.

## 2. OpenAI Codex CLI
`CODEX_HOME` defaults to `~/.codex`; on Windows that is `%USERPROFILE%\.codex`.

| Glob | Scope | Kind | Format and precedence | Doc |
|---|---|---|---|---|
| `$CODEX_HOME/AGENTS.override.md`, else `$CODEX_HOME/AGENTS.md` | user | instructions | The first non-empty file wins. | CX-agents |
| `AGENTS.override.md` > `AGENTS.md` > names in `project_doc_fallback_filenames`, in each directory from git root down to cwd | project, nested | instructions | At most one file per directory. Concatenated root to cwd, so the closest wins. Combined cap `project_doc_max_bytes` (default 32 KiB). | CX-agents |
| `model_instructions_file` (path set in config) | settings | instructions | Replaces the built-in instructions. Old key name: `experimental_instructions_file`. | CX-ref |
| `.codex/config.toml` (project root down to cwd; trusted projects only) | project | settings, MCP (`[mcp_servers.*]`), hooks (`[hooks]`) | Priority: CLI `-c` > project > profile > user > cloud > system. | CX-cfg |
| `~/.codex/config.toml`, `~/.codex/<profile>.config.toml` | user | settings, MCP | `config.toml` can hold `mcp_servers.<x>.oauth.client_secret`. | CX-cfg, CX-mcp |
| `/etc/codex/config.toml` | system (Unix) | settings | | CX-cfg |
| `requirements.toml` at `/etc/codex/` (Linux, macOS) or `%ProgramData%\OpenAI\Codex\` (Windows); `managed_config.toml` at `/etc/codex/` or, on Windows, `~/.codex/managed_config.toml`; macOS MDM domain `com.openai.codex` (keys `requirements_toml_base64`, `config_toml_base64`) | managed | settings, rules, hooks | | CX-managed |
| `.agents/skills/<name>/SKILL.md` (cwd up to repo root), `~/.agents/skills/`, `/etc/codex/skills/`, bundled system skills | project, user, admin | skill | Required: `name`, `description`. Optional `agents/openai.yaml` with `interface`, `policy.allow_implicit_invocation`, `dependencies.tools`. Same-name skills are not merged. | CX-skills |
| `~/.codex/agents/*.toml`, `.codex/agents/*.toml` | user, project | agent | Required: `name`, `description`, `developer_instructions`. Optional: `model`, `model_reasoning_effort`, and others. | CX-sub |
| `~/.codex/rules/*.rules` (`default.rules`), `.codex/rules/*.rules` (trusted only) | user, project | rules (exec policy) | Starlark `prefix_rule(pattern, decision, justification, match, not_match)`. | CX-rules |
| `hooks.json` at `/etc/codex/`, `%ProgramData%\OpenAI\Codex\`, `$CODEX_HOME/` and `.codex/`; plugins via `hooks/hooks.json` or `.codex-plugin/plugin.json` | system, user, project, plugin | hook | | CX-hooks |
| `~/.codex/prompts/*.md` (top level only) | user | command (deprecated) | Keys: `description`, `argument-hint`. | CX-prompts |
| `$CODEX_HOME/memories_v2/`, `$CODEX_HOME/memories/` | user | memory (generated) | | CX-mem |
| `$CODEX_HOME/history.jsonl`, `$CODEX_HOME/log/` | user | state | Transcripts. | CX-ref |

**Credentials (Codex):**
- `~/.codex/auth.json`, used when `cli_auth_credentials_store` is `file` or `auto` falls back to file; otherwise the OS keyring.
- MCP OAuth tokens: `mcp_oauth_credentials_store` is `auto | file | keyring`, but the file path is **not documented** (UNVERIFIED).
- `config.toml` may contain an MCP `client_secret`.
- Sources: CX-auth, CX-ref, CX-mcp.

## 3. Cursor

| Glob | Scope | Kind | Format and precedence | Doc |
|---|---|---|---|---|
| `.cursor/rules/**/*.mdc` | project, committed (subfolders allowed) | rules | Keys: `description`, `globs`, `alwaysApply`. Four modes: Always, Specific Files, Intelligently, Manual. Plain `.md` files here are ignored. Priority: Team > Project > User. | CU-rules |
| `~/.cursor/rules` (`%USERPROFILE%\.cursor\rules`) | user | rules | Local only, not synced. Settings-UI user rules are stored on the account. | CU-help |
| `AGENTS.md`, `CLAUDE.md` (root and subdirectories) | project, nested | instructions | Nested files combine with parents; the more specific wins. `CLAUDE.md` always applies. | CU-rules, CU-help |
| `.cursorrules` | project | rules (legacy, "will be deprecated") | | CU-help |
| `.cursor/skills/`, `.agents/skills/` (and nested); `~/.cursor/skills/`, `~/.agents/skills/` | project, user | skill | Also loads `.claude/skills/`, `.codex/skills/`, `~/.claude/skills/`, `~/.codex/skills/`. Keys: `name`, `description`, `paths` (legacy `globs`), `disable-model-invocation`, `icon`, `color`, `metadata`. | CU-skills |
| `.cursor/agents/`, `~/.cursor/agents/`; also `.claude/agents/`, `~/.claude/agents/`, `.grok/agents/`, `~/.grok/agents/` | project, user | agent | Keys: `name`, `description`, `model`, `readonly`, `is_background`. `.cursor/` wins over `.claude/` and `.grok/`. `.grok` loads only with the Third-Party Imports setting on. | CU-sub |
| `.cursor/hooks.json`, `~/.cursor/hooks.json`; enterprise `/Library/Application Support/Cursor/hooks.json`, `/etc/cursor/hooks.json`, `C:\ProgramData\Cursor\hooks.json` | project, user, managed | hook | Priority: Enterprise > Team > Project > User > plugin. Can load Claude Code hooks, but which files is not stated (UNVERIFIED). | CU-hooks |
| `.cursor/mcp.json`, `~/.cursor/mcp.json` | project, user | MCP | `${env:NAME}`, `${userHome}`, `${workspaceFolder}`, `envFile`. | CU-mcp |
| `~/.cursor/cli-config.json` (`$env:USERPROFILE\.cursor\cli-config.json`; `$XDG_CONFIG_HOME/cursor/`; `CURSOR_CONFIG_DIR`), `<project>/.cursor/cli.json` | user, project | settings | The project file holds permissions only. | CU-cli |
| `.cursorignore` | project | settings (ignore) | | CU-ignore |
| `.cursor/commands/*.md`, `~/.cursor/commands/` | project, user | command | **UNVERIFIED**: the official commands page returned no path text; known only from search snippets and forum posts. | (none) |

**Credentials (Cursor):** the CLI says credentials are "securely stored locally" but gives no path (UNVERIFIED). `CURSOR_API_KEY` is an environment variable. Source: CU-auth.

## 4. Gemini CLI (Google)

| Glob | Scope | Kind | Format and precedence | Doc |
|---|---|---|---|---|
| `GEMINI.md` in workspace directories and their parents; just-in-time in subdirectories up to the trusted root | project, nested | instructions | `@file.md` imports (relative or absolute). `context.fileName` renames or adds files, e.g. `["AGENTS.md","CONTEXT.md","GEMINI.md"]`. | GM-md |
| `~/.gemini/GEMINI.md` | user | instructions | The Windows path is not stated (by analogy `%USERPROFILE%\.gemini\`). | GM-md |
| `.gemini/settings.json` | project | settings, MCP (`mcpServers`), hooks | Priority, lowest to highest: defaults < system-defaults < user < project < system < environment < CLI. | GM-cfg, GM-hooks |
| `~/.gemini/settings.json` | user | settings, MCP, hooks | | GM-cfg |
| `system-defaults.json` and `settings.json` at `/etc/gemini-cli/`, `C:\ProgramData\gemini-cli\` or `/Library/Application Support/GeminiCli/` (overridable with `GEMINI_CLI_SYSTEM_SETTINGS_PATH` / `..._DEFAULTS_PATH`) | managed | settings | | GM-cfg |
| `.gemini/commands/**/*.toml`, `~/.gemini/commands/` | project, user | command | Keys: `prompt` (required), `description`. A subfolder adds a namespace (`/git:commit`). Project wins. | GM-cmd |
| `.gemini/skills/`, `.agents/skills/`, `~/.gemini/skills/`, `~/.agents/skills/` | project, user | skill | Keys: `name`, `description`. Priority: workspace > user > extension > built-in; `.agents` beats `.gemini` within a tier. | GM-skills, GM-cskill |
| `.gemini/agents/*.md`, `~/.gemini/agents/*.md` | project, user | agent | Keys: `name`, `description`, `kind`, `tools`, `mcpServers`, `model`, `temperature`, `max_turns`, `timeout_mins`. | GM-sub |
| `~/.gemini/extensions/<name>/gemini-extension.json` with `GEMINI.md`, `commands/`, `skills/`, `agents/`, `hooks/hooks.json` | user (extension) | all kinds | Keys: `name`, `version`, `description`, `migratedTo`, `mcpServers`, `contextFileName`, `excludeTools`, `plan`. | GM-ext |
| `.gemini/.env` (cwd upward), `~/.gemini/.env`, `~/.env` | project, user | settings | Can hold `GEMINI_API_KEY`. | GM-auth, GM-cfg |
| `.geminiignore` | project | ignore | The page only links to it. | GM-md |

**Credentials (Gemini CLI):**
- `~/.gemini/mcp-oauth-tokens.json` (verified).
- `.env` files that contain `GEMINI_API_KEY`.
- `~/.gemini/oauth_creds.json` and `google_accounts.json`: **UNVERIFIED**. The vendor docs only say credentials are "cached locally"; the file names come from third-party sources, which also say newer versions use the keychain.
- Sources: GM-mcp, GM-auth.

## 5. Grok Build (xAI)
`GROK_HOME` defaults to `~/.grok`. No Windows or macOS paths are documented.

| Glob | Scope | Kind | Format and precedence | Doc |
|---|---|---|---|---|
| `AGENTS.md`, `Agents.md`, `AGENT.md`, `CLAUDE.md`, `Claude.md`, `CLAUDE.local.md` in each directory; `.grok/rules/*.md`; also `.claude/rules/`, `.cursor/rules/` | project, nested | instructions, rules | Nested `AGENTS.md` scopes to its subtree; deeper files win. `.gitignore`d files are skipped. | GK-rules |
| `~/.grok/` global rules | user | instructions | Loaded first. The exact file name is not stated. | GK-rules |
| `~/.grok/config.toml` | user | settings, MCP | Compatibility toggles under `[compat.cursor]` and `[compat.claude]`, or env vars `GROK_{CURSOR,CLAUDE}_{SKILLS,RULES,AGENTS,MCPS,HOOKS}_ENABLED`. All default on. | GK-set |
| `.grok/config.toml` | project | MCP, plugins, permission only | | GK-set |
| `~/.grok/sandbox.toml`, `.grok/sandbox.toml` | user, project | settings | | GK-set |
| `/etc/grok/managed_config.toml`, `~/.grok/managed_config.toml`, `~/.grok/requirements.toml`, `/etc/grok/requirements.toml` | managed | settings | Order listed lowest to highest priority. | GK-ent |
| `.grok/skills/` (walks up to repo root), `~/.grok/skills/`, `~/.agents/skills/`, plugin `skills/` | project, user | skill | Keys: `name`, `description`, `when-to-use`, `paths`, `allowed-tools`, `argument-hint`, `user-invocable`, `disable-model-invocation`, `metadata`. Claude skills are also read. | GK-skills |
| `.grok/agents/`, `~/.grok/agents/` | project, user | agent | | GK-sub |
| `~/.grok/hooks/` (plus `~/.grok/hooks-paths`), `.grok/hooks/` (requires `/hooks-trust`) | user, project | hook | | GK-skills |
| `~/.agents/commands/` | user | command | | GK-skills |
| `.grok/plugins/`, `~/.grok/plugins/` | project, user | plugin | | GK-skills |
| Also reads `~/.claude.json`, `.cursor/mcp.json`, `.mcp.json` | project, user | MCP | Rank below `config.toml`. | GK-mcp |

**Credentials (Grok Build):** `~/.grok/auth.json`, `~/.grok/mcp_credentials.json`, and inline `api_key` values in `config.toml`. Sources: GK-ent, GK-mcp, GK-set.

## 6. GitHub Copilot (VS Code, CLI, cloud/coding agent, code review)
`COPILOT_HOME` defaults to `~/.copilot` (`C:\Users\<you>\.copilot` on Windows).

| Glob | Scope | Kind | Format and precedence | Doc |
|---|---|---|---|---|
| `.github/copilot-instructions.md` | project, committed | instructions | Priority: personal > repository > organization; all sets are sent. | GH-instr |
| `.github/instructions/**/*.instructions.md` | project | rules | Keys: `applyTo` (required on github.com), `excludeAgent` (`code-review` or `cloud-agent`); in VS Code also `name`, `description`. | GH-instr, VS-ref |
| `AGENTS.md` (anywhere; the nearest wins); `CLAUDE.md` or `GEMINI.md` at repo root | project, nested | instructions | VS Code settings: `chat.useAgentsMdFile`, `chat.useNestedAgentsMdFiles`, `chat.useClaudeMdFile`. | GH-instr, VS-ref |
| VS Code Claude harness: `CLAUDE.md`, `.claude/CLAUDE.md`, `.claude/rules/*.md` (`paths`), `CLAUDE.local.md`, `~/.claude/CLAUDE.md`, `~/.claude/rules`. Codex harness: `AGENTS.md` and `~/.codex/AGENTS.md` | project, user | instructions | | VS-ref |
| `~/.copilot/copilot-instructions.md`, `~/.copilot/instructions/**/*.instructions.md` | user | instructions | | VS-ref, GH-cli |
| `.github/prompts/*.prompt.md`; VS Code user profile folder | project, user | command | Keys: `description`, `name`, `argument-hint`, `agent`, `model`, `tools`. Runs only in the Local harness, which is deprecated and to be removed in February 2027. | VS-prompt |
| `.github/agents/*.agent.md`; org `.github` or `.github-private` repo `agents/`; enterprise `.github-private` `agents/`; `~/.copilot/agents/` | project, org, enterprise, user | agent | Keys: `name`, `description` (required), `target`, `tools`, `model`, `disable-model-invocation`, `user-invocable`, `mcp-servers`, `metadata`; VS Code adds `handoffs`, `model-policy`. | GH-agents, GH-agentloc, VS-agents, GH-cli |
| `.github/skills/`, `~/.copilot/skills/`; also `.claude/skills/`, `~/.claude/skills/`, `.agents/skills/`, `~/.agents/skills/` | project, user | skill | Keys: `name`, `description`, `argument-hint`, `user-invocable`, `disable-model-invocation`, `context`. | VS-skills, GH-cli |
| `.github/hooks/*.json`, `~/.copilot/hooks/`; inline in `.github/copilot/settings.json`, `.github/copilot/settings.local.json`, `~/.copilot/settings.json`; also `.claude/settings.json` and `.claude/settings.local.json` | project, local, user | hook, settings | | VS-hooks, GH-cli |
| `.vscode/mcp.json`, VS Code profile `mcp.json`, `.mcp.json`, `.github/mcp.json` (CLI), `~/.copilot/mcp-config.json` | project, user | MCP | VS Code `inputs` with `promptString` and `password` for secrets. | VS-mcp, GH-cli |
| `~/.copilot/config.json`, `settings.json`, `permissions-config.json`, `lsp-config.json`, `extensions/`, `installed-plugins/`, `session-state/`, `session-store.db`, `logs/` | user | settings, state | | GH-cli |

**Credentials (Copilot):**
- `~/.copilot/config.json` holds "authentication data".
- `~/.copilot/providers.json`: API keys and bearer tokens in plaintext.
- `~/.copilot/mcp-oauth-config/` and `~/.copilot/mcp-secrets/`: fallbacks used when the OS keychain is unavailable.
- VS Code secret storage holds MCP OAuth secrets (OS-backed).
- Sources: GH-cli, VS-mcp.

## 7. Windsurf, now Devin Desktop (Cognition)

| Glob | Scope | Kind | Format and precedence | Doc |
|---|---|---|---|---|
| `.devin/rules/*.md` (current); `.windsurf/rules/*.md` and `.windsurfrules` (legacy, still loaded) | project | rules | `trigger`: `always_on`, `glob`, `model_decision`, `manual` (the CLI also lists `agent`); `globs`; `description`. 12,000-character limit per file. `.devin` beats `.windsurf`. | WS-mem, DV-rules |
| `.devin/global_rules.md` (legacy `.windsurf/global_rules.md`) | project | instructions | If both exist, only `.devin` loads. | WS-mem |
| `~/.devin/rules/*.md`, `~/.devin/global_rules.md` | user | rules | | WS-mem, DV-rules |
| `~/.codeium/windsurf/memories/global_rules.md`; auto-memories in `~/.codeium/windsurf/memories/` | user (legacy) | instructions, memory | **Partially verified**: appears only in localized or older copies of the vendor page; the current English page does not list it. | (search of docs.devin.ai) |
| `AGENTS.md`, `AGENTS.local.md`, `AGENT.md` (root and each directory down to cwd) | project, local, nested | instructions | Always on. | WS-mem, DV-rules |
| `/Library/Application Support/Windsurf/rules/*.md`, `/etc/windsurf/rules/*.md`, `C:\ProgramData\Windsurf\rules\*.md` | managed | rules | From the Windsurf Plugins page. | WS-plug |
| `.devin/workflows/*.md` (fallback `.windsurf/workflows/`; any depth up to git root); `~/.codeium/windsurf/global_workflows/*.md`; `/Library/Application Support/Devin/workflows/`, `/etc/devin/workflows/`, `C:\ProgramData\Devin\workflows\` | project, user, managed | command (workflow) | 12,000-character limit. Priority: System > Workspace > Global > Built-in. | WS-wf |
| `.devin/skills/`, `.agents/skills/`, `.windsurf/skills/`; `~/.config/devin/skills/`, `~/.agents/skills/` | project, user | skill | Keys: `name`, `description`. | WS-skills |
| `.devin/hooks.v1.json` | project | hook | User-level path not stated. | WS-hooks |
| `.devin/mcp_config.json`, `.devin/mcp_config.local.json`, `~/.config/devin/mcp_config.json` | project, local, user | MCP | Legacy `~/.codeium/windsurf/mcp_config.json` is **UNVERIFIED** (not on the current page). | WS-mcp |
| Policies: registry `Software\Policies\Windsurf\Devin`; Linux `/etc/vscode/policy.json`; macOS managed preferences | managed | settings | | DV-pol |

**Credentials (Windsurf / Devin Desktop):** no Desktop credential file is documented (UNVERIFIED). The Devin CLI file is listed under section 7a.

## 7a. Devin CLI (same vendor; relevant because it imports other tools' files)
| Glob | Scope | Kind | Doc |
|---|---|---|---|
| `~/.config/devin/AGENTS.md` (`%APPDATA%\devin\AGENTS.md`); also reads `~/.claude/CLAUDE.md`, `.cursor/rules/*.md|*.mdc`, `.windsurf/rules/*.md`, `.claude/` | user, project | instructions (imported) | DV-rules |
| `~/.config/devin/config.json` (`%APPDATA%\devin\config.json`), `.devin/config.json`, `.devin/config.local.json` (`read_config_from`: `agents_standard`, `cursor`, `windsurf`, `claude`) | user, project, local | settings, hooks | DV-cfg |
| Hooks also read from `.claude/settings*.json`, `~/.claude.json`, `~/.claude/settings*.json` | project, user | hook (imported) | DV-hooks |
| Skills: also `.kiro/skills/`, `~/.kiro/skills/`, `~/.codeium/<channel>/skills/`, `.claude/skills/`, `.github/skills/`, `.codex/skills/` | project, user | skill (imported) | DV-skills |
| `/Library/Application Support/Devin/system.json`, `/etc/devin/system.json`, `C:\ProgramData\Devin\system.json` | managed | settings | DV-sys |

**Credentials (Devin CLI):** `~/.local/share/devin/credentials.toml` (or under `$XDG_DATA_HOME`), `%APPDATA%\devin\credentials.toml` on Windows. Source: DV-auth.

## 8. Cline

| Glob | Scope | Kind | Format and precedence | Doc |
|---|---|---|---|---|
| `.clinerules/` (legacy) and `.cline/rules/` | project | rules | Frontmatter `paths: [globs]`; `paths: []` means never active; no frontmatter means always active. Workspace beats global. | CL-rules, CL-cfg |
| `~/Documents/Cline/Rules` (Windows `Documents\Cline\Rules`, including OneDrive), `~/.cline/rules`, `~/Cline/Rules` | user | rules | | CL-rules |
| `AGENTS.md` (project root), `~/.agents/AGENTS.md`, `.cursorrules`, `.windsurfrules` | project, user | instructions | Detected automatically. | CL-rules |
| `.cline/workflows/`, `~/.cline/workflows/`, `~/Documents/Cline/Workflows/` | project, user | command | | CL-cfg |
| `.cline/hooks/`, `~/.cline/hooks/`, `~/Documents/Cline/Hooks/` | project, user | hook | | CL-cfg |
| `.cline/skills/`, `.agents/skills/`, `~/.cline/skills/`, `~/.agents/skills/` | project, user | skill | | CL-cfg |
| `.cline/agents/`, `~/.cline/agents/`; `plugins/`; `~/.cline/cron/` | project, user | agent, plugin | | CL-cfg |
| `~/.cline/data/settings/cline_mcp_settings.json`, `global-settings.json` (`CLINE_DATA_DIR` overrides the data directory) | user | MCP, settings | | CL-cfg |
| `.clineignore` | project | ignore (deprecated) | | CL-cfg |

**Credentials (Cline):** `~/.cline/data/settings/providers.json` ("API keys and provider configuration"); `~/.cline/data/sessions/` holds transcripts. Where the VS Code extension's secret storage lives is not documented (UNVERIFIED). Source: CL-cfg.

## 9. Kiro (AWS)
`KIRO_HOME` defaults to `~/.kiro`.

| Glob | Scope | Kind | Format and precedence | Doc |
|---|---|---|---|---|
| `.kiro/steering/*.md`, `~/.kiro/steering/*.md` | project, user | rules | Key `inclusion`: `always`, `fileMatch` (with `fileMatchPattern`), `manual`, or `auto` (with `name` and `description`). File references `#[[file:path]]` and `#[[folder:path]]`. Workspace beats global. | KR-steer |
| `AGENTS.md` (workspace root, subdirectories, `~/.kiro/steering/`) | project, nested, user | instructions | Always included. | KR-steer |
| `.kiro/skills/<n>/SKILL.md`, `~/.kiro/skills/` | project, user | skill | Keys: `name`, `description`, `license`, `compatibility`, `metadata`. Workspace wins. | KR-skills |
| `.kiro/agents/*.json|*.md`, `~/.kiro/agents/` (nested folders allowed) | project, user | agent | Keys: `name`, `description`, `tools`, `excludedTools`, `includeMcpJson`, `includePowers`, `resources`, `permissions`, `prompt`, `model`, `welcomeMessage`. | KR-agents |
| `.kiro/hooks/*.json`, `~/.kiro/hooks/` (CLI V3) | project, user | hook | | KR-hooks, KR-cfg |
| `.kiro/settings/mcp.json`, `~/.kiro/settings/mcp.json` | project, user | MCP | Agent > workspace > global. | KR-cfg |
| `~/.kiro/settings/permissions.yaml`, `~/.kiro/workspace-roots/<hash>/permissions.yaml`, `sandbox.json` (both scopes), `~/.kiro/settings/cli.json` | user, project | settings | | KR-cfg |
| `.kiro/specs/` (`requirements.md` or `bugfix.md`, `design.md`, `tasks.md`); `.kiro/workflows/`, `~/.kiro/workflows/`; `~/.kiro/powers/` | project, user | instructions, command, plugin | | KR-cfg, KR-specs, KR-powers |

**Credentials (Kiro):** **UNVERIFIED**. The vendor docs give no path. Third-party sources name `~/.aws/sso/cache/kiro-auth-token.json` (IDE) and `data.sqlite3` under `~/.local/share/kiro-cli/` or `~/Library/Application Support/kiro-cli/` (CLI). `KIRO_API_KEY` is an environment variable. Source: KR-auth.

## 10. Kimi Code CLI (Moonshot); Kimi CLI is archived
`KIMI_CODE_HOME` defaults to `~/.kimi-code`.

| Glob | Scope | Kind | Format and precedence | Doc |
|---|---|---|---|---|
| `$KIMI_CODE_HOME/AGENTS.md`; `~/.agents/AGENTS.md`; project `AGENTS.md` or `.kimi-code/AGENTS.md` | user, project | instructions | The discovery walk is not documented. | KM-agents, KM-data |
| `.kimi-code/skills/`, `.agents/skills/` (project root = nearest `.git`); `~/.kimi-code/skills/`, `~/.agents/skills/`; `extra_skill_dirs` | project, user | skill | Keys: `name`, `description`, `type`, `whenToUse`, `disableModelInvocation`, `arguments`. Priority: Project > User > Extra > Built-in. | KM-skills |
| `.kimi-code/agents/`, `.agents/agents/`, `~/.kimi-code/agents/`, `~/.agents/agents/` | project, user | agent | Keys: `name`, `description` (required), `whenToUse`, `override`, `tools`, `disallowedTools`, `subagents`. | KM-agents |
| `~/.kimi-code/config.toml` (holds `[[hooks]]`; no project hook file), `~/.kimi-code/tui.toml`, `.kimi-code/local.toml`, `~/.kimi-code/projects.json` | user, project, local | settings, hook | | KM-cfg, KM-hooks |
| `.kimi-code/mcp.json` (trusted only), `~/.kimi-code/mcp.json` | project, user | MCP | Project wins. | KM-mcp |
| Legacy: `~/.kimi/config.toml`, `~/.kimi/mcp.json`, `~/.kimi/skills/`, `.kimi/skills/`; also reads `.claude/skills/`, `.codex/skills/` and their `~` equivalents | user, project | settings, MCP, skill | Skill priority kimi > claude > codex. | KM-old, KM-oldskills |

**Credentials (Kimi):**
- Kimi Code: `~/.kimi-code/credentials/<name>.json` and `~/.kimi-code/credentials/mcp/*.json`; `config.toml` may hold `api_key`; `sessions/**/wire.jsonl` holds conversation content.
- Legacy Kimi CLI: `~/.kimi/credentials/`, `~/.kimi/mcp-oauth/`, and `~/.kimi/mcp.json` (header values can hold keys).
- Sources: KM-data, KM-cfg, KM-old.

## 11. Continue

| Glob | Scope | Kind | Format and precedence | Doc |
|---|---|---|---|---|
| `.continue/rules/*.md` (YAML form deprecated) | project | rules | Keys: `name`, `globs`, `regex`, `description`, `alwaysApply`. Loaded in lexicographic order. | CO-rules |
| `~/.continue/config.yaml` (`%USERPROFILE%\.continue\config.yaml`), `~/.continue/config.ts`; deprecated `config.json`, `.continuerc.json` | user, project | settings (rules, prompts, MCP, models) | | CO-cfg |
| `.continue/mcpServers/*.yaml|*.json` (JSON copied from other tools is accepted) | project | MCP | Secrets as `${{ secrets.X }}`. | CO-mcp |
| Prompt `.md` files with `invokable: true` (`name`, `description`) | project | command | The folder (`.continue/prompts/`) is **UNVERIFIED**; the page does not name it. | CO-prompts |
| `~/.continue/rules`; `AGENTS.md` support; `~/.continue/.env` | | | **UNVERIFIED**; not found on vendor pages. | (none) |

**Credentials (Continue):** "locally stored environment secrets" are referenced but their file is not documented (UNVERIFIED). `config.yaml` can hold inline `apiKey` values (key name not verified on the fetched page).

## 12. Aider

| Glob | Scope | Kind | Format and precedence | Doc |
|---|---|---|---|---|
| `CONVENTIONS.md` (any name), loaded only through `--read`, `/read` or the `read:` key | project | instructions | Not auto-discovered. | AD-conv |
| `.aider.conf.yml` in home, git root and cwd | user, project | settings | Loaded in that order; the last loaded wins. `--config` loads only that file. | AD-conf |
| `.env` in home, git root, cwd, or `--env-file` | user, project | settings | The last loaded wins. | AD-env |
| `.aiderignore` (git root), `.aider.model.settings.yml`, `.aider.model.metadata.json` | project | settings | | AD-conf |
| `.aider.chat.history.md`, `.aider.input.history` | project | state | Transcripts. | AD-conf |

**Credentials (Aider):** `.env` files (API keys for every provider) and `.aider.conf.yml` (may hold OpenAI and Anthropic keys). `AGENTS.md`: agents.md lists Aider, but the Aider docs do not mention it (UNVERIFIED).

## 13. AGENTS.md convention (agents.md)
Plain Markdown with no required fields. Put the main file at the repo root, add nested files in subprojects, and the nearest file to the edited file wins; explicit chat prompts override all. agents.md lists: Codex, Jules, Factory, Aider, goose, opencode, Zed, Warp, VS Code, Devin, UiPath, Junie, Amp, Cursor, RooCode, Gemini CLI, Kilo Code, Phoenix, Semgrep, GitHub Copilot coding agent, Ona, Windsurf, Augment Code. Source: AM.

---

## Merged table
Each row is verified (yes) unless the last column says no. The Doc column uses the legend keys.

| Glob | Scope | Kind | Providers that read it | Doc | Verified |
|---|---|---|---|---|---|
| `AGENTS.md` (root plus nested) | project, nested | instructions | Codex, Claude Code (fallback), Cursor, Copilot, Devin Desktop/CLI, Cline (root), Kiro, Kimi Code, Grok, Gemini (via `context.fileName`) | CX-agents, CC-mem, CU-rules, GH-instr, WS-mem, CL-rules, KR-steer, KM-agents, GK-rules, GM-md | yes |
| `AGENTS.override.md` | project, user | instructions | Codex | CX-agents | yes |
| `AGENTS.local.md`, `AGENT.md` | project, local | instructions | Devin (Desktop and CLI); Grok (`AGENT.md`) | WS-mem, DV-rules, GK-rules | yes |
| `~/.codex/AGENTS.md` | user | instructions | Codex, VS Code (Codex harness) | CX-agents, VS-ref | yes |
| `~/.agents/AGENTS.md` | user | instructions | Cline, Kimi Code | CL-rules, KM-data | yes |
| `~/.config/devin/AGENTS.md` | user | instructions | Devin CLI | DV-rules | yes |
| `CLAUDE.md`, `.claude/CLAUDE.md` | project, nested | instructions | Claude Code, Cursor, Copilot (root), VS Code, Grok, Devin CLI | CC-mem, CU-help, GH-instr, VS-ref, GK-rules, DV-rules | yes |
| `CLAUDE.local.md` | project, local | instructions | Claude Code, VS Code (Local), Grok | CC-mem, VS-ref, GK-rules | yes |
| `~/.claude/CLAUDE.md` | user | instructions | Claude Code, VS Code, Devin CLI | CC-mem, VS-ref, DV-rules | yes |
| `.claude/rules/**/*.md` | project | rules | Claude Code, VS Code, Grok | CC-mem, VS-ref, GK-rules | yes |
| `~/.claude/rules/` | user | rules | Claude Code, VS Code | CC-mem, VS-ref | yes |
| `GEMINI.md`, `~/.gemini/GEMINI.md` | project, user | instructions | Gemini CLI, Copilot (root `GEMINI.md`) | GM-md, GH-instr | yes |
| `.github/copilot-instructions.md`, `~/.copilot/copilot-instructions.md` | project, user | instructions | Copilot | GH-instr, VS-ref | yes |
| `.github/instructions/**/*.instructions.md`, `~/.copilot/instructions/` | project, user | rules | Copilot | GH-instr, VS-ref | yes |
| `.cursor/rules/**/*.mdc` | project | rules | Cursor, Grok, Devin CLI | CU-rules, GK-rules, DV-rules | yes |
| `~/.cursor/rules` | user | rules | Cursor | CU-help | yes |
| `.cursorrules` | project | rules (legacy) | Cursor, Cline | CU-help, CL-rules | yes |
| `.windsurfrules`, `.windsurf/rules/*.md` | project | rules (legacy) | Devin Desktop/CLI, Cline (`.windsurfrules`) | WS-mem, DV-rules, CL-rules | yes |
| `.devin/rules/*.md`, `.devin/global_rules.md`, `~/.devin/rules/`, `~/.devin/global_rules.md` | project, user | rules | Devin Desktop/CLI | WS-mem, DV-rules | yes |
| `~/.codeium/windsurf/memories/global_rules.md` | user | instructions (legacy) | Windsurf | localized docs.devin.ai pages | partial |
| `Windsurf/rules/*.md` under `/Library/Application Support/`, `/etc/windsurf/` or `C:\ProgramData\` | managed | rules | Windsurf | WS-plug | yes |
| `.clinerules/`, `.cline/rules/`, `~/Documents/Cline/Rules`, `~/.cline/rules` | project, user | rules | Cline | CL-rules | yes |
| `.kiro/steering/*.md`, `~/.kiro/steering/` | project, user | rules | Kiro | KR-steer | yes |
| `.continue/rules/*.md` | project | rules | Continue | CO-rules | yes |
| `.grok/rules/*.md`, `~/.grok/` | project, user | rules | Grok | GK-rules | yes |
| `CONVENTIONS.md` (explicit `--read` only) | project | instructions | Aider | AD-conv | yes |
| Managed `CLAUDE.md` (3 OS paths) | managed | instructions | Claude Code | CC-mem | yes |
| `~/.claude/projects/*/memory/**`, `.claude/agent-memory/**`, `~/.claude/agent-memory/**` | user, project | memory | Claude Code | CC-dir | yes |
| `$CODEX_HOME/memories/`, `$CODEX_HOME/memories_v2/` | user | memory | Codex | CX-mem | yes |
| `.agents/skills/*/SKILL.md` | project | skill | Codex, Cursor, Gemini, Copilot (Codex and Local harnesses), Devin, Cline, Kimi Code | CX-skills, CU-skills, GM-skills, VS-skills, WS-skills, CL-cfg, KM-skills | yes |
| `~/.agents/skills/` | user | skill | Codex, Cursor, Gemini, Copilot, Devin, Cline, Kimi Code, Grok | same as above plus GK-skills | yes |
| `.claude/skills/`, `~/.claude/skills/` | project, user | skill | Claude Code, Cursor, Copilot, Devin CLI, legacy Kimi CLI, Grok (unspecified path) | CC-skills, CU-skills, VS-skills, DV-skills, KM-oldskills, GK-skills | yes |
| `.codex/skills/`, `~/.codex/skills/` | project, user | skill | Cursor, Devin CLI, legacy Kimi CLI (not listed in Codex's own docs) | CU-skills, DV-skills, KM-oldskills | yes |
| `.cursor/skills/`, `~/.cursor/skills/` | project, user | skill | Cursor | CU-skills | yes |
| `.github/skills/`, `~/.copilot/skills/` | project, user | skill | Copilot, Devin CLI | VS-skills, GH-cli, DV-skills | yes |
| `.gemini/skills/`, `~/.gemini/skills/` | project, user | skill | Gemini | GM-skills | yes |
| `.grok/skills/`, `~/.grok/skills/` | project, user | skill | Grok | GK-skills | yes |
| `.devin/skills/`, `.windsurf/skills/`, `~/.config/devin/skills/`, `~/.codeium/<ch>/skills/` | project, user | skill | Devin Desktop/CLI | WS-skills, DV-skills | yes |
| `.cline/skills/`, `~/.cline/skills/` | project, user | skill | Cline | CL-cfg | yes |
| `.kiro/skills/`, `~/.kiro/skills/` | project, user | skill | Kiro, Devin CLI | KR-skills, DV-skills | yes |
| `.kimi-code/skills/`, `~/.kimi-code/skills/`, `.kimi/skills/`, `~/.kimi/skills/` | project, user | skill | Kimi | KM-skills, KM-oldskills | yes |
| `/etc/codex/skills/` | managed | skill | Codex | CX-skills | yes |
| `.claude/agents/`, `~/.claude/agents/` | project, user | agent | Claude Code, Cursor | CC-agents, CU-sub | yes |
| `.cursor/agents/`, `~/.cursor/agents/` | project, user | agent | Cursor | CU-sub | yes |
| `.grok/agents/`, `~/.grok/agents/` | project, user | agent | Grok, Cursor (opt-in) | GK-sub, CU-sub | yes |
| `.codex/agents/*.toml`, `~/.codex/agents/` | project, user | agent | Codex | CX-sub | yes |
| `.gemini/agents/*.md`, `~/.gemini/agents/` | project, user | agent | Gemini | GM-sub | yes |
| `.github/agents/*.agent.md`, org `agents/`, `~/.copilot/agents/` | project, org, user | agent | Copilot | GH-agentloc, VS-agents, GH-cli | yes |
| `.kiro/agents/`, `~/.kiro/agents/` | project, user | agent | Kiro | KR-agents | yes |
| `.kimi-code/agents/`, `.agents/agents/`, `~/.kimi-code/agents/`, `~/.agents/agents/` | project, user | agent | Kimi Code | KM-agents | yes |
| `.cline/agents/`, `~/.cline/agents/` | project, user | agent | Cline | CL-cfg | yes |
| `.claude/commands/**/*.md`, `~/.claude/commands/` | project, user | command | Claude Code | CC-skills | yes |
| `~/.codex/prompts/*.md` | user | command (deprecated) | Codex | CX-prompts | yes |
| `.gemini/commands/**/*.toml`, `~/.gemini/commands/` | project, user | command | Gemini | GM-cmd | yes |
| `.github/prompts/*.prompt.md` | project | command | Copilot (VS Code Local) | VS-prompt | yes |
| `.devin/workflows/`, `.windsurf/workflows/`, `~/.codeium/windsurf/global_workflows/` | project, user | command | Devin Desktop | WS-wf | yes |
| `.cline/workflows/`, `~/.cline/workflows/`, `~/Documents/Cline/Workflows/` | project, user | command | Cline | CL-cfg | yes |
| `.kiro/workflows/`, `~/.kiro/workflows/` | project, user | command | Kiro | KR-cfg | yes |
| `~/.agents/commands/` | user | command | Grok | GK-skills | yes |
| `.claude/workflows/*.js`, `~/.claude/workflows/` | project, user | command | Claude Code | CC-dir | yes |
| `.cursor/commands/*.md`, `~/.cursor/commands/` | project, user | command | Cursor | (none) | no: official page had no path text |
| `.claude/settings.json` | project | settings, hook | Claude Code, Copilot (hooks and a subset of settings), Devin CLI (hooks); Cursor (Claude hooks, files not named) | CC-set, VS-hooks, GH-cli, DV-hooks | yes (Cursor: no) |
| `.claude/settings.local.json` | project, local | settings, hook | Claude Code, Copilot, Devin CLI | CC-set, VS-hooks, DV-hooks | yes |
| `~/.claude/settings.json` | user | settings, hook | Claude Code, Devin CLI | CC-set, DV-hooks | yes |
| `~/.claude.json` | user | MCP, state | Claude Code, Grok (MCP), Devin CLI (hooks) | CC-dir, GK-mcp, DV-hooks | yes |
| `.mcp.json` | project | MCP | Claude Code, Copilot, Grok | CC-dir, VS-mcp, GK-mcp | yes |
| `.cursor/mcp.json`, `~/.cursor/mcp.json` | project, user | MCP | Cursor, Grok (project file) | CU-mcp, GK-mcp | yes |
| `.vscode/mcp.json`, `.github/mcp.json`, `~/.copilot/mcp-config.json` | project, user | MCP | Copilot | VS-mcp, GH-cli | yes |
| `.codex/config.toml`, `~/.codex/config.toml`, `/etc/codex/config.toml` | project, user, system | settings, MCP, hook | Codex | CX-cfg | yes |
| `.gemini/settings.json`, `~/.gemini/settings.json`, system `settings.json` | project, user, managed | settings, MCP, hook | Gemini | GM-cfg | yes |
| `.grok/config.toml`, `~/.grok/config.toml`, `/etc/grok/*.toml`, `~/.grok/{managed_config,requirements}.toml` | project, user, managed | settings, MCP | Grok | GK-set, GK-ent | yes |
| `.github/copilot/settings{,.local}.json`, `~/.copilot/settings.json` | project, local, user | settings, hook | Copilot | VS-hooks, GH-cli | yes |
| `.devin/config{,.local}.json`, `~/.config/devin/config.json`, `.devin/mcp_config{,.local}.json` | project, local, user | settings, MCP, hook | Devin | DV-cfg, WS-mcp | yes |
| `.kiro/settings/mcp.json`, `~/.kiro/settings/*` | project, user | MCP, settings | Kiro | KR-cfg | yes |
| `.kimi-code/{mcp.json,local.toml}`, `~/.kimi-code/{config.toml,mcp.json}` | project, user | settings, MCP, hook | Kimi Code | KM-cfg, KM-mcp | yes |
| `.continue/mcpServers/*`, `~/.continue/config.yaml` | project, user | MCP, settings | Continue | CO-mcp, CO-cfg | yes |
| `~/.cline/data/settings/cline_mcp_settings.json` | user | MCP | Cline | CL-cfg | yes |
| `.aider.conf.yml`, `.env` | user, project | settings | Aider (Gemini also reads `.env`) | AD-conf, AD-env, GM-cfg | yes |
| `~/.cursor/cli-config.json`, `.cursor/cli.json` | user, project | settings | Cursor CLI | CU-cli | yes |
| `.cursor/hooks.json`, `~/.cursor/hooks.json`, 3 enterprise paths | project, user, managed | hook | Cursor | CU-hooks | yes |
| `hooks.json` in `.codex/`, `$CODEX_HOME/`, `/etc/codex/`, `%ProgramData%\OpenAI\Codex\` | project, user, managed | hook | Codex | CX-hooks | yes |
| `.github/hooks/*.json`, `~/.copilot/hooks/` | project, user | hook | Copilot | VS-hooks | yes |
| `.devin/hooks.v1.json` | project | hook | Devin | WS-hooks, DV-hooks | yes |
| `.kiro/hooks/*.json`, `~/.kiro/hooks/` | project, user | hook | Kiro | KR-hooks | yes |
| `.grok/hooks/`, `~/.grok/hooks/` | project, user | hook | Grok | GK-skills | yes |
| `.cline/hooks/`, `~/.cline/hooks/`, `~/Documents/Cline/Hooks/` | project, user | hook | Cline | CL-cfg | yes |
| `<plugin>/hooks/hooks.json` | plugin | hook | Claude Code, Codex, Gemini extensions | CC-hooks, CX-hooks, GM-ext | yes |
| `.codex/rules/*.rules`, `~/.codex/rules/*.rules` | project, user | rules (exec policy) | Codex | CX-rules | yes |
| `.cursorignore`, `.geminiignore`, `.aiderignore`, `.clineignore` (deprecated) | project | ignore | Cursor, Gemini, Aider, Cline | CU-ignore, GM-md, AD-conf, CL-cfg | yes |
| Claude Code managed settings dir (`managed-settings.json`, `managed-settings.d/`, `managed-mcp.json`) | managed | settings, MCP, skills, agents | Claude Code | CC-managed | yes |
| `requirements.toml`, `managed_config.toml` (Codex paths) | managed | settings | Codex | CX-managed | yes |
| Gemini `system-defaults.json`, system `settings.json` (3 OS paths) | managed | settings | Gemini | GM-cfg | yes |
| `/Library/Application Support/Devin/system.json`, `/etc/devin/system.json`, `C:\ProgramData\Devin\system.json`; `Devin/workflows/` under the same roots | managed | settings, command | Devin | DV-sys, WS-wf | yes |

## Files shared by several providers
- **`AGENTS.md`:** Codex, Claude Code (fallback when no `CLAUDE.md`), Cursor, Copilot (cloud agent, VS Code, CLI), Devin Desktop/CLI (formerly Windsurf), Cline, Kiro, Kimi Code, Grok, and Gemini when `context.fileName` names it. agents.md also lists Aider, goose, opencode, Zed, Warp, Jules, Factory, Amp, Junie, RooCode, Kilo Code, Augment and others; those were not checked against their own docs.
- **`CLAUDE.md`:** Claude Code, Cursor (always applied), Copilot (root file only), VS Code Claude harness, Grok, Devin CLI.
- **`CLAUDE.local.md`:** Claude Code, VS Code, Grok.
- **`.claude/rules/`:** Claude Code, VS Code, Grok.
- **`.claude/skills/` and `~/.claude/skills/`:** Claude Code, Cursor, Copilot, Devin CLI, legacy Kimi CLI, Grok.
- **`.claude/agents/` and `~/.claude/agents/`:** Claude Code, Cursor.
- **`.claude/settings.json` and `.claude/settings.local.json` (hooks):** Claude Code, Copilot, Devin CLI; Cursor too, though it does not name the files.
- **`~/.claude.json`, `.mcp.json`:** Claude Code, Grok, Copilot (`.mcp.json`), Devin CLI (hooks in `~/.claude.json`).
- **`.agents/skills/` and `~/.agents/skills/`:** Codex, Cursor, Gemini, Copilot, Devin, Cline, Kimi Code, Grok (`~` only).
- **`~/.agents/AGENTS.md`:** Cline, Kimi Code.
- **`.codex/skills/`:** Cursor, Devin CLI, legacy Kimi CLI. Codex's own docs do not list it.
- **`.cursor/rules/`:** Cursor, Grok, Devin CLI.
- **`.cursor/mcp.json`:** Cursor, Grok.
- **`.windsurfrules`, `.windsurf/rules/`:** Devin, Cline (`.windsurfrules`). **`.cursorrules`:** Cursor, Cline.
- **`.grok/agents/`:** Grok, Cursor (opt-in).
- **`.kiro/skills/`:** Kiro, Devin CLI. **`.github/skills/`:** Copilot, Devin CLI.
- **`<plugin>/hooks/hooks.json`:** Claude Code, Codex, Gemini extensions.

## Credential deny-list (never read)
**Verified on a vendor page:**
- **Claude Code:** `~/.claude/.credentials.json` (`%USERPROFILE%\.claude\.credentials.json`, or under `$CLAUDE_CONFIG_DIR`); `~/.claude/policy-limits.json` and `policy-limits.json.stamp.json`; `~/.config/anthropic/**` and `%APPDATA%\Anthropic\**` (profiles, `configs/`, `active_config`).
- **Codex:** `~/.codex/auth.json` (`$CODEX_HOME/auth.json`).
- **Gemini CLI:** `~/.gemini/mcp-oauth-tokens.json`.
- **Grok:** `~/.grok/auth.json`, `~/.grok/mcp_credentials.json`.
- **Copilot:** `~/.copilot/config.json` (authentication data), `~/.copilot/providers.json` (plaintext keys and tokens), `~/.copilot/mcp-oauth-config/**`, `~/.copilot/mcp-secrets/**`.
- **Devin CLI:** `~/.local/share/devin/credentials.toml` (`$XDG_DATA_HOME/devin/credentials.toml`), `%APPDATA%\devin\credentials.toml`.
- **Cline:** `~/.cline/data/settings/providers.json`.
- **Kimi Code:** `~/.kimi-code/credentials/**` (including `credentials/mcp/*.json`).
- **Kimi CLI (legacy):** `~/.kimi/credentials/**`, `~/.kimi/mcp-oauth/**`.
- **Gemini and Aider:** `.env` files (home, git root, cwd, `~/.gemini/.env`, `.gemini/.env`, `~/.env`) can hold API keys.

**Config files that may contain secrets inline (verified):**
- `~/.codex/config.toml` (MCP `oauth.client_secret`)
- `~/.grok/config.toml` (`api_key`)
- `~/.kimi-code/config.toml` (`api_key`)
- `~/.kimi/mcp.json` (headers)
- `.aider.conf.yml` (OpenAI and Anthropic keys)
- `~/.claude.json` (account details, MCP env)
- `.mcp.json`, `.cursor/mcp.json`, `~/.cursor/mcp.json`, `.vscode/mcp.json` (env, headers)

**Transcripts and history that may contain secrets (verified):**
- `~/.claude/projects/**/*.jsonl`, `~/.claude/history.jsonl`, `~/.claude/paste-cache/`, `~/.claude/file-history/`
- `$CODEX_HOME/history.jsonl`, `$CODEX_HOME/log/`
- `~/.copilot/session-state/`, `~/.copilot/session-store.db`
- `~/.kimi-code/sessions/**/wire.jsonl`, `~/.kimi/sessions/`
- `~/.cline/data/sessions/`
- `.aider.chat.history.md`, `.aider.input.history`

**UNVERIFIED (third-party sources only, or the vendor doc names no path):**
- Gemini CLI: `~/.gemini/oauth_creds.json`, `~/.gemini/google_accounts.json`.
- Codex MCP OAuth file fallback: `~/.codex/.credentials.json`; the vendor doc names no file.
- Kiro: `~/.aws/sso/cache/kiro-auth-token.json` (IDE); `~/.local/share/kiro-cli/data.sqlite3` and `~/Library/Application Support/kiro-cli/data.sqlite3` (CLI).
- No path in vendor docs: Cursor CLI credentials, Windsurf/Devin Desktop credentials, Continue local secrets.
- OS keychain or keyring entries: used by Claude Code on macOS, Codex (`keyring`), Copilot, and VS Code secret storage. These are not files.

## URL legend
- **Claude Code:**
  - CC-mem https://code.claude.com/docs/en/memory
  - CC-set https://code.claude.com/docs/en/settings
  - CC-skills https://code.claude.com/docs/en/skills
  - CC-agents https://code.claude.com/docs/en/sub-agents
  - CC-hooks https://code.claude.com/docs/en/hooks
  - CC-dir https://code.claude.com/docs/en/claude-directory
  - CC-auth https://code.claude.com/docs/en/authentication
  - CC-managed https://code.claude.com/docs/en/managed-settings
- **Codex** (the old `developers.openai.com/codex/...` URLs redirect here):
  - CX-agents https://learn.chatgpt.com/docs/agent-configuration/agents-md
  - CX-cfg https://learn.chatgpt.com/docs/config-file/config-basic
  - CX-ref https://learn.chatgpt.com/docs/config-file/config-reference.md
  - CX-skills https://learn.chatgpt.com/docs/build-skills
  - CX-auth https://learn.chatgpt.com/docs/auth
  - CX-managed https://learn.chatgpt.com/docs/enterprise/managed-configuration.md
  - CX-hooks https://learn.chatgpt.com/docs/hooks.md
  - CX-sub https://learn.chatgpt.com/docs/agent-configuration/subagents.md
  - CX-rules https://learn.chatgpt.com/docs/agent-configuration/rules.md
  - CX-mcp https://learn.chatgpt.com/docs/extend/mcp.md
  - CX-mem https://learn.chatgpt.com/docs/customization/memories.md
  - CX-prompts https://learn.chatgpt.com/docs/custom-prompts.md
- **Cursor:**
  - CU-rules https://cursor.com/docs/context/rules
  - CU-help https://cursor.com/help/customization/rules
  - CU-skills https://cursor.com/docs/context/skills
  - CU-sub https://cursor.com/docs/context/subagents
  - CU-hooks https://cursor.com/docs/agent/hooks
  - CU-mcp https://cursor.com/docs/context/mcp
  - CU-cli https://cursor.com/docs/cli/reference/configuration
  - CU-auth https://cursor.com/docs/cli/reference/authentication
  - CU-ignore https://cursor.com/help/customization/ignore-files
- **Gemini CLI:**
  - GM-md https://geminicli.com/docs/cli/gemini-md/
  - GM-cfg https://geminicli.com/docs/reference/configuration/
  - GM-cmd https://geminicli.com/docs/cli/custom-commands/
  - GM-skills https://geminicli.com/docs/cli/skills/
  - GM-cskill https://geminicli.com/docs/cli/creating-skills/
  - GM-sub https://geminicli.com/docs/core/subagents/
  - GM-hooks https://geminicli.com/docs/hooks/
  - GM-mcp https://geminicli.com/docs/tools/mcp-server/
  - GM-ext https://geminicli.com/docs/extensions/reference/
  - GM-auth https://geminicli.com/docs/get-started/authentication/
- **Grok Build:**
  - GK-rules https://docs.x.ai/build/features/project-rules
  - GK-set https://docs.x.ai/build/settings/reference.md
  - GK-skills https://docs.x.ai/build/features/skills-plugins-marketplaces
  - GK-sub https://docs.x.ai/build/features/subagents
  - GK-mcp https://docs.x.ai/build/features/mcp-servers
  - GK-ent https://docs.x.ai/build/enterprise
- **GitHub Copilot:**
  - GH-instr https://docs.github.com/en/copilot/how-tos/configure-custom-instructions/add-repository-instructions
  - GH-cli https://docs.github.com/en/copilot/reference/copilot-cli-reference/cli-config-dir-reference
  - GH-agents https://docs.github.com/en/copilot/reference/custom-agents-configuration
  - GH-agentloc https://docs.github.com/en/copilot/how-tos/copilot-on-github/customize-copilot/customize-cloud-agent/create-custom-agents
  - VS-ref https://code.visualstudio.com/docs/agents/reference/custom-instructions-reference
  - VS-prompt https://code.visualstudio.com/docs/copilot/customization/prompt-files
  - VS-agents https://code.visualstudio.com/docs/copilot/customization/custom-agents
  - VS-skills https://code.visualstudio.com/docs/copilot/customization/agent-skills
  - VS-hooks https://code.visualstudio.com/docs/copilot/customization/hooks
  - VS-mcp https://code.visualstudio.com/docs/agents/reference/mcp-configuration
- **Windsurf / Devin** (`docs.windsurf.com/windsurf/cascade/memories` redirects to WS-mem):
  - WS-mem https://docs.devin.ai/desktop/cascade/memories
  - WS-wf https://docs.devin.ai/desktop/cascade/workflows
  - WS-skills https://docs.devin.ai/desktop/cascade/skills
  - WS-hooks https://docs.devin.ai/desktop/cascade/hooks
  - WS-mcp https://docs.devin.ai/desktop/cascade/mcp
  - WS-plug https://docs.devin.ai/windsurf/plugins/cascade/memories.md
  - DV-rules https://docs.devin.ai/cli/extensibility/rules.md
  - DV-cfg https://docs.devin.ai/cli/reference/configuration/global-vs-local.md
  - DV-hooks https://docs.devin.ai/cli/extensibility/hooks/overview.md
  - DV-skills https://docs.devin.ai/cli/extensibility/skills/overview.md
  - DV-sys https://docs.devin.ai/cli/enterprise/system-config.md
  - DV-auth https://docs.devin.ai/cli/enterprise/devin-auth.md
  - DV-pol https://docs.devin.ai/desktop/enterprise-policies.md
- **Cline:**
  - CL-rules https://docs.cline.bot/features/cline-rules
  - CL-cfg https://docs.cline.bot/getting-started/config.md
- **Kiro:**
  - KR-steer https://kiro.dev/docs/steering/
  - KR-skills https://kiro.dev/docs/skills/
  - KR-agents https://kiro.dev/docs/custom-agents/
  - KR-hooks https://kiro.dev/docs/hooks/
  - KR-cfg https://kiro.dev/docs/configuration/
  - KR-specs https://kiro.dev/docs/specs/
  - KR-powers https://kiro.dev/docs/powers/
  - KR-auth https://kiro.dev/docs/getting-started/authentication/
- **Kimi:**
  - KM-cfg https://moonshotai.github.io/kimi-code/en/configuration/config-files.html
  - KM-data https://moonshotai.github.io/kimi-code/en/configuration/data-locations.html
  - KM-skills https://moonshotai.github.io/kimi-code/en/customization/skills
  - KM-mcp https://moonshotai.github.io/kimi-code/en/customization/mcp.html
  - KM-hooks https://moonshotai.github.io/kimi-code/en/customization/hooks.html
  - KM-agents https://www.kimi.com/code/docs/en/kimi-code-cli/customization/agents
  - KM-old https://moonshotai.github.io/kimi-cli/en/configuration/data-locations.html
  - KM-oldskills https://moonshotai.github.io/kimi-cli/en/customization/skills.html
  - Migration guide: https://moonshotai.github.io/kimi-code/en/guides/migration
- **Continue:**
  - CO-rules https://docs.continue.dev/customize/deep-dives/rules
  - CO-cfg https://docs.continue.dev/customize/deep-dives/configuration
  - CO-mcp https://docs.continue.dev/customize/deep-dives/mcp
  - CO-prompts https://docs.continue.dev/customize/deep-dives/prompts
- **Aider:**
  - AD-conv https://aider.chat/docs/usage/conventions.html
  - AD-conf https://aider.chat/docs/config/aider_conf.html
  - AD-env https://aider.chat/docs/config/dotenv.html
- **AGENTS.md:**
  - AM https://agents.md/
