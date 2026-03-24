//! CLI exit codes and error types.

use thiserror::Error;

/// CLI exit codes per spec: 0 success, 1 runtime, 2 usage/config, 3 soft limit.
/// Doctor: 0 all pass, 1 mandatory failure, 2 warnings only.
#[derive(Error, Debug)]
pub enum CliError {
    #[error("{0}")]
    Usage(String),
    /// Config-related error; display with "Error [Config]: {}" per CLI spec.
    #[error("{0}")]
    Config(String),
    #[error("{0}")]
    SoftLimit(String),
    #[error("{0}")]
    Interrupted(String),
    #[error("{0}")]
    DoctorMandatory(String),
    #[error("{0}")]
    DoctorWarnings(String),
}

impl CliError {
    pub fn exit_code(&self) -> i32 {
        match self {
            CliError::Usage(_) => 2,
            CliError::Config(_) => 2,
            CliError::SoftLimit(_) => 3,
            CliError::Interrupted(_) => 130,
            CliError::DoctorMandatory(_) => 1,
            CliError::DoctorWarnings(_) => 2,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn usage_exit_code_is_2() {
        assert_eq!(CliError::Usage("bad args".into()).exit_code(), 2);
    }

    #[test]
    fn config_exit_code_is_2() {
        assert_eq!(CliError::Config("bad config".into()).exit_code(), 2);
    }

    #[test]
    fn soft_limit_exit_code_is_3() {
        assert_eq!(CliError::SoftLimit("cost limit".into()).exit_code(), 3);
    }

    #[test]
    fn interrupted_exit_code_is_130() {
        assert_eq!(CliError::Interrupted("ctrl-c".into()).exit_code(), 130);
    }

    #[test]
    fn doctor_mandatory_exit_code_is_1() {
        assert_eq!(
            CliError::DoctorMandatory("no api key".into()).exit_code(),
            1
        );
    }

    #[test]
    fn doctor_warnings_exit_code_is_2() {
        assert_eq!(
            CliError::DoctorWarnings("stale pricing".into()).exit_code(),
            2
        );
    }

    #[test]
    fn cli_error_display_shows_message() {
        let e = CliError::Usage("invalid flag".into());
        assert_eq!(e.to_string(), "invalid flag");
    }
}
