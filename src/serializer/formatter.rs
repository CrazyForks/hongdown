//! External code formatter execution logic.
//!
//! This module provides functionality for running external code formatters
//! on code block contents. It handles process spawning, timeout management,
//! and error handling.

use std::io::Write;
use std::process::{Command, Stdio};
use std::thread;
use std::time::{Duration, Instant};

/// Error types for formatter execution.
#[derive(Debug)]
pub enum FormatterError {
    /// Command array is empty.
    EmptyCommand,
    /// Failed to spawn the process.
    Spawn(std::io::Error),
    /// Failed to write to stdin.
    Stdin(std::io::Error),
    /// Process timed out.
    Timeout,
    /// Process exited with non-zero status.
    NonZeroExit {
        /// Exit code, if available.
        code: Option<i32>,
        /// Stderr output from the process.
        stderr: String,
    },
    /// Output is not valid UTF-8.
    InvalidUtf8(std::string::FromUtf8Error),
}

impl std::fmt::Display for FormatterError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            FormatterError::EmptyCommand => write!(f, "command is empty"),
            FormatterError::Spawn(e) => write!(f, "failed to spawn process: {}", e),
            FormatterError::Stdin(e) => write!(f, "failed to write to stdin: {}", e),
            FormatterError::Timeout => write!(f, "process timed out"),
            FormatterError::NonZeroExit { code, stderr } => {
                if let Some(c) = code {
                    write!(f, "process exited with code {}", c)?;
                } else {
                    write!(f, "process terminated by signal")?;
                }
                if !stderr.is_empty() {
                    write!(f, ": {}", stderr.trim())?;
                }
                Ok(())
            }
            FormatterError::InvalidUtf8(e) => write!(f, "output is not valid UTF-8: {}", e),
        }
    }
}

impl std::error::Error for FormatterError {
    fn source(&self) -> Option<&(dyn std::error::Error + 'static)> {
        match self {
            FormatterError::Spawn(e) => Some(e),
            FormatterError::Stdin(e) => Some(e),
            FormatterError::InvalidUtf8(e) => Some(e),
            _ => None,
        }
    }
}

/// Run an external formatter command with the given code as stdin.
///
/// # Arguments
///
/// * `command` - Command and arguments as a slice of strings.
/// * `code` - Code to format, passed via stdin.
/// * `timeout_secs` - Maximum time to wait for the process in seconds.
///
/// # Returns
///
/// The formatted code from stdout, or an error if the formatter failed.
pub fn run_formatter(
    command: &[String],
    code: &str,
    timeout_secs: u64,
) -> Result<String, FormatterError> {
    if command.is_empty() {
        return Err(FormatterError::EmptyCommand);
    }

    let (program, args) = command.split_first().unwrap();

    let mut child = Command::new(program)
        .args(args)
        .stdin(Stdio::piped())
        .stdout(Stdio::piped())
        .stderr(Stdio::piped())
        .spawn()
        .map_err(FormatterError::Spawn)?;

    // Write code to stdin. A formatter may reject its input and exit before
    // draining stdin, which makes this write fail with a broken pipe. That is
    // not necessarily a real stdin failure: if the process then exits non-zero,
    // its exit status is the meaningful error. Defer the decision until the exit
    // status is known rather than masking it here. A broken pipe on an otherwise
    // *successful* exit is still treated as an error below, so a process that
    // closes stdin early is never trusted to have produced complete output.
    let mut stdin_broken_pipe: Option<std::io::Error> = None;
    if let Some(mut stdin) = child.stdin.take() {
        match stdin.write_all(code.as_bytes()) {
            Ok(()) => {}
            Err(e) if e.kind() == std::io::ErrorKind::BrokenPipe => {
                stdin_broken_pipe = Some(e);
            }
            Err(e) => return Err(FormatterError::Stdin(e)),
        }
        // stdin is dropped here, closing it
    }

    // Wait with timeout using polling
    let timeout = Duration::from_secs(timeout_secs);
    let start = Instant::now();
    let poll_interval = Duration::from_millis(50);

    loop {
        match child.try_wait() {
            Ok(Some(status)) => {
                // Process finished
                let output = child.wait_with_output().map_err(FormatterError::Spawn)?;
                if !status.success() {
                    return Err(FormatterError::NonZeroExit {
                        code: status.code(),
                        stderr: String::from_utf8_lossy(&output.stderr).to_string(),
                    });
                }
                // The process succeeded. If it closed stdin before we could
                // deliver all of the input, its output may be based on truncated
                // input, so surface the broken pipe rather than replacing the
                // code block with that untrustworthy (possibly empty) output.
                if let Some(e) = stdin_broken_pipe.take() {
                    return Err(FormatterError::Stdin(e));
                }
                return String::from_utf8(output.stdout).map_err(FormatterError::InvalidUtf8);
            }
            Ok(None) => {
                // Still running
                if start.elapsed() >= timeout {
                    let _ = child.kill();
                    return Err(FormatterError::Timeout);
                }
                thread::sleep(poll_interval);
            }
            Err(e) => return Err(FormatterError::Spawn(e)),
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_run_formatter_empty_command() {
        let result = run_formatter(&[], "code", 5);
        assert!(matches!(result, Err(FormatterError::EmptyCommand)));
    }

    #[test]
    fn test_run_formatter_cat() {
        // cat simply outputs its input unchanged
        let result = run_formatter(&["cat".to_string()], "hello world", 5);
        assert_eq!(result.unwrap(), "hello world");
    }

    #[test]
    fn test_run_formatter_with_args() {
        // tr command to uppercase
        let result = run_formatter(
            &["tr".to_string(), "a-z".to_string(), "A-Z".to_string()],
            "hello",
            5,
        );
        assert_eq!(result.unwrap(), "HELLO");
    }

    #[test]
    fn test_run_formatter_nonzero_exit() {
        // false always exits with code 1
        let result = run_formatter(&["false".to_string()], "code", 5);
        assert!(matches!(result, Err(FormatterError::NonZeroExit { .. })));
    }

    #[test]
    fn test_run_formatter_nonzero_exit_large_stdin() {
        // `false` exits non-zero without reading stdin. A 1 MiB payload far
        // exceeds the OS pipe buffer (64 KiB or less on Linux, macOS, and
        // Windows), so the write hits a broken pipe once the child's read end
        // closes. That must not be reported as a stdin error, because the
        // process's non-zero exit is the meaningful result. This reliably
        // exercises the broken-pipe path that the small-input
        // `test_run_formatter_nonzero_exit` reaches only intermittently.
        let code = "x".repeat(1024 * 1024);
        let result = run_formatter(&["false".to_string()], &code, 5);
        assert!(matches!(result, Err(FormatterError::NonZeroExit { .. })));
    }

    #[test]
    fn test_run_formatter_success_without_consuming_stdin_errors() {
        // `true` exits zero without reading stdin, so a large payload triggers a
        // broken pipe on a successful exit. The output (here empty) must not be
        // trusted to replace the code block; the broken pipe is surfaced as an
        // error so the caller keeps the original code instead of erasing it.
        let code = "x".repeat(1024 * 1024);
        let result = run_formatter(&["true".to_string()], &code, 5);
        assert!(matches!(result, Err(FormatterError::Stdin(_))));
    }

    #[test]
    fn test_run_formatter_command_not_found() {
        let result = run_formatter(&["nonexistent_command_12345".to_string()], "code", 5);
        assert!(matches!(result, Err(FormatterError::Spawn(_))));
    }

    #[test]
    fn test_run_formatter_timeout() {
        // sleep for longer than timeout
        let result = run_formatter(&["sleep".to_string(), "10".to_string()], "", 1);
        assert!(matches!(result, Err(FormatterError::Timeout)));
    }

    #[test]
    fn test_run_formatter_multiline() {
        let input = "line1\nline2\nline3";
        let result = run_formatter(&["cat".to_string()], input, 5);
        assert_eq!(result.unwrap(), input);
    }

    #[test]
    fn test_run_formatter_unicode() {
        let input = "Hello, \u{4e16}\u{754c}! \u{1f600}";
        let result = run_formatter(&["cat".to_string()], input, 5);
        assert_eq!(result.unwrap(), input);
    }
}
