# Learning from recorded demonstrations: research digest

Compiled 2026-11-03 to 04. Web research only; no repo was read or changed.

## How to read this

Every claim carries one of three labels:

- **Measured**: a number from an evaluation, with its source number. Almost all of these were run by the authors on their own system. I found no independent replication of any of them unless stated.
- **Reported**: the authors or vendor state it, with no evaluation behind it that I could see. Vendor figures are marked "vendor".
- **Inference**: my own reading. It is not in the source.

Words used throughout:

- "Grounding" means finding the on-screen element that an instruction refers to.
- "Success rate" means tasks fully passed.
- "Points" means percentage points.

**Three limits on this evidence:**

1. My fetch tool passes pages through a small summarising model. Numbers were read back through that layer, and two or three visibly garbled (flagged below). Check any number against the paper before it becomes load-bearing.
2. Many of the most relevant papers are arXiv preprints from 2026, and some are only weeks old. Treat them as unreplicated.
3. Most benchmarks use Linux or web tasks. Windows evidence is thin (see Q1).

---

## Q1. Learning from human demonstrations for computer-use and GUI agents

### Systems that turn a demonstration into capability

**Work2Skill** [1, 2, 3] is the closest published match to this program. It records a person, then writes a SKILL.md an agent can load. arXiv v1, 9 Oct 2026.

- Captured: screen video, keyframes, mouse and keyboard events, accessibility data, on one timeline. **No audio or narration** (Reported, 1).
- Dataset: WorkDemo-350, 350 screened demonstrations, about 8.48 hours (Measured, 1).
- Result, full-pass success rate without a skill vs with its induced skill (Measured, 1):

| Executor | MyPCBench | OSWorld-Verified |
|---|---|---|
| GPT-5.5 | 23.44 to 48.44 | 70.98 to 78.67 |
| Opus-4.8 | 42.19 to 50.00 | 77.97 to 79.37 |
| Qwen3.5-35B-A3B | 9.38 to 20.31 | 32.17 to 36.01 |
| Qwen3.5-9B | 6.25 to 12.50 | 26.92 to 32.52 |

- It beat two other demonstration-based methods, ShowUI-Aloha and Demo2Tutorial, on every row above (Measured, 1).
- Skills survived changed requirements: on 64 task variants that reused the original skills, success rose 6.25 to 9.38 points over no skill (Measured, 1).
- Limits the authors state: skill finalisation checks structure only, not whether the skill works; candidate skills are never executed during induction; stronger induction does not close the gap between executors; token use is not consistently lower (Reported, 1).
- The released code says "full execution runners and evaluation integration are still pending" (Reported, 2).
- Windows is not mentioned. The paper describes macOS and Linux X11 capture only (Reported, 1, 2).

**ShowUI-Aloha** [4] records at 30 fps with FFmpeg (ddagrab on Windows, avfoundation on macOS) plus timestamped input logs, on a 170 MB Windows and macOS app.

- A parser cleans the log deterministically. GPT-4o then captions each action as a JSON record: Observation, Think, Action, Expectation (Reported, 4).
- 217 of 361 OSWorld-style tasks solved, 60.1%, with a demonstration per workflow family. On a 30-task subset, success fell from 63.3% to 36.7% with the demonstration trace removed (Measured, 4).
- Failure breakdown across 144 failed trials: element localisation 53.5%, text and field editing 16.0%, misaligned execution 14.6%, stalled or looping 8.3%, other 7.6% (Measured, 4).
- Authors warn their pass/fail metric is not directly comparable to scores from other papers (Reported, 4).

**Instruction Agent** [5] uses one demonstration, capturing only keyboard, mouse and screenshots before and after each action.

- 60% on 20 tasks that three top agents had all failed. Those agents' 0% follows from how the tasks were selected, so it is not a fair baseline (Measured, 5; my caveat).
- Removing the backtracking module gave 45%. Removing the verifier as well gave 40% (Measured, 5).
- No variance reported; success was judged by manual inspection (Reported, 5).

**GUI vs. CLI matched benchmark** [6] compared 440 desktop tasks across 18 apps with the same goals and verifiers.

- Best GUI agent 59.1% full pass. Best CLI agent with its original skills 48.2%. CLI after skills were repaired using the verifiers 69.3% (Measured, 6).
- Only 37.6% of checkpoints were reachable with the original skills; 93.8% of sampled CLI failures were skill coverage gaps (Measured, 6).
- Authors call the 69.3% a "diagnostic upper bound", not a deployable baseline, because repair used verifier information (Reported, 6).
- Patching made Documents and Spreadsheets worse, and Web stayed at 35.3 vs GUI 88.2 (Measured, 6).

**OpenAdapt** [7] is open source, MIT licence, tagline "Give your agent a program, not a mouse". It is the nearest match to the operator's principle. Numbers are in Q3.

**Claude "Record a skill"** [8] is a first-party product doing this already.

- Captures screen, mouse and keyboard, optional microphone narration, up to about 10 minutes (Reported, 8).
- Available in Claude for Mac only: "isn't available in chat, on Windows, or on Free and Enterprise plans" (Reported, 8).
- Video and audio "aren't retained"; screenshots persist (Reported, 8).
- No accuracy or success numbers published that I could find.

### Datasets from real human use

| Dataset | What was captured | Size | Result it supports |
|---|---|---|---|
| OpenCUA / AgentNet [9, 10] | Screen video, mouse and keyboard, accessibility tree | 22,625 tasks; 12K Windows, 5K macOS, 5K Ubuntu; mean 18.6 steps (Measured, 9) | More data helped: Ubuntu 3K to 10K improved results 72%; Win/Mac 3K to 14K improved 125%. OSWorld 9.8% to 18.5% when Win/Mac data was added. OpenCUA-72B reached 45.0% on OSWorld-Verified (Measured, 9) |
| CUA-Suite / VideoCUA [11, 12] | 30 fps video, cursor traces and events logged at claimed millisecond precision | About 10K tasks, 87 apps, 55 hours, 6M frames (Measured, 11) | Human evaluation of the 32B model: 57.6% stepwise. Grounding 52.4% vs action correctness 85.9% (Measured, 11). Authors call evaluation "preliminary" (Reported, 11) |
| GroundCUA [13] | Expert demonstrations, labelled screenshots | 56K screenshots, 3.56M+ human-verified labels, 87 apps (Measured, 13) | "Less than one-tenth the training data of prior work" for similar results (Reported, 13) |
| A11y-CUA [14, 15] | Video, system audio, input, Windows UI Automation snapshots, DOM | 40.4 hours, 158,325 events, 16 people (Measured, 14). CHI 2026 | Claude Sonnet 4.5: 78.33% default, 41.67% keyboard-only, 28.33% with 150% magnifier (Measured, 14) |
| GUIDE [16] | Screen video, think-aloud narration | 120 videos, 67.5 hours, from 54 novices (Measured, 16). A community list [70] says "120 novice users"; the paper says 54. CVPR 2026 | Models: behaviour state 44.6%, help prediction 55.0%. Context gain up to 50.2 points (Measured, 16) |

### Passive video mining

- **VideoAgentTrek** [17]: 39,000 YouTube tutorials produced 1.52M steps. OSWorld-Verified 9.3% to 15.8%; step accuracy 64.1% to 69.3% (Measured, 17). Absolute gains are modest.
- **Video2GUI / WildGUI** [18]: 12M trajectories from tutorial videos. Pretraining improved grounding and action benchmarks 5 to 20% (Measured, 18). ICML 2026.

### Programming by demonstration and task mining

- **WebRobot** [19]: PLDI 2022, 76 web benchmarks, "automated a majority of them" (Reported; no exact count on the page I opened).
- **Robotic process mining** [20]: real UI logs are usually "a single unsegmented sequence of events" with noise between routine instances. This is why a cleanup step is real work and not a formality (Reported, 20).
- **UiPath Task Mining** [21]: records clicks, keystrokes, hotkeys and screenshots of a task someone has already named. It explicitly does not discover unknown tasks (Reported, 21).
- **Inference:** the mature commercial lane still needs a human to name the task. That fits a design where the operator chooses what to record.

### Honest summary

- **Inference:** demonstrations help, most for weak models and for tasks agents otherwise fail. On tasks strong models already pass, gains shrink to a point or two (Work2Skill on OSWorld with Opus-4.8: 77.97 to 79.37).
- **Inference:** the headline "doubled success" results are on small or selected subsets. Expect moderate lift, not transformation.

---

## Q2. What a recording must contain

### Which streams mattered

**Best direct evidence: the Work2Skill evidence-removal test** [1], with GPT-5.5 on MyPCBench.

- Full evidence: Score 85.27, 31 tasks fully solved (Measured, 1).
- Removing indexed images, UI annotations, or key logs cost 5.02 to 10.63 Score points and 14.06 to 21.88 success points (Measured, 1).
- The biggest Score drop came from removing indexed images; the biggest success drop from removing key logs (Measured, 1).
- The "UI-Grounded Action Text Only" condition "outperforms Video-only and uses the fewest steps", and both trail full evidence (Reported, 1; per-row values appear only in a figure I could not read).
- Authors caution the key-log condition "does not isolate the log file's contribution", because keyboard information stays in the retained actions (Reported, 1).
- **Inference:** video alone was the weakest input measured. Structured actions tied to named UI elements carry more. Keep video as a source, not as the primary record.

**Video vs screenshots for training is still open.** CUA-Suite leaves it to future work (Reported, 11). DynamicUI argues single screenshots miss state in fast-changing interfaces and feeds it video (Reported, 22; no numbers on the page I opened).

**Accessibility trees are useful and unreliable.**

- OSWorld: GPT-4V 5.26% screenshot only, 12.17% with the tree. Gemini-ProV went the other way, 5.80% to 3.48%. Authors say the effect "varies by models" (Measured, 23).
- These are 2024 models, with single runs and no variance (Reported, 23).
- OpenCUA used accessibility trees for 189K grounding labels, not as agent input (Reported, 9).
- AX snapshots may not match action-time state, and coverage depends on the app (Reported, 1).
- **Inference:** capture the tree at action time, as evidence for a step's selector. Do not assume it exists.

**After a successful tool call, the next screenshot is often waste.** Dropping it and halving image history cut input cost to 53%, with 37.8% vs 33.0% accuracy after retraining (Measured, 24).

**Window and app context.** A11y-CUA logs window details plus the focused element and the element under the cursor on every event (Reported, 15). I found no study isolating the value of window metadata alone. Inference only: it is cheap and needed for segmenting.

### Narration or stated intent

I found **no study measuring whether spoken narration improves a skill or agent.** This is the largest gap against the operator's plan.

- Work2Skill, ShowUI-Aloha and Instruction Agent do not mention capturing it (Reported, 1, 4, 5).
- GUIDE has narration but fed models "visual input (excluding narration audio)". Narration was transcribed with WhisperX and used to create labels, which humans verified (Reported, 16).
- GUIDE's 50.2-point gain came from giving a model behaviour and intent labels, not narration. The paper does not say whether those labels were ground truth at test time, so it may not transfer (Reported, 16; caveat mine).
- Narration covered 78% of video duration in GUIDE (Measured, 16). Silence is normal.
- Anthropic recommends narration because it explains why you skip a step or choose between options (Reported, 8; no measurement).
- A11y-CUA left microphone recordings out of its release (Reported, 14).
- **Inference:** keep narration, treat it as a source for intent and step labels, and do not depend on it. Measure it yourself with narration-on vs narration-off skill tests. Doing so would produce a result nobody has published.

### How streams are time-aligned

| Source | Method |
|---|---|
| Work2Skill [1] | Events and frames on a video-relative timeline. Native input uses a monotonic timestamp anchored to the first frame of each segment. Pauses produce no frames. Remote events use wall clock projected through first-frame anchors. Probes match actions within a 2.5 s window, backfill within 3 s. Linux tree sampled every 2 s. Keyframes: 500 ms minimum gap, 64 per session. Authors call these "heuristic temporal associations", not measured accuracy |
| OpenCUA [9] | Pairs each action with the screenshot just before it. For clicks, backtracks to before the mouse moved, so no future information leaks into the input frame |
| CUA-Suite [11, 12] | Keyframes taken before state-changing actions; action timestamps extract frames from the video |
| A11y-CUA [14] | One timeline, with a per-task metadata file linking logs, video and tree files |
| ShowUI-Aloha [4] | Says actions get "synchronized" visuals without explaining how |

**Inference:**

- No published work I found measures alignment error, so the operator should measure it: inject a known click and read the frame offset.
- Backtracking the input frame before mouse movement is a detail that is easy to miss and affects training data quality.

---

## Q3. Demonstration to deterministic script, with AI only for the gaps

### Cost and reliability numbers

| System | Without compile | With compile | Status of the number |
|---|---|---|---|
| OpenAdapt [7] | Agent: 36.5 s median, 62 of 63 correct, about $0.08 per referral | Screen-step program: 22.5 s, 18 of 18, $0. API-step program: 3.3 s, 18 of 18, $0 | Reported (vendor). Test copies with synthetic data; the agent row is from a separate timing study; small n |
| Skyvern [25, 26] | $0.11, 278.95 s per run | $0.04, 119.92 s | Reported (vendor). Customer averages; no sample size or method. A third-party claim of "70-80%" reduction appeared in a search result; I could not find it in Skyvern's own material |
| Agentic Plan Caching [27] | Baseline | 50.31% lower cost, 27.28% lower latency | Measured. NeurIPS 2025. Reuses plan templates, adapted by small models; it is not LLM-free |
| MOBIMEM with AgentRR [28] | Baseline | Up to 50.3% more success, up to 9x lower latency | Measured. Mobile, best-case "up to" figures. No LLM-call figure |
| Stagehand cache [29] | LLM on every call | Hit: "no LLM calls, no token cost". Miss: LLM called | Reported. No measured figures |

### Compile layer order

- OpenAdapt compiles to "the most reliable layer available": page structure (DOM, UI Automation, Accessibility, AT-SPI), then direct API calls, then pixels only where nothing else exists (Reported, 7).
- Each app on each surface needs its own qualification (Reported, 7).
- AgentRR calls this low-level (precise actions, tied to layout) vs high-level experience (procedural, needs a model) and prefers the lowest level with the best replay rate (Reported, 30).
- **Inference:** this ordering is the operator's principle stated as architecture.

### Selectors vs coordinates

- **Vision-based finding of elements is the weak link.**
  - Grounding 52.4% vs action correctness 85.9% (Measured, 11).
  - Localisation is 53.5% of failures (Measured, 4).
  - WorkDemo-350's card warns its coordinates lack verified pixel alignment (Reported, 3).
- **Screen zoom alone hurts vision models.** 70% zoom was significant in 9 of 12 comparisons (Measured, 31).
- **Combining several attributes beats single selectors.** Similo failed on 72 of 598 elements, vs 146 for the state of the art, across 40 popular sites (Measured, 32). Web only, 2022.
- **Practice:** Playwright prefers role, text and test id, and warns DOM structure changes break tests (Reported, 33).
- **Windows caveats, from Microsoft's own docs:**
  - AutomationId "is not guaranteed to be stable across different releases or builds".
  - It is not mandatory, and unique only among siblings, so paths need parent context.
  - Name follows the UI language and "cannot be used as a unique identifier among siblings". RuntimeId identifiers "can be reused over time" and are for comparison only (Reported, 34, 35).
  - Microsoft warns of a "major drawback" for recording in volatile UIs (Reported, 34).
- **Inference for Windows:** store a selector bundle per step (AutomationId, ControlType, Name, parent path, process, window class, bounding box, plus a small image crop), score candidates at replay like Similo, and fall back in order. Pixels last. There is no measured Windows UI Automation equivalent of Similo that I could find.

### Parameterising

- AgentRR's low-level summaries identify variables and their constraints; over-generalising "can compromise safety", so its prototype stays conservative (Reported, 30).
- Work2Skill's policy asks the agent to generalise task-specific inputs and describe targets semantically. Its Jitter test (Q1) is the only direct evidence I found that this holds up (Measured, 1).
- **Inference:** typed values from a demonstration are the natural parameter candidates. A human should confirm them.

### Fallback and self-healing

- **Checking and recovering after each step are worth real points.** Instruction Agent: 60 full, 45 without the backtracker, 40 without both it and the verifier (Measured, 5, on 20 tasks). The verifier's effect on its own was not isolated.
- **"Saved" on screen is not proof.** Across 72 injected bad saves, the app's own "Saved" message accepted 54 and a REST read accepted 9, while a full database read accepted 0 (Reported vendor, 7). In a separate test an agent reported all 30 bad saves as done; `verify-run` flagged all 30 (Reported vendor, 7). Checks are not perfect either: a separate OpenEMR fault test caught 25 of 30 (Reported vendor, 7).
- **Skyvern's fallback ladder:** alternate selector for the same intent, then ask the model once for that step, then original prompt (Reported vendor, 25). It regenerates code after AI fallback (Reported vendor, 26).
- **Non-LLM healing exists but evidence is small:** re-extract just the broken selector from the accessibility tree, 31 of 31 passes on one site by one author (Measured, 36; too small to generalise).
- **Unrestricted LLM repair is risky.** 70% convergence per scenario family after a mean 3.4 iterations; 10% first-attempt success; 38% of reports produced no executable test; documented assertion weakening and test deletion to reach superficial convergence (Measured, 37; single case study, submitted for review).
- **Fallback is hard even for those building it.** Workflow Use lists its LLM fallback as "currently really bad", self-healing as an unchecked roadmap item, and says "we don't recommend using this in production" (Reported, 38).
- "100% reliable replay remains elusive" (Reported, 30).
- ReproBreak has 449 reproducible locator breaks from four projects, useful as a test set for web (Measured, 39). I saw no repair results in it.

**Inference:**

- Any AI patch to a script should be a proposed diff, tested against the recording's postconditions, with no ability to weaken them. A person approves it, as in OpenAdapt's Repair mode (Reported, 7).
- Allowing the fallback to change the check it must pass is the failure [37] documents.

---

## Q4. Synthetic or re-enacted data

### Where it clearly helped

- **Jedi** [40]: about 4M grounding examples from rendered components, icons, layouts, and also real screenshots, so it is not purely synthetic.
  - OSWorld with GPT-4o 5.0 to 27.0; with o3 23.0 to 51.0.
  - WindowsAgentArena 9.4 to 33.7 (Measured, 40).
  - It did not lead every benchmark: ScreenSpot-Pro 39.5 vs Qwen2.5-VL-32B 47.6 (Measured, 40).
- **Fara-7B / FaraGen** [41]: agent-generated trajectories at $0.59 to $1.08 each. Fara-7B scored 73.5 / 34.1 / 38.4 on three web benchmarks, beating similar-size models but well below SoM GPT-5 at 90.6 / 57.7 / 60.4 (Measured, 41). Its training mix also included other data, so it is not synthetic-only (Reported, 41).
- **Phi-Ground-Any** [42]: programmatic renderers with randomised resolution, aspect ratio, fonts, colours, styles. OSWorld 42.4, vs 38.2 and 37.7 for comparable models, and 44.1 for GPT-5.4 grounding itself (Measured, 42). Trained with OpenCUA data mixed in, so the synthetic share is not isolated (Reported, 42).
- **VideoCAD** [43, 44] is the clearest "scripted re-enactment rendered to video" case. It is very close to the operator's clips.
  - A rule-based bot rebuilds human-made CAD designs in the real app using Selenium and PyAutoGUI, with randomised delays. 41,005 videos at 60 FPS, with a timestamped action per frame (Reported, 44).
  - Its filter renders the result and compares it to the target, discarding bad reproductions (Reported, 44).
  - Model beat baseline on long sequences, 25.0% vs 5.9% (Measured, 44).
  - All on synthetic test data, no real-world validation. Trajectories "lack natural variability in timing, errors, and strategy" (Reported, 44).
- **See, Record, Do** [45]: LLM turns tutorials into task lists, a UI agent runs them in an instrumented OS, sensor data enables replay scripts. Over 5,000 workflows at 51 to 70% of manual cost (Measured, 45). Abstract only; PDF would not parse.

### Where it did not help

- **GUI-Perturbed** [31] is the most important warning. It applied domain randomisation directly to GUI grounding.
  - No augmentation variant improved results on average (Measured, 31).
  - Scaling from 6.5K to 25K samples degraded performance (Measured, 31).
  - Real and synthetic data both degraded the model, and synthetic showed larger drops on the perturbation types it trained on, probably fitting artefacts rather than learning invariance (Measured and Reported, 31).
  - Limit: only a low-rank fine-tune (rank-8 LoRA) on one model; authors do not claim training cannot fix this (Reported, 31).
- **LearnWeak** [46]: naive large-scale synthesis for a target domain gave "only marginal gains". Synthesis aimed at the model's own failures gained 11.6 and 11.1 points on OSWorld (Measured, 46).

### Which variation matters

- GUI-Perturbed significance counts: zoom significant in 9 of 12 comparisons, style in 1 of 12, text shrink in 0 of 12 (Measured, 31).
- Relational wording ("above", "left of") vs direct naming cost 27.1 to 56.0 points (Measured, 31).
- **Inference:** randomise DPI, scale, window size and instruction wording before themes and colours. This runs against the intuitive order and is narrow evidence (web pages, three models).

### Sim-to-real

- The robotics result is real: an object detector trained only on randomised simulation reached 1.5 cm real-world accuracy (Measured, 47). Whether that carries to GUIs is what GUI-Perturbed tested, and on its evidence it does not carry automatically.
- Phi-Ground authors say "improving alignment with real-world distributions remains an important direction". ScreenSpot-Pro 26.3 and UI-Vision 15.8 were weak, attributed to distribution differences. Fine-tuning raised ScreenSpot-Pro to 41.5 but lowered their own benchmark from 44.4 to 36.5 (Measured, 42).

### Pitfalls with evidence

1. **Artefact fitting** (31).
2. **Too-clean behaviour** with no errors or human hesitation (44).
3. **Labels overrate quality.** Fara-7B's human-verified accuracy was 62%, below automatic judge scores; verifiers agreed with humans 83.3% of the time, with 16.7% false positives and 18.4% false negatives (Measured, 41).
4. **Unachievable tasks.** 29% of early proposed tasks were unverifiable or unachievable; up to 25% of some trajectories were looping and removed (Measured, 41).
5. **Blind spots.** "No training data of behaviours beyond" critical points; Fara-7B stopped correctly on 19 of 23 tasks (Measured, 41).
6. **Mixing beats single sources.** Mixed data was more stable than scaling one type (Reported, 40).
7. **Noisy sources** can cause training failures without heavy cleaning (Reported, 42).

### A note on the operator's advantage

- **Inference:** most of these pipelines lack something the operator will have: the original human recording of the same task. That allows direct real-vs-reenacted comparison, per workflow, which I did not find in published work.
- **Inference:** scripted clips are likely strongest as evaluation and regression material, because labels are exact and a change in a model's or selector's score is attributable. Using them for training should be gated on measured lift against held-out real recordings.

---

## Q5. Packaging capability for agents

### The Agent Skills format

- A skill is a folder with `SKILL.md` (YAML frontmatter plus Markdown), and optional `scripts/`, `references/`, `assets/` (Reported, 48).
- `name` required, max 64 characters, lowercase letters, digits and hyphens, must match the folder. `description` required, max 1,024 characters, says what and when. Optional `license`, `compatibility`, `metadata`, experimental `allowed-tools` (Reported, 48).
- Three-stage loading: name and description at startup, about 100 tokens; body when activated, under 5,000 tokens and 500 lines recommended; files on demand (Reported, 48).
- Validator: `skills-ref validate` (Reported, 48).
- Originated by Anthropic, released as an open standard (Reported, 49).

### Where each agent looks

| Agent | Project path | Notes |
|---|---|---|
| Codex [50] | `.agents/skills` from CWD up to repo root | Optional `agents/openai.yaml` for UI and MCP dependencies. Plugins recommended for distribution. Follows symlinks |
| Gemini CLI [51] | `.gemini/skills` or `.agents/skills`; `.agents` wins | User approval on activation |
| GitHub Copilot [52] | `.github/skills`, `.claude/skills`, `.agents/skills` | CLI, cloud agent, IDE agent modes |
| Cursor [53] | `.agents/skills`, `.cursor/skills`; also `.claude/skills`, `.codex/skills` | Adds `paths`, `disable-model-invocation` |
| Claude Code [54] | `.claude/skills` | The part of the docs I read (about 85% of the page) lists no `.agents/skills` path. Many extra frontmatter fields, but uploads and the API give a hard error on anything beyond the spec fields |

**Inference: the format that loads into several AIs.**

- One canonical folder at `.agents/skills/<name>/`, using spec fields only, and a copy at `.claude/skills/<name>/` for Claude Code. That covers all five agents above.
- Keep vendor extras in side files, as Codex does.
- Copying is safer than symlinking on Windows, where symlink creation can need extra rights. That last point is general knowledge, not from these sources.

### Scripts

Supported script languages "depend on the agent implementation" (Reported, 48).

**Inference:** the deterministic runner should be a local CLI the skill calls, with the workflow stored in `assets/`. The skill then stays thin text, and replay costs no model.

### MCP as the live connector layer

- JSON-RPC, with tools, resources and prompts; revision 2026-07-28 (Reported, 55). Hosts must get consent before invoking tools (Reported, 55).
- **MCPB** bundles a local server plus `manifest.json` in a zip for one-click install; Node, Python, or a platform binary (Reported, 56). Inference: a Rust binary server would fit.
- **Skills over MCP:** official extension, SEP-2640 Final 13 Sep 2026, serves Agent Skills format via `skills/list`, `skills/get`, `resources/read`, with a SHA-256 per file and approval bound to the manifest (Reported, 57, 58).
  - "SDK and host support is still being implemented" (Reported, 57).
  - The client matrix shows "Partial" for ChatGPT, fast-agent and MCP Inspector (59). Other cells came through blank; icons may have been dropped by my fetch, so I cannot say who has full support.
  - Plugin and bundle packaging is out of scope for this group (Reported, 58).
- **Inference:** do not depend on this yet. Do copy its per-file digest idea for your own package integrity.

### Quality evidence: this should temper "generate a skill from every recording"

**SkillsBench** [60], 84 tasks in the main evaluation, 7 agent-model configurations:

- Curated skills +16.2 points, 24.3% to 40.6% (Measured, 60).
- **Self-generated skills -1.3 points**, 22.3% to 21.0% (Measured, 60).
- 16 of 84 tasks got worse with skills (Measured, 60).
- 1 to 3 skills: +17.8 and +18.6 points; 4 or more: +5.9 (Measured, 60).
- "Comprehensive" skills: -2.9 points (Measured, 60).
- Limits: terminal tasks only, "may not transfer to GUI agents"; benchmark skills are better than the ecosystem average, so results may be optimistic (Reported, 60).
- Note Work2Skill did find positive results for agent-written skills from demonstrations, on GUI tasks (Measured, 1). **Inference:** the demonstration evidence may be what closes that gap; the two studies differ in task type and were not compared directly.

**Whether agents use the connector you ship is not a given.** A reasoning model called a tool on 23.9% of tool-reachable tasks; the same tools gave it +4.0 points and a non-reasoning model -5.9 points (Measured, 24).

**Security:** 26.1% of 31,132 marketplace skills had at least one vulnerability, and skills bundling scripts had 2.12 times the odds (Measured, 61). The detector's precision was 86.7% and recall 82.5%, and other studies I saw only in search snippets disagree on prevalence. **Inference:** skills that carry scripts deserve review, including those a recording generates.

---

## Q6. Dataset format and privacy

### Formats in use

| Format | Shape | Fit for this project (Inference) |
|---|---|---|
| **WorkDemo-350** [3] | One folder per task: `task.json`, `episode.json`, `summary.md`, `video.mp4`, `trace/actions.jsonl`, `trace/keylog`, `evidence/` plus index. Actions carry `kind`, `media_time_ms`, `sequence_index`, `caption`, coordinates. Top-level `task_index.jsonl` | **Closest match.** Readable, per-task, event-driven, video-relative time |
| **VideoCUA** [12] | Per-task `action_log.json` with `action_type`, `timestamp`, `action_params`, plus `video.mp4` and metadata. Zip per app | Simple and useful |
| **AgentNet** [10] | JSONL per task, `traj` steps with `observation`, `thought`, `action`, `code` (PyAutoGUI), `reflection`, plus quality flags. Images in split zips. 200 GB | Good for model-ready export. HF viewer fails on mismatched columns: pin one schema from day one |
| **A11y-CUA** [15] | Session folders: `metadata_*.json`, per-app event JSON, UIA tree JSON (depth limit 4; fields `automation_id`, `control_type`, `name`, `bounding_rectangle`), `screen.mp4`, `system_audio.wav` | Best template for Windows element capture |
| **LeRobot v3** [62] | Parquet for fixed-rate frames, MP4 shards, metadata with episode offsets; `meta/tasks.jsonl`, `meta/info.json`. Many episodes in one file; decoupled from API | Built for fixed-rate robot signals. Its layering idea is good, but sparse GUI events sit awkwardly in per-frame rows |
| **WebDataset** [63] | Numbered tar shards; files grouped by shared basename; sequential streaming; no metadata needed | Export format for large-scale training only; poor for editing |

**Inference:**

- Store as readable per-recording folders with JSONL events on a video-relative clock (the WorkDemo-350 shape) plus Windows element fields (the A11y-CUA shape).
- Export to JSONL with images, or WebDataset, or LeRobot only when a training run needs it.
- Record provenance on every episode (`human`, `reenacted`, `synthetic`), so synthetic data can never silently mix into evaluation.

### How projects handle PII

| Project | What they did |
|---|---|
| AgentNet [9] | Consent form prohibiting sensitive data; no upload without manual annotator action; review before submit; human review plus GPT-4o rating (None, Low, Medium, High), rejecting tasks with private data. Weakness: GPT flagged by keyword, e.g. a blank password field. No measured privacy numbers (Reported). Consent caused selection bias (Reported) |
| A11y-CUA [14, 15] | Research laptops, not personal machines; microphone excluded from release; IRB approved. Dataset card has no privacy statement |
| WorkDemo-350 [3] | "Not claimed to be fully anonymized." Raw key logs retained that "may expose individual keystrokes". Text scan found email addresses and password-related text. No frame-by-frame review or consent records documented |
| Work2Skill recorder [1] | Full keystroke capture off by default; otherwise aggregate typing events |
| OpenAdapt [7] | Everything stays local; sanitised copies leave only after a person approves; screenshots go to a model only if model grounding is on (Reported vendor) |
| Claude Record a skill [8] | Warns against showing passwords or sensitive content: "Everything on your screen is captured". No technical redaction claimed in the article I read |
| UiPath [21, 71] | PII "masked automatically"; app allow-list; pause, capped at 1 hour per pause; forbids processing credit card data (Reported vendor) |

### Known failure modes

- **Microsoft Recall**, a sensitive-data filter from a major vendor, in a test published Aug 2025.
  - It filtered a labelled form, but captured a card form once labels like "checkout page" were removed, a usernames-and-passwords file without labels, and an SSN after an unusual prefix (Measured, one reporter's test, 64).
  - **Inference:** detectors key on labels and context, not the secret itself.
- **OCR-based redaction misses what it cannot read.**
  - `openadapt-privacy` says Tesseract missed light text on a dark title bar in its demo, and text OCR cannot read is not redacted (Reported, 65).
  - It is English only, and street addresses are caught only after a label.
  - It tested on 27 synthetic identifiers and says "measure recall on your own data" (Reported, 65).
  - Presidio's image redactor is "still in beta and not production ready" (Reported, 66).
- **Visual detection works better but is far from complete.** WebRedact reached 0.753 mAP@50 vs 0.357 for text extraction, at 20 ms CPU latency, on synthetic e-commerce images (Measured, 67). **Inference:** that figure leaves many misses. It is a useful extra layer on-device, not a guarantee. It is web only and synthetic.
- **Pixelating is not redaction.** Depix is a proof of concept for recovering plaintext from pixelated screenshots (Reported, 68). **Inference:** use a solid fill.
- **Keystroke logs reveal secrets even when the screen is clean** (Inference, supported by 3). `openadapt-privacy` joins per-character keystrokes before scrubbing, because split-up identifiers evade detection (Reported, 65).
- **AI-based redaction has a sequencing trap.** If a cloud model does the rating, the data has already left the machine (Inference; AgentNet's GPT-4o layer runs on task text and step records, 9).
- **Fail loud.** `openadapt-privacy` raises an error rather than scrubbing with a weaker model, and records provider, version and a settings hash in scrubbed output (Reported, 65). Worth copying.

### On-device controls for Windows

- UI Automation's `IsPassword` says when a focused element holds protected content; clients should disable input echo (Reported, 35). **Inference:** use it to suppress key logging for that element. Coverage depends on each app's accessibility support, so this cannot be the only control.
- `SetWindowDisplayAffinity` with `WDA_EXCLUDEFROMCAPTURE` hides your own windows from capture. It only applies to windows in your own process, and Microsoft says it is not a security feature (Reported, 69). Useful to keep recorder controls out of the video; no help with other apps' secrets.

### Inference: a privacy posture supported by this evidence

1. Prevent rather than clean up: app allow-list, a pause key, password-field suppression.
2. All processing local by default. Export is an explicit, reviewed step.
3. Layer redaction, always with a solid fill, and treat it as reducing risk rather than removing it.
4. Never promise a recording is clean. Say what was checked.
5. Test recall against the operator's own recordings with seeded fake secrets. No published recall figure applies to real desktop video.

---

## What this means for the design (Inference, all of it)

1. **Record structured evidence, not just video.** Events tied to UI elements, before and after keyframes, accessibility snapshot at action time, on one clock anchored to the video.
2. **Compile in layers:** element selector bundle, then API where one exists, then pixels last. A model only for proposing fixes.
3. **Postconditions are the backbone.** Every step gets a check, proven by reading state back rather than trusting the app's message. AI repairs may not edit their own checks.
4. **Generated skills get tested before they ship.** Replay against the recording's checks. Keep them short. SkillsBench's -1.3 is the warning. Work2Skill shows induction from demonstrations can work, yet it never runs a skill before finalising, so that step is still open.
5. **Treat re-enacted clips as test fixtures first, training data second.** Mark provenance. Randomise zoom, DPI and wording first. Admit synthetic to training only when real held-out recordings show lift.
6. **Keep real recordings as the anchor.** Real-data scale helped consistently in [9]. Pure synthetic has no strong success story here.
7. **Package as thin skill plus deterministic runner.** `.agents/skills` plus `.claude/skills`, spec fields only. MCP server for connectors; skip Skills over MCP for now.
8. **Windows-first is partly unexplored ground.** Anthropic's recorder is Mac only (8), and Work2Skill names macOS and Linux (1). ShowUI-Aloha's FFmpeg `ddagrab` approach (4) shows a subprocess recorder works on Windows, and A11y-CUA (14, 15) shows UI Automation capture works there.

---

## What I could not verify

- **Whether spoken narration improves agent or skill results.** I found no study measuring it. The evidence is indirect and it is an open question.
- **Independent replication** of any headline figure. All numbers are self-reported by their authors. Vendor numbers (OpenAdapt, Skyvern) use undisclosed or synthetic setups.
- **Measured reliability of Windows UI Automation selectors** across app updates. Similo-type numbers are web only. I found no equivalent.
- **Measured alignment error between streams.** Everyone describes methods; nobody I found reports accuracy. Work2Skill says outright that its windows are not measured bounds.
- **Per-row values of Work2Skill's evidence-removal test** appear only in a figure. I have the ranges and the ordering, not each row.
- **Full client list on agentskills.io.** The showcase page returned empty through my fetch. A third-party search result said 40+ clients; I did not verify it.
- **Which MCP clients fully support Skills over MCP.** See the blank-cell caveat in Q5.
- **Skyvern's "70-80%" reduction** appeared in a search summary only. Its own post says $0.11 to $0.04.
- **"Workflow Use" cost or speed figures.** The repo gives none.
- **Demo2Tutorial** (CVPR 2026) I know only through Work2Skill's citation and did not open it.
- **VideoAgentTrek at ICLR 2026** appeared in search results, while the arXiv page I opened lists no venue.
- **SkillsBench NeurIPS version** (16.4 points, 18 regressions) appeared only in a search summary. This digest uses arXiv v1, whose own figure caption says +12.66 against +16.2 in its text; I used the text.
- **PDF-only pages:** I could not read the full See, Record, Do PDF (abstract only) or the full Practical Limits paper (abstract only).
- **Ringer, OOPSLA 2016:** only a search snippet, so not cited.

Discrepancies and bad data I caught:

- A community list (70) gave arXiv 2506.01968 for AgentSynth; that ID is an unrelated spiking-network paper.
- A search result gave 2604.09574 for the narrated dataset; that is a mobile humanisation paper, and GUIDE is 2603.25864.
- The WorkDemo-350 card summary said "8,388 videos and 11,951 evidence images"; the project page says 8,388 evidence images and 350 demonstrations. Treat the card reading as garbled by my fetch layer.
- OpenCUA is internally inconsistent on app counts and minimum steps; A11y-CUA gives 85% and 84.6% for the same rate; VideoCAD's text and table disagree on one result.

---

## Sources (all opened unless noted)

1. Work2Skill paper: https://arxiv.org/html/2610.12984
2. Work2Skill project page: https://china-qijizhifeng.github.io/Work2Skill/
3. WorkDemo-350 dataset card: https://huggingface.co/datasets/yuqi777/WorkDemo-350
4. ShowUI-Aloha: https://arxiv.org/html/2601.07181
5. Instruction Agent: https://arxiv.org/html/2509.07098 and https://arxiv.org/abs/2509.07098
6. GUI vs. CLI: https://arxiv.org/html/2606.24551 and https://arxiv.org/abs/2606.24551
7. OpenAdapt repo: https://github.com/OpenAdaptAI/OpenAdapt
8. Claude help, How to create custom skills: https://support.claude.com/en/articles/12512198-how-to-create-custom-skills
9. OpenCUA: https://arxiv.org/html/2508.09123v3 and https://arxiv.org/abs/2508.09123
10. AgentNet dataset card: https://huggingface.co/datasets/xlangai/AgentNet
11. CUA-Suite: https://arxiv.org/html/2603.24440v1
12. VideoCUA dataset card: https://huggingface.co/datasets/ServiceNow/VideoCUA
13. GroundCUA: https://arxiv.org/abs/2511.07332
14. A11y-CUA: https://arxiv.org/html/2602.09310
15. A11y-CUA dataset card: https://huggingface.co/datasets/berkeley-hci/A11y-CUA
16. GUIDE: https://arxiv.org/html/2603.25864 and https://arxiv.org/abs/2603.25864
17. VideoAgentTrek: https://arxiv.org/abs/2510.19488
18. Video2GUI: https://arxiv.org/abs/2605.14747
19. WebRobot: https://arxiv.org/abs/2203.09993
20. Leno et al., unsegmented UI logs: https://arxiv.org/abs/2008.05782v1
21. UiPath Task Mining intro: https://docs.uipath.com/task-mining/automation-cloud/latest/user-guide/introduction-as
22. DynamicGUIBench / DynamicUI: https://arxiv.org/abs/2604.25380
23. OSWorld: https://arxiv.org/html/2404.07972 and https://arxiv.org/abs/2404.07972
24. Screenshots or Tools?: https://arxiv.org/abs/2608.03327
25. Skyvern blog: https://skyvern.com/blog/asking-ai-to-build-scrapers-should-be-easy-right
26. Skyvern code caching docs: https://skyvern.com/docs/developers/features/code-caching
27. Agentic Plan Caching: https://arxiv.org/abs/2506.14852
28. MOBIMEM: https://arxiv.org/abs/2512.15784
29. Stagehand caching docs: https://docs.stagehand.dev/v3/best-practices/caching
30. AgentRR: https://arxiv.org/html/2505.17716v1
31. GUI-Perturbed: https://arxiv.org/html/2604.14262v2
32. Similo: https://arxiv.org/abs/2208.00677
33. Playwright best practices: https://playwright.dev/docs/best-practices
34. Microsoft, Use the AutomationID property: https://learn.microsoft.com/en-us/dotnet/framework/ui-automation/use-the-automationid-property
35. Microsoft, UI Automation property identifiers: https://learn.microsoft.com/en-us/windows/win32/winauto/uiauto-automation-element-propids
36. Zero-cost self-healing: https://arxiv.org/abs/2603.20358
37. Practical Limits of Autonomous Test Repair: https://arxiv.org/abs/2605.01471
38. Workflow Use repo: https://github.com/browser-use/workflow-use
39. ReproBreak: https://arxiv.org/abs/2605.12158
40. Jedi: https://arxiv.org/html/2505.13227 and https://arxiv.org/abs/2505.13227
41. Fara-7B: https://arxiv.org/html/2511.19663 and https://arxiv.org/abs/2511.19663
42. Covering Human Action Space (Phi-Ground-Any): https://arxiv.org/html/2605.12501
43. VideoCAD abstract: https://arxiv.org/abs/2505.24838
44. VideoCAD full text: https://arxiv.org/html/2505.24838
45. See, Record, Do: https://wacv.thecvf.com/virtual/2026/poster/246
46. LearnWeak: https://arxiv.org/abs/2605.28775
47. Domain randomization (Tobin et al.): https://arxiv.org/abs/1703.06907
48. Agent Skills specification: https://agentskills.io/specification
49. Agent Skills overview: https://agentskills.io/home
50. Codex skills: https://learn.chatgpt.com/docs/build-skills
51. Gemini CLI skills: https://geminicli.com/docs/cli/skills/
52. GitHub Copilot skills: https://docs.github.com/en/copilot/concepts/agents/about-agent-skills
53. Cursor skills: https://cursor.com/docs/skills
54. Claude Code skills: https://code.claude.com/docs/en/skills
55. MCP specification: https://modelcontextprotocol.io/specification/latest
56. MCP Bundles: https://github.com/modelcontextprotocol/mcpb
57. MCP Skills extension: https://modelcontextprotocol.io/extensions/skills/overview.md
58. Skills Over MCP charter: https://modelcontextprotocol.io/community/working-groups/skills-over-mcp.md
59. MCP client matrix: https://modelcontextprotocol.io/extensions/client-matrix.md
60. SkillsBench: https://arxiv.org/html/2602.12670v1
61. Agent Skills in the Wild: https://arxiv.org/abs/2601.10338
62. LeRobotDataset v3: https://huggingface.co/docs/lerobot/lerobot-dataset-v3
63. WebDataset: https://github.com/webdataset/webdataset
64. The Register, Recall test: https://www.theregister.com/2025/08/01/microsoft_recall_captures_credit_card_info/
65. openadapt-privacy: https://github.com/OpenAdaptAI/openadapt-privacy
66. Presidio image redactor docs: https://raw.githubusercontent.com/microsoft/presidio/main/docs/image-redactor/index.md
67. WebPII: https://arxiv.org/abs/2603.17357
68. Depix: https://github.com/spipm/Depixelization_poc
69. Microsoft, SetWindowDisplayAffinity: https://learn.microsoft.com/en-us/windows/win32/api/winuser/nf-winuser-setwindowdisplayaffinity
70. Community dataset list (used as a lead only; contains at least one wrong ID): https://github.com/Khang-9966/Computer-Browser-Phone-Use-Agent-Datasets
71. UiPath Task Mining privacy: https://docs.uipath.com/task-mining/docs/protection-of-personal-data-and-privacy-of-the-users

Opened but carrying little: AppAgentX, https://arxiv.org/abs/2503.02268 (abstract has no numbers); UILoop, https://arxiv.org/abs/2604.06995; Gym-Anything, https://arxiv.org/abs/2604.06126 (200 apps, 10K+ tasks, an audit agent checks environments); Act2Cap, https://arxiv.org/abs/2406.13719 (4,189 GUI action captioning samples, generation method not stated on the page); SkillWeaver, https://arxiv.org/abs/2504.07079 (+31.8% and +39.8% relative success; skills as shareable APIs).

Seen in search results only, not opened, not relied on: skill security studies arXiv 2602.06547, 2604.03070, 2603.16572; SkillsBench NeurIPS poster; Tom's Hardware Recall test; Ringer OOPSLA 2016.
