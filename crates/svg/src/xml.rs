//! Just enough XML for the icon subset: tags with quoted attributes,
//! self-closing or not; comments and prologs skipped. Anything beyond
//! that vocabulary is a named refusal, never a guess.

use crate::{err, SvgError};

#[derive(Debug)]
pub(crate) struct Element {
    pub name: String,
    pub attrs: Vec<(String, String)>,
}

impl Element {
    pub fn attr(&self, name: &str) -> Option<&str> {
        self.attrs
            .iter()
            .find(|(n, _)| n == name)
            .map(|(_, v)| v.as_str())
    }
}

/// Every element in document order. Closing tags are consumed and
/// discarded — the subset never nests meaningfully below the root.
pub(crate) fn elements(svg: &str) -> Result<Vec<Element>, SvgError> {
    let b = svg.as_bytes();
    let mut out = Vec::new();
    let mut i = 0;
    while i < b.len() {
        if b[i] != b'<' {
            i += 1;
            continue;
        }
        let rest = &svg[i..];
        if let Some(skip) = prolog_len(rest)? {
            i += skip;
            continue;
        }
        if rest.starts_with("</") {
            match rest.find('>') {
                Some(p) => i += p + 1,
                None => return Err(err("unterminated closing tag")),
            }
            continue;
        }
        let (el, used) = element(rest)?;
        out.push(el);
        i += used;
    }
    Ok(out)
}

/// Comments, `<?…?>`, and `<!…>` prologs: skipped whole, or an error if
/// unterminated. `None` means the cursor sits on a real tag.
fn prolog_len(rest: &str) -> Result<Option<usize>, SvgError> {
    if rest.starts_with("<!--") {
        return match rest.find("-->") {
            Some(p) => Ok(Some(p + 3)),
            None => Err(err("unterminated comment")),
        };
    }
    if rest.starts_with("<?") || rest.starts_with("<!") {
        return match rest.find('>') {
            Some(p) => Ok(Some(p + 1)),
            None => Err(err("unterminated prolog")),
        };
    }
    Ok(None)
}

/// One `<name attr="v" …>` or `<name …/>`; returns bytes consumed.
fn element(rest: &str) -> Result<(Element, usize), SvgError> {
    let b = rest.as_bytes();
    let mut i = 1; // past '<'
    let start = i;
    while i < b.len() && (b[i].is_ascii_alphanumeric() || b[i] == b'-') {
        i += 1;
    }
    let name = rest[start..i].to_string();
    if name.is_empty() {
        return Err(err("empty tag name"));
    }
    let mut attrs = Vec::new();
    loop {
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        match b.get(i) {
            None => return Err(err(format!("unterminated <{name}>"))),
            Some(b'>') => return Ok((Element { name, attrs }, i + 1)),
            Some(b'/') => {
                return if b.get(i + 1) == Some(&b'>') {
                    Ok((Element { name, attrs }, i + 2))
                } else {
                    Err(err(format!("stray '/' in <{name}>")))
                };
            }
            Some(_) => {}
        }
        let a0 = i;
        while i < b.len() && b[i] != b'=' && b[i] != b'>' && !b[i].is_ascii_whitespace() {
            i += 1;
        }
        let attr_name = rest[a0..i].to_string();
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        if b.get(i) != Some(&b'=') {
            return Err(err(format!(
                "<{name}>: attribute {attr_name:?} has no value"
            )));
        }
        i += 1;
        while i < b.len() && b[i].is_ascii_whitespace() {
            i += 1;
        }
        let quote = match b.get(i) {
            Some(&q) if q == b'"' || q == b'\'' => q,
            _ => {
                return Err(err(format!("<{name}>: {attr_name:?} value is unquoted")));
            }
        };
        i += 1;
        let v0 = i;
        while i < b.len() && b[i] != quote {
            i += 1;
        }
        if i >= b.len() {
            return Err(err(format!(
                "<{name}>: {attr_name:?} value is unterminated"
            )));
        }
        attrs.push((attr_name, rest[v0..i].to_string()));
        i += 1;
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn attributes_split_across_newlines_parse() {
        let svg = "<svg\n  width=\"24\"\n  viewBox=\"0 0 24 24\"\n>\n  <path d=\"M1 1\" />\n</svg>";
        let els = elements(svg).expect("parse");
        assert_eq!(els.len(), 2);
        assert_eq!(els[0].name, "svg");
        assert_eq!(els[0].attr("viewBox"), Some("0 0 24 24"));
        assert_eq!(els[1].name, "path");
        assert_eq!(els[1].attr("d"), Some("M1 1"));
    }

    #[test]
    fn comments_and_prologs_are_skipped() {
        let svg = "<?xml version=\"1.0\"?><!-- hi --><svg viewBox=\"0 0 1 1\"></svg>";
        let els = elements(svg).expect("parse");
        assert_eq!(els.len(), 1);
        assert_eq!(els[0].name, "svg");
    }

    #[test]
    fn an_unquoted_value_is_a_named_refusal() {
        let e = elements("<svg width=24></svg>").expect_err("must refuse");
        assert!(e.0.contains("width"), "{e}");
    }
}
