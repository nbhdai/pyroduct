// Add this implementation block to error.rs, or append to your existing impl PyroError

use std::fmt;

use crate::{CapturedError, PyroError};

impl PyroError {
    /// Captures a serde_json error with a contextual snippet from the source data.
    ///
    /// This attempts to locate the line and column of the error, extracting a
    /// window of text around the failure point to assist in debugging.
    pub fn capture_json(err: serde_json::Error, data: &[u8]) -> Self {
        // Helper struct to wrap the formatted message so it can be passed to CapturedError
        #[derive(Debug)]
        struct JsonContextError(String);

        impl fmt::Display for JsonContextError {
            fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
                f.write_str(&self.0)
            }
        }

        impl std::error::Error for JsonContextError {}

        let context_msg = if let Ok(text) = std::str::from_utf8(data) {
            let line_num = err.line();
            let col_num = err.column();

            // serde_json reports lines 1-indexed.
            if line_num > 0 {
                // Find the specific line.
                // Note: If the file is huge (minified), accessing the line as a slice is cheap.
                if let Some(line_text) = text.lines().nth(line_num - 1) {
                    // Configuration for the snippet window
                    const CONTEXT_WINDOW: usize = 80;

                    // We iterate chars to handle UTF-8 correctly and handle windowing
                    // for very long lines (minified JSON).
                    let char_count = line_text.chars().count();

                    // Column is 1-indexed.
                    let error_idx = if col_num > 0 { col_num - 1 } else { 0 };

                    // Calculate start/end indices to center the window on the error
                    let (start, end, pointer_offset) = if char_count <= CONTEXT_WINDOW {
                        (0, char_count, error_idx)
                    } else {
                        let half = CONTEXT_WINDOW / 2;
                        // Try to center
                        let mut s = error_idx.saturating_sub(half);
                        let mut e = s + CONTEXT_WINDOW;

                        // Clamp to end
                        if e > char_count {
                            e = char_count;
                            s = e.saturating_sub(CONTEXT_WINDOW);
                        }
                        (s, e, error_idx - s)
                    };

                    // Extract the substring
                    let snippet: String = line_text
                        .chars()
                        .skip(start)
                        .take(end - start)
                        // Replace tabs with spaces to ensure the pointer caret aligns visually
                        .map(|c| if c == '\t' { ' ' } else { c })
                        .collect();

                    let pointer = " ".repeat(pointer_offset) + "^";

                    format!(
                        "\n\nContext (Line {}, Col {}):\n{}\n{}\n",
                        line_num, col_num, snippet, pointer
                    )
                } else {
                    String::from("\n(Line not found in data)")
                }
            } else {
                String::new()
            }
        } else {
            String::from("\n(Data is not valid UTF-8)")
        };

        let full_message = format!("JSON Error: {}{}", err, context_msg);

        // Wrap in the helper struct, then into CapturedError, then PyroError
        Self::deserialization(CapturedError::new(JsonContextError(full_message)))
    }
}
