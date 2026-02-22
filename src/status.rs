/// Result of ticking a behavior tree node.
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    Running,
    Success,
    Failure,
}

impl Status {
    pub fn is_done(self) -> bool {
        !matches!(self, Status::Running)
    }

    pub fn is_success(self) -> bool {
        matches!(self, Status::Success)
    }

    pub fn is_failure(self) -> bool {
        matches!(self, Status::Failure)
    }

    pub fn invert(self) -> Self {
        match self {
            Status::Success => Status::Failure,
            Status::Failure => Status::Success,
            Status::Running => Status::Running,
        }
    }
}

#[cfg(test)]
mod tests {
    use super::Status;

    #[test]
    fn status_invert_success() {
        assert_eq!(Status::Success.invert(), Status::Failure);
    }

    #[test]
    fn status_invert_failure() {
        assert_eq!(Status::Failure.invert(), Status::Success);
    }

    #[test]
    fn status_invert_running() {
        assert_eq!(Status::Running.invert(), Status::Running);
    }

    #[test]
    fn status_is_done() {
        assert!(!Status::Running.is_done());
        assert!(Status::Success.is_done());
        assert!(Status::Failure.is_done());
    }

    #[test]
    fn status_is_success() {
        assert!(Status::Success.is_success());
        assert!(!Status::Running.is_success());
        assert!(!Status::Failure.is_success());
    }

    #[test]
    fn status_is_failure() {
        assert!(Status::Failure.is_failure());
        assert!(!Status::Running.is_failure());
        assert!(!Status::Success.is_failure());
    }
}
