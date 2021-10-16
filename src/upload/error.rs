use std::error::Error;
use std::fmt;

#[derive(Debug)]
pub struct UploadError {
    description: String,
}

impl UploadError {
    pub fn new(msg: &str) -> UploadError {
        Self {
            description: msg.to_string(),
        }
    }
}

impl fmt::Display for UploadError {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        write!(f, "{}", self.description)
    }
}

impl Error for UploadError {
    fn description(&self) -> &str {
        &self.description
    }
}
