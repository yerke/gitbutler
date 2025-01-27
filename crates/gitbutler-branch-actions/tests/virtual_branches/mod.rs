use std::{fs, path, path::PathBuf, str::FromStr};

use gitbutler_branch::BranchCreateRequest;
use gitbutler_branch_actions::VirtualBranchActions;
use gitbutler_error::error::Marker;
use gitbutler_project::{self as projects, Project, ProjectId};
use gitbutler_reference::Refname;
use gitbutler_testsupport::{paths, TestProject, VAR_NO_CLEANUP};
use tempfile::TempDir;

struct Test {
    repository: TestProject,
    project_id: ProjectId,
    project: Project,
    projects: projects::Controller,
    controller: VirtualBranchActions,
    data_dir: Option<TempDir>,
}

impl Drop for Test {
    fn drop(&mut self) {
        if std::env::var_os(VAR_NO_CLEANUP).is_some() {
            let _ = self.data_dir.take().unwrap().into_path();
        }
    }
}

impl Default for Test {
    fn default() -> Self {
        let data_dir = paths::data_dir();
        let projects = projects::Controller::from_path(data_dir.path());

        let test_project = TestProject::default();
        let project = projects
            .add(test_project.path())
            .expect("failed to add project");

        Self {
            repository: test_project,
            project_id: project.id,
            controller: VirtualBranchActions {},
            projects,
            project,
            data_dir: Some(data_dir),
        }
    }
}

impl Test {
    /// Consume this instance and keep the temp directory that held the local repository, returning it.
    /// Best used inside a `dbg!(test.debug_local_repo())`
    #[allow(dead_code)]
    pub fn debug_local_repo(mut self) -> PathBuf {
        let repo = std::mem::take(&mut self.repository);
        repo.debug_local_repo()
    }
}

mod amend;
mod apply_virtual_branch;
mod convert_to_real_branch;
mod create_commit;
mod create_virtual_branch_from_branch;
mod delete_virtual_branch;
mod init;
mod insert_blank_commit;
mod list;
mod list_details;
mod move_commit_file;
mod move_commit_to_vbranch;
mod oplog;
mod references;
mod reorder_commit;
mod reset_virtual_branch;
mod selected_for_changes;
mod set_base_branch;
mod squash;
mod unapply_ownership;
mod undo_commit;
mod update_base_branch;
mod update_commit_message;
mod upstream;
mod verify_branch;

#[test]
fn resolve_conflict_flow() {
    let Test {
        repository,
        project,
        controller,
        ..
    } = &Test::default();

    // make sure we have an undiscovered commit in the remote branch
    {
        fs::write(repository.path().join("file.txt"), "first").unwrap();
        let first_commit_oid = repository.commit_all("first");
        fs::write(repository.path().join("file.txt"), "second").unwrap();
        repository.commit_all("second");
        fs::write(repository.path().join("third.txt"), "three").unwrap();
        repository.commit_all("third");
        repository.push();
        repository.reset_hard(Some(first_commit_oid));
    }

    controller
        .set_base_branch(project, &"refs/remotes/origin/master".parse().unwrap())
        .unwrap();

    {
        // make a branch that conflicts with the remote branch, but doesn't know about it yet
        let branch1_id = controller
            .create_virtual_branch(project, &BranchCreateRequest::default())
            .unwrap();
        fs::write(repository.path().join("file.txt"), "conflict").unwrap();

        let (branches, _) = controller.list_virtual_branches(project).unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].id, branch1_id);
        assert!(branches[0].active);
    };

    let unapplied_branch = {
        // fetch remote. There is now a conflict, so the branch will be unapplied
        let unapplied_branches = controller.update_base_branch(project).unwrap();
        assert_eq!(unapplied_branches.len(), 1);

        // there is a conflict now, so the branch should be inactive
        let (branches, _) = controller.list_virtual_branches(project).unwrap();
        assert_eq!(branches.len(), 0);

        Refname::from_str(&unapplied_branches[0]).unwrap()
    };

    let branch1_id = {
        // when we apply conflicted branch, it has conflict
        let branch1_id = controller
            .create_virtual_branch_from_branch(project, &unapplied_branch, None)
            .unwrap();

        let (branches, _) = controller.list_virtual_branches(project).unwrap();
        assert_eq!(branches.len(), 1);
        assert!(branches[0].active);
        assert!(branches[0].conflicted);
        assert_eq!(branches[0].files.len(), 2); // third.txt should be present during conflict

        // and the conflict markers are in the file
        assert_eq!(
            fs::read_to_string(repository.path().join("file.txt")).unwrap(),
            "<<<<<<< ours\nconflict\n=======\nsecond\n>>>>>>> theirs\n"
        );

        branch1_id
    };

    {
        // can't commit conflicts
        assert!(matches!(
            controller
                .create_commit(project, branch1_id, "commit conflicts", None, false)
                .unwrap_err()
                .downcast_ref(),
            Some(Marker::ProjectConflict)
        ));
    }

    {
        // fixing the conflict removes conflicted mark
        fs::write(repository.path().join("file.txt"), "resolved").unwrap();
        controller.list_virtual_branches(project).unwrap();
        let commit_oid = controller
            .create_commit(project, branch1_id, "resolution", None, false)
            .unwrap();

        let commit = repository.find_commit(commit_oid).unwrap();
        assert_eq!(commit.parent_count(), 2);

        let (branches, _) = controller.list_virtual_branches(project).unwrap();
        assert_eq!(branches.len(), 1);
        assert_eq!(branches[0].id, branch1_id);
        assert!(branches[0].active);
        assert!(!branches[0].conflicted);
    }
}
