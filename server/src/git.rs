use tokio::process::Command;
use uuid::Uuid;

/// Merge `branch` into `base_branch` inside `repo_path` using a temporary
/// worktree so the main working tree is never disturbed.
///
/// Returns the merge commit hash on success, or an error string on failure
/// (including merge conflicts).
pub async fn merge_branch(
    repo_path: &str,
    branch: &str,
    base_branch: &str,
    message: &str,
) -> Result<String, String> {
    let tmp = std::env::temp_dir().join(format!("mandatum-merge-{}", Uuid::new_v4()));
    let tmp_str = tmp.to_string_lossy().to_string();

    // 1. Create a temporary worktree on base_branch
    let add = Command::new("git")
        .args(["-C", repo_path, "worktree", "add", &tmp_str, base_branch])
        .output()
        .await
        .map_err(|e| format!("git worktree add failed: {}", e))?;

    if !add.status.success() {
        return Err(format!(
            "git worktree add: {}",
            String::from_utf8_lossy(&add.stderr).trim()
        ));
    }

    // 2. Merge the feature branch (no-ff so there's always a merge commit)
    let merge = Command::new("git")
        .args(["-C", &tmp_str, "merge", "--no-ff", branch, "-m", message])
        .output()
        .await
        .map_err(|e| format!("git merge failed: {}", e))?;

    if !merge.status.success() {
        // Abort the merge so the worktree is clean before we remove it
        let _ = Command::new("git")
            .args(["-C", &tmp_str, "merge", "--abort"])
            .output()
            .await;
        let _ = cleanup_worktree(repo_path, &tmp_str).await;
        return Err(format!(
            "Merge conflict or error: {}",
            String::from_utf8_lossy(&merge.stderr).trim()
        ));
    }

    // 3. Capture the merge commit hash
    let rev = Command::new("git")
        .args(["-C", &tmp_str, "rev-parse", "HEAD"])
        .output()
        .await
        .map_err(|e| format!("git rev-parse failed: {}", e))?;

    let hash = String::from_utf8_lossy(&rev.stdout).trim().to_string();

    // 4. Clean up
    let _ = cleanup_worktree(repo_path, &tmp_str).await;

    Ok(hash)
}

async fn cleanup_worktree(repo_path: &str, worktree: &str) {
    let _ = Command::new("git")
        .args(["-C", repo_path, "worktree", "remove", "--force", worktree])
        .output()
        .await;
    // Belt-and-suspenders: remove the directory if git didn't
    let _ = tokio::fs::remove_dir_all(worktree).await;
}
