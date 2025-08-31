pub type TResult<T> = std::result::Result<T, Error>;

pub type StdResult<T, E: std::error::Error> = std::result::Result<T, E>;

#[derive(Debug)]
pub enum Error {
    Git(GitError),
    Generic(String),
}

#[derive(Debug)]
pub struct GitError {
    args: Vec<String>,
    output: std::process::Output,
}

impl Error {
    pub fn print(&self) -> String {
        match &self {
            Error::Git(err) => format!(
                "Git command failed\n git {}\n Status Code: {:#?}\nStdout: {}\nStderr: {}",
                err.args.join(" "),
                err.output.status.code(),
                String::from_utf8_lossy(&err.output.stdout),
                String::from_utf8_lossy(&err.output.stderr),
            ),
            Error::Generic(msg) => format!("Error: {}", msg),
        }
    }
}

pub fn git_error(output: std::process::Output, args: Vec<String>) -> Error {
    Error::Git(GitError { args, output })
}

pub trait ToGeneric {
    type Ok;
    fn to_generic(self) -> TResult<Self::Ok>;
    fn with_comment(self, comment: &str) -> TResult<Self::Ok>;
}

impl<T, E: std::error::Error> ToGeneric for std::result::Result<T, E> {
    type Ok = T;

    fn to_generic(self) -> TResult<T> {
        match self {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::Generic(err.to_string())),
        }
    }

    fn with_comment(self, comment: &str) -> TResult<T> {
        match self {
            Ok(val) => Ok(val),
            Err(err) => Err(Error::Generic(format!("{}: {}", comment, err))),
        }
    }
}
