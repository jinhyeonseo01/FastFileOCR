use pulldown_cmark::{Event, Options, Parser, Tag};
use serde::{Deserialize, Serialize};

#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Block {
    pub kind: String,
    pub text: String,
    pub markdown: String,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub level: Option<u8>,
    #[serde(default, skip_serializing_if = "Option::is_none")]
    pub cells: Option<Vec<crate::table::Cell>>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub rows: Option<Vec<Vec<String>>>,
}

pub fn blocks(markdown: &str) -> Vec<Block> {
    let mut result = Vec::new();
    let mut depth = 0usize;
    let mut start = 0usize;
    let mut kind = String::new();
    let mut level = None;
    let mut text = String::new();
    let mut rows: Vec<Vec<String>> = Vec::new();
    let mut row: Vec<String> = Vec::new();
    let mut cell = String::new();
    let mut in_cell = false;
    for (event, range) in Parser::new_ext(
        markdown,
        Options::ENABLE_TABLES | Options::ENABLE_STRIKETHROUGH,
    )
    .into_offset_iter()
    {
        match event {
            Event::Start(tag) => {
                if depth == 0 {
                    start = range.start;
                    kind = match &tag {
                        Tag::Heading { .. } => "heading",
                        Tag::Table(_) => "table",
                        Tag::List(_) => "list",
                        Tag::CodeBlock(_) => "code",
                        Tag::BlockQuote(_) => "quote",
                        Tag::HtmlBlock => "html",
                        _ => "paragraph",
                    }
                    .into();
                    level = if let Tag::Heading { level, .. } = &tag {
                        Some(*level as u8)
                    } else {
                        None
                    };
                    text.clear();
                    rows.clear();
                }
                if matches!(tag, Tag::TableRow | Tag::TableHead) {
                    row.clear();
                }
                if matches!(tag, Tag::TableCell) {
                    in_cell = true;
                    cell.clear();
                }
                depth += 1;
            }
            Event::End(tag) => {
                use pulldown_cmark::TagEnd;
                if matches!(tag, TagEnd::TableCell) {
                    row.push(cell.trim().to_string());
                    in_cell = false;
                    text.push('\t');
                }
                if matches!(tag, TagEnd::TableRow | TagEnd::TableHead) {
                    rows.push(row.clone());
                    text.push('\n');
                }
                if matches!(tag, TagEnd::Item | TagEnd::Paragraph) {
                    text.push('\n');
                }
                depth = depth.saturating_sub(1);
                if depth == 0 {
                    result.push(Block {
                        kind: kind.clone(),
                        text: text.trim().into(),
                        markdown: markdown[start..range.end].into(),
                        level,
                        cells: None,
                        rows: if kind == "table" {
                            Some(rows.clone())
                        } else {
                            None
                        },
                    });
                }
            }
            Event::Text(value)
            | Event::Code(value)
            | Event::InlineMath(value)
            | Event::DisplayMath(value) => {
                text.push_str(&value);
                if in_cell {
                    cell.push_str(&value);
                }
            }
            Event::Html(value) | Event::InlineHtml(value) => {
                text.push_str(&value);
                if depth == 0 {
                    result.push(Block {
                        kind: "html".into(),
                        text: value.to_string(),
                        markdown: value.to_string(),
                        level: None,
                        cells: None,
                        rows: None,
                    });
                    text.clear();
                }
            }
            Event::SoftBreak | Event::HardBreak => {
                text.push('\n');
                if in_cell {
                    cell.push(' ');
                }
            }
            Event::Rule if depth == 0 => result.push(Block {
                kind: "separator".into(),
                text: String::new(),
                markdown: "---".into(),
                level: None,
                cells: None,
                rows: None,
            }),
            _ => {}
        }
    }
    for block in &mut result {
        if block.kind == "html" {
            if let Some((rows, cells)) = crate::table::html_cells(&block.markdown) {
                block.kind = "table".into();
                block.text = rows
                    .iter()
                    .map(|r| r.join("\t"))
                    .collect::<Vec<_>>()
                    .join("\n");
                block.rows = Some(rows);
                block.cells = Some(cells);
            }
        }
    }
    result
}

pub fn plain_text(markdown: &str) -> String {
    blocks(markdown)
        .iter()
        .map(|b| b.text.as_str())
        .collect::<Vec<_>>()
        .join("\n\n")
}
pub fn safe_html(markdown: &str) -> String {
    let mut html = String::new();
    pulldown_cmark::html::push_html(&mut html, Parser::new_ext(markdown, Options::ENABLE_TABLES));
    ammonia::Builder::default()
        .rm_tags(&["img", "video", "audio", "iframe", "svg", "math"])
        .add_tag_attributes("td", &["rowspan", "colspan"])
        .add_tag_attributes("th", &["rowspan", "colspan"])
        .clean(&html)
        .to_string()
}

#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn preserves_korean_table_and_order() {
        let b = blocks("# 견적서\n\n고객: 김진\n\n| 품목 | 금액 |\n| --- | --- |\n| 책 | 12,000 |\n\n- 첫째\n- 둘째\n");
        assert_eq!(
            b.iter().map(|x| x.kind.as_str()).collect::<Vec<_>>(),
            ["heading", "paragraph", "table", "list"]
        );
        assert_eq!(b[0].level, Some(1));
        assert_eq!(b[2].rows.as_ref().unwrap()[1], ["책", "12,000"]);
    }
    #[test]
    fn code_symbols_and_jamo_are_not_decoded() {
        assert_eq!(
            plain_text("```js\nconst ㄱ = 'ㅅㅁ';\n```"),
            "const ㄱ = 'ㅅㅁ';"
        );
    }
    #[test]
    fn untrusted_html_cannot_execute_or_load_images() {
        let html = safe_html("<script>alert(1)</script><img src='https://example.com/x'><p onclick='evil()'>hello</p>");
        assert!(!html.contains("<script"));
        assert!(!html.contains("<img"));
        assert!(!html.contains("onclick"));
    }
}
