use thiserror::Error;

#[derive(Error, Debug)]
pub enum Error {
    #[error("create runtime failed")]
    RunTimeCreate(#[from] std::io::Error),
    #[error("runtime spawn job failed")]
    RunTimeSpawn(#[from] futures::task::SpawnError),
}

pub type Result<T> = std::result::Result<T, Error>;
