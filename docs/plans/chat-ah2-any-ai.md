# AH2: add any AI through one contract

**R305, clarified by R306.** This pass comes before ST4 (ruling 2026-11-17).
**Status: approved 2026-11-19** (rulings in requirements.md under R306; R307 corrects the local-AI design).

## 1. Context

R305 asks for "clear UI that allows me to add custom / i.e clyffy". R306
clarifies it: *local or custom should not matter. There is a minimum
requirement for an AI to connect.* The schema defines that minimum, and the
rest of what we know about a provider is documented in the same place,
canonically.

Today (read 2026-11-18):

- **The built-in catalog is already data** (AH1a, `dpl.ai-catalog/1`):
  - the vendored ACP registry (`crates/ai/core/catalog/acp-registry.json`);
  - DevPULSE's overlay (`catalog/devpulse.json`).

  It describes detection (presence, installs, tells), look and the target
  capabilities. It does **not** say how to talk to an AI. That is an `adapter`
  name pointing at Rust code (`codex`, `grok`).
- **Adding your own AI is hardcoded.**
  - `dpl_ai_profiles::SUPPLEMENTS` has three fixed rows: Clyffy,
    "Add local provider" and "Add custom provider".
  - `ProviderConfigurationKind { Local, Custom }` gives each kind its own
    rules (loopback only, HTTPS only), its own copy in
    `crates/chat/window/src/lib.rs:6260-6330` and its own DB value.
  - A saved profile holds a name, an endpoint and a model, and nothing else.
    It cannot chat: "Its compatible transport adapter is not qualified yet."
- **No AI can be reached over HTTP.** ADR-011 makes `ureq` the single outbound
  HTTPS client, used only by Imagine against a compiled-in endpoint. Imagine
  reads its key from `XAI_API_KEY` and writes it nowhere.
- **Clyffy** is a supplement row ("declared but not qualified"). Its backing
  (an API key, a subscription, a local or Hugging Face model) is CL1 in §4.8.4.

## 2. Goal and non-goals

**Goal.**
- One versioned schema, `dpl.ai-provider/1`, describes **every** AI DevPULSE
  can reach: built-in, from the ACP registry, or added by the operator.
- It states the minimum an AI needs to connect, and it documents everything
  else we know.
- One clear "Add an AI" flow writes an entry in that schema.
- It proves the minimum with **Test connection** before saving.
- Local versus remote is a fact derived from the address, never a kind the
  operator picks.

**Not in AH2:**
- Clyffy's router, its modes, and TaskAssist (CL1–CL3). The schema reserves
  `backing` so Clyffy becomes an instance of the same contract in CL1.
- Roles and rooms (RM, RM1b).
- Storage moves (ST4). Entries stay in chat's store, which ST4 relocates.
- Installing anything.

## 3. Ground truth

| What | Where | Fact |
|---|---|---|
| Catalog loader | `crates/ai/core/src/catalog.rs` | It parses the registry and the overlay at build time (`include_str!`), checks `schema == "dpl.ai-catalog/1"`, and leaks to `'static`. It refuses duplicates, unknown capabilities, upper-case needles, and a desktop entry with an adapter. |
| Capability vocabulary | `crates/ai/core/src/capability.rs` | 22 stable slugs. A target capability is "intended"; it is qualified only by what the runtime advertises at connect (`qualified_capabilities`). |
| Runtime contract | `crates/ai/core/src/adapter.rs` | `ProviderRuntime`: commands in, events out, never blocking. It has a typed `RuntimeFailureKind`. Any new transport implements this. |
| Profiles | `crates/ai/profiles/src/lib.rs` | Turso `chat/provider-profiles.db`; table `chat_provider_profiles(profile_key, kind, display_name, endpoint, model)`. Kinds are `local` and `custom`. |
| Profile UI | `crates/chat/window/src/lib.rs:6078-6130, 6260-6330` | More AIs → "ADD A PROVIDER" lists the two Configure supplements; the dialog has three text fields. |
| Host wiring | `crates/app/src/chat.rs:817-851, 1899` | The store opens only when the provider runtime is enabled; `HostIntent::SaveProviderProfile`. |
| Prior art: ACP registry | `FORMAT.md` (AH1 plan §3) | Per agent: `distribution` by `binary` (per target: archive, cmd, args, env, sha256), `npx` or `uvx`. This is the **stdio agent** minimum. |
| Prior art: models.dev | `github.com/sst/models.dev`, fetched 2026-11-18 | Provider: `name`, `env[]` (auth variables), `doc`, `api` (base URL, "required with openai-compatible"). Model: `modalities{input,output}`, `capabilities{tools,reasoning,…}`, `limit{context,input,output}`, `cost{input,output,cache}`, `open_weights`, `release_date`, `knowledge`, `status`. This is the **documented model** shape we align field names with. |
| Prior art: OpenAI-compatible HTTP | Hugging Face Inference Providers docs, fetched 2026-11-18 | `https://router.huggingface.co/v1`, `Authorization: Bearer <HF token>`, `GET /v1/models` lists models with context length and pricing. vLLM, OpenRouter and most vendors speak the same request. |
| Local servers | R307, 2026-11-19 | Any server the operator runs must connect by pointing at it, whatever it is (vLLM, LM Studio or anything else that speaks a supported protocol). No product is named in code or in the dialog. vLLM, LM Studio, llama.cpp and Ollama all serve `GET /v1/models` and `POST /v1/chat/completions`. A server on another machine on the operator's network (for example the GB10) is usually plain `http://<lan-ip>:<port>`. |
| Key handling law | ADR-017 §6, ADR-011, `dpl-ai-imagine::resolve_api_key` | A registry never holds a credential. Imagine's key comes from the environment and is never written. |

## 4. Design

### 4.1 The minimum to connect

An AI is connectable when the schema can answer four questions. Nothing else
is required.

| # | Question | Field | Values |
|---|---|---|---|
| 1 | Which wire protocol? | `connection.protocol` | A closed set, one value per transport DevPULSE implements:<br>• `acp`: Agent Client Protocol, JSON-RPC over stdio (Kimi, Gemini, Grok and the rest of the registry);<br>• `openai-chat`: `POST {base_url}/chat/completions` with SSE, and `GET {base_url}/models` (vLLM, Hugging Face, OpenRouter, llama.cpp and most vendors);<br>• `codex-app-server`;<br>• `claude-stream-json`;<br>• `desktop-app`: open or focus only, no chat (R12). |
| 2 | Where is it? | `connection.endpoint` | `{ "command": "<program>", "args": [...] }` for stdio protocols, or `{ "base_url": "https://…/v1" }` for HTTP. |
| 3 | How does it know it's you? | `auth.method` | • `none`;<br>• `vendor-login`: the vendor's own CLI holds the login, and the existing presence spec checks it;<br>• `api-key`: `auth.secret` **names** where the key lives (4.4) and never holds it;<br>• `device-code`: the vendor's own flow. |
| 4 | Which model? | `model.default` | Required for `openai-chat`; optional for agents that choose their own. |

Validation lives in **one** place and replaces the two kinds:
- `http://` is accepted for **this computer** (loopback) and for **your
  network** (private ranges such as 10/8, 172.16/12, 192.168/16, and link-local
  and `.local` names), so a vLLM server on the GB10 connects. The dialog marks
  such an address as "not encrypted, stays on your network".
- An address on the public internet needs `https://`.
- No credentials may sit in a URL.
- `command` resolves to a real executable, never a `.cmd` shim, with no shell
  (§4.8.2 rule 2).

Where the AI runs is **derived** from the endpoint and shown, never authored:
**this computer**, **your network** or **the internet**.

**Nothing names a product.** Nothing in the code or the dialog is specific to
vLLM, LM Studio, Ollama, Hugging Face or any other server. What a server is
gets read from its own answer and recorded as a documented fact: the `owned_by`
value of its `/models` reply and its `Server` header, marked verified.

**The protocol enum grows only when a transport lands in code.** That is the
one place a contract is code, by design: data picks among transports, and code
implements them (R302: "never hardcoding except when best practice building
contracts").

### 4.2 Documented, optional, captured once

Every block below is optional. Each one is validated when present, and each
one marks every fact with `verified: true|false`.

- **`identity`**: `id` (kebab-case, required), `label` (required),
  `short_label`, `vendor`, `summary`, `homepage`, `docs`, `license`, `since`.
- **`look`**: `glyph`, `accent`.
- **`models[]`**, with field names from models.dev so a later import maps 1:1:
  - `id`, `name`, `family`, `open_weights`, `release_date`, `knowledge`,
    `status`;
  - `modalities {input[], output[]}`;
  - `capabilities {tools, reasoning, structured_output, attachments}`;
  - `limit {context, input, output}`;
  - `cost {input, output, cache_read, cache_write}`, in USD per million tokens.
- **`capabilities[]`**: the existing 22 slugs, as *targets*. Qualification is
  unchanged: earned at connect, never assumed.
- **`billing`**: `free`, `subscription` or `metered`, plus a plan note.
- **`detection`**: today's `presence`, `installs` and `tells`, moved under one
  key.
- **`acp`**: the registry id it claims (as today).
- **`backing`**: reserved for an assistant such as Clyffy. It names another
  entry's `id` and model. Validated now, used from CL1.
- **`provenance`** (required): `source` (`builtin`, `acp-registry` or
  `operator`), `added` (date), `evidence` (link or doc anchor), `schema`.

### 4.3 One schema, three sources, one loader

- `schemas/ai-provider.v1.schema.json` (JSON Schema 2020-12, like
  `role.v1`) is **the** contract. It holds a description on every field.
- The overlay moves once to `dpl.ai-catalog/2`. Its `entries` are
  `dpl.ai-provider/1` documents. ACP registry agents are mapped into the same
  shape at load (`protocol: acp`, `source: acp-registry`).
- Operator entries are `dpl.ai-provider/1` documents in chat's store.
- `catalog.rs` produces one list from all three. The `adapter` field goes:
  the protocol says which transport serves an entry.
- Tests prove two things:
  - every built-in entry validates against the JSON Schema;
  - the serde types and the schema agree, using the pattern `dpl-workflows`
    already uses (`jsonschema` dev-dep).
- **Migration, once (pre-release, no shims):**
  - `chat_provider_profiles` rows become provider documents:
    - `local` becomes `openai-chat` with its loopback `base_url`;
    - `custom` becomes `openai-chat` with its HTTPS `base_url`.
  - The old table, `ProviderConfigurationKind`, the two Configure
    supplements and their dialog are deleted in the same PR.

### 4.4 Where an API key lives (operator decision, see the questions)

The schema stores only a **reference**, never a key:
`auth.secret = { "store": "os" | "env", "name": "…" }`.
- **`os`**: the operating system's credential store (Windows Credential
  Manager, macOS Keychain, Secret Service). The operator pastes the key once
  into a masked field. DevPULSE writes it straight to the credential store and
  never logs it, shows it or exports it. This needs one new dependency
  (`keyring`).
- **`env`**: the name of an environment variable, as Imagine does today.

### 4.5 The "Add an AI" flow (ColdLight, touch-first, tooltips on every control)

More AIs replaces "ADD A PROVIDER" and its two rows with one row, **Add an
AI**. It opens a dialog with three short steps. The operator never picks a
protocol or a product:

1. **Where is it?** One field takes:
   - a web address on this computer, the network or the internet, any host and
     any port (`192.168.1.40:8000`, `localhost:1234`,
     `https://router.huggingface.co/v1`);
   - or a program to run (an ACP agent);
   - or a desktop app's file.

   A missing scheme or `/v1` is filled in. Beside the field, **Find on this
   computer** lists the servers already listening on this computer that
   answer `/v1/models`. Whatever is running (vLLM, LM Studio, llama.cpp or
   anything else) shows up by what it reports, with no list of products and no
   list of ports in the code. It reads the OS table of listening loopback
   sockets, then asks each one; see 4.7.
2. **Sign-in.** The default is no key. **Needs a key** opens a masked field
   whose key goes straight to the OS credential store (4.4). The dialog shows
   **This computer**, **Your network** or **The internet** as soon as it can
   tell.
3. **Test connection.** It **detects** the protocol and checks each of the
   four minimum requirements (4.1), showing a tick or the plain reason next to
   each:
   - **A web address:** it asks `GET {address}/models`, then `{address}/v1/models`.
     An OpenAI-shaped answer makes it `openai-chat`, and the base URL that
     answered is the one saved. The model list fills a picker, with context
     length and price when the server reports them. Nothing is generated, so
     no tokens are spent. A 401 or 403 says "this server needs a key".
   - **`acp`:** it spawns the command in a job object with a scrubbed
     environment and no project folder, completes the `initialize`
     handshake, records the capabilities the agent advertises, and shuts it
     down.
   - **`desktop-app`:** it checks the path exists.

   The probe runs on the host worker and never blocks a frame. Name and look
   are prefilled, and **More details** opens the documented blocks (4.2).
   Save is enabled only when the test has passed.

A saved AI shows in More AIs beside the built-in ones, in the same row and
state model, with **Edit**, **Test again** and **Remove**. It earns a tab once
it is Ready, like every other AI. Frontier and local AIs sit in the same list,
the same way, so any of them can back Clyffy or join a room later.

### 4.7 Find on this computer

- On **Find**, never on a timer: one read of the OS table of TCP sockets
  listening on loopback. Only the address and port are read, not the owning
  process's memory or command line.
- Each port gets one bounded `GET /v1/models` (1 s timeout, 4 at a time, on
  the host worker). Every port that answers in OpenAI shape is listed by what
  it reports (`owned_by`, its model ids, its `Server` header), with **Use
  this**.
- Servers on the operator's **network** (another machine, such as the GB10)
  are added by typing their address. DevPULSE does not scan the network.
- Reading the socket table needs one new dependency, a cross-platform
  listening-socket reader such as the `listeners` crate. It is added to the
  external reach inventory for the operator's sign-off before it lands.

### 4.6 Sub-passes (one milestone, two PRs, merged on the local gate; R304)

**AH2a: the contract.**
- The JSON Schema and its doc.
- `dpl.ai-catalog/2`, with the built-in entries rewritten into it.
- One loader.
- The profile migration, with the old kinds deleted.
- The "Add an AI" dialog and Test connection.

**AH2b: a saved web-address AI can chat** (scope is the operator's call).
- An `openai-chat` `ProviderRuntime`: SSE streaming, cancel, typed failures
  and usage when it is reported.
- The trust prompt still gates it (AH1b).

The generic `acp` runtime stays A2b, as approved.

## 5. Wiring

- **`dpl-ai-core`:**
  - the schema types;
  - `catalog.rs` (v2 loader);
  - `protocol`, a new enum module;
  - `adapter` stays the runtime contract.
- **`dpl-ai-profiles`:** the store holds provider documents (a JSON column,
  validated on read and on write); the migration; `SUPPLEMENTS` reduced to
  Clyffy.
- **`dpl-ai-host`:**
  - the Test connection probe (`ureq` for HTTP; `dpl-proc` with a job object
    for stdio);
  - the `openai-chat` runtime in AH2b;
  - the credential-store reader, if `os` is chosen.
- **`dpl-chat-window`:** the dialog (intents only; no I/O in the window).
- **`app/src/chat.rs`:** routes the new intents to the host.
- **Docs:**
  - `docs/design/ai-provider-contract.md` is the one truth for the schema:
    every field, why it exists, validation, and the versioning and migration
    rule. It is listed in `docs/README.md`, and ADR-017 links to it.
  - ADR-011 is amended to allow operator-entered addresses (HTTPS, or plain HTTP on this computer
    or the local network, per 4.1), through the same `ureq` client.
  - The external reach inventory gains `keyring` (chosen 2026-11-19) and
    a listening-socket reader for Find (4.7).
- No new crate. No `chat/*` → `ai/providers/*` edge. No vendor names under
  `crates/chat/`, and no product name anywhere in the Add an AI path.

## 6. Proof

- **Unit tests:**
  - every built-in and ACP entry validates against the JSON Schema;
  - the serde types and the schema agree;
  - each of the four minimum rules refuses by name: no protocol, an endpoint
    of the wrong shape, `http` off loopback, credentials in a URL, an
    `api-key` with no secret reference, and a missing model for
    `openai-chat`;
  - migrating a `local` row and a `custom` row produces the expected
    documents;
  - a key never appears in a stored document, a log line or a pilot snapshot.
- **The HTTP probe** is tested against a loopback test listener in the unit
  test (status codes, a malformed body, a timeout).
- **A real-run drill against a real server the operator is running** (any
  OpenAI-compatible server on this computer or the network; nothing in the
  drill names one):
  - Find, or type its address;
  - Test connection detects `openai-chat` and lists its models;
  - save it, and it shows in More AIs;
  - in AH2b, one real streamed reply.
  CI has no such server, so this drill runs on this laptop and its evidence
  goes in §8.
- **A pilot scenario, `chat-add-ai`:** opens the dialog, picks a starting
  point, fills the fields, and checks that Save stays disabled until the test
  passes. It runs against a scratch chat state home and a scratch library.
- **Shots** at 1100×700 and maximized, on a scratch library.
- **`cargo green`** passes locally before each merge, and main's hosted run is
  read after it.

## 7. Risks

- **A key in the wrong place.** Mitigation: the reference-only schema, a
  masked field, a redaction test, and no key in logs, pilot metadata or the
  ledger.
- **Drift between the JSON Schema and serde.** Mitigation: the schema-pin
  test runs in the gate.
- **Over-documenting turns into required fields.** Mitigation: only the four
  questions are required, and everything else is optional and marked
  verified or unverified.
- **The protocol enum used as a back door to hardcoding.** Mitigation: a
  value is added only with its transport and its tests, in one PR.
- **The probe spawning arbitrary commands.** Mitigation: the operator typed
  the command; it runs only on an explicit **Test**, with no shell, a scrubbed
  environment, a job object, no project folder and a 10 s bound.

## 8. Evidence

(Filled in as the pass lands; the PR carries it, and this file is deleted on
landing.)
