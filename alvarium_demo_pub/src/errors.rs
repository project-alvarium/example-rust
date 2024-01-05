use thiserror::Error;
pub type Result<T> = core::result::Result<T, Error>;


#[derive(Debug, Error)]
pub enum Error {
    #[error("Fern error: {0}")]
    LoggerSetupError(log::SetLoggerError),
    #[error("Core Alvarium error: {0}")]
    AlvariumCoreError(alvarium_annotator::Error),

    #[error("Alvarium error: {0}")]
    AlvariumSdkError(alvarium_sdk_rust::errors::Error),
}

impl From<alvarium_annotator::Error> for Error {
    fn from(e: alvarium_annotator::Error) -> Self {
        Error::AlvariumCoreError(e)
    }
}

impl From<alvarium_sdk_rust::errors::Error> for Error {
    fn from(e: alvarium_sdk_rust::errors::Error) -> Self {
        Error::AlvariumSdkError(e)
    }
}

impl From<Error> for alvarium_sdk_rust::errors::Error {
    fn from(e: Error) -> Self {
        alvarium_sdk_rust::errors::Error::External(Box::new(e))
    }
}