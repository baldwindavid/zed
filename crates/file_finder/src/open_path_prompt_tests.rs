use std::sync::Arc;

use gpui::{AppContext, Entity, TestAppContext, VisualTestContext};
use picker::{Picker, PickerDelegate};
use project::Project;
use serde_json::json;
use ui::rems;
use util::{path, paths::PathStyle};
use workspace::{AppState, Workspace};

use crate::OpenPathDelegate;

#[gpui::test]
async fn test_open_path_prompt(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            path!("/root"),
            json!({
                "a1": "A1",
                "a2": "A2",
                "a3": "A3",
                "dir1": {},
                "dir2": {
                    "c": "C",
                    "d1": "D1",
                    "d2": "D2",
                    "d3": "D3",
                    "dir3": {},
                    "dir4": {}
                }
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), [path!("/root").as_ref()], cx).await;

    let (picker, cx) = build_open_path_prompt(project, false, PathStyle::local(), cx);

    insert_query(path!("sadjaoislkdjasldj"), &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), Vec::<String>::new());

    let query = path!("/root");
    insert_query(query, &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["root"]);

    #[cfg(not(windows))]
    let expected_separator = "./";
    #[cfg(windows)]
    let expected_separator = ".\\";

    // If the query ends with a slash, the picker should show the contents of the directory.
    let query = path!("/root/");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        collect_match_candidates(&picker, cx),
        vec![expected_separator, "a1", "a2", "a3", "dir1", "dir2"]
    );

    // Show candidates for the query "a".
    let query = path!("/root/a");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        collect_match_candidates(&picker, cx),
        vec!["a1", "a2", "a3"]
    );

    // Show candidates for the query "d".
    let query = path!("/root/d");
    insert_query(query, &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["dir1", "dir2"]);

    let query = path!("/root/dir2");
    insert_query(query, &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["dir2"]);

    let query = path!("/root/dir2/");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        collect_match_candidates(&picker, cx),
        vec![expected_separator, "c", "d1", "d2", "d3", "dir3", "dir4"]
    );

    // Show candidates for the query "d".
    let query = path!("/root/dir2/d");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        collect_match_candidates(&picker, cx),
        vec!["d1", "d2", "d3", "dir3", "dir4"]
    );

    let query = path!("/root/dir2/di");
    insert_query(query, &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["dir3", "dir4"]);
}

#[gpui::test]
async fn test_open_path_prompt_completion(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            path!("/root"),
            json!({
                "a": "A",
                "dir1": {},
                "dir2": {
                    "c": "C",
                    "d": "D",
                    "dir3": {},
                    "dir4": {}
                }
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), [path!("/root").as_ref()], cx).await;

    let (picker, cx) = build_open_path_prompt(project, false, PathStyle::local(), cx);

    // Confirm completion for the query "/root", since it's a directory, it should add a trailing slash.
    let query = path!("/root");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        confirm_completion(query, 0, &picker, cx).unwrap(),
        path!("/root/")
    );

    // Confirm completion for the query "/root/", selecting the first candidate "a", since it's a file, it should not add a trailing slash.
    let query = path!("/root/");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        confirm_completion(query, 0, &picker, cx),
        None,
        "First entry is `./` and when we confirm completion, it is tabbed below"
    );
    assert_eq!(
        confirm_completion(query, 1, &picker, cx).unwrap(),
        path!("/root/a"),
        "Second entry is the first entry of a directory that we want to be completed"
    );

    // Confirm completion for the query "/root/", selecting the second candidate "dir1", since it's a directory, it should add a trailing slash.
    let query = path!("/root/");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        confirm_completion(query, 2, &picker, cx).unwrap(),
        path!("/root/dir1/")
    );

    let query = path!("/root/a");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        confirm_completion(query, 0, &picker, cx).unwrap(),
        path!("/root/a")
    );

    let query = path!("/root/d");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        confirm_completion(query, 1, &picker, cx).unwrap(),
        path!("/root/dir2/")
    );

    let query = path!("/root/dir2");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        confirm_completion(query, 0, &picker, cx).unwrap(),
        path!("/root/dir2/")
    );

    let query = path!("/root/dir2/");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        confirm_completion(query, 1, &picker, cx).unwrap(),
        path!("/root/dir2/c")
    );

    let query = path!("/root/dir2/");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        confirm_completion(query, 3, &picker, cx).unwrap(),
        path!("/root/dir2/dir3/")
    );

    let query = path!("/root/dir2/d");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        confirm_completion(query, 0, &picker, cx).unwrap(),
        path!("/root/dir2/d")
    );

    let query = path!("/root/dir2/d");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        confirm_completion(query, 1, &picker, cx).unwrap(),
        path!("/root/dir2/dir3/")
    );

    let query = path!("/root/dir2/di");
    insert_query(query, &picker, cx).await;
    assert_eq!(
        confirm_completion(query, 1, &picker, cx).unwrap(),
        path!("/root/dir2/dir4/")
    );
}

#[gpui::test]
#[cfg_attr(not(target_os = "windows"), ignore)]
async fn test_open_path_prompt_on_windows(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            path!("/root"),
            json!({
                "a": "A",
                "dir1": {},
                "dir2": {}
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), [path!("/root").as_ref()], cx).await;

    let (picker, cx) = build_open_path_prompt(project, false, PathStyle::local(), cx);

    // Support both forward and backward slashes.
    let query = "C:/root/";
    insert_query(query, &picker, cx).await;
    assert_eq!(
        collect_match_candidates(&picker, cx),
        vec![".\\", "a", "dir1", "dir2"]
    );
    assert_eq!(
        confirm_completion(query, 0, &picker, cx),
        None,
        "First entry is `.\\` and when we confirm completion, it is tabbed below"
    );
    assert_eq!(
        confirm_completion(query, 1, &picker, cx).unwrap(),
        "C:/root/a",
        "Second entry is the first entry of a directory that we want to be completed"
    );

    let query = "C:\\root/";
    insert_query(query, &picker, cx).await;
    assert_eq!(
        collect_match_candidates(&picker, cx),
        vec![".\\", "a", "dir1", "dir2"]
    );
    assert_eq!(
        confirm_completion(query, 1, &picker, cx).unwrap(),
        "C:\\root/a"
    );

    let query = "C:\\root\\";
    insert_query(query, &picker, cx).await;
    assert_eq!(
        collect_match_candidates(&picker, cx),
        vec![".\\", "a", "dir1", "dir2"]
    );
    assert_eq!(
        confirm_completion(query, 1, &picker, cx).unwrap(),
        "C:\\root\\a"
    );

    // Confirm completion for the query "C:/root/d", selecting the second candidate "dir2", since it's a directory, it should add a trailing slash.
    let query = "C:/root/d";
    insert_query(query, &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["dir1", "dir2"]);
    assert_eq!(
        confirm_completion(query, 1, &picker, cx).unwrap(),
        "C:/root/dir2\\"
    );

    let query = "C:\\root/d";
    insert_query(query, &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["dir1", "dir2"]);
    assert_eq!(
        confirm_completion(query, 0, &picker, cx).unwrap(),
        "C:\\root/dir1\\"
    );

    let query = "C:\\root\\d";
    insert_query(query, &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["dir1", "dir2"]);
    assert_eq!(
        confirm_completion(query, 0, &picker, cx).unwrap(),
        "C:\\root\\dir1\\"
    );
}

#[gpui::test]
#[cfg_attr(not(target_os = "windows"), ignore)]
async fn test_open_path_prompt_on_windows_with_remote(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            "/root",
            json!({
                "a": "A",
                "dir1": {},
                "dir2": {}
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), ["/root".as_ref()], cx).await;

    let (picker, cx) = build_open_path_prompt(project, false, PathStyle::Posix, cx);

    let query = "/root/";
    insert_query(query, &picker, cx).await;
    assert_eq!(
        collect_match_candidates(&picker, cx),
        vec!["./", "a", "dir1", "dir2"]
    );
    assert_eq!(
        confirm_completion(query, 1, &picker, cx).unwrap(),
        "/root/a"
    );

    // Confirm completion for the query "/root/d", selecting the second candidate "dir2", since it's a directory, it should add a trailing slash.
    let query = "/root/d";
    insert_query(query, &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["dir1", "dir2"]);
    assert_eq!(
        confirm_completion(query, 1, &picker, cx).unwrap(),
        "/root/dir2/"
    );

    let query = "/root/d";
    insert_query(query, &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["dir1", "dir2"]);
    assert_eq!(
        confirm_completion(query, 0, &picker, cx).unwrap(),
        "/root/dir1/"
    );
}

#[gpui::test]
async fn test_new_path_prompt(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            path!("/root"),
            json!({
                "a1": "A1",
                "a2": "A2",
                "a3": "A3",
                "dir1": {},
                "dir2": {
                    "c": "C",
                    "d1": "D1",
                    "d2": "D2",
                    "d3": "D3",
                    "dir3": {},
                    "dir4": {}
                }
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), [path!("/root").as_ref()], cx).await;

    let (picker, cx) = build_open_path_prompt(project, true, PathStyle::local(), cx);

    insert_query(path!("/root"), &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["root"]);

    insert_query(path!("/root/d"), &picker, cx).await;
    assert_eq!(
        collect_match_candidates(&picker, cx),
        vec!["d", "dir1", "dir2"]
    );

    insert_query(path!("/root/dir1"), &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["dir1"]);

    insert_query(path!("/root/dir12"), &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["dir12"]);

    insert_query(path!("/root/dir1"), &picker, cx).await;
    assert_eq!(collect_match_candidates(&picker, cx), vec!["dir1"]);
}

#[gpui::test]
async fn test_open_path_prompt_with_preselect(cx: &mut TestAppContext) {
    let app_state = init_test(cx);
    app_state
        .fs
        .as_fake()
        .insert_tree(
            path!("/root"),
            json!({
                "a1": "A1",
                "a2": "A2",
                "a3": "A3",
                "dir1": {},
                "dir2": {}
            }),
        )
        .await;

    let project = Project::test(app_state.fs.clone(), [path!("/root").as_ref()], cx).await;

    // Test preselect with DirectoryLister::Local
    let (picker, cx) =
        build_open_path_prompt_with_preselect(project, std::path::Path::new(path!("/root/a2")), cx);

    // The query should be the parent directory with trailing separator
    let query = picker.update(cx, |p, cx| p.query(cx));
    assert!(
        query.ends_with('/') || query.ends_with('\\'),
        "Query should end with separator: {query}"
    );

    // Wait for matches to load
    insert_query(&query, &picker, cx).await;

    // The candidates should show all files in the directory
    let candidates = collect_match_candidates(&picker, cx);
    assert!(
        candidates.contains(&"a1".to_string()),
        "Should contain a1: {candidates:?}"
    );
    assert!(
        candidates.contains(&"a2".to_string()),
        "Should contain a2: {candidates:?}"
    );

    // The selected index should be the preselected file "a2"
    let selected_index = picker.update(cx, |p, _| p.delegate.selected_index());
    let selected_candidate = candidates.get(selected_index);
    assert_eq!(
        selected_candidate,
        Some(&"a2".to_string()),
        "Selected should be 'a2', got index {selected_index} which is {selected_candidate:?}"
    );
}

fn init_test(cx: &mut TestAppContext) -> Arc<AppState> {
    cx.update(|cx| {
        let state = AppState::test(cx);
        theme::init(theme::LoadThemes::JustBase, cx);
        super::init(cx);
        editor::init(cx);
        state
    })
}

fn build_open_path_prompt(
    project: Entity<Project>,
    creating_path: bool,
    path_style: PathStyle,
    cx: &mut TestAppContext,
) -> (Entity<Picker<OpenPathDelegate>>, &mut VisualTestContext) {
    let (tx, _) = futures::channel::oneshot::channel();
    let lister = project::DirectoryLister::Project(project.clone());
    let delegate = OpenPathDelegate::new(tx, lister.clone(), creating_path, path_style);

    let (workspace, cx) = cx.add_window_view(|window, cx| Workspace::test_new(project, window, cx));
    (
        workspace.update_in(cx, |_, window, cx| {
            cx.new(|cx| {
                let picker = Picker::uniform_list(delegate, window, cx)
                    .width(rems(34.))
                    .modal(false);
                let query = lister.default_query(cx);
                picker.set_query(query, window, cx);
                picker
            })
        }),
        cx,
    )
}

fn build_open_path_prompt_with_preselect<'a>(
    project: Entity<Project>,
    initial_path: &std::path::Path,
    cx: &'a mut TestAppContext,
) -> (Entity<Picker<OpenPathDelegate>>, &'a mut VisualTestContext) {
    let (tx, _) = futures::channel::oneshot::channel();
    let fs = project.read_with(cx, |p, _| p.fs().clone());
    let lister =
        project::DirectoryLister::Local(project.clone(), fs, Some(initial_path.to_path_buf()));
    let preselect = lister.preselect_filename();
    let mut delegate = OpenPathDelegate::new(tx, lister.clone(), false, PathStyle::local());
    if let Some(filename) = preselect {
        delegate = delegate.with_preselect(filename);
    }

    let (workspace, cx) = cx.add_window_view(|window, cx| Workspace::test_new(project, window, cx));
    (
        workspace.update_in(cx, |_, window, cx| {
            cx.new(|cx| {
                let picker = Picker::uniform_list(delegate, window, cx)
                    .width(rems(34.))
                    .modal(false);
                let query = lister.default_query(cx);
                picker.set_query(query, window, cx);
                picker
            })
        }),
        cx,
    )
}

async fn insert_query(
    query: &str,
    picker: &Entity<Picker<OpenPathDelegate>>,
    cx: &mut VisualTestContext,
) {
    picker
        .update_in(cx, |f, window, cx| {
            f.delegate.update_matches(query.to_string(), window, cx)
        })
        .await;
}

fn confirm_completion(
    query: &str,
    select: usize,
    picker: &Entity<Picker<OpenPathDelegate>>,
    cx: &mut VisualTestContext,
) -> Option<String> {
    picker.update_in(cx, |f, window, cx| {
        if f.delegate.selected_index() != select {
            f.delegate.set_selected_index(select, window, cx);
        }
        f.delegate.confirm_completion(query.to_string(), window, cx)
    })
}

fn collect_match_candidates(
    picker: &Entity<Picker<OpenPathDelegate>>,
    cx: &mut VisualTestContext,
) -> Vec<String> {
    picker.update(cx, |f, _| f.delegate.collect_match_candidates())
}
