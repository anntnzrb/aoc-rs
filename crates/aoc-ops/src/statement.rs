//! Plain-text extraction of puzzle articles and submission responses from `AoC` HTML.

use crate::error::OpsError;

/// One HTML tag found by [`next_tag`].
struct Tag<'a> {
    /// Byte offset of `<`.
    start: usize,
    /// Byte offset just past `>`.
    end: usize,
    /// Lowercase element name.
    name: String,
    closing: bool,
    /// Raw attribute text between the name and `>`.
    attributes: &'a str,
}

/// Finds the next tag at or after `from`. As in HTML, `<` opens a tag only before a letter, `/`
/// or `!`; any other `<` is text. Text without a closing `>` holds no tag.
fn next_tag(html: &str, from: usize) -> Option<Tag<'_>> {
    let rest = html.get(from..)?;
    let open = rest.match_indices('<').find_map(|(index, _)| {
        let follower = rest.get(index.checked_add(1)?..)?.chars().next()?;
        (follower.is_ascii_alphabetic() || matches!(follower, '/' | '!')).then_some(index)
    })?;
    let start = from.checked_add(open)?;
    let body_start = start.checked_add(1)?;
    let close = html.get(body_start..)?.find('>')?;
    let body = html.get(body_start..body_start.checked_add(close)?)?;
    let end = body_start.checked_add(close)?.checked_add(1)?;
    let (closing, body) = body
        .strip_prefix('/')
        .map_or((false, body), |inner| (true, inner));
    let name_length = body
        .find(|c: char| !c.is_ascii_alphanumeric())
        .unwrap_or(body.len());
    let name = body.get(..name_length)?.to_ascii_lowercase();
    let attributes = body.get(name_length..)?;
    Some(Tag {
        start,
        end,
        name,
        closing,
        attributes,
    })
}

/// Iterates over every tag in `html`, in document order.
fn tags(html: &str) -> impl Iterator<Item = Tag<'_>> {
    std::iter::successors(next_tag(html, 0), move |tag| next_tag(html, tag.end))
}

/// Returns the inner HTML of each `name` element whose opening tag satisfies `keep`.
fn elements<'a>(html: &'a str, name: &str, keep: impl Fn(&str) -> bool) -> Vec<&'a str> {
    let mut found = Vec::new();
    let mut open: Option<usize> = None;
    for tag in tags(html).filter(|tag| tag.name == name) {
        match (tag.closing, open) {
            (false, None) if keep(tag.attributes) => open = Some(tag.end),
            (true, Some(start)) => {
                if let Some(inner) = html.get(start..tag.start) {
                    found.push(inner);
                }
                open = None;
            }
            (false, None | Some(_)) | (true, None) => {}
        }
    }
    found
}

/// Whether a tag's attributes list `day-desc` among its classes.
fn has_day_desc_class(attributes: &str) -> bool {
    attributes.split_once("class=").is_some_and(|(_, value)| {
        value
            .trim_start_matches(['"', '\''])
            .split(['"', '\''])
            .next()
            .unwrap_or_default()
            .split_ascii_whitespace()
            .any(|class| class == "day-desc")
    })
}

/// Decodes the named and numeric character references `AoC` pages use.
fn decode_entities(text: &str) -> String {
    let mut decoded = String::with_capacity(text.len());
    let mut rest = text;
    while let Some(amp) = rest.find('&') {
        decoded.push_str(rest.get(..amp).unwrap_or_default());
        let tail = rest.get(amp..).unwrap_or_default();
        let entity = tail
            .find(';')
            .filter(|&semi| semi <= 10)
            .and_then(|semi| Some((tail.get(1..semi)?, semi)));
        let replacement = entity.and_then(|(name, semi)| {
            let character = match name {
                "amp" => Some('&'),
                "lt" => Some('<'),
                "gt" => Some('>'),
                "quot" => Some('"'),
                "apos" => Some('\''),
                "nbsp" => Some('\u{a0}'),
                numeric => numeric
                    .strip_prefix("#x")
                    .or_else(|| numeric.strip_prefix("#X"))
                    .and_then(|hex| u32::from_str_radix(hex, 16).ok())
                    .or_else(|| numeric.strip_prefix('#').and_then(|dec| dec.parse().ok()))
                    .and_then(char::from_u32),
            }?;
            Some((character, semi))
        });
        if let Some((character, semi)) = replacement {
            decoded.push(character);
            rest = tail.get(semi.saturating_add(1)..).unwrap_or_default();
        } else {
            decoded.push('&');
            rest = tail.get(1..).unwrap_or_default();
        }
    }
    decoded.push_str(rest);
    decoded
}

/// Strips tags, keeps block breaks and list markers, decodes entities and ends with one LF.
pub(crate) fn plain_text(html: &str) -> String {
    let mut text = String::with_capacity(html.len());
    let mut cursor = 0;
    for tag in tags(html) {
        text.push_str(&decode_entities(
            html.get(cursor..tag.start).unwrap_or_default(),
        ));
        match (tag.name.as_str(), tag.closing) {
            ("li", false) => text.push_str("\n- "),
            ("p" | "pre" | "li" | "h2" | "br" | "ul", _) => text.push('\n'),
            _ => {}
        }
        cursor = tag.end;
    }
    text.push_str(&decode_entities(html.get(cursor..).unwrap_or_default()));
    let text = text.replace("\r\n", "\n").replace('\r', "\n");
    let mut collapsed = String::with_capacity(text.len());
    let mut newlines = 0_u8;
    for character in text.chars() {
        if character == '\n' {
            newlines = newlines.saturating_add(1);
            if newlines > 2 {
                continue;
            }
        } else {
            newlines = 0;
        }
        collapsed.push(character);
    }
    format!("{}\n", collapsed.trim())
}

/// Drops `<p>` paragraphs that disclose an already accepted answer.
fn without_past_answers(article: &str) -> String {
    let mut kept = String::with_capacity(article.len());
    let mut cursor = 0;
    let mut paragraph: Option<usize> = None;
    for tag in tags(article).filter(|tag| tag.name == "p") {
        match (tag.closing, paragraph) {
            (false, None) => paragraph = Some(tag.start),
            (true, Some(start)) => {
                let block = article.get(start..tag.end).unwrap_or_default();
                if plain_text(block).starts_with("Your puzzle answer was") {
                    kept.push_str(article.get(cursor..start).unwrap_or_default());
                    cursor = tag.end;
                }
                paragraph = None;
            }
            (false, Some(_)) | (true, None) => {}
        }
    }
    kept.push_str(article.get(cursor..).unwrap_or_default());
    kept
}

/// Keeps only `day-desc` articles as plain text, removing past-answer paragraphs.
pub(crate) fn extract(html: &str) -> Result<String, OpsError> {
    let articles: Vec<String> = elements(html, "article", has_day_desc_class)
        .into_iter()
        .map(|article| {
            plain_text(&without_past_answers(article))
                .trim_end()
                .to_owned()
        })
        .collect();
    if articles.is_empty() {
        return Err(OpsError::Html("no puzzle articles found"));
    }
    Ok(format!("{}\n", articles.join("\n\n")))
}

/// Returns the plain text of the first article inside `<main>`.
pub(crate) fn submission(html: &str) -> Result<String, OpsError> {
    elements(html, "main", |_| true)
        .first()
        .and_then(|main| elements(main, "article", |_| true).first().copied())
        .map(plain_text)
        .ok_or(OpsError::Html("no submission article found"))
}

#[cfg(test)]
mod tests {
    use super::{extract, plain_text, submission};

    #[test]
    fn plain_text_keeps_structure_and_decodes_entities() {
        let html = "<h2>--- T ---</h2><p>a &lt;b&gt; &amp; <em>c</em>&#33;&#x41;</p>\
                    <ul><li>one</li><li>two</li></ul><pre><code>x\r\ny</code></pre>&bogus; & z";
        assert_eq!(
            plain_text(html),
            "--- T ---\n\na <b> & c!A\n\n- one\n\n- two\n\nx\ny\n&bogus; & z\n",
            "tags become breaks, entities decode, runs of blank lines collapse"
        );
    }

    #[test]
    fn raw_angle_brackets_outside_tags_stay_text() {
        let html = "<pre><code>p=< 9,  1> v=<-1, 0>\na < b > c</code></pre>";
        assert_eq!(
            plain_text(html),
            "p=< 9,  1> v=<-1, 0>\na < b > c\n",
            "a `<` not followed by a letter, `/` or `!` opens no tag"
        );
    }

    #[test]
    fn extract_keeps_only_day_desc_articles_without_answers() {
        let html = "<main><article class=\"day-desc\"><p>Part one.</p></article>\
                    <p>Your puzzle answer was <code>42</code>.</p>\
                    <article class=\"other\"><p>skip</p></article>\
                    <article class=\"day-desc extra\"><p>Part two.</p>\
                    <p>Your puzzle answer was <code>7</code>.</p></article></main>";
        assert_eq!(
            extract(html).unwrap(),
            "Part one.\n\nPart two.\n",
            "answers and foreign articles are removed"
        );
    }

    #[test]
    fn extract_requires_an_article() {
        assert!(extract("<main><p>login</p></main>").is_err(), "no article");
        assert!(
            extract("<article class=\"day-desc\">open").is_err(),
            "an unclosed article is not extracted"
        );
    }

    #[test]
    fn submission_reads_the_article_inside_main() {
        let html = "<article>outside</article><main>\n<article><p>That's the \
                    <span>right answer</span>!</p></article></main>";
        assert_eq!(
            submission(html).unwrap(),
            "That's the right answer!\n",
            "the main article is used"
        );
        assert!(submission("<main><p>x</p></main>").is_err(), "no article");
        assert!(submission("<article>x</article>").is_err(), "no main");
    }
}
