use gpui::{TestAppContext, VisualTestContext};
use picker::Picker;
use project::Project;
use serde_json::json;
use workspace::{AppState, Workspace};

use super::*;

#[ctor::ctor]
fn init_logger() {
    zlog::init_test();
}

fn init_test(cx: &mut TestAppContext) -> Arc<AppState> {
    cx.update(|cx| {
        let state = AppState::test(cx);
        theme::init(theme::LoadThemes::JustBase, cx);
        super::init(cx);
        state
    })
}

fn build_directory_browser(
    project: Entity<Project>,
    cx: &mut TestAppContext,
) -> (
    Entity<Picker<DirectoryBrowserDelegate>>,
    Entity<Workspace>,
    &mut VisualTestContext,
) {
    let (workspace, cx) = cx.add_window_view(|window, cx| Workspace::test_new(project, window, cx));
    let picker = open_directory_browser(&workspace, cx);
    (picker, workspace, cx)
}

#[track_caller]
fn open_directory_browser(
    workspace: &Entity<Workspace>,
    cx: &mut VisualTestContext,
) -> Entity<Picker<DirectoryBrowserDelegate>> {
    cx.dispatch_action(Toggle);
    workspace.update(cx, |workspace, cx| {
        workspace
            .active_modal::<DirectoryBrowser>(cx)
            .expect("directory browser is not open")
            .read(cx)
            .picker
            .clone()
    })
}

#[gpui::test]
async fn test_directory_listing(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            "/root",
            json!({
                "src": {
                    "main.rs": "",
                    "lib.rs": "",
                },
                "README.md": "",
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), ["/root".as_ref()], cx).await;
    let (picker, _workspace, cx) = build_directory_browser(project, cx);

    picker.update(cx, |picker, _| {
        let entries: Vec<_> = picker
            .delegate
            .filtered_entries
            .iter()
            .map(|e| e.display_name())
            .collect();
        // No parent directory at worktree root
        assert_eq!(entries, vec!["src", "README.md"]);
    });
}

#[gpui::test]
async fn test_navigate_into_directory(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            "/root",
            json!({
                "src": {
                    "main.rs": "",
                    "lib.rs": "",
                },
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), ["/root".as_ref()], cx).await;
    let (picker, _workspace, cx) = build_directory_browser(project, cx);

    // Select "src" (index 0, no parent directory at worktree root)
    picker.update_in(cx, |picker, window, cx| {
        picker.delegate.set_selected_index(0, window, cx);
    });

    cx.dispatch_action(menu::Confirm);

    picker.update(cx, |picker, _| {
        let entries: Vec<_> = picker
            .delegate
            .filtered_entries
            .iter()
            .map(|e| e.display_name())
            .collect();
        // Now in subdirectory, parent directory entry appears
        assert_eq!(entries, vec!["..", "lib.rs", "main.rs"]);
    });
}

#[gpui::test]
async fn test_navigate_to_parent(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            "/root",
            json!({
                "src": {
                    "main.rs": "",
                },
                "README.md": "",
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), ["/root".as_ref()], cx).await;
    let (picker, _workspace, cx) = build_directory_browser(project, cx);

    // Select "src" (index 0, no parent directory at worktree root)
    picker.update_in(cx, |picker, window, cx| {
        picker.delegate.set_selected_index(0, window, cx);
    });

    cx.dispatch_action(menu::Confirm);

    // Now in subdirectory, parent directory entry appears
    picker.update(cx, |picker, _| {
        assert!(picker
            .delegate
            .filtered_entries
            .iter()
            .any(|e| e.display_name() == ".."));
    });

    cx.dispatch_action(NavigateToParent);

    // Back at worktree root, no parent directory
    picker.update(cx, |picker, _| {
        let entries: Vec<_> = picker
            .delegate
            .filtered_entries
            .iter()
            .map(|e| e.display_name())
            .collect();
        assert_eq!(entries, vec!["src", "README.md"]);
    });
}

#[gpui::test]
async fn test_hidden_files_toggle(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            "/root",
            json!({
                ".hidden": "",
                "visible.txt": "",
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), ["/root".as_ref()], cx).await;
    let (picker, _workspace, cx) = build_directory_browser(project, cx);

    picker.update(cx, |picker, _| {
        let entries: Vec<_> = picker
            .delegate
            .filtered_entries
            .iter()
            .map(|e| e.display_name())
            .collect();
        // No parent directory at worktree root
        assert_eq!(entries, vec!["visible.txt"]);
        assert!(!picker.delegate.show_hidden_files);
    });

    cx.dispatch_action(ToggleShowHiddenFiles);

    picker.update(cx, |picker, _| {
        let entries: Vec<_> = picker
            .delegate
            .filtered_entries
            .iter()
            .map(|e| e.display_name())
            .collect();
        assert_eq!(entries, vec![".hidden", "visible.txt"]);
        assert!(picker.delegate.show_hidden_files);
    });

    cx.dispatch_action(ToggleShowHiddenFiles);

    picker.update(cx, |picker, _| {
        let entries: Vec<_> = picker
            .delegate
            .filtered_entries
            .iter()
            .map(|e| e.display_name())
            .collect();
        assert_eq!(entries, vec!["visible.txt"]);
    });
}

#[gpui::test]
async fn test_search_filtering(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            "/root",
            json!({
                "apple.txt": "",
                "banana.txt": "",
                "cherry.txt": "",
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), ["/root".as_ref()], cx).await;
    let (picker, _workspace, cx) = build_directory_browser(project, cx);

    picker.update(cx, |picker, _| {
        // 3 entries: just the 3 files (no parent directory at worktree root)
        assert_eq!(picker.delegate.filtered_entries.len(), 3);
    });

    cx.simulate_input("ban");

    picker.update(cx, |picker, _| {
        let entries: Vec<_> = picker
            .delegate
            .filtered_entries
            .iter()
            .map(|e| e.display_name())
            .collect();
        assert_eq!(entries, vec!["banana.txt"]);
    });

    picker.update_in(cx, |picker, window, cx| {
        picker.set_query("", window, cx);
    });

    picker.update(cx, |picker, _| {
        assert_eq!(picker.delegate.filtered_entries.len(), 3);
    });
}

#[gpui::test]
async fn test_directories_sorted_before_files(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            "/root",
            json!({
                "aaa_file.txt": "",
                "zzz_dir": {
                    "nested.txt": "",
                },
                "bbb_file.txt": "",
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), ["/root".as_ref()], cx).await;
    let (picker, _workspace, cx) = build_directory_browser(project, cx);

    picker.update(cx, |picker, _| {
        let entries: Vec<_> = picker
            .delegate
            .filtered_entries
            .iter()
            .map(|e| e.display_name())
            .collect();
        // No parent directory at worktree root
        assert_eq!(entries, vec!["zzz_dir", "aaa_file.txt", "bbb_file.txt"]);
    });
}

#[gpui::test]
async fn test_empty_directory(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            "/root",
            json!({
                "empty_dir": {},
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), ["/root".as_ref()], cx).await;
    let (picker, _workspace, cx) = build_directory_browser(project, cx);

    // Select "empty_dir" (index 0, no parent directory at worktree root)
    picker.update_in(cx, |picker, window, cx| {
        picker.delegate.set_selected_index(0, window, cx);
    });

    cx.dispatch_action(menu::Confirm);

    picker.update(cx, |picker, _| {
        let entries: Vec<_> = picker
            .delegate
            .filtered_entries
            .iter()
            .map(|e| e.display_name())
            .collect();
        // Now in subdirectory, parent directory entry appears
        assert_eq!(entries, vec![".."]);
    });
}

#[gpui::test]
async fn test_case_insensitive_search(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            "/root",
            json!({
                "MyFile.txt": "",
                "another.txt": "",
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), ["/root".as_ref()], cx).await;
    let (picker, _workspace, cx) = build_directory_browser(project, cx);

    cx.simulate_input("myfile");

    picker.update(cx, |picker, _| {
        let entries: Vec<_> = picker
            .delegate
            .filtered_entries
            .iter()
            .map(|e| e.display_name())
            .collect();
        assert_eq!(entries, vec!["MyFile.txt"]);
    });
}
