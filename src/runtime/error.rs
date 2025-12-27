use std::fmt;

#[derive(Debug)]
pub enum RuntimeError {
    PoisonedLock,
}

impl fmt::Display for RuntimeError {
    fn fmt(&self, f: &mut fmt::Formatter<'_>) -> fmt::Result {
        match self {
            RuntimeError::PoisonedLock =>
                write!(f, "runtime internal lock poisoned"),
        }
    }
}

impl std::error::Error for RuntimeError {}
