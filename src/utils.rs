use std::result::Result;
use std::error::Error;
use std::path::PathBuf;

pub trait OptionAsResult<T, E: Error> {

    fn as_result(&self, error: E) -> Result<T, E>;

}

impl<T: Clone, E: Error> OptionAsResult<T, E> for Option<T> {
    fn as_result(&self, error: E) -> Result<T, E> {
        match self {
            Some(_) => Ok(self.as_ref().unwrap().clone()),
            None => Err(error),
        }
    }
}

pub trait ToOwnedStr {
    fn as_string(self) -> String;
}

impl ToOwnedStr for PathBuf {
    fn as_string(self) -> String {
        self.to_str().unwrap().to_string()
    }
}

pub trait LoggableResult<T, E: Error> {

    fn log_info(self, message: &str) -> Result<T, E>;
    fn log_error(self, message: &str) -> Result<T, E>;
    fn log_error_and_ignore(self, message: &str) -> ();
}

impl<T, E: Error> LoggableResult<T, E> for Result<T, E> {
    fn log_info(self, message: &str) -> Result<T, E> {
        if self.is_err() {
            info!("{}: {}", message, self.as_ref().err().unwrap());
        }
        self
    }
    fn log_error(self, message: &str) -> Result<T, E> {
        if self.is_err() {
            error!("{}: {}", message, self.as_ref().err().unwrap());
        }
        self
    }

    fn log_error_and_ignore(self, message: &str) -> () {
        let _ = self.log_error(message);
    }
}

pub fn join_executable_path(to_join: &str) -> Option<String> {
    std::env::current_exe().unwrap().parent().map(|path| path.join(to_join).as_string())
}
