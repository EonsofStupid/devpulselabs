//! Adopting the operator's official Codex into the tool depot.
//!
//! R278: vendor CLIs are "properly registered as sub processes that can be
//! easily updated once they have the binary". The operator installs and
//! updates Codex the vendor's way; adoption copies that build into
//! DevPULSE's depot, and only after proving it:
//!
//! 1. **Signed by OpenAI.** Every file in the folder carries a valid
//!    Authenticode signature whose subject is OpenAI's. Windows checks it
//!    (`Get-AuthenticodeSignature`): this workspace denies `unsafe`, so the
//!    trust check is the OS's own, run as a quiet subprocess.
//! 2. **New enough.** The build reports a version at least the one the
//!    vendored schema came from.
//! 3. **The protocol still fits.** The build's OWN schema
//!    (`codex app-server generate-json-schema`) still declares every method,
//!    notification and server request this adapter uses. That, not a
//!    version range, is what admits a newer Codex.
//!
//! Then [`dpl_tools_deps::install_tool`] swaps it in. The app never runs
//! Codex from the operator's install folder; it only offers to adopt from it.

use crate::activation::{probe_version, profiled_command, CodexVersion};
use crate::runtime::{METHODS_USED, NOTIFICATIONS_MAPPED, SERVER_REQUESTS_HANDLED};
use std::collections::BTreeMap;
use std::fmt;
use std::fs;
use std::path::{Path, PathBuf};
#[cfg(windows)]
use std::process::Command;
use std::process::Stdio;

/// The certificate subject every adopted file must be signed with: OpenAI's
/// code-signing identity, as Windows reports it.
pub const PUBLISHER: &str = "CN=\"OpenAI OpCo, LLC\"";

/// A file's Authenticode signature, as Windows reports it.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Signature {
    /// `Valid` when the signature and its chain verify.
    pub status: String,
    /// The signer certificate's subject.
    pub subject: String,
}

/// A candidate build's own protocol: the three schema files it generates.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Protocol {
    pub client_requests: String,
    pub server_notifications: String,
    pub server_requests: String,
    pub native_responses: BTreeMap<String, String>,
}

const NATIVE_SCHEMAS: &[(&str, &str)] = &[
    (
        "ThreadListResponse.json",
        include_str!("../schema/ThreadListResponse.json"),
    ),
    (
        "ThreadResumeResponse.json",
        include_str!("../schema/ThreadResumeResponse.json"),
    ),
    (
        "ThreadTurnsListResponse.json",
        include_str!("../schema/ThreadTurnsListResponse.json"),
    ),
];

/// What adoption asks of a candidate. The real one runs Windows and the
/// candidate itself ([`SystemInspector`]); tests use fakes.
pub trait Inspector {
    fn signature(&self, file: &Path) -> Result<Signature, String>;
    fn version(&self, executable: &Path) -> Result<CodexVersion, String>;
    fn protocol(&self, executable: &Path) -> Result<Protocol, String>;
}

/// Why a build was not adopted. Every refusal names the file or the
/// protocol item, so the operator sees what to fix.
#[derive(Debug, Clone, PartialEq, Eq)]
pub enum AdoptionError {
    NotFound(PathBuf),
    Unsigned { file: String, status: String },
    WrongPublisher { file: String, subject: String },
    Inspect(String),
    TooOld(CodexVersion),
    ProtocolLoss(Vec<String>),
    Install(String),
}

impl fmt::Display for AdoptionError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::NotFound(path) => write!(f, "no Codex was found at {}", path.display()),
            Self::Unsigned { file, status } => write!(
                f,
                "{file} is not validly signed (Windows says {status}), so it was not adopted"
            ),
            Self::WrongPublisher { file, subject } => write!(
                f,
                "{file} is signed by someone other than OpenAI ({subject}), so it was not adopted"
            ),
            Self::Inspect(reason) => write!(f, "this Codex could not be checked: {reason}"),
            Self::TooOld(version) => write!(
                f,
                "Codex {version} is older than DevPULSE needs; update Codex, then try again"
            ),
            Self::ProtocolLoss(missing) => write!(
                f,
                "this Codex no longer speaks what DevPULSE uses ({}), so it was not adopted",
                missing.join(", ")
            ),
            Self::Install(reason) => write!(f, "Codex could not be installed: {reason}"),
        }
    }
}

impl std::error::Error for AdoptionError {}

/// A build that passed every check, ready to install.
#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Verified {
    /// The folder shipped whole (the executable and its signed companions).
    pub folder: PathBuf,
    /// The executable's name inside `folder`.
    pub entrypoint: PathBuf,
    pub version: CodexVersion,
}

/// Prove a candidate without changing anything.
pub fn verify(executable: &Path, inspector: &dyn Inspector) -> Result<Verified, AdoptionError> {
    if !executable.is_file() {
        return Err(AdoptionError::NotFound(executable.to_path_buf()));
    }
    let folder = executable
        .parent()
        .ok_or_else(|| AdoptionError::NotFound(executable.to_path_buf()))?;
    let entrypoint = PathBuf::from(
        executable
            .file_name()
            .ok_or_else(|| AdoptionError::NotFound(executable.to_path_buf()))?,
    );
    let mut files = fs::read_dir(folder)
        .map_err(|error| AdoptionError::Inspect(format!("{}: {error}", folder.display())))?
        .filter_map(Result::ok)
        .map(|entry| entry.path())
        .filter(|path| path.is_file())
        .collect::<Vec<_>>();
    files.sort();
    for file in &files {
        let name = file
            .file_name()
            .map_or_else(String::new, |name| name.to_string_lossy().into_owned());
        let signature = inspector
            .signature(file)
            .map_err(|reason| AdoptionError::Inspect(format!("{name}: {reason}")))?;
        if signature.status != "Valid" {
            return Err(AdoptionError::Unsigned {
                file: name,
                status: signature.status,
            });
        }
        if !signature.subject.starts_with(PUBLISHER) {
            return Err(AdoptionError::WrongPublisher {
                file: name,
                subject: signature.subject,
            });
        }
    }
    let version = inspector
        .version(executable)
        .map_err(AdoptionError::Inspect)?;
    if !version.is_activation_supported() {
        return Err(AdoptionError::TooOld(version));
    }
    let protocol = inspector
        .protocol(executable)
        .map_err(AdoptionError::Inspect)?;
    let missing = protocol_losses(&protocol);
    if !missing.is_empty() {
        return Err(AdoptionError::ProtocolLoss(missing));
    }
    Ok(Verified {
        folder: folder.to_path_buf(),
        entrypoint,
        version,
    })
}

/// Everything this adapter uses that `protocol` no longer declares.
pub fn protocol_losses(protocol: &Protocol) -> Vec<String> {
    let declares = |schema: &str, name: &str| schema.contains(&format!("\"{name}\""));
    let mut missing = Vec::new();
    for method in METHODS_USED {
        if !declares(&protocol.client_requests, method) {
            missing.push(format!("request {method}"));
        }
    }
    for notification in NOTIFICATIONS_MAPPED {
        if !declares(&protocol.server_notifications, notification) {
            missing.push(format!("notification {notification}"));
        }
    }
    for request in SERVER_REQUESTS_HANDLED {
        if !declares(&protocol.server_requests, request) {
            missing.push(format!("server request {request}"));
        }
    }
    // Native resume imports persisted history and acknowledged authority. A
    // method-name match cannot qualify that boundary. Until a new release is
    // explicitly qualified, require the full structural response contract,
    // including every referenced local definition. Documentation may change.
    for (name, pinned) in NATIVE_SCHEMAS {
        let candidate = protocol.native_responses.get(*name);
        if candidate.and_then(|text| semantic_schema(text).ok()) != semantic_schema(pinned).ok() {
            missing.push(format!("response contract {name}"));
        }
    }
    let candidate: serde_json::Value =
        serde_json::from_str(&protocol.client_requests).unwrap_or_default();
    let pinned: serde_json::Value =
        serde_json::from_str(include_str!("../schema/ClientRequest.json"))
            .expect("vendored request schema");
    for name in [
        "ThreadListParams",
        "ThreadResumeParams",
        "ThreadTurnsListParams",
    ] {
        if semantic_value(candidate["definitions"][name].clone())
            != semantic_value(pinned["definitions"][name].clone())
        {
            missing.push(format!("request contract {name}"));
        }
    }
    missing
}

fn semantic_schema(text: &str) -> Result<serde_json::Value, serde_json::Error> {
    serde_json::from_str(text).map(semantic_value)
}

fn semantic_value(mut value: serde_json::Value) -> serde_json::Value {
    match &mut value {
        serde_json::Value::Object(object) => {
            for key in ["description", "title", "$schema", "examples"] {
                object.remove(key);
            }
            for (key, child) in object {
                if matches!(
                    key.as_str(),
                    "properties" | "definitions" | "$defs" | "patternProperties"
                ) {
                    if let serde_json::Value::Object(entries) = child {
                        for schema in entries.values_mut() {
                            *schema = semantic_value(schema.take());
                        }
                    }
                } else if !matches!(key.as_str(), "default" | "enum" | "const") {
                    *child = semantic_value(child.take());
                }
            }
        }
        serde_json::Value::Array(array) => {
            for child in array {
                *child = semantic_value(child.take());
            }
        }
        _ => {}
    }
    value
}

/// Prove `executable`, then install its folder into the depot at
/// `tools_root` as the current Codex.
pub fn adopt(
    executable: &Path,
    tools_root: &Path,
    inspector: &dyn Inspector,
) -> Result<(Verified, dpl_tools_deps::Installed), AdoptionError> {
    let verified = verify(executable, inspector)?;
    let version = verified.version.to_string();
    let installed = dpl_tools_deps::install_tool(
        tools_root,
        &dpl_tools_deps::ToolSource {
            id: dpl_tools_deps::CODEX,
            version: &version,
            path: &verified.folder,
            entrypoint: Some(&verified.entrypoint),
        },
    )
    .map_err(AdoptionError::Install)?;
    Ok((verified, installed))
}

/// Where the vendor's own Windows installer puts Codex. Found ONLY to offer
/// adoption; nothing is ever run from here.
pub fn installed_codex() -> Option<PathBuf> {
    let local = std::env::var_os("LOCALAPPDATA")?;
    let executable = PathBuf::from(local)
        .join("Programs")
        .join("OpenAI")
        .join("Codex")
        .join("bin")
        .join(format!("codex{}", std::env::consts::EXE_SUFFIX));
    executable.is_file().then_some(executable)
}

/// The real inspector: Windows checks signatures, and the candidate reports
/// its own version and protocol.
#[derive(Debug, Default, Clone, Copy)]
pub struct SystemInspector;

impl Inspector for SystemInspector {
    fn signature(&self, file: &Path) -> Result<Signature, String> {
        windows_signature(file)
    }

    fn version(&self, executable: &Path) -> Result<CodexVersion, String> {
        probe_version(executable, &std::env::temp_dir()).map_err(|error| error.to_string())
    }

    fn protocol(&self, executable: &Path) -> Result<Protocol, String> {
        let out = std::env::temp_dir().join(format!(
            "dpl-codex-protocol-{}-{}",
            std::process::id(),
            std::time::SystemTime::now()
                .duration_since(std::time::UNIX_EPOCH)
                .map_or(0, |elapsed| elapsed.as_nanos())
        ));
        let result = generate_protocol(executable, &out);
        let _ = fs::remove_dir_all(&out);
        result
    }
}

/// Regenerate this adapter's vendored schema from `executable` (the build the
/// minimum moves to): the three schema files plus `VERSION`, written into
/// `schema_dir`. `cargo xtask vendor-schema codex` runs it; the census tests
/// then check the adapter against what was written.
pub fn vendor_schema(executable: &Path, schema_dir: &Path) -> Result<CodexVersion, String> {
    let version =
        probe_version(executable, &std::env::temp_dir()).map_err(|error| error.to_string())?;
    let out = std::env::temp_dir().join(format!("dpl-codex-vendor-{}", std::process::id()));
    let protocol = generate_protocol(executable, &out);
    let _ = fs::remove_dir_all(&out);
    let protocol = protocol?;
    for (name, text) in [
        ("ClientRequest.json", &protocol.client_requests),
        ("ServerNotification.json", &protocol.server_notifications),
        ("ServerRequest.json", &protocol.server_requests),
    ] {
        fs::write(schema_dir.join(name), text).map_err(|error| format!("{name}: {error}"))?;
    }
    for (name, text) in &protocol.native_responses {
        fs::write(schema_dir.join(name), text).map_err(|error| format!("{name}: {error}"))?;
    }
    fs::write(
        schema_dir.join("VERSION"),
        format!(
            "codex-cli {version}
"
        ),
    )
    .map_err(|error| format!("VERSION: {error}"))?;
    Ok(version)
}

fn generate_protocol(executable: &Path, out: &Path) -> Result<Protocol, String> {
    fs::create_dir_all(out).map_err(|error| format!("{}: {error}", out.display()))?;
    let mut command = profiled_command(executable, out);
    // Schema generation never borrows an ambient account/config profile.
    command.env("CODEX_HOME", out.join("isolated-home"));
    command
        .args(["app-server", "generate-json-schema", "--out"])
        .arg(out)
        .stdin(Stdio::null())
        .stdout(Stdio::null())
        .stderr(Stdio::null());
    let status = command
        .status()
        .map_err(|error| format!("the schema could not be generated: {error}"))?;
    if !status.success() {
        return Err(format!(
            "the schema generator exited with status {}",
            status.code().unwrap_or(-1)
        ));
    }
    let read =
        |name: &str| fs::read_to_string(out.join(name)).map_err(|error| format!("{name}: {error}"));
    Ok(Protocol {
        client_requests: read("ClientRequest.json")?,
        server_notifications: read("ServerNotification.json")?,
        server_requests: read("ServerRequest.json")?,
        native_responses: NATIVE_SCHEMAS
            .iter()
            .map(|(name, _)| read(&format!("v2/{name}")).map(|text| ((*name).to_owned(), text)))
            .collect::<Result<_, _>>()?,
    })
}

/// Ask Windows for `file`'s Authenticode signature. The path travels in an
/// environment variable, never in the script text, so no file name can
/// change what the script does.
#[cfg(windows)]
fn windows_signature(file: &Path) -> Result<Signature, String> {
    let system_root = std::env::var_os("SystemRoot")
        .ok_or_else(|| "the Windows folder is unknown (SystemRoot is unset)".to_owned())?;
    let powershell = PathBuf::from(system_root)
        .join("System32")
        .join("WindowsPowerShell")
        .join("v1.0")
        .join("powershell.exe");
    let mut command = Command::new(powershell);
    dpl_proc::quiet(&mut command);
    command
        .args([
            "-NoProfile",
            "-NonInteractive",
            "-Command",
            "$s = Get-AuthenticodeSignature -LiteralPath $env:DPL_SIGNED_FILE; \
             [Console]::Out.Write(\"$($s.Status)|$($s.SignerCertificate.Subject)\")",
        ])
        .env("DPL_SIGNED_FILE", file)
        .stdin(Stdio::null())
        .stderr(Stdio::null());
    let output = command
        .output()
        .map_err(|error| format!("Windows could not be asked: {error}"))?;
    parse_signature(&String::from_utf8_lossy(&output.stdout))
}

#[cfg(not(windows))]
fn windows_signature(_file: &Path) -> Result<Signature, String> {
    // Only Windows can vouch here. Off Windows, adoption is refused rather
    // than allowed unchecked; Linux gets its own proof (published digests)
    // in a later pass.
    Err("signature checks run on Windows only".to_owned())
}

/// `Valid|CN="OpenAI OpCo, LLC", O=…` → its two halves. Only Windows asks
/// (and the tests check the parsing on every OS).
#[cfg(any(windows, test))]
fn parse_signature(output: &str) -> Result<Signature, String> {
    let (status, subject) = output
        .trim()
        .split_once('|')
        .ok_or_else(|| "Windows gave no signature answer".to_owned())?;
    Ok(Signature {
        status: status.trim().to_owned(),
        subject: subject.trim().to_owned(),
    })
}

#[cfg(test)]
mod tests {
    use super::*;
    use std::sync::atomic::{AtomicUsize, Ordering};

    static NEXT: AtomicUsize = AtomicUsize::new(0);

    const OPENAI: &str = "CN=\"OpenAI OpCo, LLC\", O=\"OpenAI OpCo, LLC\", L=San Francisco";

    struct Fake {
        unsigned: Option<&'static str>,
        subject: &'static str,
        version: CodexVersion,
        drop_method: Option<&'static str>,
    }

    impl Fake {
        fn good() -> Self {
            Self {
                unsigned: None,
                subject: OPENAI,
                version: CodexVersion::new(0, 163, 0),
                drop_method: None,
            }
        }
    }

    impl Inspector for Fake {
        fn signature(&self, file: &Path) -> Result<Signature, String> {
            let name = file
                .file_name()
                .map(|name| name.to_string_lossy().into_owned());
            let status = if name.as_deref() == self.unsigned {
                "NotSigned"
            } else {
                "Valid"
            };
            Ok(Signature {
                status: status.to_owned(),
                subject: self.subject.to_owned(),
            })
        }

        fn version(&self, _executable: &Path) -> Result<CodexVersion, String> {
            Ok(self.version)
        }

        fn protocol(&self, _executable: &Path) -> Result<Protocol, String> {
            let client = include_str!("../schema/ClientRequest.json");
            Ok(Protocol {
                client_requests: self.drop_method.map_or_else(
                    || client.to_owned(),
                    |method| client.replace(&format!("\"{method}\""), "\"removed-method\""),
                ),
                server_notifications: include_str!("../schema/ServerNotification.json").to_owned(),
                server_requests: include_str!("../schema/ServerRequest.json").to_owned(),
                native_responses: NATIVE_SCHEMAS
                    .iter()
                    .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
                    .collect(),
            })
        }
    }

    /// An install folder shaped like the vendor's: the CLI and its helper.
    fn install_folder() -> PathBuf {
        let sequence = NEXT.fetch_add(1, Ordering::Relaxed);
        let folder =
            std::env::temp_dir().join(format!("dpl-codex-adopt-{}-{sequence}", std::process::id()));
        fs::create_dir_all(&folder).expect("folder");
        let executable = folder.join(format!("codex{}", std::env::consts::EXE_SUFFIX));
        fs::write(&executable, b"codex").expect("codex");
        fs::write(folder.join("codex-code-mode-host.exe"), b"host").expect("host");
        #[cfg(unix)]
        {
            use std::os::unix::fs::PermissionsExt;
            fs::set_permissions(&executable, fs::Permissions::from_mode(0o755))
                .expect("executable permission");
        }
        executable
    }

    #[test]
    fn an_openai_signed_current_build_is_verified_with_its_companions() {
        let executable = install_folder();
        let verified = verify(&executable, &Fake::good()).expect("verified");
        assert_eq!(verified.version, CodexVersion::new(0, 163, 0));
        assert_eq!(verified.folder, executable.parent().expect("folder"));
    }

    #[test]
    fn an_unsigned_companion_refuses_the_whole_folder() {
        let executable = install_folder();
        let fake = Fake {
            unsigned: Some("codex-code-mode-host.exe"),
            ..Fake::good()
        };
        assert!(matches!(
            verify(&executable, &fake),
            Err(AdoptionError::Unsigned { file, .. }) if file == "codex-code-mode-host.exe"
        ));
    }

    #[test]
    fn a_build_signed_by_anyone_else_is_refused() {
        let executable = install_folder();
        let fake = Fake {
            subject: "CN=Someone Else",
            ..Fake::good()
        };
        assert!(matches!(
            verify(&executable, &fake),
            Err(AdoptionError::WrongPublisher { .. })
        ));
    }

    #[test]
    fn an_older_build_is_refused() {
        let executable = install_folder();
        let fake = Fake {
            version: CodexVersion::new(0, 145, 0),
            ..Fake::good()
        };
        assert_eq!(
            verify(&executable, &fake),
            Err(AdoptionError::TooOld(CodexVersion::new(0, 145, 0)))
        );
    }

    #[test]
    fn a_build_that_dropped_a_method_we_call_is_refused_by_name() {
        let executable = install_folder();
        let fake = Fake {
            drop_method: Some("turn/start"),
            ..Fake::good()
        };
        assert_eq!(
            verify(&executable, &fake),
            Err(AdoptionError::ProtocolLoss(vec![
                "request turn/start".to_owned()
            ]))
        );
    }

    #[test]
    fn the_vendored_schema_passes_its_own_protocol_check() {
        let vendored = Protocol {
            client_requests: include_str!("../schema/ClientRequest.json").to_owned(),
            server_notifications: include_str!("../schema/ServerNotification.json").to_owned(),
            server_requests: include_str!("../schema/ServerRequest.json").to_owned(),
            native_responses: NATIVE_SCHEMAS
                .iter()
                .map(|(name, text)| ((*name).to_owned(), (*text).to_owned()))
                .collect(),
        };
        assert_eq!(protocol_losses(&vendored), Vec::<String>::new());
    }

    #[test]
    fn response_field_or_dependency_changes_refuse_adoption_but_docs_do_not() {
        let mut protocol = Fake::good().protocol(Path::new("unused")).unwrap();
        let name = "ThreadResumeResponse.json";
        let original = protocol.native_responses[name].clone();
        let mut schema: serde_json::Value = serde_json::from_str(&original).unwrap();
        schema["description"] = serde_json::json!("updated documentation");
        protocol
            .native_responses
            .insert(name.into(), schema.to_string());
        assert!(protocol_losses(&protocol).is_empty());
        schema["definitions"]["Thread"]["properties"]["updatedAt"]["type"] =
            serde_json::json!("string");
        protocol
            .native_responses
            .insert(name.into(), schema.to_string());
        assert!(protocol_losses(&protocol).contains(&format!("response contract {name}")));
        protocol.native_responses.remove(name);
        assert!(protocol_losses(&protocol).contains(&format!("response contract {name}")));
    }

    #[test]
    fn windows_signature_answers_parse() {
        assert_eq!(
            parse_signature("Valid|CN=\"OpenAI OpCo, LLC\", O=\"OpenAI OpCo, LLC\"\r\n"),
            Ok(Signature {
                status: "Valid".to_owned(),
                subject: "CN=\"OpenAI OpCo, LLC\", O=\"OpenAI OpCo, LLC\"".to_owned(),
            })
        );
        assert!(parse_signature("").is_err());
    }
}
