use scraper::{Html, Selector};
use serde::{Deserialize, Serialize};
#[derive(Debug, Clone, Serialize, Deserialize)]
#[serde(rename_all = "camelCase")]
pub struct Cell {
    pub row: usize,
    pub column: usize,
    pub row_span: usize,
    pub column_span: usize,
    pub text: String,
}
fn escape(text: &str) -> String {
    text.replace('&', "&amp;")
        .replace('<', "&lt;")
        .replace('>', "&gt;")
        .replace('"', "&quot;")
}
const TOKENS: [&str; 6] = ["<fcel>", "<ecel>", "<lcel>", "<ucel>", "<xcel>", "<nl>"];
/// Decodes the model's native table serialization; no extra OCR or inferred geometry.
pub fn otsl_html(raw: &str) -> Result<String, String> {
    let mut rest = raw.trim();
    let mut rows: Vec<Vec<(&str, String)>> = vec![];
    let mut row = vec![];
    while !rest.is_empty() {
        let tag = TOKENS
            .iter()
            .find(|t| rest.starts_with(**t))
            .ok_or(crate::i18n::text("tableToken"))?;
        rest = &rest[tag.len()..];
        let end = TOKENS
            .iter()
            .filter_map(|t| rest.find(t))
            .min()
            .unwrap_or(rest.len());
        let value = rest[..end].trim().to_string();
        rest = &rest[end..];
        if *tag == "<nl>" {
            if !value.is_empty() {
                return Err(crate::i18n::text("tableTextOutside").into());
            }
            if !row.is_empty() {
                rows.push(std::mem::take(&mut row));
            }
        } else {
            if *tag != "<fcel>" && !value.is_empty() {
                return Err(crate::i18n::text("tableMergedText").into());
            }
            row.push((*tag, value));
        }
    }
    if !row.is_empty() {
        rows.push(row);
    }
    let width = rows
        .first()
        .map(Vec::len)
        .ok_or(crate::i18n::text("tableEmpty"))?;
    if width == 0 || rows.iter().any(|r| r.len() != width) || width * rows.len() > 10000 {
        return Err(crate::i18n::text("tableIncomplete").into());
    }
    let mut cells: Vec<Cell> = vec![];
    let mut owners = vec![vec![None; width]; rows.len()];
    for (r, row) in rows.iter().enumerate() {
        for (c, (tag, text)) in row.iter().enumerate() {
            let owner = match *tag {
                "<fcel>" | "<ecel>" => {
                    cells.push(Cell {
                        row: r,
                        column: c,
                        row_span: 1,
                        column_span: 1,
                        text: text.clone(),
                    });
                    cells.len() - 1
                }
                "<lcel>" => {
                    let owner = c
                        .checked_sub(1)
                        .and_then(|x| owners[r][x])
                        .ok_or(crate::i18n::text("tableHMerge"))?;
                    let cell: &mut Cell = &mut cells[owner];
                    if cell.row != r {
                        return Err(crate::i18n::text("tableHPosition").into());
                    }
                    cell.column_span = c - cell.column + 1;
                    owner
                }
                "<ucel>" => {
                    let owner = r
                        .checked_sub(1)
                        .and_then(|y| owners[y][c])
                        .ok_or(crate::i18n::text("tableVMerge"))?;
                    let cell: &mut Cell = &mut cells[owner];
                    if cell.column != c {
                        return Err(crate::i18n::text("tableVPosition").into());
                    }
                    cell.row_span = r - cell.row + 1;
                    owner
                }
                "<xcel>" => {
                    let left = c.checked_sub(1).and_then(|x| owners[r][x]);
                    let above = r.checked_sub(1).and_then(|y| owners[y][c]);
                    if left.is_none() || left != above {
                        return Err(crate::i18n::text("tableCrossMerge").into());
                    }
                    left.unwrap()
                }
                _ => unreachable!(),
            };
            owners[r][c] = Some(owner);
        }
    }
    for (index, cell) in cells.iter().enumerate() {
        for row in owners.iter().skip(cell.row).take(cell.row_span) {
            if row
                .iter()
                .skip(cell.column)
                .take(cell.column_span)
                .any(|x| *x != Some(index))
            {
                return Err(crate::i18n::text("tableOverlap").into());
            }
        }
    }
    let mut html = String::from("<table>\n");
    for r in 0..rows.len() {
        html.push_str("<tr>");
        for cell in cells.iter().filter(|cell| cell.row == r) {
            html.push_str(&format!(
                "<td rowspan=\"{}\" colspan=\"{}\">{}</td>",
                cell.row_span,
                cell.column_span,
                escape(&cell.text)
            ));
        }
        html.push_str("</tr>\n");
    }
    html.push_str("</table>");
    Ok(html)
}
pub fn normalize(raw: &str, mode: &str) -> (String, Option<String>) {
    if mode == "table" && TOKENS.iter().any(|token| raw.contains(token)) {
        match otsl_html(raw) {
            Ok(html) => (html, None),
            Err(e) => (format!("~~~text\n{raw}\n~~~"), Some(e)),
        }
    } else {
        (raw.to_string(), None)
    }
}
pub fn html_cells(html: &str) -> Option<(Vec<Vec<String>>, Vec<Cell>)> {
    let fragment = Html::parse_fragment(html);
    let table_selector = Selector::parse("table").unwrap();
    let table = fragment.select(&table_selector).next()?;
    let mut cells = vec![];
    let mut grid: Vec<Vec<String>> = vec![];
    let mut occupied = std::collections::HashSet::new();
    for (r, row) in table.select(&Selector::parse("tr").unwrap()).enumerate() {
        if r >= 1000 {
            return None;
        }
        while grid.len() <= r {
            grid.push(vec![]);
        }
        let mut c = 0;
        for element in row.select(&Selector::parse("td,th").unwrap()) {
            while occupied.contains(&(r, c)) {
                c += 1;
            }
            let span = |attr| {
                element
                    .value()
                    .attr(attr)
                    .and_then(|v| v.parse::<usize>().ok())
                    .unwrap_or(1)
                    .clamp(1, 100)
            };
            let rs = span("rowspan");
            let cs = span("colspan");
            if (r + rs) * (c + cs) > 10000 {
                return None;
            }
            let text = element.text().collect::<String>();
            for y in r..r + rs {
                while grid.len() <= y {
                    grid.push(vec![]);
                }
                if grid[y].len() < c + cs {
                    grid[y].resize(c + cs, String::new());
                }
                for x in c..c + cs {
                    occupied.insert((y, x));
                }
            }
            grid[r][c] = text.clone();
            cells.push(Cell {
                row: r,
                column: c,
                row_span: rs,
                column_span: cs,
                text,
            });
            c += cs;
        }
    }
    Some((grid, cells))
}
#[cfg(test)]
mod tests {
    use super::*;
    #[test]
    fn merged_cells_preserved_and_text_escaped() {
        let html = otsl_html("<fcel>A < B<lcel><fcel>C<nl><ucel><xcel><fcel>D<nl>").unwrap();
        let (rows, cells) = html_cells(&html).unwrap();
        assert_eq!(cells[0].column_span, 2);
        assert_eq!(cells[0].row_span, 2);
        assert_eq!(cells[0].text, "A < B");
        assert_eq!(rows[1][2], "D");
        assert!(html.contains("A &lt; B"));
    }
    #[test]
    fn malformed_table_keeps_raw_instead_of_inventing_cells() {
        let raw = "<fcel>A<fcel>B<nl><fcel>C<nl>";
        let (text, warning) = normalize(raw, "table");
        assert!(text.contains(raw));
        assert!(warning.is_some());
        assert!(otsl_html("<lcel><nl>").is_err());
    }
    #[test]
    fn missing_first_cell_marker_is_visible_and_warns() {
        let raw = "Item<fcel>Quantity<fcel>Amount<nl><fcel>Pen<fcel>3<fcel>3000<nl>";
        let (text, warning) = normalize(raw, "table");
        assert!(warning.is_some());
        assert!(text.starts_with("~~~text"));
        assert!(text.contains(raw));
    }
    #[test]
    fn empty_and_unicode_cells_are_preserved() {
        let html = otsl_html("<fcel>품명<fcel>금액<nl><ecel><fcel>15,000원<nl>").unwrap();
        let (rows, _) = html_cells(&html).unwrap();
        assert_eq!(rows[1], ["", "15,000원"]);
    }
}
