use thiserror::Error;

#[derive(Error, Debug)]
pub enum AgentError<E> {
    #[error("Agent error")]
    AgentError(Box<E>),
    #[error("IO Error")]
    IoError(std::io::Error),
    #[error("Unauthorized")]
    Unauthorized,
}