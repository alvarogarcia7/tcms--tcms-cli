use regex::Regex;
use std::path::Path;
use testcase_models::TestExecutionLog;

pub struct LogCleaner {
    timestamp_regex: Regex,
    unix_timestamp_regex: Regex,
    relative_time_regex: Regex,
    ansi_code_regex: Regex,
    absolute_path_regex: Regex,
}

impl LogCleaner {
    pub fn new() -> Self {
        Self {
            timestamp_regex: Regex::new(
                r"\d{4}-\d{2}-\d{2}T\d{2}:\d{2}:\d{2}(?:\.\d+)?(?:Z|[+-]\d{2}:\d{2})",
            )
            .unwrap(),
            unix_timestamp_regex: Regex::new(r"\b\d{10,13}\b").unwrap(),
            relative_time_regex: Regex::new(
                r"\b\d+\s*(?:second|sec|minute|min|hour|hr|day|week|month|year)s?\s+ago\b",
            )
            .unwrap(),
            ansi_code_regex: Regex::new(r"(?:\x1b\[[0-9;]*[a-zA-Z]|\[[0-9;]*m)").unwrap(),
            absolute_path_regex: Regex::new(r#"(?:^|[\s'"(])((?:[A-Za-z]:[/\\]|/)[\w\-./\\]+)"#)
                .unwrap(),
        }
    }

    pub fn remove_timestamps(&self, text: &str) -> String {
        let text = self.timestamp_regex.replace_all(text, "[TIMESTAMP]");
        let text = self.unix_timestamp_regex.replace_all(&text, "[TIMESTAMP]");
        let text = self
            .relative_time_regex
            .replace_all(&text, "[RELATIVE_TIME]");
        text.to_string()
    }

    pub fn remove_ansi_codes(&self, text: &str) -> String {
        self.ansi_code_regex.replace_all(text, "").to_string()
    }

    pub fn normalize_paths(&self, text: &str) -> String {
        self.absolute_path_regex
            .replace_all(text, |caps: &regex::Captures| {
                let path_str = &caps[1];
                let path = Path::new(path_str);

                let normalized = if let Ok(current_dir) = std::env::current_dir() {
                    if let Ok(rel_path) = path.strip_prefix(&current_dir) {
                        rel_path.to_string_lossy().to_string()
                    } else {
                        path.file_name()
                            .map(|f| f.to_string_lossy().to_string())
                            .unwrap_or_else(|| "[PATH]".to_string())
                    }
                } else {
                    path.file_name()
                        .map(|f| f.to_string_lossy().to_string())
                        .unwrap_or_else(|| "[PATH]".to_string())
                };

                format!("{}{}", caps[0].chars().next().unwrap_or(' '), normalized)
            })
            .to_string()
    }

    pub fn normalize_whitespace(&self, text: &str) -> String {
        let multiple_spaces = Regex::new(r" {2,}").unwrap();
        let multiple_newlines = Regex::new(
            r#"
{3,}"#,
        )
        .unwrap();

        let text = multiple_spaces.replace_all(text, " ");
        let text = multiple_newlines.replace_all(&text, "\n\n");

        // Trim each line individually to remove leading/trailing spaces per line
        let lines: Vec<&str> = text.lines().collect();
        let trimmed_lines: Vec<String> = lines.iter().map(|line| line.trim().to_string()).collect();

        trimmed_lines.join("\n").trim().to_string()
    }

    pub fn process_backspaces(&self, text: &str) -> String {
        let mut result = String::new();

        for ch in text.chars() {
            if ch == '\x08' || ch == '\x7f' {
                result.pop();
            } else {
                result.push(ch);
            }
        }

        result
    }

    pub fn remove_control_characters(&self, text: &str) -> String {
        text.chars()
            .filter(|&ch| {
                let code = ch as u32;
                !((code < 32 && ch != '\n' && ch != '\r' && ch != '\t') || code == 127)
            })
            .collect()
    }

    pub fn clean_script_capture(&self, text: &str) -> String {
        // First process backspaces before stripping ANSI codes
        // because strip_ansi_escapes might remove backspace characters
        let text = self.process_backspaces(text);

        // Then strip ANSI codes
        let bytes = text.as_bytes();
        let stripped = strip_ansi_escapes::strip(bytes);
        let text = String::from_utf8_lossy(&stripped).to_string();

        // Finally remove remaining control characters
        self.remove_control_characters(&text)
    }

    pub fn clean_execution_log(&self, log: &TestExecutionLog) -> TestExecutionLog {
        let mut cleaned_log = log.clone();

        cleaned_log.timestamp = self.remove_timestamps(&cleaned_log.timestamp);

        let mut cleaned_output = cleaned_log.actual_output.clone();
        cleaned_output = self.remove_ansi_codes(&cleaned_output);
        cleaned_output = self.remove_timestamps(&cleaned_output);
        cleaned_output = self.normalize_paths(&cleaned_output);
        cleaned_output = self.normalize_whitespace(&cleaned_output);
        cleaned_log.actual_output = cleaned_output;

        if let Some(ref error_msg) = cleaned_log.error_message {
            let mut cleaned_error = error_msg.clone();
            cleaned_error = self.remove_ansi_codes(&cleaned_error);
            cleaned_error = self.remove_timestamps(&cleaned_error);
            cleaned_error = self.normalize_paths(&cleaned_error);
            cleaned_error = self.normalize_whitespace(&cleaned_error);
            cleaned_log.error_message = Some(cleaned_error);
        }

        cleaned_log
    }
}

impl Default for LogCleaner {
    fn default() -> Self {
        Self::new()
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_remove_timestamps_rfc3339() {
        let cleaner = LogCleaner::new();
        let text = "Event at 2024-01-15T10:30:45Z and 2024-01-15T10:30:45.123+00:00";
        let result = cleaner.remove_timestamps(text);
        assert_eq!(result, "Event at [TIMESTAMP] and [TIMESTAMP]");
    }

    #[test]
    fn test_remove_timestamps_unix() {
        let cleaner = LogCleaner::new();
        let text = "Unix timestamp: 1705315845 and milliseconds: 1705315845123";
        let result = cleaner.remove_timestamps(text);
        assert_eq!(
            result,
            "Unix timestamp: [TIMESTAMP] and milliseconds: [TIMESTAMP]"
        );
    }

    #[test]
    fn test_remove_timestamps_relative() {
        let cleaner = LogCleaner::new();
        let text = r"Updated 5 minutes ago and 2 hours ago";
        let result = cleaner.remove_timestamps(text);
        assert_eq!(result, "Updated [RELATIVE_TIME] and [RELATIVE_TIME]");
    }

    #[test]
    fn test_remove_ansi_codes() {
        let cleaner = LogCleaner::new();
        let text = "\x1b[31mError\x1b[0m and \x1b[1;32mSuccess\x1b[0m";
        let result = cleaner.remove_ansi_codes(text);
        assert_eq!(result, "Error and Success");
    }

    #[test]
    fn test_normalize_paths() {
        let cleaner = LogCleaner::new();
        let text = r"File at /home/user/project/file.txt and C:/Users/test/file.txt";
        let result = cleaner.normalize_paths(text);
        assert!(result.contains("file.txt"));
    }

    #[test]
    fn test_normalize_whitespace() {
        let cleaner = LogCleaner::new();
        let text = "Line1    with   spaces\n\n\n\n\nLine2";
        let result = cleaner.normalize_whitespace(text);
        assert_eq!(result, "Line1 with spaces\n\nLine2");
    }

    #[test]
    fn test_clean_execution_log() {
        let cleaner = LogCleaner::new();
        let log = TestExecutionLog {
            test_case_id: "TC001".to_string(),
            sequence_id: 1,
            timestamp: "2024-01-15T10:30:45Z".to_string(),
            actual_output: "\x1b[31mError\x1b[0m at 2024-01-15T10:30:45Z in /tmp/file.txt"
                .to_string(),
            actual_success: false,
            duration_ms: 1000,
            error_message: Some("Failed at 2024-01-15T10:30:45Z".to_string()),
        };

        let cleaned = cleaner.clean_execution_log(&log);
        assert_eq!(cleaned.timestamp, "[TIMESTAMP]");
        assert_eq!(cleaned.actual_output, "Error at [TIMESTAMP] in file.txt");
        assert_eq!(
            cleaned.error_message,
            Some("Failed at [TIMESTAMP]".to_string())
        );
    }
}
