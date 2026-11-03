//! AsciiDoc rendering of the report blocks (`|===` tables with a header row).

use crate::report::Block;

pub fn render(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in blocks {
        match block {
            Block::Title(t) => out.push_str(&format!("= {t}\n\n")),
            Block::Heading(h) => out.push_str(&format!("== {h}\n\n")),
            Block::Text(t) => out.push_str(&format!("{t}\n\n")),
            Block::Table { header, rows } => {
                let cols = vec!["1"; header.len()].join(",");
                out.push_str(&format!("[cols=\"{cols}\",options=\"header\"]\n|===\n"));
                out.push_str(&row(header));
                for r in rows {
                    out.push_str(&row(r));
                }
                out.push_str("|===\n\n");
            }
        }
    }
    out
}

fn row(cells: &[String]) -> String {
    let escaped: Vec<String> = cells.iter().map(|c| escape(c)).collect();
    format!("| {}\n", escaped.join(" | "))
}

/// A pipe is escaped (`\|`) and a line break becomes a space, so a cell stays on one table row.
pub fn escape(cell: &str) -> String {
    cell.replace('|', "\\|")
        .replace("\r\n", " ")
        .replace(['\r', '\n'], " ")
}
