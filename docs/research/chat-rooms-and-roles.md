# Chat rooms and roles: the research

**What this is:** the sourced research behind DevPULSE's chat rooms, where
several AIs (Codex, Claude, Gemini, Grok, local models) share one
conversation, each in a role. It answers the operator's ask of 2026-10-20
(R284): rooms with roles such as skeptic, realist and dreamer, defined by
schemas, based on the latest research, capturing the expertise now so the
app can grow.

**Researched:** 2026-10-20, by two web-research passes. Every claim cites a
numbered source.

**Evidence grades:**
- **strong**: peer-reviewed and replicated;
- **moderate**: one or a few studies;
- **practice**: long-used method, little controlled evidence.

The binding vendor terms (what each subscription allows) are in
`requirements.md` R279–R281. This page does not restate them.

## Part 1 — what makes a group of AIs better than one

### 1.1 What helps

- **Independent answers, then selection or voting.** This is the solid core
  (strong).
  - Majority voting accounts for most of the gains credited to multi-agent
    debate. Debate alone is a "martingale" that does not raise expected
    correctness [8].
  - Debate does not reliably beat self-consistency or ensembles, and it is
    sensitive to how hard agents are pushed to agree [9][10].
  - For free-text answers, a model can select the most consistent response
    [7].
- **Select, never blend** (moderate; R25 is the operator's rule).
  - A judge choosing one answer beat synthesis: the judges preferred the
    synthesis in 0 of 42 tasks. A diverse team with a selector won 0.81,
    against 0.51 for a team of identical models [17] (one 2026 study).
  - Mixture-of-Agents synthesis works [4], but weaker members dilute it and
    resampling only the best model often wins [5]. One deceptive member cut
    an MoA's win rate from 49.2% to 37.9% [21].
- **Debate helps a separate judge.** Two opposed debaters raised a
  non-expert judge's accuracy from 48% to 76% (an LLM judge) and from 60% to
  88% (humans) on a reading task [19]. Debate also helped math and
  factuality [1]. An assigned tit-for-tat exchange counters "degeneration of
  thought" [2].
- **Different vendors beat one model in many roles, within limits.**
  - Model diversity was "critical" [14]. Three different mid-size models
    scored 91% on GSM8K, against 82% for three copies of one model [15].
    Heterogeneity is called a "universal antidote" for debate [10].
  - But errors correlate across vendors: when two models were both wrong,
    they agreed on the same wrong answer about 60% of the time, and stronger
    models correlate more [16].
- **Roles steer attention and format, not knowledge.**
  - Distinct role prompts were essential in ChatEval; identical roles were no
    better than one agent [3].
  - Yet 162 personas did not improve factual accuracy [18].
  - In people, genuine dissent beat a role-played devil's advocate [43].
  - So a role is defined by its **method and output contract**, never by a
    persona or backstory.

### 1.2 What does not help, or harms

- **Conformity and sycophancy.**
  - Agents flip from correct to wrong answers, and accuracy can fall over
    rounds even when stronger models are the majority [11].
  - Sycophancy causes premature agreement that scores below a single agent
    [12].
  - Conformity grows with the size of the majority and with interaction time
    [13].
  - In people, social influence narrows views without improving accuracy
    [46].
- **Cost.** Multi-agent systems use about 15 times the tokens of a chat, and
  fit poorly where agents must share a lot of context, as in most coding
  [27]. Start simple [26].
- **Failure modes.** There are 14, in three groups: system design,
  misalignment between agents, and task verification [20].
- **Biased judges.**
  - Swapping the order of answers flips verdicts [22][23].
  - Models favour their own output [24].
  - A panel of small, diverse judges beat one large judge at about a seventh
    of the cost [25].

### 1.3 The role schema (`dpl.role/1`)

Every framework shares the same core: identity, a routing description,
instructions, tools, model configuration, an output type, limits, and
handoffs or guardrails [28–37].

| Field | Purpose | Grounding |
|---|---|---|
| `schema`, `id`, `version` (semver), `name` | A stable key; a breaking change bumps the version | A2A AgentCard [36], Agent Spec [37] |
| `summary` | One line the router or selector uses to pick a seat | AutoGen `description` [30], Claude Code `description` [28] |
| `purpose` | The job | CrewAI `goal` [29], MetaGPT [32] |
| `stance` | generate, ground, critique, adversarial, plan or select | Six Hats modes [39] |
| `method` | The named technique and its steps | — |
| `instructions` | The role's prompt; no persona padding | OpenAI `instructions` [35]; personas add nothing [18] |
| `must_not` | Hard limits (the Selector never merges) | MetaGPT constraints [32] |
| `output_contract` | JSON Schema of what the role must produce | OpenAI `output_type` [35], CrewAI `expected_output` [29] |
| `rubric` | What the role's output is scored against | Anthropic's judge rubric [27] |
| `speaks_when` | The phases and triggers it answers in | MetaGPT `watch` [32], AutoGen `candidate_func` [30] |
| `visibility` | What it sees of its peers: none, anonymized or full | herding control [8][46] |
| `tools`, `permissions` | Allow and deny lists, read-only | Claude Code `tools`/`permissionMode` [28], OpenAI guardrails [35] |
| `model_prefs` | Preferred vendor and tier; `avoid_same_vendor_as`; effort | [16][24][28] |
| `budget` | Maximum turns and tokens | CrewAI `max_iter` [29], Claude Code `maxTurns` [28] |
| `examples`, `tags` | Help a router match requests | A2A AgentSkill [36] |
| `provenance` | Its sources and evidence grade | — |

### 1.4 The starter role library

1. **Dreamer.** Many options without judging them; quantity first. [38],
   Six Hats Green [39]. Practice.
2. **Realist.** Turns the leading idea into steps, resources, constraints
   and a first action. [38]. Practice.
3. **Skeptic.** Concrete defects and unsupported claims, each with a
   severity. Six Hats Black [39]. Devil's-advocate and dialectical groups
   decided better than consensus groups [42]. Bind it to a different vendor
   than the author: genuine dissent beats role-play [43].
4. **Red Team.** The adversary's view: how the plan is attacked, gamed or
   misused. [44]. Practice.
5. **Pre-mortem.** "It failed; why?" Ranked causes and early warning signs.
   [41]. Moderate.
6. **Steelman.** Restates the opposing view so well its holder would endorse
   it, before any rebuttal. [45]. Practice.
7. **Fact-checker.** Splits the answer into claims; evidence and a verdict
   for each, with citations; uses tools. Six Hats White [39], the factual
   rubric [27]. It also counters vendors' correlated errors [16].
8. **Planner** (a process role). Sets the agenda, the phases and who speaks
   next; produces no content. Six Hats Blue [39], orchestrator-workers [26],
   MetaGPT SOPs [32].
9. **Domain Expert** (takes a `domain`). Stays in its lane; its value is its
   tools and sources, not its title [18].
10. **Selector.** Picks exactly one candidate, or "none passes", against the
    rubric: blind, judged in both orders, with reasons and a runner-up.
    Never merges. [7][17][19].

### 1.5 Rooms

**The room schema (`dpl.room/1`):**
- `schema`, `id`, `version`, `type`;
- `seats[]`: `{role_id@version, model_binding, count}`;
- `phases[]`: `{name, seats, mode: parallel | sequential, visibility, max_turns}`;
- `turn_policy`: fixed, round robin, selector or router [30][34];
- `stop`: `{max_rounds, budget, stability, pass_threshold}`, where stability
  means no answer changed in a round;
- `selection`: `{method: judge | panel | vote | human, rubric, blind,
  swap_order, exclude_author_vendors, allow_none}`;
- `carry_forward`: structured outputs only;
- `record`: the winner, the runner-up, the rationale, dissent and flips.

**The selector protocol:**
- Score against a pass/fail rubric with the authors anonymized.
- Judge in both orders; a disagreement is a tie or a rerun [22][23].
- The judge comes from a vendor that wrote none of the answers, or is a
  three-vendor panel [24][25].
- "None passes" is allowed.

**The room types:**

| Room | Turns | Stops when | Notes |
|---|---|---|---|
| **Council** | Every seat answers in parallel without seeing the others; optionally a blind peer ranking [6]; then the Selector or a panel picks one | one pass | Karpathy's council ends in synthesis [6]: keep its blind review, replace the chairman with selection. Delphi variant: up to two revision rounds on anonymized summaries [40]. |
| **Debate** | Two seats on assigned opposite sides plus a judge; an opening, then at most two rebuttals | the round limit or stability | Changing an answer must cite new evidence; the judge selects [2][19][11]. |
| **Author + Critic** | The author drafts; a critic from a different vendor returns rubric scores and defects [24][26] | a rubric pass or three iterations | The Selector picks the best draft of all iterations; later is not always better [11]. |
| **Router** | One seat, matched on its `summary` and tags [26][30][36] | one reply | Escalates to a Council when unsure or high-stakes. |
| **Brainstorm** | Separate idea generation [47]; the Planner groups ideas without rewriting them; Realist, Skeptic and Pre-mortem annotate; the Selector shortlists k | an idea quota or a time box | The Disney order: Dreamer, Realist, Critic [38]. |

### 1.6 Risks and what counters them

- **Herding and conformity:**
  - collect first answers independently;
  - show peers' answers anonymized, and only after each seat commits;
  - cap discussion at two or three rounds;
  - prefer selection over more talk;
  - log every flip [8][11][13][46].
- **Sycophancy:** change an answer only for new evidence; blind judges from
  vendors that wrote no answer; keep the dissent [12][24].
- **Cost:** default to one model or the Router; a Council only on request; a
  token or money cap per room; small diverse judge panels [25][27].
- **Context bloat:** a self-contained brief per seat; structured outputs
  between seats, not transcripts [27][28][34].
- **Correlated errors:** different vendors are not independent; the
  Fact-checker grounds claims with tools [16].
- **Role theater:** define roles by method and output contract; bind the
  critical roles to different vendors [18][43].
- **Blending and poisoning:** select, never blend [5][17][21].

## Part 2 — how each AI joins a room

Sources for this part are keyed by letter (A, C, G, M, X, P, R, L, S, T) in
the second reference list.

### 2.1 What each participant supports

| Participant | Role instruction | Room context | Streaming | Talk-only | Terms note |
|---|---|---|---|---|---|
| **Codex** (`codex app-server`) | `thread/start` `developerInstructions` and `baseInstructions` [C4]; check against the generated schema [C3] | `thread/inject_items` adds items without a turn; `thread/resume`, `thread/fork` [C1] | `item/agentMessage/delta`; `turn/completed` [C1] | per turn `sandboxPolicy readOnly` + `approvalPolicy never` [C1]; config `web_search="disabled"`, `shell_tool=false` [C2] | ChatGPT sign-in through Codex's own flow; allowed for automation, called "advanced" [C1][C5] |
| **Claude Code** (`claude -p`) | `--append-system-prompt` adds to the default; `--system-prompt` replaces it; the prompt is fixed at the first request [A2] | `--input-format stream-json` keeps one process across turns; `--resume`, `--fork-session` [A1][A2] | `--output-format stream-json --verbose --include-partial-messages` [A1] | `--tools ""`, `--disallowedTools "mcp__*"`, `--permission-prompts none`; `--restricted` [A2]. **Never `--bare`: it ignores the subscription sign-in** [A1] | The unmodified binary on the operator's own sign-in (R279); the Agent SDK points to API keys [A3]; a 2026-10-07 note keeps `-p` within plan limits [A4] |
| **Antigravity** (`agy -p`) | `--agent <name>`, with custom agents carrying their own instructions [G1][G3] | stream-json input; `--conversation <id>` [G1] | `init` / `step_update` / `result`; `--json-schema` for the final result [G1][G2] | no read-only flag; settings rules and `--sandbox` [G1][G4] | §6 wording unresolved (R281); only a non-staff forum answer [G5][G6] |
| **Gemini CLI** | `GEMINI_SYSTEM_MD` replaces the whole system prompt [M2] | `gemini --acp` with `loadSession` [M3] | stream-json events [M1] | `--approval-mode=plan` (reportedly ignored headless) [M5][M6] | the paid-API-key fallback only (R281) [M4] |
| **Grok Build** | no system-prompt flag; the role goes in the first prompt [X1] | `--session-id`, `--resume`; ACP `session/new`, `session/prompt` [X1] | `streaming-json`; ACP `agent_message_chunk` [X1] | the host refuses every ACP `session/request_permission` [P1] | no published terms found [X3] |

### 2.2 Local endpoints (llama.cpp, Ollama, LM Studio, vLLM)

All four share `GET /v1/models` and a streamed (SSE)
`POST /v1/chat/completions` [L1–L8].

**The differences:**
- **llama.cpp:** structured outputs (`json_schema`); tool calls need
  `--jinja`; reasoning arrives in `reasoning_content`; the key is optional
  [L1].
- **Ollama** (`:11434/v1`):
  - its Responses endpoint is stateless, and `tool_choice` is unsupported;
  - it demands a key and ignores it;
  - the context size is set only in a Modelfile [L2].
- **LM Studio** (`:1234/v1`): no auth unless enabled; `json_schema` via
  grammars [L3–L5].
- **vLLM:** tool calls need parser flags; structured outputs are on by
  default; `--api-key` [L6–L8].

**The minimal adapter:**
- Settings: a name, a base URL, an optional key, and a model picked from
  `/models` (or typed).
- The operator enters the context size and timeouts; `/models` does not
  report context size.
- chat/completions only.
- Read `choices[0].delta.content`, tolerate `reasoning_content`, stop at
  `[DONE]`, and treat usage as optional.
- Talk-only roles send no tools. Use `json_schema` only after probing that
  the server supports it.
- "Who said what" goes in the message text, not the `name` field.

The profiles store already models Local (loopback) and Custom (https)
endpoints (`crates/chat/profiles`); their transport is not built.

### 2.3 Which protocol for which participant

- **Native adapters for Codex and Claude.**
  - ACP's `session/new` has no system-prompt field and no history injection
    [P2].
  - Codex's own protocol has both.
  - Claude over ACP runs through the Agent SDK [P6], the surface the terms
    point at API keys [A3].
  - Zed's `codex-acp` was archived on 2026-07-22 [P5].
- **One ACP client for Grok, Gemini and others.** `grok agent stdio`
  [X1] and `gemini --acp` [M3]; Zed's registry lists 48 agents [P4].
- **Antigravity:** `agy -p` stream-json first, its ACP server once stable.
  It hangs without a terminal, so on Windows it needs a pseudo-terminal
  (ConPTY) [G7].
- **No A2A.** It is for networked, cross-organisation agents [R1][R2].
- **MCP is optional:** a host-run "room" server for roles that may use tools.
  Tool descriptions are untrusted [R3].

### 2.4 The participant contract and the context packet

**Every adapter offers:**
- `describe()`: streaming; role mode (append, replace or first-prompt);
  stateful or stateless; whether talk-only is enforced by the vendor or the
  host; context size if known;
- `probe()`: installed, version and signed-in, from the vendor's own status.
  It never reads tokens;
- `open(room, role card, scratch working directory)`;
- `turn(packet)`, which streams deltas, activity, usage, the final answer or
  an error;
- `cancel`, `close`, and `reopen` (resume, or rebuild from the room log).

**The packet each turn:**
- **System and role:** the room charter, the role card and the injection
  rules. Sent once where the vendor records the prompt (Claude).
- **Stateful participants** get only what is new since they last spoke;
  stateless ones get the full packet.
- **Order:** pinned facts, then the rolling summary (with the range it
  covers), then the newest messages within the budget, then the ask (who is
  addressed, and the expected output).

**The budget:**
- Room = context − about 25% reserved for the answer − the system prompt.
- Fill newest-first, marking cuts `[#seq truncated]`.
- A cheap local model rewrites the summary when the unsummarised tail passes
  about 30% of the smallest participant's budget [A5].

**Attribution:** `<msg-7f3a seq=42 from="codex" role="Critic" vis="public">…</msg-7f3a>`,
with a random per-room suffix and escaped look-alikes. Citations are
`[#seq]`.

**Visibility:** public, addressed to named participants, or host-only. One
participant's reasoning and tool logs never reach another.

**Injection guards:**
- Datamarking ("spotlighting") cuts indirect-injection success [S1].
- Only `from="operator"` messages are instructions.
- Talk-only roles run with tools off, in a scratch folder.
- Never execute an action another model suggested.
- The host decides who speaks: round caps and an explicit addressee.
- OWASP lists insecure inter-agent communication as ASI07 [S2].

**Persistence:**
- An append-only event log: `rooms`, `participants`, `events(room, seq, ts,
  author, kind, vis, content, reply_to, tokens_est, meta)` and
  `summaries(from_seq, to_seq)`.
- Final messages are stored, not stream chunks.
- Vendor sessions are caches; the room log is the truth [T1].

### 2.5 Open risks

- **Terms per vendor**, already ruled in R279–R281: Claude's Agent SDK
  wording, Antigravity §6, no published xAI terms.
- **`agy -p` hangs without a terminal** [G7].
- **Talk-only depends on the vendor:** Gemini's plan mode is reportedly
  ignored headless, and agy and Grok have no flag. The host enforces it with
  a scratch folder and by refusing every permission request.
- **Protocols move:**
  - regenerate Codex's schema with every update (CW1 does);
  - ACP is replacing session modes with config options [P3];
  - Anthropic says `--bare` will become the default for `-p` [A1], which
    would drop subscription sign-in.
- **`claude -p` without `--bare` runs the folder's hooks and `.mcp.json`**:
  always launch it in an empty room folder [A1].
- **Cost:** many participants reading a transcript every turn spends
  subscription limits fast.
- **Unverified:** vLLM's Responses endpoint, and how to discover a local
  model's context size.

## References

1. Du et al., "Improving Factuality and Reasoning in Language Models through Multiagent Debate," ICML 2024. https://arxiv.org/abs/2305.14325
2. Liang et al., "Encouraging Divergent Thinking in LLMs through Multi-Agent Debate," EMNLP 2024. https://aclanthology.org/2024.emnlp-main.992
3. Chan et al., "ChatEval," ICLR 2024. https://arxiv.org/abs/2308.07201
4. Wang et al. (Together AI), "Mixture-of-Agents Enhances LLM Capabilities," ICLR 2025. https://arxiv.org/abs/2406.04692
5. Li et al., "Rethinking Mixture-of-Agents," arXiv Feb 2025. https://arxiv.org/abs/2502.00674
6. Karpathy, llm-council (GitHub), Nov 2025 (an unsupported weekend hack). https://github.com/karpathy/llm-council
7. Chen et al. (Google DeepMind), "Universal Self-Consistency for LLM Generation," 2023/2024. https://arxiv.org/abs/2311.17311
8. Choi, Zhu, Li, "Debate or Vote," NeurIPS 2025. https://arxiv.org/abs/2508.17536
9. Smit et al., "Should we be going MAD?," ICML 2024. https://proceedings.mlr.press/v235/smit24a.html
10. Zhang et al., "Stop Overvaluing Multi-Agent Debate," arXiv Feb 2025. https://arxiv.org/abs/2502.08788
11. Wynn, Satija, Hadfield, "Talk Isn't Always Cheap," arXiv Sep 2025. https://arxiv.org/abs/2509.05396
12. Yao et al., "Peacemaker or Troublemaker: How Sycophancy Shapes Multi-Agent Debate," arXiv Sep 2025. https://arxiv.org/abs/2509.23055
13. Weng, Chen, Wang, "Do as We Do, Not as You Think: the Conformity of LLMs," ICLR 2025. https://arxiv.org/abs/2501.13381
14. Chen, Saha, Bansal, "ReConcile," ACL 2024. https://aclanthology.org/2024.acl-long.381
15. Hegazy, "Diversity of Thought Elicits Stronger Reasoning," arXiv Oct 2024 (not peer reviewed). https://arxiv.org/abs/2410.12853
16. Kim et al., "Correlated Errors in Large Language Models," ICML 2025. https://arxiv.org/abs/2506.07962
17. Maryanskyy et al., "When Agents Disagree: The Selection Bottleneck in Multi-Agent LLM Pipelines," arXiv Mar 2026. https://arxiv.org/abs/2603.20324
18. Zheng et al., "Personas in System Prompts Do Not Improve Performances of LLMs," Findings of EMNLP 2024. https://arxiv.org/abs/2311.10054
19. Khan et al., "Debating with More Persuasive LLMs Leads to More Truthful Answers," ICML 2024. https://arxiv.org/abs/2402.06782
20. Cemri et al., "Why Do Multi-Agent LLM Systems Fail?," NeurIPS 2025. https://arxiv.org/abs/2503.13657
21. Wolf, Yoon, Bogunovic, "Deception and Robustness in Mixture of LLMs," 2025. https://arxiv.org/abs/2503.05856
22. Wang et al., "Large Language Models are not Fair Evaluators," ACL 2024. https://arxiv.org/abs/2305.17926
23. Zheng et al., "Judging LLM-as-a-Judge with MT-Bench and Chatbot Arena," NeurIPS 2023. https://arxiv.org/abs/2306.05685
24. Panickssery, Bowman, Feng, "LLM Evaluators Recognize and Favor Their Own Generations," NeurIPS 2024. https://arxiv.org/abs/2404.13076
25. Verga et al. (Cohere), "Replacing Judges with Juries," arXiv Apr 2024. https://arxiv.org/abs/2404.18796
26. Anthropic, "Building effective agents," Dec 2024. https://www.anthropic.com/engineering/building-effective-agents
27. Anthropic, "How we built our multi-agent research system," Jun 2025. https://www.anthropic.com/engineering/multi-agent-research-system
28. Anthropic, Claude Code docs, "Subagents." https://code.claude.com/docs/en/sub-agents
29. CrewAI, "Agents" docs. https://docs.crewai.com/en/concepts/agents
30. Microsoft AutoGen, "Selector Group Chat." https://microsoft.github.io/autogen/stable/user-guide/agentchat-user-guide/selector-group-chat.html
31. AG2, Group Chat orchestration. https://docs.ag2.ai/latest/docs/user-guide/advanced-concepts/orchestration/group-chat/introduction/
32. Hong et al., "MetaGPT," 2023/2024. https://arxiv.org/abs/2308.00352
33. Li et al., "CAMEL," NeurIPS 2023. https://arxiv.org/abs/2303.17760
34. LangChain, langgraph-supervisor. https://pypi.org/project/langgraph-supervisor/
35. OpenAI, Agents SDK, "Agents." https://openai.github.io/openai-agents-python/agents/
36. A2A Protocol Specification v1.0.0 (AgentCard, AgentSkill). https://a2a-protocol.org/latest/specification/
37. Oracle, "Open Agent Specification," arXiv Oct 2025. https://arxiv.org/abs/2510.04173
38. Dilts, *Strategies of Genius Vol. 1* (1994); the Disney strategy. https://www.mindtools.com/abyeaeb/the-disney-creative-strategy
39. de Bono, *Six Thinking Hats* (1985). https://library.hee.nhs.uk/patient-information/patient-information-resources/six-thinking-hats ; IEEE 2023 study: https://ieeexplore.ieee.org/document/10361151
40. Rowe & Wright, "The Delphi technique as a forecasting tool," IJF 15(4), 1999. https://iaorifors.com/paper/29874
41. Klein, "Performing a Project Premortem," HBR Sep 2007 (URL not verified); Mitchell, Russo & Pennington, JBDM 2(1), 1989.
42. Schweiger, Sandberg & Ragan, AMJ 29(1), 1986; Schweiger, Sandberg & Rechner, AMJ 1989 (from secondary summaries).
43. Nemeth, Brown, Rogers, "Devil's advocate versus authentic dissent," EJSP 2001. https://doi.org/10.1002/ejsp.58
44. UK MoD DCDC, *Red Teaming Handbook*, 3rd ed., 2021. https://www.gov.uk/government/publications/a-guide-to-red-teaming
45. Dennett, *Intuition Pumps* (2013), Rapoport's rules. https://effectiviology.substack.com/p/rapoports-rules
46. Lorenz et al., "How social influence can undermine the wisdom of crowd effect," PNAS 2011. https://pmc.ncbi.nlm.nih.gov/articles/PMC3107299
47. Diehl & Stroebe, "Productivity loss in brainstorming groups," JPSP 1987. https://homepages.se.edu/cvonbergen/files/2013/01/Productivity-Loss-In-Brainstorming_Toward-the-Solution-of-a-Riddle.pdf

## References, part 2 (read 2026-10-20 unless dated)

- A1 Anthropic, "Run Claude Code programmatically." https://code.claude.com/docs/en/headless
- A2 Anthropic, "CLI reference." https://code.claude.com/docs/en/cli-reference
- A3 Anthropic, "Legal and compliance." https://code.claude.com/docs/en/legal-and-compliance
- A4 Anthropic Help Center, "Use the Claude Agent SDK with your Claude plan" (2026-06-15, 2026-10-07). https://support.claude.com/en/articles/15036540-use-the-claude-agent-sdk-with-your-claude-plan
- A5 Anthropic, "Effective context engineering for AI agents" (2025-09-29). https://www.anthropic.com/engineering/effective-context-engineering-for-ai-agents
- C1 OpenAI, "Codex App Server." https://learn.chatgpt.com/docs/app-server
- C2 OpenAI, "Config basics." https://learn.chatgpt.com/docs/config-file/config-basic
- C3 D. Vaughan, "The Codex App Server: A Complete Guide" (third-party, 2026-04-15/10-20). https://codex.danielvaughan.com/2026/04/15/codex-app-server-complete-guide/
- C4 openai/codex app-server README (third-party mirror). https://forge.lthn.ai/core/core-agent-ide/src/commit/361d43b969f0b0c4e08f3ab53ede22bbdee06c71/codex-rs/app-server/README.md
- C5 OpenAI, "Non-interactive mode." https://learn.chatgpt.com/docs/non-interactive-mode
- G1 Google, "Antigravity CLI headless." https://antigravity.google/docs/cli/headless
- G2 antigravity-cli 1.1.8 release notes. https://newreleases.io/project/github/google-antigravity/antigravity-cli/release/1.1.8
- G3 Google, "/agents." https://antigravity.google/docs/cli/commands/agents
- G4 Google, "CLI reference." https://antigravity.google/docs/cli/reference/
- G5 Google, "Antigravity Terms" §6. https://antigravity.google/terms
- G6 Google AI Developers Forum, headless-agy thread (2026-09-28, non-staff reply). https://discuss.ai.google.dev/t/headless-agy-runs-launched-by-a-script-from-another-ai-coding-tool-within-the-additional-terms/185494
- G7 google-antigravity/antigravity-cli issue #318 (2026-06-06 to 09-15). https://github.com/google-antigravity/antigravity-cli/issues/318
- M1 Gemini CLI, "Headless" (2026-03-10). https://www.geminicli.com/docs/cli/headless
- M2 Gemini CLI, "System prompt override" (2026-06-18). https://www.geminicli.com/docs/cli/system-prompt
- M3 Gemini CLI, "ACP mode" (2026-04-10). https://www.geminicli.com/docs/cli/acp-mode
- M4 Gemini CLI, "License, Terms of Service, and Privacy Notices." https://www.geminicli.com/docs/resources/tos-privacy
- M5 Google Developers Blog, "Plan mode is now available in Gemini CLI" (2026-03-11). https://developers.googleblog.com/en/plan-mode-now-available-in-gemini-cli/
- M6 Little Bear Apps, "Untether: Gemini runner" (third-party). https://littlebearapps.com/help/untether/gemini-runner
- X1 xAI, "Grok Build CLI: Headless & scripting." https://docs.x.ai/build/cli/headless-scripting
- X2 Zed, "Grok Build." https://zed.dev/acp/agent/grok-build
- X3 amplifying.ai, "Grok Build"; DataCamp, "Grok Build tutorial" (third-party). https://amplifying.ai/coding-agents/grok-build , https://www.datacamp.com/tutorial/grok-build-tutorial
- P1 ACP, "Overview." https://agentclientprotocol.com/protocol/overview
- P2 ACP, "Session setup." https://agentclientprotocol.com/protocol/session-setup
- P3 ACP, "Session config options." https://agentclientprotocol.com/protocol/session-config-options
- P4 Zed, "ACP agents"; "Google Antigravity." https://zed.dev/acp , https://zed.dev/acp/agent/antigravity-acp
- P5 zed-industries/codex-acp (archived 2026-07-22). https://github.com/zed-industries/codex-acp
- P6 agentclientprotocol/claude-agent-acp. https://github.com/agentclientprotocol/claude-agent-acp
- R1 A2A Protocol Specification v1.0.0. https://a2a-protocol.org/latest/specification/
- R2 A2A Protocol, "A2A and MCP." https://a2a-protocol.org/latest/topics/a2a-and-mcp/
- R3 Model Context Protocol, Specification 2026-07-28. https://modelcontextprotocol.io/specification/latest
- L1 ggml-org, llama.cpp server README. https://github.com/ggml-org/llama.cpp/blob/master/tools/server/README.md
- L2 Ollama, "OpenAI compatibility." https://docs.ollama.com/api/openai-compatibility
- L3 LM Studio, "OpenAI Compatibility Endpoints." https://lmstudio.ai/docs/developer/openai-compat
- L4 LM Studio, "Authentication." https://lmstudio.ai/docs/developer/core/authentication
- L5 LM Studio, "Structured Output." https://lmstudio.ai/docs/developer/openai-compat/structured-output
- L6 vLLM, "Tool Calling" (v0.23.0). https://docs.vllm.ai/en/v0.23.0/features/tool_calling/
- L7 vLLM, "Structured Outputs" (v0.17.0). https://docs.vllm.ai/en/v0.17.0/features/structured_outputs/
- L8 vLLM, "OpenAI-Compatible Server" (v0.28.0, a search snippet only). https://docs.vllm.ai/en/v0.28.0/serving/online_serving/openai_compatible_server/
- S1 Hines et al. (Microsoft), "Defending Against Indirect Prompt Injection Attacks With Spotlighting," 2024. https://arxiv.org/abs/2403.14720
- S2 OWASP GenAI, "Top 10 for Agentic Applications 2026," via Giskard (2025-12-18). https://www.giskard.ai/knowledge/owasp-top-10-for-agentic-application-2026
- T1 tursodatabase/turso README. https://github.com/tursodatabase/turso
