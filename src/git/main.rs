use std::process;

pub fn main() -> String {
    let result = process::Command::new("git")
        .args(["remote", "show", "origin"])
        .output()
        .unwrap();
    if !result.status.success() {
        panic!("Git command failed");
    };

    String::from_utf8_lossy(&result.stdout)
        .lines()
        .map(|line| line.trim_ascii())
        .filter(|line| line.starts_with("HEAD branch"))
        .map(|line| line.strip_prefix("HEAD branch: "))
        .next()
        .unwrap() // .expect("failed to get default branch")
        .unwrap() // .expect("failed to get default branch")
        .to_owned()
}
