# Standards adoption assessment: guided UI procedures

**Research date:** 2026-11-02.  
**Status:** engineering recommendation based on primary sources and local review;
not a published standard, partner commitment or implementation result.  
**Related:** [initial review](guided-workflow-standard-review.md),
[product/research brief](scripted-ui-events-brief.md), R294–R296.

## 1. Recommendation

Do not begin with a new general-purpose UI event language. Begin with a small
**UI-procedure interoperability profile**, evaluated against existing workflow
standards, that another team can implement independently.

The adoption proposition to test is:

> An application exposes one capability-and-observation adapter; multiple tools
> can then automate it, guide a person through it, and produce demonstrations
> whose steps mean the same thing.

This is a proposed value proposition, not evidence of demand. Its beneficiaries
must confirm that the integration savings justify their implementation cost.

Use DevPULSE as the first authoring product and reference integration. Keep the
interoperability contract independent of DevPULSE, egui, the model, recorder,
renderer and website player. A2UI should be a supported presentation/action
binding, not a requirement that applications rebuild their UI.

A new protocol remains an option only if a concrete mapping experiment exposes
an important semantic gap that an existing profile cannot express cleanly.
Current sources do not establish that need.

## 2. Existing standards substantially reduce the new work

| Existing work | Already covers | Engineering consequence |
|---|---|---|
| [Open Workflow Specification v1.0.3](https://github.com/open-workflow-specification/specification/blob/v1.0.3/dsl-reference.md), formerly Serverless Workflow | Control flow, waits, retries, event correlation, custom calls, metadata and extension mechanisms; MCP/A2A bindings also appear in this tag. | First candidate for a bounded UI-domain profile. Do not invent equivalent sequence/branch/retry grammar. Its site calls 1.0.3 current while GitHub marks the release prerelease; pin the exact tested revision. Its MCP binding uses an older initialization model, so current MCP compatibility must be tested separately. [Release evidence](https://github.com/open-workflow-specification/specification/releases). |
| [Arazzo 1.1.0](https://spec.openapis.org/arazzo/v1.1.0.html) | API workflows, dependencies, inputs/outputs, success criteria and success/failure routing; OpenAPI and AsyncAPI sources. | Good fit for API-native tasks and a constrained interchange mapping. “Outcome conditions” are already established. Human UI manipulation needs an observation adapter; mandatory semantics cannot hide in optional ignored extensions. |
| [SCXML](https://www.w3.org/TR/scxml/) | Event-driven state-machine semantics, guards, hierarchy, concurrency and history. | Reference for precise execution behavior. Do not introduce it alongside another canonical workflow model without a concrete interoperability need. |
| [WebDriver BiDi, October 8 2026 Working Draft](https://www.w3.org/TR/2026/WD-webdriver-bidi-20261008/) | Browser commands, contexts, input and event subscriptions. | Browser adapter foundation; command acknowledgement does not establish a business outcome. Draft status is not final Recommendation status. |
| [MCP Tasks](https://modelcontextprotocol.io/extensions/tasks/overview) and [Skills](https://modelcontextprotocol.io/extensions/skills/overview) | Run lifecycle and distribution/discovery of workflow instructions/resources. | Reuse these bindings. Neither makes prose or scripts a deterministic executable contract. The current extension overview lists both as Beta; a Final SEP is not the same as a Stable extension. |
| [A2UI actions](https://a2ui.org/concepts/actions/) and [catalogs](https://a2ui.org/concepts/catalogs/) | Trusted components/functions, local actions, routed events, negotiation and validation. | Use a small catalog and ordinary action bindings first. An A2UI renderer does not automatically expose the semantics of another application's UI. |
| [CloudEvents](https://github.com/cloudevents/spec/blob/main/cloudevents/spec.md) | A common event envelope and contextual metadata. | Evaluate an external binding for facts. Define the UI-domain payload rather than another universal envelope. Preserve the internal Rust signal doctrine. |

The gap hypothesis is narrower than the initial proposal: **portable target and
capability identity, authoritative completion evidence, equivalent completion
by a human or automation, and presentation linked to those same steps**.

That intersection needs a mapping experiment and outside implementer feedback.
It is not established as a novel invention merely because the reviewed
standards do not package it together.

### Mapping experiment before format design

Take three concrete procedures: edit and save a document; open a secondary
window and change a setting; start an asynchronous export and observe its
completion. Encode each using a restricted Open Workflow profile and the
current internal typed workflow structure. Try Arazzo for the API-native case.

Account for every field and transition. Classify each as existing standard,
UI-domain schema, optional presentation, or unresolved semantic conflict.
Test both human-follow and automated completion. Record what cannot round-trip.

Adopt an external representation only if this proves useful. Preserve the
existing workflow registry, schemas, validation and Rust ownership; a profile
importer must not become a competing engine. Exclude arbitrary shell, scripts
and containers from the bounded execution subset. Reject unsupported required
features explicitly; do not claim full Open Workflow conformance.

## 3. What successful adoption cases actually demonstrate

These are documented mechanisms, not a claim that copying them guarantees
success.

**LSP:** the 2016 announcement connected a working Microsoft protocol with
Red Hat's language-server work and Codenvy's editor integration. Its economic
benefit was reusable integration across independent tools. Our equivalent must
be measurable adapter reuse, not merely an attractive DevPULSE editor.
[Microsoft launch](https://code.visualstudio.com/blogs/2016/06/27/common-language-protocol).

**MCP Apps:** it reconciled patterns already demonstrated by MCP-UI and OpenAI's
Apps SDK. Its launch included a maturing SDK and support from multiple hosts.
The relevant lesson is to find implementers already duplicating a boundary and
give them a shared solution with a migration path.
[Official launch](https://blog.modelcontextprotocol.io/posts/2026-01-26-mcp-apps/).

**OpenTelemetry:** OpenTracing and OpenCensus converged to reduce fragmentation;
compatibility bridges let adopters migrate incrementally. We need converters
that preserve representable behavior and report losses, so adoption does not
require replacing a recorder, runtime and renderer together.
[Project history](https://www.cncf.io/blog/2019/05/21/a-brief-history-of-opentelemetry-so-far/),
[migration guide](https://opentelemetry.io/docs/compatibility/migration/opentracing/).

My inference: adoption depends on reducing someone else's integration and
maintenance burden. A polished end-user UI establishes product demand.
Independent implementations establish whether the shared contract is useful.

## 4. Minimum core to standardize

The first normative core should contain four things:

1. **Capability/target references:** stable identifiers, typed input/output
   schemas, version compatibility, instance/window lifetime and ambiguity rules.
2. **Completion evidence:** predicate, authoritative observer, run/step
   correlation, freshness and evidence strength. Distinguish input delivered,
   UI changed and operation committed. Reuse an existing predicate mechanism
   where possible.
3. **Human/automation equivalence:** a human-follow step and an automated step
   evaluate the same declared outcome. Recorded demonstrations label their
   evidence as recorded; they cannot masquerade as live completion.
4. **Capability and failure rules:** missing observation, unsupported version,
   stale evidence, duplicate events, cancellation and required-feature refusal.

Teaching media belongs in a linked manifest using the same step IDs.
A2UI rendering, MCP transport and browser/native adapters are separate bindings.

Do not put motion editing, a new video codec, model prompting, training datasets,
a universal permission service, marketplace discovery, or every OS input event
in version 0.1. Those expand the implementer's obligation without proving the
shared boundary.

Portability needs three distinct claims:

- **Document portability:** two tools can parse and preserve the artifact.
- **Runtime portability:** independent consumers perform/check the same procedure
  against the same supported app.
- **Cross-application substitution:** different apps satisfy an equivalent
  business capability. This is a harder, separately qualified domain contract.

The first two are the initial target. “Works across apps” must not imply that
arbitrary controls in unrelated apps are automatically interchangeable.

## 5. Who would adopt, and what they get

These are candidate partner categories, not confirmed interest or contacted
organizations.

| Implementer | Benefit we must prove | Smallest reasonable adoption |
|---|---|---|
| App maintainer with repeated onboarding/support workflows | One integration reused by guidance, automation and test tools. | Register a few typed capabilities and truthful outcome observations; retain the existing UI. |
| Agent/runtime maintainer | Reuse verified procedures with fewer model calls and fewer UI ambiguities. | Consume the bounded profile and expose outcomes through an existing tool/task interface. |
| Guide/training publisher | A guide remains useful outside its originating recorder and can track actual practice. | Export/import the teaching manifest and step IDs; live observation is optional and explicitly declared. |
| A2UI host/renderer maintainer | Useful guided experiences without a bespoke integration per workflow vendor. | Small catalog/action binding and state updates with graceful fallback. |
| Automation/recorder maintainer | Shared import/export and validation rather than another custom partner format. | Loss-aware trace conversion and a conformance adapter. |

A commercial recorder may prefer lock-in and have little incentive to help.
Start with communities already experiencing integration duplication, rather than
assuming every capture vendor wants portability.

A practical initial partner set is an application maintainer, an independent
workflow/agent runtime implementer, and a teaching/viewer implementer. Open
Workflow maintainers, the MCP Tasks/Events or Skills groups, and A2UI renderer
maintainers are relevant technical review venues; interest is unverified.

Before freezing syntax, conduct a small set of structured implementer interviews
when outreach is authorized. Ask for an existing integration, its maintenance
cost, the minimum reusable boundary, features they would refuse, and whether
they will implement a prototype. An endorsement without engineering time is a
weak signal.

## 6. A local adoption blocker: distribution rights

The current repository [LICENSE](../../LICENSE) is deliberately read-only and
reserves use/modification/distribution rights. Public visibility does not supply
the reusable implementation package this adoption plan needs.

Recommendation: deliberately separate the specification, schemas, fixtures,
validator and adapter/reference SDK into a package with explicit reusable
contribution and distribution terms. The authoring product can retain its own
licensing policy. Do not copy proprietary dependencies or product code into that
package without resolving its rights.

For the official MCP route, SEP-2133 explicitly requires Apache-2.0 availability
for extensions and states contributor grants.
[Official extension requirements](https://modelcontextprotocol.io/seps/2133-extensions).

No license has been changed. This is an adoption prerequisite to resolve before
asking outside developers to reuse code, not permission to relicense the repo.
Independent implementation from a clearly licensed specification and fixtures
must also be possible without the proprietary product.

Governance should publish decision authority, contribution rules, version policy,
an issue/RFC process and security reporting. External implementers must be able
to influence breaking decisions. Foundation transfer is a later option; it is
not a substitute for an initial useful implementation.

## 7. What MCP adoption actually requires

The current [extension lifecycle](https://modelcontextprotocol.io/extensions/overview)
documents Experimental → Beta → Stable/Core. Official incubation needs a
WG/IG and maintainer. Beta requires an Extensions Track SEP and a reference
implementation in an official SDK. Stable requires another SEP, two independent
interoperating implementations including client and server, documented real use
through the known-implementations list, and conformance for observable protocol
behavior. Support remains optional.
[Stable requirements](https://modelcontextprotocol.io/seps/3392-extension-lifecycle#stable).

The [SEP process](https://modelcontextprotocol.io/community/sep-guidelines)
requires prior linked group discussion, a sponsor and a runnable prototype.
The repository currently accepts proposal PRs from collaborators. Observable
Standards Track changes require normative-requirement traceability and merged
conformance before Final. Generic SEP guidance and extension-specific SDK
requirements differ; apply the latter when proposing an extension.

This makes the route:

1. Test an application-level profile using existing tools/resources/actions.
2. Bring the demonstrated interoperability gap and runnable kit to the relevant
   existing group; ask whether an existing mechanism already suffices.
3. Incubate only the additional behavior that multiple implementers need.
4. Pursue the extension path if consensus identifies protocol-level work.
5. Stabilize after independent use, rather than promise compatibility before
   learning from it.

MCP's own [design principles](https://modelcontextprotocol.io/community/design-principles)
favor composition, graceful degradation and demonstrated patterns. A proposal
whose main justification is shorter prompts or cheaper current models is weak.
Durable justification is portable execution/evidence across implementations.

A2UI can initially use custom catalogs and ordinary actions; changing the core
protocol is not a prerequisite. Prefer contributions that improve an existing
renderer or catalog before requesting a broad new standard. The
[A2UI roadmap](https://a2ui.org/roadmap/) provides its current contribution channels.

Neither official extension status nor a merged specification obliges SDKs,
hosts or app vendors to adopt. Usage still must justify the cost.

## 8. Delivery gates: evidence before expansion

These are recommended internal gates, not fabricated results or additional
formal requirements imposed by standards bodies.

| Gate | Concrete artifact | Pass criterion |
|---|---|---|
| A: shared problem | Existing-standard mapping and implementer problem statements | Another implementer identifies the same missing boundary and commits to trying it. If existing standards suffice, publish a profile/example instead of a new protocol. |
| B: reliable vertical slice | Corrected recorder/Coach foundation; one native procedure | Record, parameterize, watch/live-demonstrate, follow and execute with truthful outcomes and retained source data. |
| C: implementer kit | Small normative spec, schemas, fixtures, validator, reference adapter and bounded runner | A developer can run positive and negative cases with documented setup; no private product service required. |
| D: independence | Outside consumer or adapter implementation | A team implements from the written contract, then interoperates without private undocumented explanations. Two adapters written by us alone do not prove this. |
| E: integration economics | Time/code/dependency measurements and comparative run results | The adapter is cheaper to maintain than bespoke integrations and retains that benefit during a version/layout change. |
| F: standardization | Upstream discussion, revisions, sponsor and conformance | Follow the selected project's current formal requirements; maintain public compatibility and usage records. |

Reference kit components: Rust core compatible with this repo; language-neutral
schemas and fixtures; a thin SDK in an outside implementer's language where
needed; a command-line validator; sample producer/consumer; and a conformance
runner. Native DevPULSE remains Rust. Separate SDK artifacts do not justify a
second language/runtime inside the app.

An outside implementer should be able to start with a few capabilities.
Observation, execution, teaching and capture support are independently declared.
An unsupported host gets an honest static/manual fallback. A missing required
outcome check stops verified execution.

### Benchmark design

Use the same task set, initial state, apps and failure injections for:

- an agent reasoning through individual UI actions;
- a conventional hand-authored automation script;
- the proposed reusable procedure.

Measure authoring and maintenance effort separately from runtime costs. Report
success, false completion, duplicate side effects, recovery/manual intervention,
p50/p95 latency, model calls/tokens and transferred bytes. Include failures and
failed retries in the denominator. Zero calls on a known path does not prove
lower total cost than a conventional script.

At minimum the conformance corpus needs happy-path manual/automatic completion,
wrong window, stale state, duplicate trigger, hidden/ambiguous target, delayed
completion after a highlight disappears, dropped observation, cancel/restart,
version mismatch and an unsupported required capability. The stop/capture and
preservation defects from the initial review must be addressed before claiming
recording reliability.

Publish test vectors and results by implementation/version. A JSON Schema can
validate shape; it cannot establish execution semantics by itself.

## 9. What would make me stop or change direction

- No independent team sees enough value to implement: continue as a useful
  DevPULSE product feature; stop calling the format an emerging standard.
- Existing workflow+A2UI conventions express the complete contract: contribute
  a profile, tooling and examples upstream.
- The artifact needs our recorder/model/renderer to work: remove that dependency
  before claiming interoperability.
- Semantic app adapters are unavailable: offer watchable guides and clearly
  limited legacy automation; do not imply verified universal app support.
- Integration cost exceeds the bespoke alternative: narrow the core.
- Repeated version drift breaks outcomes: improve capability contracts and
  conformance rather than add more permissive visual guesses.
- The benchmark only beats a model on every click but loses to ordinary
  automation: revise the economic claim to teaching/reuse/interchange benefits.

## 10. Next concrete work

The next standards research deliverable should be the three-procedure mapping
experiment, a small gap proposal and an implementer evaluation kit plan—not
another broad vision document. In parallel, product work should fix the audited
capture/Coach defects and prove one workflow.

The first outside review request, once outreach is authorized, should ask a
specific question: “Can your tool consume this procedure and report the same
manual/automatic completion evidence? Which fields or behaviors prevent that?”
Bring runnable examples when available; do not imply they exist today.

Separate a standards package when its licensing and minimal scope are decided.
Do not fork an entire product or establish a new foundation merely to signal
seriousness. Adoption would be demonstrated by independent implementation,
repeat use and shared maintenance, not by the name of the format.

## Evidence status

Two independent research passes covered overlap and adoption cases; root checked
current MCP requirements and the local license. No implementer interviews,
outreach, protocol implementation, new repository, license changes or benchmark
results occurred in this research pass. Source pages can evolve; release/tag
caveats above are intentional.
Both researchers fact-checked the assessment; the formal Stable-stage usage
and conformance conditions were clarified from that review.

