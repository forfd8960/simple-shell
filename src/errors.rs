use thiserror::Error;

#[derive(Debug, Error)]
pub enum ShellErrors {
    #[error("Not support : {0}")]
    NotSupportedCmd(String),
    
    #[error("{0}: {1}")]
    CmdError(String, String),
}
