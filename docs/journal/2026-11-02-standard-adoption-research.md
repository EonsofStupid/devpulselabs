# UI procedure standards adoption research — 2026-11-02

R295 clarified turnkey guided authoring plus advanced inspection. R296 rejected
paraphrase and requested senior engineering research on actual standard adoption.
Same isolated worktree/branch, baseline main711fc72; docs only, uncommitted.

New deliverable: `docs/research/ui-procedure-standard-adoption.md`. Root inspected
local LICENSE (explicitly read-only), current MCP extension/SEP rules and A2UI
catalogs. Independent researchers covered Open Workflow/Arazzo/SCXML/WebDriver
overlap and LSP/MCP Apps/OpenTelemetry adoption. Both reviewed final assessment.

Recommendation materially narrows initial proposal: test a UI-domain profile
over existing workflow semantics; specify target/capability identity, completion
evidence, manual/automation equivalence and failure rules. Preserve Rust engine
and signal doctrine; exclude arbitrary scripts/shell/container execution.
Open Workflow1.0.3 tag is a candidate with release/prerelease and older MCP
binding caveats. Arazzo1.1.0 already has success criteria. Do not claim new
workflow grammar or outcome semantics are an invention.

Adoption needs independent implementers and lower integration/maintenance cost.
Separate reusable spec/schema/fixtures/SDK terms from current proprietary product
before distribution. No license amended. MCP official extension Beta requires
official SDK reference; Stable requires independent client/server interoperability,
real use and conformance for observable behavior. Final SEP is not Stable stage.

Next: map three real procedures to candidate standards and internal model;
produce minimal spec/fixtures/validator plan, fix audited foundations, then prove
one native vertical slice and outside implementation. Outreach only when
authorized. No interviews, messages, implementation, repo fork or publication
performed. Keep product work and standards claims separate. Initial review's
twelve defects remain unfixed.
