//! Operator verb catalog introspection (wire names).

/// One catalog entry for `GET /verbs`.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub struct VerbInfo {
    pub name: &'static str,
    pub status: &'static str,
    pub target: &'static str,
}

/// Stable catalog listed by the control plane.
pub fn verb_catalog() -> &'static [VerbInfo] {
    &[
        VerbInfo {
            name: "inventory",
            status: "shipped",
            target: "desktop",
        },
        VerbInfo {
            name: "presence",
            status: "shipped",
            target: "app",
        },
        VerbInfo {
            name: "import",
            status: "shipped",
            target: "app",
        },
        VerbInfo {
            name: "resolve",
            status: "shipped",
            target: "fabric",
        },
        VerbInfo {
            name: "invoke",
            status: "shipped",
            target: "fabric",
        },
        VerbInfo {
            name: "click",
            status: "shipped",
            target: "fabric",
        },
        VerbInfo {
            name: "mount",
            status: "shipped",
            target: "fabric",
        },
        VerbInfo {
            name: "move",
            status: "shipped",
            target: "window",
        },
        VerbInfo {
            name: "resize",
            status: "shipped",
            target: "window",
        },
        VerbInfo {
            name: "focus",
            status: "shipped",
            target: "window",
        },
        VerbInfo {
            name: "observe",
            status: "shipped",
            target: "app",
        },
        VerbInfo {
            name: "open",
            status: "shipped",
            target: "app",
        },
        VerbInfo {
            name: "close",
            status: "shipped",
            target: "app",
        },
        VerbInfo {
            name: "type_text",
            status: "shipped",
            target: "app",
        },
        VerbInfo {
            name: "press_key",
            status: "shipped",
            target: "app",
        },
        VerbInfo {
            name: "select",
            status: "shipped",
            target: "app",
        },
        VerbInfo {
            name: "scroll",
            status: "shipped",
            target: "app",
        },
        VerbInfo {
            name: "drag",
            status: "planned",
            target: "fabric",
        },
        VerbInfo {
            name: "wait_until",
            status: "shipped",
            target: "predicate",
        },
        VerbInfo {
            name: "assert_state",
            status: "shipped",
            target: "app",
        },
    ]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn catalog_includes_window_verbs() {
        let names: Vec<_> = verb_catalog().iter().map(|v| v.name).collect();
        assert!(names.contains(&"move"));
        assert!(names.contains(&"resize"));
        assert!(names.contains(&"focus"));
        assert!(names.contains(&"observe"));
        assert!(verb_catalog()
            .iter()
            .any(|v| v.name == "move" && v.status == "shipped"));
    }
}
