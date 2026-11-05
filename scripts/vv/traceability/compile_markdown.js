// Compiles Markdown documents with markdown-it (CommonMark + GFM tables) and checks the result.
// A document fails when:
//   - it does not parse;
//   - a line that looks like a table row is not part of a table token (a broken table renders as a paragraph);
//   - the rows of one table do not all have the same number of cells as its header.
// Usage: node compile_markdown.js <markdown-it module dir> <file.md>...
const path = require('path');
const fs = require('fs');
const MarkdownIt = require(path.join(process.argv[2], 'markdown-it'));

const md = new MarkdownIt({ html: false, linkify: false });

function tables(tokens) {
  // Returns one array of row widths per table token (header row first).
  const out = [];
  let current = null;
  let cells = 0;
  for (const t of tokens) {
    if (t.type === 'table_open') current = [];
    else if (t.type === 'table_close') { out.push(current); current = null; }
    else if (t.type === 'tr_open' && current) cells = 0;
    else if (t.type === 'tr_close' && current) current.push(cells);
    else if ((t.type === 'th_open' || t.type === 'td_open') && current) cells++;
  }
  return out;
}

function check(file) {
  const problems = [];
  const src = fs.readFileSync(file, 'utf8');
  let tokens;
  try {
    tokens = md.parse(src, {});
  } catch (e) {
    return [`parse error: ${e.message}`];
  }
  // Table lines of the source that are not delimiter rows (|---|---|) must all be rows of a table token.
  const isDelimiter = (l) => /^\s*\|?\s*:?-{3,}:?\s*(\|\s*:?-{3,}:?\s*)*\|?\s*$/.test(l);
  const rowLines = src.split('\n').filter((l) => /^\s*\|.*\|\s*$/.test(l) && !isDelimiter(l)).length;
  const found = tables(tokens);
  const rows = found.reduce((n, widths) => n + widths.length, 0);
  if (rowLines !== rows) problems.push(`${rowLines - rows} table row line(s) rendered outside a table`);
  // GFM drops the cells of a row beyond the header's count, so the parsed tokens cannot show them: count the
  // cells of each source table block (unescaped pipes) and flag every row wider than its header.
  const cellsOf = (l) => {
    const t = l.trim().replace(/^\|/, '').replace(/\|$/, '');
    let n = 1, esc = false;
    for (const c of t) { if (esc) esc = false; else if (c === '\\') esc = true; else if (c === '|') n++; }
    return n;
  };
  let block = [];
  const flush = () => {
    if (block.length > 1) {
      const header = cellsOf(block[0]);
      block.slice(2).forEach((l, r) => {
        if (cellsOf(l) > header) problems.push(`table row ${r + 1} has ${cellsOf(l)} cells, header has ${header} (excess cells are dropped): ${l.trim().slice(0, 80)}`);
      });
    }
    block = [];
  };
  for (const l of src.split('\n')) {
    if (/^\s*\|.*\|\s*$/.test(l)) block.push(l); else flush();
  }
  flush();
  return problems;
}

let failures = 0;
const files = process.argv.slice(3);
for (const file of files) {
  const problems = check(file);
  if (problems.length) {
    failures++;
    console.log(`FAIL ${file}: ${problems.join('; ')}`);
  } else {
    console.log(`ok   ${file}`);
  }
}
console.log(`${files.length} compiled, ${failures} failed`);
process.exit(failures ? 1 : 0);
