# Pinned upstream schemas

Retrieved 2026-11-02 for the local mapping experiment. Vendored originals are
unaltered. Both schemas declare JSON Schema draft 2020-12. Validation uses the
existing Rust `jsonschema` dependency with default features disabled; tests and
the mapping CLI need no network retrieval. Schema validity proves shape, not
capability availability, runtime behavior, or full standard conformance.

| File | Primary source | SHA-256 |
| --- | --- | --- |
| `open-workflow-1.0.3.upstream.yaml` | [tagged source](https://raw.githubusercontent.com/open-workflow-specification/specification/v1.0.3/schema/workflow.yaml) | `df3326da907e17e743e9d9e888839481d1557abdf91c592e74f27506ff523f0b` |
| `open-workflow-1.0.3.derived.json` | Derived locally from the preceding YAML | `c699e15534f18f94aa70853dec3985dcd7ca0be1541d936a246cad5a2e87b699` |
| `arazzo-1.1-2026-04-15.upstream.json` | [dated official schema](https://spec.openapis.org/arazzo/1.1/schema/2026-04-15) | `37be908409bdb2f7bffe61fa23685c7e84cbeebfafac475a1d01dbc50ff7ab9e` |
| `open-workflow-LICENSE` | [tagged Apache-2.0 license](https://raw.githubusercontent.com/open-workflow-specification/specification/v1.0.3/LICENSE) | `c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4` |
| `arazzo-LICENSE` | [upstream Apache-2.0 license](https://raw.githubusercontent.com/OAI/Arazzo-Specification/main/LICENSE) | `c71d239df91726fc519c6eb72d318ec65820627232b2f796219e87dcf35d0ab4` |

The Apache-2.0 licenses apply to their respective vendored upstream material;
this does not change the surrounding repository's licensing. The Arazzo license
URL is the upstream main branch as retrieved on the date above, pinned here by
content hash. The schema itself uses the dated official URL. Preserve these
license files with redistributed copies.

## One-time conversion

The root agent converted the exact OW YAML bytes offline using Node and the
official `nodeca/js-yaml` 4.1.0 `dist/js-yaml.js` distribution, temporarily saved
as untracked `target/gf0-schema-tools/js-yaml.cjs`. Converter SHA-256:
`c738b0020702aa631abd8de23a902a3079d99d3060e44166955cb51f570881c4`.
The conversion was `jsYaml.load(yamlText)` with its default schema followed by
`JSON.stringify(value, null, 2) + '\n'`, encoded as UTF-8. No schema rules or
identifiers were edited. The original YAML remains available for auditing.

PyYAML was considered but was not used for the resulting JSON. No YAML parser
or converter dependency is introduced into the Rust application or runtime.

## Version and validation boundaries

The Open Workflow website calls 1.0.3 current; its GitHub release is marked
prerelease. This experiment pins that tag and the bytes above rather than main.
The Arazzo document version is 1.1.0; the dated schema covers the 1.1 family.
The local profile separately refuses unsupported versions and features, since
upstream schema version patterns alone do not establish runtime compatibility.

Upstream schemas do not validate the meaning of custom `dpl.*` calls or profile
metadata. Reference resolution, required capabilities and observation semantics
need separate checks. The Arazzo schema's JSON Schema metaschema reference is
handled by the validator; the local fixture API description is never fetched or
invoked. Its operation IDs are checked locally by `tests/standards.rs`.
