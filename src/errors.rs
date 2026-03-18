use thiserror::Error;

use crate::parser::Token;

#[derive(Debug, Error)]
pub enum ShellErrors {
    #[error("Not support : {0}")]
    NotSupportedCmd(String),

    #[error("{0}: {1}")]
    CmdError(String, String),

    #[error("IO error: {0}")]
    IoError(#[from] std::io::Error),

    #[error("Unexpected token: expected {expected:?}, found {found:?}")]
    UnExpectedToken { expected: Token, found: Token },

    #[error("Unexpected end of input")]
    UnExpectedEndOfInput,

    #[error("Expect file name")]
    ExpectedFileName,

    #[error("unexpected redirect operator")]
    UnExpectedRedirectOp,
}
