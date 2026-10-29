//! Host-projected project storage controls. No filesystem access lives here.
use super::*;
use dpl_chat_projects::ProjectId;

pub const PROJECTS_ID: PilotId = PilotId::new("chat.projects");
pub const PROJECT_LABEL_ID: PilotId = PilotId::new("chat.projects.label");
pub const PROJECT_WORKSPACE_ID: PilotId = PilotId::new("chat.projects.workspace");
pub const PROJECT_SETUP_ID: PilotId = PilotId::new("chat.projects.setup");
pub const PROJECT_OPEN_ID: PilotId = PilotId::new("chat.projects.open");
pub const PROJECT_SELECT_ID: PilotId = PilotId::new("chat.projects.select");
pub const PROJECT_RETRY_ID: PilotId = PilotId::new("chat.projects.retry-save");
pub const PROJECT_COPY_ID: PilotId = PilotId::new("chat.projects.save-copy");

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectChoice {
    pub id: ProjectId,
    pub label: String,
    pub workspace: PathBuf,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct ProjectView {
    pub machine: String,
    pub active: Option<ProjectChoice>,
    pub choices: Vec<ProjectChoice>,
    pub busy: bool,
    pub can_switch: bool,
    pub can_save_copy: bool,
    pub message: String,
}

impl Default for ProjectView {
    fn default() -> Self {
        Self {
            machine: "Checking this computer".into(),
            active: None,
            choices: Vec::new(),
            busy: false,
            can_switch: true,
            can_save_copy: false,
            message: String::new(),
        }
    }
}

#[derive(Debug, Clone, Default)]
pub(super) struct ProjectUi {
    open: bool,
    label: String,
    workspace: String,
}

impl ChatWindow {
    /// Freeze conversation edits while the host commits a project switch.
    /// Autosaving alone must not lock typing; it uses a separate busy flag.
    pub fn set_project_input_locked(&mut self, locked: bool) {
        self.project_input_locked = locked;
    }

    pub fn open_project_setup(&mut self) {
        self.project_ui.open = true;
    }

    pub fn set_project_view(&mut self, view: ProjectView) {
        self.project_view = view;
    }

    pub fn project_archive(&self) -> Result<serde_json::Value, String> {
        self.runtime.archive()
    }

    /// Validate before the host commits a project binding. A rejected archive
    /// leaves both the current conversations and every UI selection intact.
    pub fn validate_project_archive(&self, value: &serde_json::Value) -> Result<(), String> {
        self.runtime.clone().restore_archive(value)
    }

    pub fn restore_project_archive(&mut self, value: &serde_json::Value) -> Result<(), String> {
        self.runtime.restore_archive(value)?;
        self.history_focus = None;
        self.history_page = None;
        self.composer_status = None;
        self.provider_tab = None;
        Ok(())
    }

    pub fn clear_project_state(&mut self) {
        self.runtime.clear_project_state();
        self.history_focus = None;
        self.history_page = None;
        self.composer_status = None;
        self.provider_tab = None;
    }

    pub fn project_provider_views(&self) -> Vec<ProviderView> {
        self.runtime.provider_views()
    }

    pub(super) fn project_session_read_only(&self) -> bool {
        self.runtime.selected_provider().is_some_and(|provider| {
            provider.registration.sessions.iter().any(|session| {
                Some(session.id) == provider.active_session
                    && session.state == SessionState::ReadOnly
            })
        })
    }

    pub(super) fn project_bar(&mut self, ui: &mut egui::Ui) {
        let fluid = cl::Fluid::of(ui.max_rect());
        ui.horizontal_wrapped(|ui| {
            ui.label(
                RichText::new(&self.project_view.machine)
                    .size(fluid.fs_body_sm())
                    .color(cl::ink::MUTED),
            );
            ui.separator();
            if let Some(active) = &self.project_view.active {
                let _ = cl::tip(
                    ui.label(RichText::new(&active.label).color(Tone::Cyan.color())),
                    &active.workspace.display().to_string(),
                );
            } else {
                ui.label(RichText::new("Set up project chat storage").color(Tone::Amber.color()));
            }
            if quiet_button(
                ui,
                PROJECTS_ID,
                "Projects",
                "Choose a workspace for project chats",
            )
            .clicked()
            {
                self.project_ui.open = !self.project_ui.open;
            }
            if self.project_view.busy {
                ui.spinner();
            }
            if !self.project_view.message.is_empty() {
                ui.label(
                    RichText::new(&self.project_view.message)
                        .small()
                        .color(cl::ink::MUTED),
                );
            }
        });
    }

    pub(super) fn project_popup(&mut self, ui: &mut egui::Ui, intents: &mut Vec<ChatWindowIntent>) {
        if !self.project_ui.open {
            return;
        }
        let mut open = true;
        let fluid = cl::Fluid::of(ui.max_rect());
        egui::Window::new("Project chats")
            .id(egui::Id::new("chat.projects.popup"))
            .open(&mut open)
            .collapsible(false)
            .resizable(true)
            .default_width(fluid.panel_width())
            .frame(egui::Frame::window(ui.style()).stroke(Stroke::new(1.0, Tone::Cyan.color())))
            .show(ui.ctx(), |ui| {
                ui.label(
                    "Choose the project folder where DevPULSE should keep its chats and drafts.",
                );
                ui.label(
                    RichText::new("Standalone app storage is not connected yet.")
                        .small()
                        .color(cl::ink::MUTED),
                );
                ui.label(
                    RichText::new(
                        "Saved chats can be read; native session resume is not connected yet.",
                    )
                    .small()
                    .color(cl::ink::MUTED),
                );
                ui.separator();
                if !self.project_view.message.is_empty() {
                    ui.label(&self.project_view.message);
                }
                let can_change = !self.project_input_locked
                    && !self.project_view.busy
                    && self.project_view.can_switch;
                if !can_change {
                    ui.label(
                        RichText::new(
                            "Project changes are waiting for the current work to finish.",
                        )
                        .color(Tone::Amber.color()),
                    );
                }
                ui.add_enabled_ui(can_change, |ui| {
                    project_input(
                        ui,
                        PROJECT_LABEL_ID,
                        "Project label",
                        "A short name for this project's chats",
                        &mut self.project_ui.label,
                    );
                    project_input(
                        ui,
                        PROJECT_WORKSPACE_ID,
                        "Full project folder",
                        "Enter the full path to your project folder",
                        &mut self.project_ui.workspace,
                    );
                    let workspace = PathBuf::from(self.project_ui.workspace.trim());
                    let has_folder = workspace.is_absolute();
                    ui.horizontal_wrapped(|ui| {
                        if project_button(
                            ui,
                            PROJECT_SETUP_ID,
                            "Set up project",
                            "Create project chat storage in this folder",
                            has_folder && !self.project_ui.label.trim().is_empty(),
                            fluid.tap_min(),
                        )
                        .clicked()
                        {
                            intents.push(ChatWindowIntent::SetupProject {
                                workspace: workspace.clone(),
                                label: self.project_ui.label.trim().to_owned(),
                            });
                        }
                        if project_button(
                            ui,
                            PROJECT_OPEN_ID,
                            "Open project",
                            "Open a folder already marked for project chat storage",
                            has_folder,
                            fluid.tap_min(),
                        )
                        .clicked()
                        {
                            intents.push(ChatWindowIntent::OpenProject { workspace: workspace.clone() });
                        }
                        if self.project_view.active.is_some() && self.project_view.can_save_copy
                            && project_button(ui, PROJECT_COPY_ID, "Save a copy",
                                "Save current chats and drafts in a new project folder; preserve the original project",
                                has_folder && !self.project_ui.label.trim().is_empty(), fluid.tap_min()).clicked() {
                            intents.push(ChatWindowIntent::SaveProjectCopy {
                                workspace, label: self.project_ui.label.trim().to_owned(),
                            });
                        }
                    });
                    if !self.project_view.choices.is_empty() {
                        ui.separator();
                        ui.label("Your projects");
                        egui::ScrollArea::vertical()
                            .max_height(fluid.tap_min() * 4.0)
                            .show(ui, |ui| {
                                for choice in &self.project_view.choices {
                                    let active = self
                                        .project_view
                                        .active
                                        .as_ref()
                                        .is_some_and(|current| current.id == choice.id);
                                    let response = cl::tip(
                                        ui.selectable_label(active, &choice.label),
                                        &choice.workspace.display().to_string(),
                                    );
                                    let mut metadata = NodeMetadata::new(
                                        PROJECT_SELECT_ID.child(&PilotKey::new(choice.id.as_str())),
                                        Role::Button,
                                        format!("Activate project {}", choice.label),
                                    );
                                    metadata.description =
                                        Some(choice.workspace.display().to_string());
                                    ui.pilot_track(&response, metadata);
                                    if response.clicked() && !active {
                                        intents.push(ChatWindowIntent::SelectProject(
                                            choice.id.clone(),
                                        ));
                                    }
                                }
                            });
                    }
                });
                if self.project_view.active.is_some()
                    && project_button(
                        ui,
                        PROJECT_RETRY_ID,
                        "Retry save",
                        "Retry saving the active project's chats and drafts",
                        !self.project_view.busy && !self.project_input_locked,
                        fluid.tap_min(),
                    )
                    .clicked()
                {
                    intents.push(ChatWindowIntent::RetryProjectSave);
                }
            });
        self.project_ui.open = open;
    }
}

fn project_input(ui: &mut egui::Ui, id: PilotId, label: &str, tooltip: &str, value: &mut String) {
    ui.label(label);
    let response = cl::tip(
        ui.add(egui::TextEdit::singleline(value).desired_width(f32::INFINITY)),
        tooltip,
    );
    let mut metadata = NodeMetadata::new(id, Role::TextInput, label);
    metadata.description = Some(tooltip.into());
    metadata.value = Some(value.clone());
    ui.pilot_track(&response, metadata);
}

fn project_button(
    ui: &mut egui::Ui,
    id: PilotId,
    label: &str,
    tooltip: &str,
    enabled: bool,
    height: f32,
) -> egui::Response {
    let response = cl::tip(
        ui.add_enabled(
            enabled,
            egui::Button::new(label).min_size(egui::vec2(0.0, height)),
        ),
        tooltip,
    );
    ui.pilot_track(&response, NodeMetadata::new(id, Role::Button, tooltip));
    response
}
