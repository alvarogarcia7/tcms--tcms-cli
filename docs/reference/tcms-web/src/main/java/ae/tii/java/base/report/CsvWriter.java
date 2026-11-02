package ae.tii.java.base.report;

import java.nio.charset.StandardCharsets;
import java.util.List;

/**
 * Minimal RFC 4180 CSV writer for the report export (DECISIONS.md D-100): CRLF line ends, quoting
 * of fields with commas, quotes or line breaks, a UTF-8 byte-order mark so spreadsheets read the
 * encoding right, and protection against formula injection (a field starting with =, +, - or @ is
 * prefixed with an apostrophe). Pure.
 */
public final class CsvWriter {

  private static final String BOM = "﻿";

  private final StringBuilder out = new StringBuilder(BOM);

  public CsvWriter row(List<String> fields) {
    for (int i = 0; i < fields.size(); i++) {
      if (i > 0) {
        out.append(',');
      }
      out.append(escape(fields.get(i)));
    }
    out.append("\r\n");
    return this;
  }

  public String text() {
    return out.toString();
  }

  public byte[] bytes() {
    return out.toString().getBytes(StandardCharsets.UTF_8);
  }

  static String escape(String field) {
    if (field == null || field.isEmpty()) {
      return "";
    }
    String value = field;
    char first = value.charAt(0);
    if (first == '='
        || first == '+'
        || first == '-'
        || first == '@'
        || first == '\t'
        || first == '\r') {
      value = "'" + value;
    }
    boolean quote =
        value.indexOf(',') >= 0
            || value.indexOf('"') >= 0
            || value.indexOf('\n') >= 0
            || value.indexOf('\r') >= 0;
    if (!quote) {
      return value;
    }
    return '"' + value.replace("\"", "\"\"") + '"';
  }
}
