use std::path::PathBuf;

use crate::error::Error;

pub struct Config {
    pub dirname: String,
}

impl Config {
    pub fn verify(&self) -> Result<(), Error> {
        let path = PathBuf::from(&self.dirname);
        if !path.is_dir() {
            return Err(Error::InvalidConfig {
                desc: String::from("Provided path is not a directory"),
            });
        }
        Ok(())
    }
}
