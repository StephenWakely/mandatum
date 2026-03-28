use tokio::process::Command;
use uuid::Uuid;
use tracing::{info, warn};

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

    info!(repo = repo_path, branch, base = base_branch, "merge: starting");

    // 1. Create a detached-HEAD worktree at base_branch's current commit.
    //    Using --detach avoids "already checked out" errors when base_branch
    //    is the active branch in the main working tree.
    info!(worktree = %tmp_str, base = base_branch, "merge: creating detached temp worktree");
    let add = Command::new("git")
        .args(["-C", repo_path, "worktree", "add", "--detach", &tmp_str, base_branch])
        .output()
        .await
        .map_err(|e| format!("git worktree add failed: {}", e))?;

    if !add.status.success() {
        let stderr = String::from_utf8_lossy(&add.stderr).trim().to_string();
        warn!(error = %stderr, "merge: worktree add failed");
        return Err(format!("git worktree add: {}", stderr));
    }

    // 2. Merge the feature branch (no-ff so there's always a merge commit)
    info!(branch, base = base_branch, "merge: running git merge --no-ff");
    let merge = Command::new("git")
        .args(["-C", &tmp_str, "merge", "--no-ff", branch, "-m", message])
        .output()
        .await
        .map_err(|e| format!("git merge failed: {}", e))?;

    if !merge.status.success() {
        let stderr = String::from_utf8_lossy(&merge.stderr).trim().to_string();
        warn!(branch, base = base_branch, error = %stderr, "merge: conflict or error, aborting");
        let _ = Command::new("git")
            .args(["-C", &tmp_str, "merge", "--abort"])
            .output()
            .await;
        let _ = cleanup_worktree(repo_path, &tmp_str).await;
        return Err(format!("Merge conflict or error: {}", stderr));
    }

    // 3. Capture the merge commit hash
    let rev = Command::new("git")
        .args(["-C", &tmp_str, "rev-parse", "HEAD"])
        .output()
        .await
        .map_err(|e| format!("git rev-parse failed: {}", e))?;

    let hash = String::from_utf8_lossy(&rev.stdout).trim().to_string();

    // 4. Advance base_branch to the merge commit via update-ref.
    //    This works even when base_branch is checked out in another worktree
    //    (git branch -f would be blocked in that case).
    info!(branch, base = base_branch, commit = %hash, "merge: advancing branch ref");
    let refname = format!("refs/heads/{}", base_branch);
    let update = Command::new("git")
        .args(["-C", repo_path, "update-ref", &refname, &hash])
        .output()
        .await
        .map_err(|e| format!("git update-ref failed: {}", e))?;

    if !update.status.success() {
        let stderr = String::from_utf8_lossy(&update.stderr).trim().to_string();
        warn!(error = %stderr, "merge: update-ref failed");
        let _ = cleanup_worktree(repo_path, &tmp_str).await;
        return Err(format!("git update-ref: {}", stderr));
    }

    // 5. Update the main working tree to match the new HEAD.
    //    update-ref moves the branch pointer but leaves the working tree and
    //    index pointing at the old commit.  reset --hard brings them in sync.
    info!(repo = repo_path, commit = %hash, "merge: resetting main worktree to new HEAD");
    let reset = Command::new("git")
        .args(["-C", repo_path, "reset", "--hard", "HEAD"])
        .output()
        .await
        .map_err(|e| format!("git reset failed: {}", e))?;

    if !reset.status.success() {
        let stderr = String::from_utf8_lossy(&reset.stderr).trim().to_string();
        warn!(error = %stderr, "merge: reset --hard failed (ref was updated, working tree may be stale)");
        // Non-fatal: the ref is correct, just the working tree lags
    }

    info!(branch, base = base_branch, commit = %hash, "merge: success");

    // 6. Clean up
    info!(worktree = %tmp_str, "merge: removing temp worktree");
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

/// Remove all agent worktrees associated with a task branch.
/// Worktrees are named `.worktrees/<safe_branch>` with optional suffixes
/// like `-review`, `-test`, `-docs` for downstream agents.
pub async fn cleanup_task_worktrees(repo_path: &str, branch: &str) {
    let safe = branch.replace('/', "__");
    let suffixes = ["", "-review", "-test", "-docs"];
    for suffix in suffixes {
        let rel = format!(".worktrees/{}{}", safe, suffix);
        let abs = format!("{}/{}", repo_path, rel);
        if tokio::fs::metadata(&abs).await.is_ok() {
            info!(worktree = %abs, "cleanup: removing task worktree");
            cleanup_worktree(repo_path, &abs).await;
        }
    }
    // Prune stale worktree metadata from git's internal records
    let _ = Command::new("git")
        .args(["-C", repo_path, "worktree", "prune"])
        .output()
        .await;
}
