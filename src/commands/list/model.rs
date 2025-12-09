use std::path::PathBuf;
use worktrunk::git::{IntegrationReason, LineDiff};

use super::ci_status::PrStatus;
use super::columns::ColumnKind;

/// Display fields shared between WorktreeInfo and BranchInfo
/// These contain formatted strings with ANSI colors for json-pretty output
#[derive(Clone, serde::Serialize, Default)]
pub struct DisplayFields {
    #[serde(skip_serializing_if = "Option::is_none")]
    pub commits_display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub branch_diff_display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub upstream_display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub ci_status_display: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub status_display: Option<String>,
    /// Pre-formatted single-line representation for statusline tools.
    /// Format: `branch  status  @working  commits  ^branch_diff  upstream  ci` (2-space separators)
    ///
    /// Use via JSON: `wt list --format=json | jq '.[] | select(.is_current) | .statusline'`
    #[serde(skip_serializing_if = "Option::is_none")]
    pub statusline: Option<String>,
}

impl DisplayFields {
    pub(crate) fn from_common_fields(
        counts: &Option<AheadBehind>,
        branch_diff: &Option<BranchDiffTotals>,
        upstream: &Option<UpstreamStatus>,
        _pr_status: &Option<Option<PrStatus>>,
    ) -> Self {
        let commits_display = counts
            .as_ref()
            .and_then(|c| ColumnKind::AheadBehind.format_diff_plain(c.ahead, c.behind));

        let branch_diff_display = branch_diff.as_ref().and_then(|bd| {
            ColumnKind::BranchDiff.format_diff_plain(bd.diff.added, bd.diff.deleted)
        });

        let upstream_display = upstream.as_ref().and_then(|u| {
            u.active().and_then(|(_, upstream_ahead, upstream_behind)| {
                ColumnKind::Upstream.format_diff_plain(upstream_ahead, upstream_behind)
            })
        });

        // CI column shows only the indicator (●/○/◐), not text
        // Let render.rs handle it via render_indicator()
        let ci_status_display = None;

        Self {
            commits_display,
            branch_diff_display,
            upstream_display,
            ci_status_display,
            status_display: None,
            statusline: None,
        }
    }
}

/// Type-specific data for worktrees
#[derive(Clone, serde::Serialize, Default)]
pub struct WorktreeData {
    pub path: PathBuf,
    pub bare: bool,
    pub detached: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub locked: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub prunable: Option<String>,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_tree_diff: Option<LineDiff>,
    /// Diff between working tree and main branch.
    /// `None` means "not computed yet" or "not computed" (optimization: skipped when trees differ).
    /// `Some(Some((0, 0)))` means working tree matches main exactly.
    /// `Some(Some((a, d)))` means a lines added, d deleted vs main.
    /// `Some(None)` means computation was skipped.
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_tree_diff_with_main: Option<Option<LineDiff>>,
    /// Git operation in progress (rebase/merge)
    #[serde(skip_serializing_if = "git_operation_is_none")]
    pub git_operation: GitOperationState,
    pub is_main: bool,
    /// Whether this is the current worktree (matches $PWD)
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_current: bool,
    /// Whether this was the previous worktree (from WT_PREVIOUS_BRANCH)
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub is_previous: bool,
    /// Whether the worktree path doesn't match what the template would generate.
    /// Only true when: has branch name, not main worktree, and path differs from template.
    #[serde(skip_serializing_if = "std::ops::Not::not")]
    pub path_mismatch: bool,
    #[serde(skip_serializing_if = "Option::is_none")]
    pub working_diff_display: Option<String>,
}

impl WorktreeData {
    /// Create WorktreeData from a Worktree, with all computed fields set to None.
    pub(crate) fn from_worktree(
        wt: &worktrunk::git::Worktree,
        is_main: bool,
        is_current: bool,
        is_previous: bool,
    ) -> Self {
        Self {
            // Identity fields (known immediately from worktree list)
            path: wt.path.clone(),
            bare: wt.bare,
            detached: wt.detached,
            locked: wt.locked.clone(),
            prunable: wt.prunable.clone(),
            is_main,
            is_current,
            is_previous,

            // Computed fields start as None (filled progressively)
            ..Default::default()
        }
    }
}

/// Discriminator for item type (worktree vs branch)
///
/// WorktreeData is boxed to reduce the size of ItemKind enum (304 bytes → 24 bytes).
/// This reduces stack pressure when passing ListItem by value and improves cache locality
/// in `Vec<ListItem>` by keeping the discriminant and common fields together.
#[derive(serde::Serialize)]
#[serde(tag = "type", rename_all = "lowercase")]
pub enum ItemKind {
    Worktree(Box<WorktreeData>),
    Branch,
}

#[derive(serde::Serialize, Clone, Default, Debug)]
pub struct CommitDetails {
    pub timestamp: i64,
    pub commit_message: String,
}

#[derive(serde::Serialize, Default, Copy, Clone, Debug)]
pub struct AheadBehind {
    pub ahead: usize,
    pub behind: usize,
}

impl AheadBehind {
    /// Compute divergence states from ahead/behind counts and upstream status.
    ///
    /// Returns (main_divergence, upstream_divergence) as a tuple of `Divergence` values.
    pub fn compute_divergences(&self, upstream: &UpstreamStatus) -> (Divergence, Divergence) {
        let main_divergence = Divergence::from_counts(self.ahead, self.behind);
        let upstream_divergence = match upstream.active() {
            None => Divergence::None,
            Some((_, ahead, behind)) => Divergence::from_counts_with_remote(ahead, behind),
        };

        (main_divergence, upstream_divergence)
    }
}

#[derive(serde::Serialize, Default, Copy, Clone, Debug)]
pub struct BranchDiffTotals {
    #[serde(rename = "branch_diff")]
    pub diff: LineDiff,
}

#[derive(serde::Serialize, Default, Clone, Debug)]
pub struct UpstreamStatus {
    #[serde(rename = "upstream_remote")]
    pub(super) remote: Option<String>,
    #[serde(rename = "upstream_ahead")]
    pub(super) ahead: usize,
    #[serde(rename = "upstream_behind")]
    pub(super) behind: usize,
}

impl UpstreamStatus {
    pub fn active(&self) -> Option<(&str, usize, usize)> {
        self.remote
            .as_deref()
            .map(|remote| (remote, self.ahead, self.behind))
    }

    #[cfg(test)]
    pub(crate) fn from_parts(remote: Option<String>, ahead: usize, behind: usize) -> Self {
        Self {
            remote,
            ahead,
            behind,
        }
    }
}

/// Unified item for displaying worktrees and branches in the same table
#[derive(serde::Serialize)]
pub struct ListItem {
    // Common fields (present for both worktrees and branches)
    #[serde(rename = "head_sha")]
    pub head: String,
    /// Branch name - None for detached worktrees
    pub branch: Option<String>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub commit: Option<CommitDetails>,

    // TODO: Evaluate if skipping these fields in JSON when None is correct behavior.
    // Currently, main worktree omits counts/branch_diff (since it doesn't compare to itself),
    // but consumers may expect these fields to always be present (even if zero).
    // Consider: always include with default values vs current "omit when not computed" approach.
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub counts: Option<AheadBehind>,
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub branch_diff: Option<BranchDiffTotals>,
    /// Whether HEAD's tree SHA matches main's tree SHA.
    /// True when committed content is identical regardless of commit history.
    /// Internal field used to compute `BranchState::Integrated(TreesMatch)`.
    #[serde(skip)]
    pub committed_trees_match: Option<bool>,
    /// Whether branch has file changes beyond the merge-base with main.
    /// False when three-dot diff (`main...branch`) is empty.
    /// Internal field used for integration detection (no unique content).
    #[serde(skip)]
    pub has_file_changes: Option<bool>,
    /// Whether merging branch into main would add changes (merge simulation).
    /// False when `git merge-tree --write-tree main branch` produces same tree as main.
    /// Only computed with `--full` flag. Catches squash-merged branches where main advanced.
    #[serde(skip)]
    pub would_merge_add: Option<bool>,
    /// Whether branch HEAD is an ancestor of main (or same commit).
    /// True means branch is already part of main's history.
    /// This is the cheapest integration check (~1ms).
    #[serde(skip)]
    pub is_ancestor: Option<bool>,

    // TODO: Same concern as counts/branch_diff above - should upstream fields always be present?
    #[serde(flatten, skip_serializing_if = "Option::is_none")]
    pub upstream: Option<UpstreamStatus>,

    /// CI/PR status: None = not loaded, Some(None) = no CI, Some(Some(status)) = has CI
    pub pr_status: Option<Option<PrStatus>>,
    /// Git status symbols - None until all dependencies are ready.
    /// Note: This field is not serialized directly. JSON output converts to JsonItem first.
    #[serde(skip)]
    pub status_symbols: Option<StatusSymbols>,

    // Display fields for json-pretty format (with ANSI colors)
    #[serde(flatten)]
    pub display: DisplayFields,

    // Type-specific data (worktree vs branch)
    #[serde(flatten)]
    pub kind: ItemKind,
}

pub struct ListData {
    pub items: Vec<ListItem>,
}

impl ListItem {
    /// Create a ListItem for a branch (not a worktree)
    pub(crate) fn new_branch(head: String, branch: String) -> Self {
        Self {
            head,
            branch: Some(branch),
            commit: None,
            counts: None,
            branch_diff: None,
            committed_trees_match: None,
            has_file_changes: None,
            would_merge_add: None,
            is_ancestor: None,
            upstream: None,
            pr_status: None,
            status_symbols: None,
            display: DisplayFields::default(),
            kind: ItemKind::Branch,
        }
    }

    pub fn branch_name(&self) -> &str {
        self.branch.as_deref().unwrap_or("(detached)")
    }

    pub fn is_main(&self) -> bool {
        matches!(&self.kind, ItemKind::Worktree(data) if data.is_main)
    }

    pub fn head(&self) -> &str {
        &self.head
    }

    pub fn commit_details(&self) -> CommitDetails {
        self.commit.clone().unwrap_or_default()
    }

    pub fn counts(&self) -> AheadBehind {
        self.counts.unwrap_or_default()
    }

    pub fn branch_diff(&self) -> BranchDiffTotals {
        self.branch_diff.unwrap_or_default()
    }

    pub fn upstream(&self) -> UpstreamStatus {
        self.upstream.clone().unwrap_or_default()
    }

    pub fn worktree_data(&self) -> Option<&WorktreeData> {
        match &self.kind {
            ItemKind::Worktree(data) => Some(data),
            ItemKind::Branch => None,
        }
    }

    pub fn worktree_path(&self) -> Option<&PathBuf> {
        self.worktree_data().map(|data| &data.path)
    }

    pub fn pr_status(&self) -> Option<Option<&PrStatus>> {
        self.pr_status.as_ref().map(|opt| opt.as_ref())
    }

    /// Determine if the item contains no unique work and can likely be removed.
    ///
    /// Returns true when any of these conditions indicate the branch content
    /// is already integrated into main:
    ///
    /// 1. **Same commit** - branch HEAD is ancestor of main or same commit.
    ///    The branch is already part of main's history.
    /// 2. **No file changes** - three-dot diff (`main...branch`) is empty.
    ///    Catches squash-merged branches where commits exist but add no files.
    /// 3. **Tree matches main** - tree SHA equals main's tree SHA.
    ///    Catches rebased/squash-merged branches with identical content.
    /// 4. **Merge simulation** (`--full` only) - merging branch into main wouldn't
    ///    change main's tree. Catches squash-merged branches where main has advanced.
    /// 5. **Working tree matches main** (worktrees only) - uncommitted changes
    ///    don't diverge from main.
    pub(crate) fn is_potentially_removable(&self) -> bool {
        if self.is_main() {
            return false;
        }

        // Helper: check if working tree is clean
        let is_working_tree_clean = || {
            self.worktree_data()
                .map(|data| {
                    data.working_tree_diff
                        .as_ref()
                        .map(|d| d.is_empty())
                        .unwrap_or(true)
                })
                .unwrap_or(true) // Branches without worktrees are "clean"
        };

        // Check 1: Branch is ancestor of main (same commit or already merged)
        if self.is_ancestor == Some(true) && is_working_tree_clean() {
            return true;
        }

        // Check 2: No file changes beyond merge-base (three-dot diff empty)
        // This is the primary integration check - catches most cases including
        // squash-merged branches where commits exist but don't add file changes.
        if self.has_file_changes == Some(false) && is_working_tree_clean() {
            return true;
        }

        // Check 3: Tree SHA matches main (handles squash merge/rebase)
        if self.committed_trees_match == Some(true) && is_working_tree_clean() {
            return true;
        }

        // Check 4: Merge simulation (--full only)
        // Merging branch into main wouldn't add changes - content already integrated.
        // This catches cases where main has advanced past the squash-merge point.
        if self.would_merge_add == Some(false) && is_working_tree_clean() {
            return true;
        }

        let counts = self.counts();

        // Check 5 (worktrees): Working tree matches main
        if let Some(data) = self.worktree_data() {
            let no_commits_and_clean = counts.ahead == 0
                && data
                    .working_tree_diff
                    .as_ref()
                    .map(|d| d.is_empty())
                    .unwrap_or(true);
            let matches_main = data
                .working_tree_diff_with_main
                .and_then(|opt_diff| opt_diff)
                .map(|diff| diff.is_empty())
                .unwrap_or(false);
            no_commits_and_clean || matches_main
        } else {
            // Branch without worktree: fallback to ahead == 0
            counts.ahead == 0
        }
    }

    /// Format this item as a single-line statusline string.
    ///
    /// Format: `branch  status  @working  commits  ^branch_diff  upstream  ci`
    /// Uses 2-space separators between non-empty parts.
    pub fn format_statusline(&self) -> String {
        let mut parts: Vec<String> = Vec::new();

        // 1. Branch name
        parts.push(self.branch_name().to_string());

        // 2. Status symbols (compact, no grid alignment)
        if let Some(ref symbols) = self.status_symbols {
            let status = symbols.format_compact();
            if !status.is_empty() {
                parts.push(status);
            }
        }

        // 3. Working diff (worktrees only)
        // Prefix with @ ("at" current state) to distinguish from branch diff (^)
        if let Some(data) = self.worktree_data()
            && let Some(ref diff) = data.working_tree_diff
            && !diff.is_empty()
            && let Some(formatted) =
                ColumnKind::WorkingDiff.format_diff_plain(diff.added, diff.deleted)
        {
            parts.push(format!("@{formatted}"));
        }

        // 4. Commits ahead/behind main
        if let Some(formatted) =
            ColumnKind::AheadBehind.format_diff_plain(self.counts().ahead, self.counts().behind)
        {
            parts.push(formatted);
        }

        // 5. Branch diff vs main (line changes)
        // Prefix with ^ (main) to distinguish from working diff (@)
        let branch_diff = self.branch_diff();
        if !branch_diff.diff.is_empty()
            && let Some(formatted) = ColumnKind::BranchDiff
                .format_diff_plain(branch_diff.diff.added, branch_diff.diff.deleted)
        {
            parts.push(format!("^{formatted}"));
        }

        // 6. Upstream status
        if let Some(ref upstream) = self.upstream
            && let Some((_, ahead, behind)) = upstream.active()
            && let Some(formatted) = ColumnKind::Upstream.format_diff_plain(ahead, behind)
        {
            parts.push(formatted);
        }

        // 7. CI status
        if let Some(Some(ref pr_status)) = self.pr_status {
            parts.push(pr_status.format_indicator());
        }

        parts.join("  ")
    }

    /// Populate display fields for JSON output and statusline.
    ///
    /// Call after all computed fields (counts, diffs, upstream, CI) are available.
    pub fn finalize_display(&mut self) {
        use super::columns::ColumnKind;

        self.display = DisplayFields::from_common_fields(
            &self.counts,
            &self.branch_diff,
            &self.upstream,
            &self.pr_status,
        );
        self.display.statusline = Some(self.format_statusline());

        if let ItemKind::Worktree(ref mut wt_data) = self.kind
            && let Some(ref working_tree_diff) = wt_data.working_tree_diff
        {
            wt_data.working_diff_display = ColumnKind::WorkingDiff
                .format_diff_plain(working_tree_diff.added, working_tree_diff.deleted);
        }
    }

    /// Compute status symbols for this item.
    ///
    /// This is idempotent and can be called multiple times as new data arrives.
    /// It will recompute with the latest available data.
    ///
    /// Branches get a subset of status symbols (no working tree changes or worktree attrs).
    pub(crate) fn compute_status_symbols(
        &mut self,
        default_branch: Option<&str>,
        has_merge_tree_conflicts: bool,
        user_marker: Option<String>,
        working_tree_status: Option<WorkingTreeStatus>,
        has_conflicts: bool,
    ) {
        // Common fields for both worktrees and branches
        let default_counts = AheadBehind::default();
        let default_upstream = UpstreamStatus::default();
        let counts = self.counts.as_ref().unwrap_or(&default_counts);
        let upstream = self.upstream.as_ref().unwrap_or(&default_upstream);
        let (main_divergence, upstream_divergence) = counts.compute_divergences(upstream);

        match &self.kind {
            ItemKind::Worktree(data) => {
                // Full status computation for worktrees
                // Use default_branch directly (None for main worktree)

                // Worktree state - priority: path_mismatch > prunable > locked
                let worktree_state = if data.path_mismatch {
                    WorktreeState::PathMismatch
                } else if data.prunable.is_some() {
                    WorktreeState::Prunable
                } else if data.locked.is_some() {
                    WorktreeState::Locked
                } else {
                    WorktreeState::None
                };

                // Determine base branch state (only for non-main worktrees with base branch)
                let base_state = determine_worktree_base_state(
                    data.is_main,
                    default_branch,
                    self.is_ancestor,
                    self.counts.as_ref().map(|c| c.behind),
                    self.committed_trees_match.unwrap_or(false),
                    self.has_file_changes,
                    self.would_merge_add,
                    data.working_tree_diff.as_ref(),
                    &data.working_tree_diff_with_main,
                );

                // Apply priority: Conflicts > Rebase > Merge > WouldConflict > base_state
                let branch_state = if has_conflicts {
                    BranchState::Conflicts
                } else if data.git_operation == GitOperationState::Rebase {
                    BranchState::Rebase
                } else if data.git_operation == GitOperationState::Merge {
                    BranchState::Merge
                } else if has_merge_tree_conflicts {
                    BranchState::WouldConflict
                } else {
                    base_state
                };

                // Override main_divergence for the main worktree
                let main_divergence = if data.is_main {
                    Divergence::IsMain
                } else {
                    main_divergence
                };

                self.status_symbols = Some(StatusSymbols {
                    branch_state,
                    worktree_state,
                    main_divergence,
                    upstream_divergence,
                    working_tree: working_tree_status.unwrap_or_default(),
                    user_marker,
                });
            }
            ItemKind::Branch => {
                // Simplified status computation for branches
                // Only compute symbols that apply to branches (no working tree, git operation, or worktree attrs)

                // Branch state - branches can show WouldConflict or integration states
                let branch_state = if has_merge_tree_conflicts {
                    BranchState::WouldConflict
                } else if self.is_ancestor == Some(true) {
                    // Branch HEAD is ancestor of main
                    if self.counts.as_ref().is_some_and(|c| c.behind == 0) {
                        // Same commit as main
                        BranchState::SameCommit
                    } else {
                        // Main has moved past this branch
                        BranchState::Integrated(IntegrationReason::Ancestor)
                    }
                } else if self.has_file_changes == Some(false) {
                    // No file changes beyond merge-base - content is integrated
                    BranchState::Integrated(IntegrationReason::NoAddedChanges)
                } else if self.committed_trees_match == Some(true) {
                    // Tree SHA matches main - content is identical
                    BranchState::Integrated(IntegrationReason::TreesMatch)
                } else if self.would_merge_add == Some(false) {
                    // Merge simulation shows no changes would be added (--full only)
                    BranchState::Integrated(IntegrationReason::MergeAddsNothing)
                } else if let Some(ref c) = self.counts {
                    if c.ahead == 0 && c.behind == 0 {
                        // Same commit as main (fallback when is_ancestor not computed)
                        BranchState::SameCommit
                    } else {
                        BranchState::None
                    }
                } else {
                    BranchState::None
                };

                self.status_symbols = Some(StatusSymbols {
                    branch_state,
                    worktree_state: WorktreeState::Branch,
                    main_divergence,
                    upstream_divergence,
                    working_tree: WorkingTreeStatus::default(),
                    user_marker,
                });
            }
        }
    }
}

/// Determine branch state for a worktree.
///
/// Returns a [`BranchState`] indicating whether the branch content is integrated into main
/// and how (via [`IntegrationReason`]).
///
/// # States (priority order)
///
/// 1. **`SameCommit`**: Branch HEAD is exactly the same commit as main.
///
/// 2. **`Integrated(Ancestor)`**: Branch HEAD is ancestor of main (main has moved past).
///
/// 3. **`Integrated(NoAddedChanges)`**: Commits exist but no file changes beyond merge-base.
///    The three-dot diff (`main...branch`) is empty.
///
/// 4. **`Integrated(TreesMatch)`**: Tree SHA matches main, or working tree matches main.
///    Examples: merge commits that pull in main, rebases, squash merges.
///
/// 5. **`Integrated(MergeAddsNothing)`**: Merge simulation adds nothing to main.
///    Catches squash-merged branches where main has advanced.
///
/// # Parameters
///
/// - `is_ancestor`: Whether branch HEAD is ancestor of main (None = not computed)
/// - `behind_main`: Number of commits branch is behind main (None = not computed)
/// - `committed_trees_match`: Whether committed tree SHAs match (HEAD^{tree} == main^{tree})
/// - `has_file_changes`: Whether three-dot diff has file changes (None = not computed)
/// - `would_merge_add`: Whether merge simulation shows changes (None = not computed, `--full` only)
/// - `working_tree_diff_with_main`: Diff between working tree and main. May be `None` (not
///   computed) or `Some(None)` (skipped). When unavailable, assumes no match.
#[allow(clippy::too_many_arguments)]
fn determine_worktree_base_state(
    is_main: bool,
    default_branch: Option<&str>,
    is_ancestor: Option<bool>,
    behind_main: Option<usize>,
    committed_trees_match: bool,
    has_file_changes: Option<bool>,
    would_merge_add: Option<bool>,
    working_tree_diff: Option<&LineDiff>,
    working_tree_diff_with_main: &Option<Option<LineDiff>>,
) -> BranchState {
    if is_main || default_branch.is_none() {
        return BranchState::None;
    }

    let is_clean = working_tree_diff.map(|d| d.is_empty()).unwrap_or(true);

    // Priority 1: Branch is exactly the same commit as main (ancestor with 0 behind)
    if is_ancestor == Some(true) && behind_main == Some(0) && is_clean {
        return BranchState::SameCommit;
    }

    // Priority 2: Branch is ancestor of main but main has moved past (already merged)
    if is_ancestor == Some(true) && is_clean {
        return BranchState::Integrated(IntegrationReason::Ancestor);
    }

    // Priority 3: No file changes beyond merge-base (squash-merged)
    if has_file_changes == Some(false) && is_clean {
        return BranchState::Integrated(IntegrationReason::NoAddedChanges);
    }

    // Priority 4: Tree SHA matches main (squash merge/rebase with identical content)
    if committed_trees_match && is_clean {
        return BranchState::Integrated(IntegrationReason::TreesMatch);
    }

    // Priority 5: Working tree matches main
    let working_tree_matches_main = working_tree_diff_with_main
        .as_ref()
        .and_then(|opt| opt.as_ref())
        .is_some_and(|diff| diff.is_empty());

    if working_tree_matches_main {
        return BranchState::Integrated(IntegrationReason::TreesMatch);
    }

    // Priority 6: Merge simulation shows no changes (--full only)
    if would_merge_add == Some(false) && is_clean {
        return BranchState::Integrated(IntegrationReason::MergeAddsNothing);
    }

    BranchState::None
}

/// Divergence state relative to a reference (main branch or upstream remote).
///
/// Unified representation for both main-branch divergence and upstream-remote divergence.
/// The display symbols differ based on context:
///
/// | Variant   | Main symbols | Upstream symbols |
/// |-----------|--------------|------------------|
/// | None      | (empty)      | (empty)          |
/// | IsMain    | `^`          | (not used)       |
/// | InSync    | (not used)   | `\|`             |
/// | Ahead     | `↑`          | `⇡`              |
/// | Behind    | `↓`          | `⇣`              |
/// | Diverged  | `↕`          | `⇅`              |
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum Divergence {
    /// No relationship to reference (up-to-date for main, no remote configured for upstream)
    #[default]
    None,
    /// This IS the main branch (only used in Main context)
    IsMain,
    /// In sync with upstream remote (only used in Upstream context)
    InSync,
    /// Has commits the reference doesn't have
    Ahead,
    /// Missing commits from the reference
    Behind,
    /// Both ahead and behind the reference
    Diverged,
}

/// Context for divergence display (determines which symbols to use)
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum DivergenceContext {
    /// Divergence relative to main branch (uses ↑↓↕^)
    Main,
    /// Divergence relative to upstream remote (uses ⇡⇣⇅|)
    Upstream,
}

impl Divergence {
    /// Compute divergence state from ahead/behind counts.
    ///
    /// For main context: returns `None` for 0/0 (caller sets `IsMain` for main branch itself)
    /// For upstream context with known remote: use `from_counts_with_remote` instead
    pub fn from_counts(ahead: usize, behind: usize) -> Self {
        match (ahead, behind) {
            (0, 0) => Self::None,
            (_, 0) => Self::Ahead,
            (0, _) => Self::Behind,
            _ => Self::Diverged,
        }
    }

    /// Compute divergence state when a remote tracking branch exists.
    ///
    /// Returns `InSync` for 0/0 since we know a remote exists.
    /// For cases where there's no remote, use `Divergence::None` directly.
    pub fn from_counts_with_remote(ahead: usize, behind: usize) -> Self {
        match (ahead, behind) {
            (0, 0) => Self::InSync,
            (_, 0) => Self::Ahead,
            (0, _) => Self::Behind,
            _ => Self::Diverged,
        }
    }

    /// Get the display symbol for this divergence state in the given context.
    pub fn symbol(self, ctx: DivergenceContext) -> &'static str {
        match (self, ctx) {
            (Self::None, _) => "",
            (Self::IsMain, _) => "^",
            (Self::InSync, _) => "|",
            (Self::Ahead, DivergenceContext::Main) => "↑",
            (Self::Ahead, DivergenceContext::Upstream) => "⇡",
            (Self::Behind, DivergenceContext::Main) => "↓",
            (Self::Behind, DivergenceContext::Upstream) => "⇣",
            (Self::Diverged, DivergenceContext::Main) => "↕",
            (Self::Diverged, DivergenceContext::Upstream) => "⇅",
        }
    }

    /// Returns styled symbol (dimmed) for the given context, or None for None variant.
    pub fn styled(self, ctx: DivergenceContext) -> Option<String> {
        use color_print::cformat;
        if self == Self::None {
            None
        } else {
            Some(cformat!("<dim>{}</>", self.symbol(ctx)))
        }
    }
}

/// Worktree state indicator
///
/// Shows the "location" state of a worktree or branch:
/// - For worktrees: whether the path matches the template, or has issues
/// - For branches (without worktree): shows / to distinguish from worktrees
///
/// Priority order for worktrees: PathMismatch > Prunable > Locked
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, strum::IntoStaticStr)]
pub enum WorktreeState {
    #[strum(serialize = "")]
    /// Normal worktree (path matches template, not locked or prunable)
    #[default]
    None,
    /// Path doesn't match what the template would generate (red flag = "not at home")
    PathMismatch,
    /// Prunable (worktree directory missing)
    Prunable,
    /// Locked (protected from removal)
    Locked,
    /// Branch indicator (for branches without worktrees)
    Branch,
}

impl std::fmt::Display for WorktreeState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::None => Ok(()),
            Self::PathMismatch => write!(f, "⚑"),
            Self::Prunable => write!(f, "⊟"),
            Self::Locked => write!(f, "⊞"),
            Self::Branch => write!(f, "/"),
        }
    }
}

impl serde::Serialize for WorktreeState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Branch state: operation states, conflict states, or integration states
///
/// Represents the primary state of a branch/worktree in a single position.
/// Priority order determines which symbol is shown when multiple conditions apply:
/// 1. Conflicts (✘) - blocking, must resolve
/// 2. Rebase (⤴) - active operation
/// 3. Merge (⤵) - active operation
/// 4. WouldConflict (✗) - potential problem (merge-tree simulation)
/// 5. SameCommit (_) - removable, branch is exactly the same commit as main
/// 6. Integrated (⊂) - removable, content is in main via different history
///
/// The `Integrated` variant carries an [`IntegrationReason`] explaining how the
/// content was integrated (ancestor, trees match, no added changes, or merge adds nothing).
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default)]
pub enum BranchState {
    /// Normal working branch
    #[default]
    None,
    /// Actual merge conflicts with main (unmerged paths in working tree)
    Conflicts,
    /// Rebase in progress
    Rebase,
    /// Merge in progress
    Merge,
    /// Merge-tree conflicts with main (simulated via git merge-tree)
    WouldConflict,
    /// Branch HEAD is exactly the same commit as main
    SameCommit,
    /// Content is integrated into main via different history
    Integrated(IntegrationReason),
}

impl std::fmt::Display for BranchState {
    fn fmt(&self, f: &mut std::fmt::Formatter) -> std::fmt::Result {
        match self {
            Self::None => Ok(()),
            Self::Conflicts => write!(f, "✘"),
            Self::Rebase => write!(f, "⤴"),
            Self::Merge => write!(f, "⤵"),
            Self::WouldConflict => write!(f, "✗"),
            Self::SameCommit => write!(f, "_"),
            // All integration reasons use ⊂ (content is subset of main)
            Self::Integrated(_) => write!(f, "⊂"),
        }
    }
}

impl BranchState {
    /// Returns styled symbol with appropriate color, or None for None variant.
    ///
    /// Color semantics:
    /// - ERROR (red): Conflicts - blocking problems
    /// - WARNING (yellow): Rebase, Merge, WouldConflict - active/stuck states
    /// - HINT (dimmed): SameCommit, Integrated - low urgency removability indicators
    pub fn styled(&self) -> Option<String> {
        use color_print::cformat;
        match self {
            Self::None => None,
            Self::Conflicts => Some(cformat!("<red>{self}</>")),
            Self::Rebase | Self::Merge | Self::WouldConflict => Some(cformat!("<yellow>{self}</>")),
            Self::SameCommit | Self::Integrated(_) => Some(cformat!("<dim>{self}</>")),
        }
    }

    /// Returns the integration reason if this is an integrated state, None otherwise.
    pub fn integration_reason(&self) -> Option<IntegrationReason> {
        match self {
            Self::Integrated(reason) => Some(*reason),
            _ => None,
        }
    }

    /// Returns the JSON string representation for branch_state field.
    pub fn as_json_str(self) -> Option<&'static str> {
        match self {
            Self::None => None,
            Self::Conflicts => Some("conflicts"),
            Self::Rebase => Some("rebase"),
            Self::Merge => Some("merge"),
            Self::WouldConflict => Some("would_conflict"),
            Self::SameCommit => Some("same_commit"),
            Self::Integrated(_) => Some("integrated"),
        }
    }
}

impl serde::Serialize for BranchState {
    fn serialize<S>(&self, serializer: S) -> Result<S::Ok, S::Error>
    where
        S: serde::Serializer,
    {
        serializer.serialize_str(&self.to_string())
    }
}

/// Git operation state for a worktree
///
/// Represents whether a worktree is in the middle of a git operation.
#[derive(Debug, Clone, Copy, PartialEq, Eq, Default, serde::Serialize, strum::IntoStaticStr)]
#[serde(rename_all = "lowercase")]
#[strum(serialize_all = "lowercase")]
pub enum GitOperationState {
    #[strum(serialize = "")]
    #[serde(rename = "")]
    #[default]
    None,
    /// Rebase in progress (rebase-merge or rebase-apply directory exists)
    Rebase,
    /// Merge in progress (MERGE_HEAD exists)
    Merge,
}

/// Tracks which status symbol positions are actually used across all items
/// and the maximum width needed for each position.
///
/// This allows the Status column to:
/// 1. Only allocate space for positions that have data
/// 2. Pad each position to a consistent width for vertical alignment
///
/// Stores maximum character width for each of 8 positions (including user marker).
/// A width of 0 means the position is unused.
#[derive(Debug, Clone, Copy, Default)]
pub struct PositionMask {
    /// Maximum width for each position: [0, 1, 2, 3, 4, 5, 6, 7]
    /// 0 = position unused, >0 = max characters needed
    widths: [usize; 8],
}

impl PositionMask {
    // Render order indices (0-7) - symbols appear in this order left-to-right
    // Working tree split into 3 fixed positions for vertical alignment
    const STAGED: usize = 0; // + (staged changes)
    const MODIFIED: usize = 1; // ! (modified files)
    const UNTRACKED: usize = 2; // ? (untracked files)
    const BRANCH_STATE: usize = 3; // Branch state (conflicts, rebase, merge, integrated, etc.)
    const MAIN_DIVERGENCE: usize = 4;
    const UPSTREAM_DIVERGENCE: usize = 5;
    const WORKTREE_STATE: usize = 6;
    const USER_MARKER: usize = 7;

    /// Full mask with all positions enabled (for JSON output and progressive rendering)
    /// Allocates realistic widths based on common symbol sizes to ensure proper grid alignment
    pub const FULL: Self = Self {
        widths: [
            1, // STAGED: + (1 char)
            1, // MODIFIED: ! (1 char)
            1, // UNTRACKED: ? (1 char)
            1, // BRANCH_STATE: ✘⤴⤵✗_⊂ (1 char, priority: conflicts > rebase > merge > would_conflict > same_commit > integrated)
            1, // MAIN_DIVERGENCE: ^, ↑, ↓, ↕ (1 char)
            1, // UPSTREAM_DIVERGENCE: ⇡, ⇣, ⇅ (1 char)
            1, // WORKTREE_STATE: / for branches, ⚑⊟⊞ for worktrees (priority: path_mismatch > prunable > locked)
            2, // USER_MARKER: single emoji or two chars (allocate 2)
        ],
    };

    /// Get the allocated width for a position
    pub(crate) fn width(&self, pos: usize) -> usize {
        self.widths[pos]
    }
}

/// Structured status symbols for aligned rendering
///
/// Symbols are categorized to enable vertical alignment in table output:
/// - Working tree: +, !, ? (staged, modified, untracked - priority order)
/// - Branch/op state: ✘, ⤴, ⤵, ✗, _, ⊂ (combined position with priority)
/// - Main divergence: ^, ↕, ↑, ↓
/// - Upstream divergence: ⇅, ⇡, ⇣
/// - Worktree state: / for branches, ⚑⊟⊞ for worktrees (priority-only)
/// - User marker: custom labels, emoji
///
/// ## Mutual Exclusivity
///
/// **Combined with priority (branch state + git operation):**
/// Priority: ✘ > ⤴ > ⤵ > ✗ > _ > ⊂
/// - ✘: Actual conflicts (must resolve)
/// - ⤴: Rebase in progress
/// - ⤵: Merge in progress
/// - ✗: Merge-tree conflicts (potential problem)
/// - _: Same commit as main (removable)
/// - ⊂: Content integrated (removable)
///
/// **Mutually exclusive (enforced by type system):**
/// - ^ vs ↕ vs ↑ vs ↓: Main divergence (Divergence with Main context)
/// - ⇅ vs ⇡ vs ⇣: Upstream divergence (Divergence with Upstream context)
///
/// **Priority-only (can co-occur but only highest priority shown):**
/// - ⚑ vs ⊟ vs ⊞: Worktree attrs (priority: path_mismatch ⚑ > prunable ⊟ > locked ⊞)
/// - /: Branch indicator (mutually exclusive with ⚑⊟⊞ as branches can't have worktree attrs)
///
/// **NOT mutually exclusive (can co-occur):**
/// - Working tree symbols (+!?): Can have multiple types of changes
#[derive(Debug, Clone, Default)]
pub struct StatusSymbols {
    /// Branch state (mutually exclusive with priority)
    /// Priority: Conflicts (✘) > Rebase (⤴) > Merge (⤵) > WouldConflict (✗) > SameCommit (_) > Integrated (⊂)
    pub(crate) branch_state: BranchState,

    /// Worktree state: / for branches, ⚑⊟⊞ for worktrees (priority: path_mismatch > prunable > locked)
    pub(crate) worktree_state: WorktreeState,

    /// Main branch divergence state (mutually exclusive)
    pub(crate) main_divergence: Divergence,

    /// Remote/upstream divergence state (mutually exclusive)
    pub(crate) upstream_divergence: Divergence,

    /// Working tree changes (NOT mutually exclusive, can have multiple)
    pub(crate) working_tree: WorkingTreeStatus,

    /// User-defined status annotation (custom labels, e.g., 💬, 🤖)
    pub(crate) user_marker: Option<String>,
}

impl StatusSymbols {
    /// Render symbols with selective alignment based on position mask
    ///
    /// Only includes positions present in the mask. This ensures vertical
    /// scannability - each symbol type appears at the same column position
    /// across all rows, while minimizing wasted space.
    ///
    /// See [`StatusSymbols`] struct doc for symbol categories.
    pub fn render_with_mask(&self, mask: &PositionMask) -> String {
        use worktrunk::styling::StyledLine;

        let mut result = String::with_capacity(64);

        if self.is_empty() {
            return result;
        }

        // Grid-based rendering: each position gets a fixed width for vertical alignment.
        // CRITICAL: Always use PositionMask::FULL for consistent spacing between progressive and final rendering.
        // The mask provides the maximum width needed for each position across all rows.
        // Accept wider Status column with whitespace as tradeoff for perfect alignment.
        for (pos, styled_content, has_data) in self.styled_symbols() {
            let allocated_width = mask.width(pos);

            if has_data {
                // Use StyledLine to handle width calculation (strips ANSI codes automatically)
                let mut segment = StyledLine::new();
                segment.push_raw(styled_content);
                segment.pad_to(allocated_width);
                result.push_str(&segment.render());
            } else {
                // Fill empty position with spaces for alignment
                for _ in 0..allocated_width {
                    result.push(' ');
                }
            }
        }

        result
    }

    /// Check if symbols are empty
    pub fn is_empty(&self) -> bool {
        self.branch_state == BranchState::None
            && self.worktree_state == WorktreeState::None
            && self.main_divergence == Divergence::None
            && self.upstream_divergence == Divergence::None
            && !self.working_tree.is_dirty()
            && self.user_marker.is_none()
    }

    /// Render status symbols in compact form for statusline (no grid alignment).
    ///
    /// Uses the same styled symbols as `render_with_mask()`, just without padding.
    pub fn format_compact(&self) -> String {
        self.styled_symbols()
            .into_iter()
            .filter_map(|(_, styled, has_data)| has_data.then_some(styled))
            .collect()
    }

    /// Build styled symbols array with position indices.
    ///
    /// Returns: `[(position_mask, styled_string, has_data); 8]`
    ///
    /// Order: working_tree (+!?) → branch_op_state → main_divergence → upstream_divergence → worktree_state → user_marker
    ///
    /// Styling follows semantic meaning:
    /// - Cyan: Working tree changes (activity indicator)
    /// - Yellow: Git operations, locked/prunable (stuck states needing attention)
    /// - Dimmed: Branch state symbols + divergence arrows (low urgency)
    fn styled_symbols(&self) -> [(usize, String, bool); 8] {
        use color_print::cformat;

        // Working tree symbols split into 3 fixed columns for vertical alignment
        let style_working = |has: bool, sym: char| -> (String, bool) {
            if has {
                (cformat!("<cyan>{sym}</>"), true)
            } else {
                (String::new(), false)
            }
        };
        let (staged_str, has_staged) = style_working(self.working_tree.staged, '+');
        let (modified_str, has_modified) = style_working(self.working_tree.modified, '!');
        let (untracked_str, has_untracked) = style_working(self.working_tree.untracked, '?');

        let (branch_state_str, has_branch_state) = self
            .branch_state
            .styled()
            .map_or((String::new(), false), |s| (s, true));
        let (main_divergence_str, has_main_divergence) = self
            .main_divergence
            .styled(DivergenceContext::Main)
            .map_or((String::new(), false), |s| (s, true));
        let (upstream_divergence_str, has_upstream_divergence) = self
            .upstream_divergence
            .styled(DivergenceContext::Upstream)
            .map_or((String::new(), false), |s| (s, true));

        let worktree_state_str = match self.worktree_state {
            WorktreeState::None => String::new(),
            // Branch indicator (/) is informational (dimmed)
            WorktreeState::Branch => cformat!("<dim>{}</>", self.worktree_state),
            // Path mismatch (⚑) is a stronger warning (red)
            WorktreeState::PathMismatch => cformat!("<red>{}</>", self.worktree_state),
            // Other worktree attrs (⊟⊞) are warnings (yellow)
            _ => cformat!("<yellow>{}</>", self.worktree_state),
        };

        let user_marker_str = self.user_marker.as_deref().unwrap_or("").to_string();

        // CRITICAL: Display order is working_tree first (staged, modified, untracked), then other symbols.
        // NEVER change this order - it ensures progressive and final rendering match exactly.
        [
            (PositionMask::STAGED, staged_str, has_staged),
            (PositionMask::MODIFIED, modified_str, has_modified),
            (PositionMask::UNTRACKED, untracked_str, has_untracked),
            (
                PositionMask::BRANCH_STATE,
                branch_state_str,
                has_branch_state,
            ),
            (
                PositionMask::MAIN_DIVERGENCE,
                main_divergence_str,
                has_main_divergence,
            ),
            (
                PositionMask::UPSTREAM_DIVERGENCE,
                upstream_divergence_str,
                has_upstream_divergence,
            ),
            (
                PositionMask::WORKTREE_STATE,
                worktree_state_str,
                self.worktree_state != WorktreeState::None,
            ),
            (
                PositionMask::USER_MARKER,
                user_marker_str,
                self.user_marker.is_some(),
            ),
        ]
    }
}

/// Working tree changes as structured booleans
///
/// This is the canonical internal representation. Display strings are derived from this.
#[derive(Debug, Clone, Copy, Default, serde::Serialize)]
pub struct WorkingTreeStatus {
    pub staged: bool,
    pub modified: bool,
    pub untracked: bool,
    pub renamed: bool,
    pub deleted: bool,
}

impl WorkingTreeStatus {
    /// Create from git status parsing results
    pub fn new(
        staged: bool,
        modified: bool,
        untracked: bool,
        renamed: bool,
        deleted: bool,
    ) -> Self {
        Self {
            staged,
            modified,
            untracked,
            renamed,
            deleted,
        }
    }

    /// Returns true if any changes are present
    pub fn is_dirty(&self) -> bool {
        self.staged || self.modified || self.untracked || self.renamed || self.deleted
    }

    /// Format as display string for JSON serialization and raw output (e.g., "+!?").
    ///
    /// For styled terminal rendering, use `StatusSymbols::styled_symbols()` instead.
    pub fn to_symbols(self) -> String {
        let mut s = String::with_capacity(5);
        if self.staged {
            s.push('+');
        }
        if self.modified {
            s.push('!');
        }
        if self.untracked {
            s.push('?');
        }
        if self.renamed {
            s.push('»');
        }
        if self.deleted {
            s.push('✘');
        }
        s
    }
}

/// Helper for serde skip_serializing_if
fn git_operation_is_none(state: &GitOperationState) -> bool {
    *state == GitOperationState::None
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_working_tree_status_is_dirty() {
        // Empty status is not dirty
        assert!(!WorkingTreeStatus::default().is_dirty());

        // Each flag individually makes it dirty
        assert!(WorkingTreeStatus::new(true, false, false, false, false).is_dirty());
        assert!(WorkingTreeStatus::new(false, true, false, false, false).is_dirty());
        assert!(WorkingTreeStatus::new(false, false, true, false, false).is_dirty());
        assert!(WorkingTreeStatus::new(false, false, false, true, false).is_dirty());
        assert!(WorkingTreeStatus::new(false, false, false, false, true).is_dirty());

        // Multiple flags
        assert!(WorkingTreeStatus::new(true, true, true, true, true).is_dirty());
    }

    #[test]
    fn test_working_tree_status_to_symbols() {
        // Empty
        assert_eq!(WorkingTreeStatus::default().to_symbols(), "");

        // Individual symbols
        assert_eq!(
            WorkingTreeStatus::new(true, false, false, false, false).to_symbols(),
            "+"
        );
        assert_eq!(
            WorkingTreeStatus::new(false, true, false, false, false).to_symbols(),
            "!"
        );
        assert_eq!(
            WorkingTreeStatus::new(false, false, true, false, false).to_symbols(),
            "?"
        );
        assert_eq!(
            WorkingTreeStatus::new(false, false, false, true, false).to_symbols(),
            "»"
        );
        assert_eq!(
            WorkingTreeStatus::new(false, false, false, false, true).to_symbols(),
            "✘"
        );

        // Combined symbols (order: staged, modified, untracked, renamed, deleted)
        assert_eq!(
            WorkingTreeStatus::new(true, true, false, false, false).to_symbols(),
            "+!"
        );
        assert_eq!(
            WorkingTreeStatus::new(true, true, true, false, false).to_symbols(),
            "+!?"
        );
        assert_eq!(
            WorkingTreeStatus::new(true, true, true, true, true).to_symbols(),
            "+!?»✘"
        );
    }

    #[test]
    fn test_branch_state_as_json_str() {
        assert_eq!(BranchState::None.as_json_str(), None);
        assert_eq!(BranchState::Conflicts.as_json_str(), Some("conflicts"));
        assert_eq!(BranchState::Rebase.as_json_str(), Some("rebase"));
        assert_eq!(BranchState::Merge.as_json_str(), Some("merge"));
        assert_eq!(
            BranchState::WouldConflict.as_json_str(),
            Some("would_conflict")
        );
        assert_eq!(BranchState::SameCommit.as_json_str(), Some("same_commit"));
        assert_eq!(
            BranchState::Integrated(IntegrationReason::TreesMatch).as_json_str(),
            Some("integrated")
        );
    }

    #[test]
    fn test_integration_reason_as_json_str() {
        assert_eq!(IntegrationReason::SameCommit.as_json_str(), "same_commit");
        assert_eq!(IntegrationReason::Ancestor.as_json_str(), "ancestor");
        assert_eq!(IntegrationReason::TreesMatch.as_json_str(), "trees_match");
        assert_eq!(
            IntegrationReason::NoAddedChanges.as_json_str(),
            "no_added_changes"
        );
        assert_eq!(
            IntegrationReason::MergeAddsNothing.as_json_str(),
            "merge_adds_nothing"
        );
    }

    #[test]
    fn test_branch_state_integration_reason() {
        // Non-integrated states return None
        assert_eq!(BranchState::None.integration_reason(), None);
        assert_eq!(BranchState::Conflicts.integration_reason(), None);
        assert_eq!(BranchState::SameCommit.integration_reason(), None);

        // Integrated states return the reason
        assert_eq!(
            BranchState::Integrated(IntegrationReason::Ancestor).integration_reason(),
            Some(IntegrationReason::Ancestor)
        );
        assert_eq!(
            BranchState::Integrated(IntegrationReason::TreesMatch).integration_reason(),
            Some(IntegrationReason::TreesMatch)
        );
        assert_eq!(
            BranchState::Integrated(IntegrationReason::NoAddedChanges).integration_reason(),
            Some(IntegrationReason::NoAddedChanges)
        );
        assert_eq!(
            BranchState::Integrated(IntegrationReason::MergeAddsNothing).integration_reason(),
            Some(IntegrationReason::MergeAddsNothing)
        );
    }
}
