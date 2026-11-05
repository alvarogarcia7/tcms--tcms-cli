//! Markdown rendering of the report blocks (GitHub-flavoured pipe tables).

use crate::report::{escape_markdown_cell, Block};

pub fn render(blocks: &[Block]) -> String {
    let mut out = String::new();
    for block in blocks {
        match block {
            Block::Title(t) => out.push_str(&format!("# {t}\n\n")),
            Block::Heading(h) => out.push_str(&format!("## {h}\n\n")),
            Block::Text(t) => out.push_str(&format!("{t}\n\n")),
            Block::Table { header, rows } => {
                out.push_str(&row(header));
                out.push_str(&format!("|{}\n", " --- |".repeat(header.len())));
                for r in rows {
                    out.push_str(&row(r));
                }
                out.push('\n');
            }
        }
    }
    out
}

fn row(cells: &[String]) -> String {
    let escaped: Vec<String> = cells.iter().map(|c| escape_markdown_cell(c)).collect();
    format!("| {} |\n", escaped.join(" | "))
}
