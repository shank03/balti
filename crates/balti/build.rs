use std::process::Command;

fn main() -> Result<(), Box<dyn std::error::Error + 'static>> {
    let version = std::env::var("CARGO_PKG_VERSION")?;
    println!("cargo:rustc-env=BALTI_VERSION={version}");

    if let Ok(output) = Command::new("git").args(["rev-parse", "HEAD"]).output()
        && output.status.success()
    {
        let git_sha = String::from_utf8_lossy(&output.stdout);
        let git_sha = git_sha.trim();

        println!("cargo:rustc-env=BALTI_COMMIT_SHA={git_sha}");
        Ok(())
    } else {
        Err(std::env::VarError::NotPresent.into())
    }
}
