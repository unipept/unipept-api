#[derive(Debug)]
pub enum AppError {
    ServerStartError,
}

impl From<std::io::Error> for AppError {
    fn from(_error: std::io::Error) -> Self {
        AppError::ServerStartError
    }
}
