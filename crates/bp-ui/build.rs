use std::process::Command;

fn main() {
    slint_build::compile("ui/app.slint").expect("compile ui/app.slint");
    build_identity();
}

/// Which commit this binary was built from, for Help ▸ About (release plan
/// M1-02). A bug report that says "0.1.0" names a dozen builds; one that
/// names a commit names one.
///
/// "unknown" rather than a failed build when there is no git -- a source
/// tarball, or a Windows machine without git on the path. The modified flag
/// says whether tracked files differed from that commit when this script
/// last ran; it reruns when HEAD, the branch or the index moves, so an edit
/// that is never staged can leave it saying "clean" on a developer's build.
/// A release is built from a clean checkout, where that cannot happen.
fn build_identity() {
    let git = |args: &[&str]| -> Option<String> {
        let out = Command::new("git").args(args).output().ok()?;
        out.status
            .success()
            .then(|| String::from_utf8_lossy(&out.stdout).trim().to_owned())
    };

    let commit = git(&["rev-parse", "--short=12", "HEAD"]).unwrap_or_else(|| "unknown".to_owned());
    let state = match git(&["status", "--porcelain", "--untracked-files=no"]) {
        Some(changes) if changes.is_empty() => "clean",
        Some(_) => "modified",
        None => "unknown",
    };
    println!("cargo:rustc-env=BP_BUILD_COMMIT={commit}");
    println!("cargo:rustc-env=BP_BUILD_STATE={state}");

    // HEAD for a checkout of another branch, the branch's own ref for a new
    // commit on it, and the index for staging. In a worktree the refs live
    // in the common directory, not the worktree's own.
    if let Some(dir) = git(&["rev-parse", "--git-dir"]) {
        println!("cargo:rerun-if-changed={dir}/HEAD");
        println!("cargo:rerun-if-changed={dir}/index");
    }
    if let (Some(common), Some(head)) = (
        git(&["rev-parse", "--git-common-dir"]),
        git(&["rev-parse", "--symbolic-full-name", "HEAD"]),
    ) && head.starts_with("refs/")
    {
        println!("cargo:rerun-if-changed={common}/{head}");
        println!("cargo:rerun-if-changed={common}/packed-refs");
    }
}
