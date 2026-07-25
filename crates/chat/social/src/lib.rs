//! Typed Social-domain state without a live transport.
//!
//! Production starts empty and unavailable. Authored fixtures may populate
//! deterministic people, projects, conversations, and messages, but cannot
//! claim that a Social transport is qualified.

use std::collections::BTreeMap;
use std::fmt;

macro_rules! owned_id {
    ($name:ident, $prefix:literal) => {
        #[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord)]
        pub struct $name(String);

        impl $name {
            pub fn new(value: impl Into<String>) -> Result<Self, SocialModelError> {
                let value = value.into();
                if value.trim().is_empty() {
                    return Err(SocialModelError::EmptyIdentity($prefix));
                }
                Ok(Self(value))
            }

            pub fn as_str(&self) -> &str {
                &self.0
            }
        }

        impl fmt::Display for $name {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                write!(f, "{}:{}", $prefix, self.0)
            }
        }
    };
}

owned_id!(PersonId, "person");
owned_id!(ProjectId, "project");
owned_id!(SocialConversationId, "conversation");
owned_id!(SocialMessageId, "message");

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum Presence {
    Online,
    Away,
    InFlow,
    Offline,
}

impl Presence {
    pub const fn label(self) -> &'static str {
        match self {
            Self::Online => "Online",
            Self::Away => "Away",
            Self::InFlow => "In flow",
            Self::Offline => "Offline",
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Person {
    pub id: PersonId,
    pub display_name: String,
    pub handle: String,
    pub presence: Presence,
    pub activity: Option<String>,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct Project {
    pub id: ProjectId,
    pub name: String,
    pub summary: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocialThreadKind {
    Direct { person_id: PersonId },
    Group,
    Project { project_id: ProjectId },
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocialConversation {
    pub id: SocialConversationId,
    pub title: String,
    pub kind: SocialThreadKind,
    pub unread: u32,
    pub pinned: bool,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub struct SocialMessage {
    pub id: SocialMessageId,
    pub conversation_id: SocialConversationId,
    pub author_id: PersonId,
    pub body: String,
    pub timestamp: String,
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocialTransportAvailability {
    Unavailable { reason: String },
}

impl SocialTransportAvailability {
    pub fn reason(&self) -> &str {
        match self {
            Self::Unavailable { reason } => reason,
        }
    }

    pub const fn can_send(&self) -> bool {
        false
    }
}

impl Default for SocialTransportAvailability {
    fn default() -> Self {
        Self::Unavailable {
            reason: "DevPULSE Social transport is not qualified in this slice".into(),
        }
    }
}

#[derive(Debug, Clone, PartialEq, Eq)]
pub enum SocialModelError {
    EmptyIdentity(&'static str),
    DuplicatePerson(PersonId),
    DuplicateProject(ProjectId),
    DuplicateConversation(SocialConversationId),
    UnknownPerson(PersonId),
    UnknownProject(ProjectId),
    UnknownConversation(SocialConversationId),
    MessageConversationMismatch {
        expected: SocialConversationId,
        actual: SocialConversationId,
    },
}

impl fmt::Display for SocialModelError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            Self::EmptyIdentity(kind) => write!(f, "{kind} identity cannot be empty"),
            Self::DuplicatePerson(id) => write!(f, "duplicate {id}"),
            Self::DuplicateProject(id) => write!(f, "duplicate {id}"),
            Self::DuplicateConversation(id) => write!(f, "duplicate {id}"),
            Self::UnknownPerson(id) => write!(f, "unknown {id}"),
            Self::UnknownProject(id) => write!(f, "unknown {id}"),
            Self::UnknownConversation(id) => write!(f, "unknown {id}"),
            Self::MessageConversationMismatch { expected, actual } => {
                write!(f, "message belongs to {actual}, not {expected}")
            }
        }
    }
}

impl std::error::Error for SocialModelError {}

#[derive(Debug, Clone, Default)]
pub struct SocialModel {
    people: BTreeMap<PersonId, Person>,
    projects: BTreeMap<ProjectId, Project>,
    conversations: BTreeMap<SocialConversationId, SocialConversation>,
    messages: BTreeMap<SocialConversationId, Vec<SocialMessage>>,
    drafts: BTreeMap<SocialConversationId, String>,
    selected: Option<SocialConversationId>,
    transport: SocialTransportAvailability,
}

impl SocialModel {
    pub fn production() -> Self {
        Self::default()
    }

    pub fn transport(&self) -> &SocialTransportAvailability {
        &self.transport
    }

    pub fn people(&self) -> impl ExactSizeIterator<Item = &Person> {
        self.people.values()
    }

    pub fn projects(&self) -> impl ExactSizeIterator<Item = &Project> {
        self.projects.values()
    }

    pub fn conversations(&self) -> impl ExactSizeIterator<Item = &SocialConversation> {
        self.conversations.values()
    }

    pub fn total_unread(&self) -> u32 {
        self.conversations
            .values()
            .map(|conversation| conversation.unread)
            .sum()
    }

    pub fn selected(&self) -> Option<&SocialConversation> {
        self.selected
            .as_ref()
            .and_then(|id| self.conversations.get(id))
    }

    pub fn messages_for(
        &self,
        id: &SocialConversationId,
    ) -> Result<&[SocialMessage], SocialModelError> {
        if !self.conversations.contains_key(id) {
            return Err(SocialModelError::UnknownConversation(id.clone()));
        }
        Ok(self.messages.get(id).map_or(&[], Vec::as_slice))
    }

    pub fn draft(&self, id: &SocialConversationId) -> Result<&str, SocialModelError> {
        if !self.conversations.contains_key(id) {
            return Err(SocialModelError::UnknownConversation(id.clone()));
        }
        Ok(self.drafts.get(id).map_or("", String::as_str))
    }

    pub fn insert_person(&mut self, person: Person) -> Result<(), SocialModelError> {
        let id = person.id.clone();
        if self.people.contains_key(&id) {
            return Err(SocialModelError::DuplicatePerson(id));
        }
        self.people.insert(id, person);
        Ok(())
    }

    pub fn insert_project(&mut self, project: Project) -> Result<(), SocialModelError> {
        let id = project.id.clone();
        if self.projects.contains_key(&id) {
            return Err(SocialModelError::DuplicateProject(id));
        }
        self.projects.insert(id, project);
        Ok(())
    }

    pub fn insert_conversation(
        &mut self,
        conversation: SocialConversation,
    ) -> Result<(), SocialModelError> {
        self.validate_thread(&conversation.kind)?;
        let id = conversation.id.clone();
        if self.conversations.contains_key(&id) {
            return Err(SocialModelError::DuplicateConversation(id));
        }
        self.conversations.insert(id.clone(), conversation);
        if self.selected.is_none() {
            self.selected = Some(id);
        }
        Ok(())
    }

    pub fn replace_messages(
        &mut self,
        id: &SocialConversationId,
        messages: Vec<SocialMessage>,
    ) -> Result<(), SocialModelError> {
        if !self.conversations.contains_key(id) {
            return Err(SocialModelError::UnknownConversation(id.clone()));
        }
        for message in &messages {
            if &message.conversation_id != id {
                return Err(SocialModelError::MessageConversationMismatch {
                    expected: id.clone(),
                    actual: message.conversation_id.clone(),
                });
            }
            if !self.people.contains_key(&message.author_id) {
                return Err(SocialModelError::UnknownPerson(message.author_id.clone()));
            }
        }
        self.messages.insert(id.clone(), messages);
        Ok(())
    }

    pub fn select(&mut self, id: &SocialConversationId) -> Result<(), SocialModelError> {
        if !self.conversations.contains_key(id) {
            return Err(SocialModelError::UnknownConversation(id.clone()));
        }
        self.selected = Some(id.clone());
        Ok(())
    }

    pub fn set_draft(
        &mut self,
        id: &SocialConversationId,
        draft: impl Into<String>,
    ) -> Result<(), SocialModelError> {
        if !self.conversations.contains_key(id) {
            return Err(SocialModelError::UnknownConversation(id.clone()));
        }
        self.drafts.insert(id.clone(), draft.into());
        Ok(())
    }

    fn validate_thread(&self, kind: &SocialThreadKind) -> Result<(), SocialModelError> {
        match kind {
            SocialThreadKind::Direct { person_id } if !self.people.contains_key(person_id) => {
                Err(SocialModelError::UnknownPerson(person_id.clone()))
            }
            SocialThreadKind::Project { project_id } if !self.projects.contains_key(project_id) => {
                Err(SocialModelError::UnknownProject(project_id.clone()))
            }
            SocialThreadKind::Direct { .. }
            | SocialThreadKind::Group
            | SocialThreadKind::Project { .. } => Ok(()),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn fixture_model() -> (SocialModel, SocialConversationId, SocialConversationId) {
        let mut model = SocialModel::production();
        let person_id = PersonId::new("maya").expect("person id");
        model
            .insert_person(Person {
                id: person_id.clone(),
                display_name: "Maya Chen".into(),
                handle: "@maya".into(),
                presence: Presence::InFlow,
                activity: Some("Library polish".into()),
            })
            .expect("person");
        let project_id = ProjectId::new("devpulse").expect("project id");
        model
            .insert_project(Project {
                id: project_id.clone(),
                name: "DevPULSE".into(),
                summary: "Native creative workspace".into(),
            })
            .expect("project");
        let direct_id = SocialConversationId::new("maya-dm").expect("direct id");
        model
            .insert_conversation(SocialConversation {
                id: direct_id.clone(),
                title: "Maya Chen".into(),
                kind: SocialThreadKind::Direct { person_id },
                unread: 2,
                pinned: true,
            })
            .expect("direct");
        let project_thread = SocialConversationId::new("devpulse-team").expect("project thread");
        model
            .insert_conversation(SocialConversation {
                id: project_thread.clone(),
                title: "# product-room".into(),
                kind: SocialThreadKind::Project { project_id },
                unread: 0,
                pinned: false,
            })
            .expect("project conversation");
        (model, direct_id, project_thread)
    }

    #[test]
    fn production_social_is_empty_and_truthfully_unavailable() {
        let model = SocialModel::production();
        assert_eq!(model.people().len(), 0);
        assert_eq!(model.conversations().len(), 0);
        assert!(!model.transport().can_send());
        assert!(model.transport().reason().contains("not qualified"));
    }

    #[test]
    fn direct_group_and_project_identities_remain_distinct() {
        let (model, direct, project) = fixture_model();
        assert_ne!(direct, project);
        assert_eq!(model.conversations().len(), 2);
        assert_eq!(model.total_unread(), 2);
        assert!(matches!(
            model.selected().map(|thread| &thread.kind),
            Some(SocialThreadKind::Direct { .. })
        ));
    }

    #[test]
    fn conversation_drafts_and_selection_are_isolated() {
        let (mut model, direct, project) = fixture_model();
        model.set_draft(&direct, "direct draft").expect("draft");
        model
            .set_draft(&project, "project draft")
            .expect("project draft");
        model.select(&project).expect("select");
        assert_eq!(model.draft(&direct).expect("direct draft"), "direct draft");
        assert_eq!(
            model.draft(&project).expect("project draft"),
            "project draft"
        );
        assert_eq!(model.selected().map(|thread| &thread.id), Some(&project));
    }

    #[test]
    fn messages_must_match_their_thread_and_known_author() {
        let (mut model, direct, project) = fixture_model();
        let error = model
            .replace_messages(
                &direct,
                vec![SocialMessage {
                    id: SocialMessageId::new("one").expect("message"),
                    conversation_id: project.clone(),
                    author_id: PersonId::new("maya").expect("person"),
                    body: "wrong thread".into(),
                    timestamp: "09:41".into(),
                }],
            )
            .expect_err("mismatch");
        assert!(matches!(
            error,
            SocialModelError::MessageConversationMismatch { .. }
        ));
    }
}
