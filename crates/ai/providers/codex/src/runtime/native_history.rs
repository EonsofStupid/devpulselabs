//! Exact 0.163.0 list/resume/turn-page contracts; no filesystem or credentials.
use super::{Outbound, PermissionMode};
use dpl_chat_runtime::seam::{
    RuntimeEvent, RuntimeHistoryItem, RuntimeSessionSummary, RuntimeUpdate,
};
use serde_json::{json, Value};
use std::collections::HashSet;
use std::path::{Path, PathBuf};
use std::time::{Duration, Instant};

pub(super) const MAX_FRAME_BYTES: usize = 4 * 1024 * 1024;
const MAX_HISTORY_BYTES: usize = 16 * 1024 * 1024;
const MAX_TEXT_BYTES: usize = 1024 * 1024;
const PAGE_SIZE: usize = 64;
const MAX_PAGES: usize = 128;
const MAX_ITEMS: usize = 32_768;
const MAX_ID_BYTES: usize = 4096;
pub(super) const DEADLINE: Duration = Duration::from_secs(120);

#[derive(Debug)]
struct ListContinuation {
    token: String,
    cursor: Option<String>,
    archived: bool,
}

#[derive(Debug)]
struct Listing {
    next: Option<ListContinuation>,
    cursors: HashSet<(bool, String)>,
    ids: HashSet<String>,
    pages: usize,
    pending: bool,
    archived: bool,
}

#[derive(Debug)]
struct Loading {
    id: String,
    mode: PermissionMode,
    session: Option<RuntimeSessionSummary>,
    items: Vec<RuntimeHistoryItem>,
    turns: HashSet<String>,
    item_ids: HashSet<String>,
    cursors: HashSet<String>,
    pages: usize,
    bytes: usize,
}

#[derive(Debug)]
pub(super) struct NativeHistory {
    workspace: PathBuf,
    listing: Option<Listing>,
    listing_generation: u64,
    loading: Option<Loading>,
    started: Option<Instant>,
    // thread/started can precede or follow resume's response. Neither is a
    // new session. Bounded by the lifetime session count, never disk identity.
    resumed: HashSet<String>,
}

impl NativeHistory {
    pub(super) fn new(workspace: PathBuf) -> Self {
        Self {
            workspace,
            listing: None,
            listing_generation: 0,
            loading: None,
            started: None,
            resumed: HashSet::new(),
        }
    }

    pub(super) fn busy(&self) -> bool {
        self.started.is_some()
    }

    pub(super) fn suppresses(&self, id: &str) -> bool {
        self.loading.as_ref().is_some_and(|load| load.id == id)
    }

    pub(super) fn resumed(&self, id: &str) -> bool {
        self.resumed.contains(id)
    }

    pub(super) fn expired(&self, now: Instant) -> bool {
        self.started
            .is_some_and(|start| now.saturating_duration_since(start) >= DEADLINE)
    }

    pub(super) fn session_id(&self) -> Option<String> {
        self.loading.as_ref().map(|load| load.id.clone())
    }

    pub(super) fn abort(&mut self) {
        self.loading = None;
        self.listing = None;
        self.started = None;
    }

    pub(super) fn list(&mut self, cursor: Option<&str>) -> Result<Outbound, String> {
        if self.busy() {
            return Err("a native session operation is already running".into());
        }
        if let Some(cursor) = cursor {
            bounded(cursor, MAX_ID_BYTES)?;
            if self
                .listing
                .as_ref()
                .and_then(|list| list.next.as_ref())
                .map(|next| next.token.as_str())
                != Some(cursor)
            {
                return Err("the session cursor does not belong to this listing".into());
            }
        } else {
            self.listing_generation = self
                .listing_generation
                .checked_add(1)
                .ok_or("native listing generation limit reached")?;
            self.listing = Some(Listing {
                next: None,
                cursors: HashSet::new(),
                ids: HashSet::new(),
                pages: 0,
                pending: false,
                archived: false,
            });
        }
        let list = self.listing.as_mut().ok_or("missing session listing")?;
        // Only the current one-use token authorizes a continuation. The native
        // cursor remains opaque and private, including at the archive boundary.
        let provider_cursor = list.next.take().and_then(|next| {
            list.archived = next.archived;
            next.cursor
        });
        list.pending = true;
        self.started = Some(Instant::now());
        Ok(Outbound::Request {
            method: "thread/list",
            params: json!({
                "cwd": self.workspace, "cursor": provider_cursor, "limit": PAGE_SIZE,
                "sortKey": "updated_at", "sortDirection": "desc", "archived": list.archived,
                "sourceKinds": ["cli", "appServer", "vscode"], "useStateDbOnly": false
            }),
        })
    }

    pub(super) fn resume(&mut self, id: &str, mode: PermissionMode) -> Result<Outbound, String> {
        bounded(id, MAX_ID_BYTES)?;
        if id.is_empty() || self.busy() || self.resumed.len() >= 4096 {
            return Err("native resume is busy or its session limit was reached".into());
        }
        self.resumed.insert(id.to_owned());
        self.listing = None;
        self.loading = Some(Loading {
            id: id.to_owned(),
            mode,
            session: None,
            items: Vec::new(),
            turns: HashSet::new(),
            item_ids: HashSet::new(),
            cursors: HashSet::new(),
            pages: 0,
            bytes: 0,
        });
        self.started = Some(Instant::now());
        let sandbox = if mode == PermissionMode::Workspace {
            "workspace-write"
        } else {
            "read-only"
        };
        Ok(Outbound::Request {
            method: "thread/resume",
            params: json!({
                "threadId": id, "cwd": self.workspace, "excludeTurns": true,
                "sandbox": sandbox, "approvalPolicy": "on-request",
                "config": { "sandbox_workspace_write.network_access": false }
            }),
        })
    }

    /// Returns a complete event or the next request. A caller aborts this
    /// transaction on every error, including inability to queue the next page.
    pub(super) fn response(
        &mut self,
        method: &str,
        result: &Value,
    ) -> Result<(Option<RuntimeEvent>, Option<Outbound>), String> {
        if !self.busy() {
            return Ok((None, None));
        }
        if method == "thread/list" {
            let list = self.listing.as_mut().ok_or("unexpected session page")?;
            if !list.pending {
                return Err("unsolicited session page".into());
            }
            let data = array(result, "data")?;
            if data.len() > PAGE_SIZE || list.pages >= MAX_PAGES {
                return Err("session listing limit exceeded".into());
            }
            let mut sessions = Vec::with_capacity(data.len());
            for thread in data {
                let summary = summary(thread, &self.workspace)?;
                if !list.ids.insert(summary.provider_session_id.clone()) {
                    return Err("duplicate session across listing pages".into());
                }
                sessions.push(summary);
            }
            let provider_cursor = cursor(result)?;
            if let Some(next) = &provider_cursor {
                if !list.cursors.insert((list.archived, next.clone())) {
                    return Err("session listing cursor loop or limit".into());
                }
            }
            // An exhausted active phase still has one continuation: the first
            // archived page, whose provider cursor must be null. Never expose a
            // made-up provider cursor or mix pagination cursors between phases.
            let has_next = provider_cursor.is_some() || !list.archived;
            if has_next && list.pages + 1 >= MAX_PAGES {
                return Err("session listing page limit exceeded".into());
            }
            let archived = list.archived || provider_cursor.is_none();
            let replace = list.pages == 0;
            list.pages += 1;
            list.next = has_next.then(|| ListContinuation {
                token: format!("codex-list-{}-{}", self.listing_generation, list.pages),
                cursor: provider_cursor,
                archived,
            });
            let next = list.next.as_ref().map(|next| next.token.clone());
            list.pending = false;
            self.started = None;
            return Ok((
                Some(RuntimeEvent::SessionsPage {
                    sessions,
                    next_cursor: next,
                    replace,
                }),
                None,
            ));
        }
        let load = self.loading.as_mut().ok_or("unexpected history response")?;
        // Bound complete wire data, including fields not interpreted below.
        load.bytes = load
            .bytes
            .checked_add(result.to_string().len())
            .ok_or("history size overflow")?;
        if load.bytes > MAX_HISTORY_BYTES {
            return Err("native history exceeds the load limit".into());
        }
        if method == "thread/resume" {
            if load.session.is_some() {
                return Err("duplicate resume response".into());
            }
            let thread = result.get("thread").ok_or("resume omitted thread")?;
            let session = summary(thread, &self.workspace)?;
            if session.provider_session_id != load.id {
                return Err("resume returned a different thread".into());
            }
            same_workspace(string(result, "cwd")?, &self.workspace)?;
            let sandbox = result.get("sandbox").ok_or("resume omitted sandbox")?;
            let expected = if load.mode == PermissionMode::Workspace {
                "workspaceWrite"
            } else {
                "readOnly"
            };
            if string(sandbox, "type")? != expected
                || result.get("approvalPolicy") != Some(&json!("on-request"))
            {
                return Err("resume changed the requested permission policy".into());
            }
            if sandbox
                .get("networkAccess")
                .is_some_and(|value| value != &Value::Bool(false))
            {
                return Err("resume enabled network access".into());
            }
            if let Some(roots) = sandbox.get("writableRoots") {
                for root in roots.as_array().ok_or("invalid writable roots")? {
                    same_workspace(
                        root.as_str().ok_or("invalid writable root")?,
                        &self.workspace,
                    )?;
                }
            }
            load.session = Some(session);
            return Ok((None, Some(turn_page(&load.id, None))));
        }
        if method != "thread/turns/list" || load.session.is_none() {
            return Err("unexpected native response method".into());
        }
        let turns = array(result, "data")?;
        if turns.len() > PAGE_SIZE || load.pages >= MAX_PAGES {
            return Err("history page limit exceeded".into());
        }
        for turn in turns {
            let turn_id = identity(turn, "id")?;
            if !load.turns.insert(turn_id.to_owned()) {
                return Err("duplicate turn across history pages".into());
            }
            if turn.get("itemsView").is_some_and(|value| value != "full") {
                return Err("history page contains incomplete turn items".into());
            }
            match string(turn, "status")? {
                "completed" | "failed" | "interrupted" => {}
                _ => return Err("cannot attach a thread with an unfinished or unknown turn".into()),
            }
            for item in array(turn, "items")? {
                let item_id = identity(item, "id")?;
                if load.items.len() >= MAX_ITEMS || !load.item_ids.insert(item_id.to_owned()) {
                    return Err("duplicate history item or item limit exceeded".into());
                }
                load.items.push(RuntimeHistoryItem {
                    provider_turn_id: turn_id.to_owned(),
                    provider_item_id: item_id.to_owned(),
                    updates: item_updates(item)?,
                });
            }
        }
        load.pages += 1;
        if let Some(next) = cursor(result)? {
            if !load.cursors.insert(next.clone()) || load.pages >= MAX_PAGES {
                return Err("history cursor loop or page limit".into());
            }
            return Ok((None, Some(turn_page(&load.id, Some(&next)))));
        }
        let load = self.loading.take().ok_or("missing completed history")?;
        self.started = None;
        Ok((
            Some(RuntimeEvent::SessionLoaded {
                session: load.session.ok_or("missing resumed session")?,
                items: load.items,
            }),
            None,
        ))
    }
}

pub(super) fn is_native(method: &str) -> bool {
    matches!(
        method,
        "thread/list" | "thread/resume" | "thread/turns/list"
    )
}

fn turn_page(id: &str, cursor: Option<&str>) -> Outbound {
    Outbound::Request {
        method: "thread/turns/list",
        params: json!({ "threadId": id, "cursor": cursor, "limit": PAGE_SIZE, "sortDirection": "asc", "itemsView": "full" }),
    }
}

fn bounded(text: &str, limit: usize) -> Result<(), String> {
    if text.len() > limit || text.contains('\0') {
        Err("native field exceeds its string bound".into())
    } else {
        Ok(())
    }
}

fn string<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    let text = value
        .get(key)
        .and_then(Value::as_str)
        .ok_or_else(|| format!("native response omitted string {key}"))?;
    bounded(text, MAX_TEXT_BYTES)?;
    Ok(text)
}

fn identity<'a>(value: &'a Value, key: &str) -> Result<&'a str, String> {
    let id = string(value, key)?;
    bounded(id, MAX_ID_BYTES)?;
    if id.is_empty() {
        return Err("empty native identity".into());
    }
    Ok(id)
}

fn array<'a>(value: &'a Value, key: &str) -> Result<&'a Vec<Value>, String> {
    value
        .get(key)
        .and_then(Value::as_array)
        .ok_or_else(|| format!("native response omitted array {key}"))
}

fn cursor(value: &Value) -> Result<Option<String>, String> {
    match value.get("nextCursor") {
        None | Some(Value::Null) => Ok(None),
        Some(Value::String(text)) if !text.is_empty() => {
            bounded(text, MAX_ID_BYTES)?;
            Ok(Some(text.clone()))
        }
        _ => Err("invalid native page cursor".into()),
    }
}

fn same_workspace(cwd: &str, workspace: &Path) -> Result<(), String> {
    bounded(cwd, MAX_ID_BYTES)?;
    let cwd = Path::new(cwd);
    // No filesystem lookup through provider data. The host supplies a canonical
    // bound workspace; never accept parent traversal, even for equal inputs.
    if [cwd, workspace].iter().any(|path| {
        path.components()
            .any(|component| component == std::path::Component::ParentDir)
    }) {
        return Err("native workspace contains parent traversal".into());
    }
    if cwd == workspace {
        return Ok(());
    }
    #[cfg(windows)]
    {
        use std::path::{Component, Prefix};
        // Exact Windows 0.163.0 qualification showed thread/resume removes
        // the verbatim disk prefix from both returned cwd fields. Normalize
        // only that proven spelling difference, not directory case, UNC,
        // device paths, or relative paths.
        let normalize_disk = |path: &Path| -> Option<PathBuf> {
            let mut components = path.components();
            let Component::Prefix(prefix) = components.next()? else {
                return None;
            };
            let letter = match prefix.kind() {
                Prefix::Disk(letter) | Prefix::VerbatimDisk(letter) => letter,
                _ => return None,
            };
            if components.next()? != Component::RootDir {
                return None;
            }
            let mut normalized = PathBuf::from(format!("{}:\\", char::from(letter)));
            for component in components {
                let Component::Normal(name) = component else {
                    return None;
                };
                normalized.push(name);
            }
            Some(normalized)
        };
        let normalized = normalize_disk(cwd);
        if normalized.is_some() && normalized == normalize_disk(workspace) {
            return Ok(());
        }
    }
    Err("native thread is outside the bound workspace".into())
}

fn summary(thread: &Value, workspace: &Path) -> Result<RuntimeSessionSummary, String> {
    same_workspace(string(thread, "cwd")?, workspace)?;
    let title = match thread.get("name") {
        Some(Value::String(name)) if !name.is_empty() => {
            bounded(name, MAX_TEXT_BYTES)?;
            name.clone()
        }
        None | Some(Value::Null) | Some(Value::String(_)) => string(thread, "preview")?.to_owned(),
        _ => return Err("invalid native thread name".into()),
    };
    let updated = thread
        .get("updatedAt")
        .and_then(Value::as_i64)
        .ok_or("native thread omitted integer updatedAt")?;
    Ok(RuntimeSessionSummary {
        provider_session_id: identity(thread, "id")?.to_owned(),
        title: Some(title),
        working_directory: workspace.to_path_buf(),
        updated_at: Some(updated.to_string()),
    })
}

fn item_updates(item: &Value) -> Result<Vec<RuntimeUpdate>, String> {
    let mut updates = Vec::new();
    match string(item, "type")? {
        "userMessage" => {
            for part in array(item, "content")? {
                if string(part, "type")? == "text" {
                    updates.push(RuntimeUpdate::UserText(string(part, "text")?.to_owned()));
                } else {
                    // Do not dereference historical attachments or emit paths/URLs
                    // from unsupported payloads. Explicit evidence beats omission.
                    updates.push(RuntimeUpdate::Other(
                        "Native user attachment retained by Codex; preview unavailable".into(),
                    ));
                }
            }
        }
        "agentMessage" => updates.push(RuntimeUpdate::AssistantText(
            string(item, "text")?.to_owned(),
        )),
        "plan" => updates.push(RuntimeUpdate::Plan(string(item, "text")?.to_owned())),
        "reasoning" => {
            for key in ["summary", "content"] {
                if let Some(parts) = item.get(key) {
                    for part in parts.as_array().ok_or("invalid reasoning fragments")? {
                        let text = part.as_str().ok_or("invalid reasoning text")?;
                        bounded(text, MAX_TEXT_BYTES)?;
                        updates.push(RuntimeUpdate::ThoughtText(text.to_owned()));
                    }
                }
            }
        }
        // Historical command text is display-only. Never dispatched as a tool.
        "commandExecution" => {
            updates.push(RuntimeUpdate::Tool(string(item, "command")?.to_owned()));
            if let Some(Value::String(output)) = item.get("aggregatedOutput") {
                bounded(output, MAX_TEXT_BYTES)?;
                updates.push(RuntimeUpdate::Tool(output.clone()));
            }
        }
        _ => updates.push(RuntimeUpdate::Other(
            "Native history item retained by Codex; preview unavailable".into(),
        )),
    }
    if updates.len() > 4096 {
        return Err("native item fragment limit exceeded".into());
    }
    Ok(updates)
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn workspace_parent_traversal_is_refused_even_when_inputs_match() {
        assert!(same_workspace("/project/../project", Path::new("/project/../project")).is_err());
        assert!(same_workspace("/project/../project", Path::new("/project")).is_err());
        assert!(same_workspace("/project", Path::new("/project/../project")).is_err());
    }

    #[cfg(windows)]
    #[test]
    fn resume_accepts_only_the_proven_windows_disk_prefix_difference() {
        let bound = PathBuf::from(r"\\?\D:\localdev\project");
        assert!(same_workspace(r"D:\localdev\project", &bound).is_ok());
        assert!(same_workspace(
            r"\\?\D:\localdev\project",
            Path::new(r"D:\localdev\project")
        )
        .is_ok());
        for other in [
            r"D:\localdev\Project",
            r"D:\localdev\other",
            r"D:\localdev\x\..\project",
            r"D:localdev\project",
            r"\\.\D:\localdev\project",
        ] {
            assert!(same_workspace(other, &bound).is_err(), "{other}");
        }
        assert!(same_workspace(
            r"\\server\share\project",
            Path::new(r"\\?\UNC\server\share\project")
        )
        .is_err());
        let mut native = NativeHistory::new(bound);
        native.resume("thread-1", PermissionMode::ReadOnly).unwrap();
        let mut result = resumed();
        result["cwd"] = json!(r"D:\localdev\project");
        result["thread"]["cwd"] = json!(r"D:\localdev\project");
        let (event, next) = native.response("thread/resume", &result).unwrap();
        assert!(event.is_none());
        assert!(matches!(
            next,
            Some(Outbound::Request {
                method: "thread/turns/list",
                ..
            })
        ));
    }

    fn fixture() -> NativeHistory {
        NativeHistory::new(PathBuf::from("/project"))
    }
    fn thread(id: &str) -> Value {
        json!({ "id": id, "cwd": "/project", "preview": "saved conversation", "updatedAt": 42 })
    }
    fn resumed() -> Value {
        json!({ "thread": thread("thread-1"), "cwd": "/project", "approvalPolicy": "on-request", "sandbox": {"type":"readOnly", "networkAccess":false} })
    }
    fn page(id: &str, next: Value) -> Value {
        json!({ "data": [{ "id": id, "status":"completed", "itemsView":"full", "items":[
        {"type":"userMessage", "id":format!("user-{id}"), "content":[{"type":"text","text":"question"}]},
        {"type":"agentMessage", "id":format!("assistant-{id}"), "text":"answer"}
    ]}], "nextCursor": next })
    }

    #[test]
    fn history_is_staged_until_last_page_and_preserves_native_identity() {
        let mut native = fixture();
        let request = native.resume("thread-1", PermissionMode::ReadOnly).unwrap();
        let Outbound::Request { params, .. } = request else {
            panic!("request");
        };
        assert_eq!(params["excludeTurns"], true);
        assert_eq!(params["sandbox"], "read-only");
        assert!(params.get("history").is_none());
        let (event, followup) = native.response("thread/resume", &resumed()).unwrap();
        assert!(event.is_none());
        let Outbound::Request { method, params } = followup.unwrap() else {
            panic!("request");
        };
        assert_eq!(method, "thread/turns/list");
        assert_eq!(params["itemsView"], "full");
        assert_eq!(params["sortDirection"], "asc");
        let (event, followup) = native
            .response(method, &page("turn-1", json!("next")))
            .unwrap();
        assert!(event.is_none());
        assert!(followup.is_some());
        let (event, followup) = native
            .response(method, &page("turn-2", Value::Null))
            .unwrap();
        assert!(followup.is_none());
        let Some(RuntimeEvent::SessionLoaded { session, items }) = event else {
            panic!("complete snapshot");
        };
        assert_eq!(session.provider_session_id, "thread-1");
        assert_eq!(items.len(), 4);
        assert_eq!(items[0].provider_turn_id, "turn-1");
        assert_eq!(items[0].provider_item_id, "user-turn-1");
        assert_eq!(
            items[0].updates,
            vec![RuntimeUpdate::UserText("question".into())]
        );
        assert_eq!(
            items[3].updates,
            vec![RuntimeUpdate::AssistantText("answer".into())]
        );
        assert!(!native.busy());
        assert!(native.resumed("thread-1"));
    }

    #[test]
    fn resume_refuses_identity_workspace_and_authority_changes() {
        for (pointer, changed) in [
            ("/thread/id", json!("other")),
            ("/cwd", json!("/other")),
            ("/thread/cwd", json!("/other")),
            ("/sandbox/type", json!("dangerFullAccess")),
            ("/sandbox/networkAccess", json!(true)),
            ("/approvalPolicy", json!("never")),
        ] {
            let mut native = fixture();
            native.resume("thread-1", PermissionMode::ReadOnly).unwrap();
            let mut result = resumed();
            *result.pointer_mut(pointer).unwrap() = changed;
            assert!(
                native.response("thread/resume", &result).is_err(),
                "{pointer}"
            );
        }
    }

    #[test]
    fn malformed_duplicate_partial_and_running_history_never_publish_a_snapshot() {
        for corrupt in [
            json!({"data":null}),
            {
                let mut value = page("t", Value::Null);
                value["data"][0]["itemsView"] = json!("summary");
                value
            },
            {
                let mut value = page("t", Value::Null);
                value["data"][0]["status"] = json!("inProgress");
                value
            },
            {
                let mut value = page("t", Value::Null);
                value["data"][0]["items"][1]["id"] = json!("user-t");
                value
            },
        ] {
            let mut native = fixture();
            native.resume("thread-1", PermissionMode::ReadOnly).unwrap();
            native.response("thread/resume", &resumed()).unwrap();
            assert!(native.response("thread/turns/list", &corrupt).is_err());
        }
        let mut native = fixture();
        native.resume("thread-1", PermissionMode::ReadOnly).unwrap();
        native.response("thread/resume", &resumed()).unwrap();
        native
            .response("thread/turns/list", &page("a", json!("same")))
            .unwrap();
        assert!(native
            .response("thread/turns/list", &page("b", json!("same")))
            .is_err());
    }

    #[test]
    fn listing_is_scoped_and_only_its_own_cursor_can_continue() {
        let mut native = fixture();
        let Outbound::Request { params, .. } = native.list(None).unwrap() else {
            panic!("request");
        };
        assert_eq!(params["cwd"], "/project");
        assert_eq!(params["archived"], false);
        assert!(native.resume("thread-1", PermissionMode::ReadOnly).is_err());
        let (first, _) = native
            .response(
                "thread/list",
                &json!({"data":[thread("one")],"nextCursor":"next"}),
            )
            .unwrap();
        let Some(RuntimeEvent::SessionsPage {
            next_cursor: Some(next),
            replace: true,
            ..
        }) = first
        else {
            panic!("first page must continue");
        };
        assert_ne!(next, "next");
        assert!(native.list(Some("foreign")).is_err());
        assert!(native.list(Some("next")).is_err());
        let Outbound::Request { params, .. } = native.list(Some(&next)).unwrap() else {
            panic!("request");
        };
        assert_eq!(params["cursor"], "next");
        assert_eq!(params["archived"], false);
        let (second, _) = native
            .response(
                "thread/list",
                &json!({"data":[thread("two")],"nextCursor":null}),
            )
            .unwrap();
        let Some(RuntimeEvent::SessionsPage {
            next_cursor: Some(archived),
            replace: false,
            ..
        }) = second
        else {
            panic!("active exhaustion must continue into archives");
        };
        assert!(native.list(Some(&next)).is_err(), "one-use cursor");
        let Outbound::Request { params, .. } = native.list(Some(&archived)).unwrap() else {
            panic!("request");
        };
        assert_eq!(params["cursor"], Value::Null);
        assert_eq!(params["archived"], true);
        let (last, followup) = native
            .response(
                "thread/list",
                &json!({"data":[thread("archived")],"nextCursor":null}),
            )
            .unwrap();
        assert!(followup.is_none());
        assert!(matches!(last, Some(RuntimeEvent::SessionsPage {
            sessions, next_cursor: None, replace: false,
        }) if sessions[0].provider_session_id == "archived"));
        assert!(native.list(Some(&archived)).is_err(), "completed cursor");
    }

    fn listed(native: &mut NativeHistory, data: Vec<Value>, cursor: Value) -> Option<String> {
        let (event, followup) = native
            .response("thread/list", &json!({"data":data,"nextCursor":cursor}))
            .unwrap();
        assert!(followup.is_none());
        let Some(RuntimeEvent::SessionsPage { next_cursor, .. }) = event else {
            panic!("listing page");
        };
        next_cursor
    }

    #[test]
    fn listing_cursors_are_scoped_to_phase_and_refresh_generation() {
        let mut native = fixture();
        native.list(None).unwrap();
        let old = listed(&mut native, vec![], json!("same-native-cursor")).unwrap();
        native.list(None).unwrap();
        let current = listed(&mut native, vec![], json!("same-native-cursor")).unwrap();
        assert_ne!(old, current);
        assert!(native.list(Some(&old)).is_err());
        native.list(Some(&current)).unwrap();
        let archived = listed(&mut native, vec![], Value::Null).unwrap();
        native.list(Some(&archived)).unwrap();
        let archived_next = listed(&mut native, vec![], json!("same-native-cursor")).unwrap();
        let Outbound::Request { params, .. } = native.list(Some(&archived_next)).unwrap() else {
            panic!("request");
        };
        assert_eq!(params["cursor"], "same-native-cursor");
        assert_eq!(params["archived"], true);
        assert!(
            native
                .response(
                    "thread/list",
                    &json!({"data":[],"nextCursor":"same-native-cursor"})
                )
                .is_err(),
            "a repeated cursor within the archived phase is refused"
        );
    }

    #[test]
    fn archived_pages_share_duplicate_identity_and_total_page_limits() {
        let mut native = fixture();
        native.list(None).unwrap();
        let archived = listed(&mut native, vec![thread("same")], Value::Null).unwrap();
        native.list(Some(&archived)).unwrap();
        assert!(native
            .response(
                "thread/list",
                &json!({"data":[thread("same")],"nextCursor":null})
            )
            .is_err());

        let mut native = fixture();
        native.list(None).unwrap();
        // Even an empty active list must query archives. Exhaust the single
        // combined budget there; it cannot be reset at the phase boundary.
        let mut next = listed(&mut native, vec![], Value::Null).unwrap();
        for index in 1..MAX_PAGES - 1 {
            native.list(Some(&next)).unwrap();
            next = listed(&mut native, vec![], json!(format!("page-{index}"))).unwrap();
        }
        native.list(Some(&next)).unwrap();
        assert!(native
            .response("thread/list", &json!({"data":[],"nextCursor":"over-limit"}))
            .is_err());
    }

    #[test]
    fn empty_listing_finishes_only_after_archive_response() {
        let mut native = fixture();
        native.list(None).unwrap();
        let archived = listed(&mut native, vec![], Value::Null).unwrap();
        native.list(Some(&archived)).unwrap();
        assert!(listed(&mut native, vec![], Value::Null).is_none());
        assert!(!native.busy());
    }

    #[test]
    fn attachments_and_unknown_items_are_explicit_bounded_diagnostics() {
        let updates = item_updates(
            &json!({"type":"userMessage","content":[{"type":"localImage","path":"SECRET"}]}),
        )
        .unwrap();
        assert!(matches!(&updates[0], RuntimeUpdate::Other(text) if !text.contains("SECRET")));
        let updates = item_updates(&json!({"type":"futureItem","secret":"SECRET"})).unwrap();
        assert!(matches!(&updates[0], RuntimeUpdate::Other(text) if !text.contains("SECRET")));
        assert!(
            item_updates(&json!({"type":"agentMessage","text":"x".repeat(MAX_TEXT_BYTES+1)}))
                .is_err()
        );
    }
}
