use std::time::Instant;

use crate::errors::*;

pub struct GitCommand {
    args: Vec<String>,
}

impl GitCommand {
    pub fn new<S: AsRef<std::ffi::OsStr>, I: IntoIterator<Item = S>>(args: I) -> TResult<Self> {
        Ok(Self {
            args: args
                .into_iter()
                .map(|s| s.as_ref().to_owned().into_string())
                .collect::<StdResult<Vec<String>, _>>()
                .map_err(|e| Error::Generic(format!("Invalid argument: {:?}", e)))?,
        })
    }

    pub fn execute(&self) -> TResult<String> {
        let start_time = Instant::now();
        let output = std::process::Command::new("git")
            .args(&self.args)
            .output()
            .with_comment(
                format!("Failed to execute git command: git {}", self.args.join(" ")).as_str(),
            )?;

        let duration = start_time.elapsed();
        println!(
            "[debug] `git {}` done {:?}. Result: {:#?}",
            self.args.join(" "),
            duration,
            output.status.code()
        );

        match output.status.success() {
            true => Ok(String::from_utf8_lossy(&output.stdout).into_owned()),
            false => Err(git_error(output, self.args.clone())),
        }
    }
}
