//! Turning machine ids into the operator's words.
//!
//! One home, because the lane has now twice paid for the same helper existing
//! in two places: `dpl_doc::enters` and `dpl_doc::quote_lines` exist precisely
//! so a surface cannot hand-list a vocabulary the validator owns, and then
//! `pretty` was copied between two censuses one layer up anyway.
//!
//! Nothing here decides WHAT a thing is called — the registries do that. This
//! only makes an id readable when the id is all there is.

/// A folder-or-registry id as a person would say it: `title-card` → "Title
/// card".
pub(crate) fn pretty(id: &str) -> String {
    let mut out = id.replace(['-', '_'], " ");
    if let Some(first) = out.get_mut(0..1) {
        first.make_ascii_uppercase();
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn an_id_becomes_a_sentence_not_a_slug() {
        assert_eq!(pretty("title-card"), "Title card");
        assert_eq!(pretty("how_ai_works"), "How ai works");
        assert_eq!(pretty(""), "");
    }
}
