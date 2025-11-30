#[cfg(test)]
mod directory_browser_tests;

use file_icons::FileIcons;
use gpui::{
    actions, px, Action, AnyElement, App, Context, DismissEvent, Entity, EntityId, EventEmitter,
    FocusHandle, Focusable, ParentElement, Render, Styled, Task, WeakEntity, Window,
};
use picker::{Picker, PickerDelegate};
use project::{Project, ProjectPath, WorktreeId};
use settings::Settings;
use std::{path::Path, sync::Arc};
use ui::{
    prelude::*, Button, ContextMenu, Icon, IconButton, IconName, IconSize, Indicator, KeyBinding,
    Label, ListItem, ListItemSpacing, PopoverMenu, PopoverMenuHandle, TintColor, Tooltip,
};
use util::{ResultExt, paths::PathStyle, rel_path::RelPath};
use workspace::{ModalView, Pane, SplitDirection, Workspace, item::PreviewTabsSettings, pane};
use worktree::Entry;

actions!(
    directory_browser,
    [
        Toggle,
        NavigateToParent,
        ToggleFilterMenu,
        ToggleShowHiddenFiles,
        ToggleSplitMenu,
    ]
);

impl ModalView for DirectoryBrowser {
    fn on_before_dismiss(
        &mut self,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> workspace::DismissDecision {
        let submenu_focused = self.picker.update(cx, |picker, cx| {
            picker
                .delegate
                .filter_popover_menu_handle
                .is_focused(window, cx)
                || picker
                    .delegate
                    .split_popover_menu_handle
                    .is_focused(window, cx)
        });
        workspace::DismissDecision::Dismiss(!submenu_focused)
    }
}

pub struct DirectoryBrowser {
    picker: Entity<Picker<DirectoryBrowserDelegate>>,
}

pub fn init(cx: &mut App) {
    cx.observe_new(DirectoryBrowser::register).detach();
}

impl DirectoryBrowser {
    fn register(workspace: &mut Workspace, _window: Option<&mut Window>, _: &mut Context<Workspace>) {
        workspace.register_action(|workspace, _: &Toggle, window, cx| {
            if workspace.active_modal::<Self>(cx).is_some() {
                return;
            }
            Self::open(workspace, window, cx);
        });
    }

    fn new(
        workspace: WeakEntity<Workspace>,
        project: Entity<Project>,
        worktree_id: WorktreeId,
        current_path: Arc<RelPath>,
        initial_selected_path: Option<String>,
        original_pane: WeakEntity<Pane>,
        original_active_item_id: Option<EntityId>,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) -> Self {
        let directory_browser = cx.entity().downgrade();

        let picker = cx.new(|cx| {
            let focus_handle = cx.focus_handle();
            let delegate = DirectoryBrowserDelegate::new(
                directory_browser,
                workspace,
                project,
                worktree_id,
                current_path,
                initial_selected_path,
                original_pane,
                original_active_item_id,
                focus_handle,
            );
            Picker::uniform_list(delegate, window, cx).max_height(Some(rems(40.).into()))
        });
        picker.update(cx, |picker, cx| {
            picker.delegate.load_entries(cx);
        });

        Self { picker }
    }

    fn open(workspace: &mut Workspace, window: &mut Window, cx: &mut Context<Workspace>) {
        let project = workspace.project().clone();

        let active_item = workspace.active_item(cx);

        let (worktree_id, current_path, initial_selected_path) = if let Some(item) = &active_item {
            if let Some(project_path) = item.project_path(cx) {
                let parent = project_path
                    .path
                    .parent()
                    .map(|p| p.to_owned().into())
                    .unwrap_or_else(|| RelPath::empty().to_owned().into());
                let file_name = project_path
                    .path
                    .file_name()
                    .map(|s| s.to_string());
                (project_path.worktree_id, parent, file_name)
            } else {
                return;
            }
        } else if let Some(worktree) = project.read(cx).worktrees(cx).next() {
            let worktree = worktree.read(cx);
            (worktree.id(), RelPath::empty().to_owned().into(), None)
        } else {
            return;
        };

        let weak_workspace = cx.entity().downgrade();
        let original_pane = workspace.active_pane().downgrade();
        let original_active_item_id = workspace
            .active_pane()
            .read(cx)
            .active_item()
            .map(|item| item.item_id());

        workspace.toggle_modal(window, cx, |window, cx| {
            DirectoryBrowser::new(
                weak_workspace,
                project,
                worktree_id,
                current_path,
                initial_selected_path,
                original_pane,
                original_active_item_id,
                window,
                cx,
            )
        });
    }

    fn handle_navigate_to_parent(
        &mut self,
        _: &NavigateToParent,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.picker.update(cx, |picker, cx| {
            picker.delegate.navigate_to_parent(window, cx);
        });
    }

    fn handle_split_toggle_menu(
        &mut self,
        _: &ToggleSplitMenu,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.picker.update(cx, |picker, cx| {
            picker.delegate.split_popover_menu_handle.toggle(window, cx);
        });
    }

    fn handle_filter_toggle_menu(
        &mut self,
        _: &ToggleFilterMenu,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.picker.update(cx, |picker, cx| {
            picker.delegate.filter_popover_menu_handle.toggle(window, cx);
        });
    }

    fn handle_toggle_show_hidden(
        &mut self,
        _: &ToggleShowHiddenFiles,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.picker.update(cx, |picker, cx| {
            picker.delegate.show_hidden_files = !picker.delegate.show_hidden_files;
            picker.delegate.load_entries(cx);
            picker.refresh(window, cx);
        });
    }

    fn go_to_file_split_left(
        &mut self,
        _: &pane::SplitLeft,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.go_to_file_split_inner(SplitDirection::Left, window, cx)
    }

    fn go_to_file_split_right(
        &mut self,
        _: &pane::SplitRight,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.go_to_file_split_inner(SplitDirection::Right, window, cx)
    }

    fn go_to_file_split_up(
        &mut self,
        _: &pane::SplitUp,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.go_to_file_split_inner(SplitDirection::Up, window, cx)
    }

    fn go_to_file_split_down(
        &mut self,
        _: &pane::SplitDown,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        self.go_to_file_split_inner(SplitDirection::Down, window, cx)
    }

    fn go_to_file_split_inner(
        &mut self,
        split_direction: SplitDirection,
        window: &mut Window,
        cx: &mut Context<Self>,
    ) {
        let result = self.picker.update(cx, |picker, _cx| {
            let delegate = &picker.delegate;
            let Some(entry) = delegate.filtered_entries.get(delegate.selected_index).cloned() else {
                return None;
            };

            match entry {
                DirectoryBrowserEntry::ParentDirectory => None,
                DirectoryBrowserEntry::Entry(e) => {
                    if e.is_dir() {
                        None
                    } else {
                        Some((
                            ProjectPath {
                                worktree_id: delegate.worktree_id,
                                path: e.path,
                            },
                            delegate.workspace.upgrade(),
                        ))
                    }
                }
            }
        });

        let Some((project_path, Some(workspace))) = result else { return };

        let open_task = workspace.update(cx, move |workspace, cx| {
            workspace.split_path_preview(project_path, false, Some(split_direction), window, cx)
        });
        open_task.detach_and_log_err(cx);
        cx.emit(DismissEvent);
    }
}

impl EventEmitter<DismissEvent> for DirectoryBrowser {}

impl Focusable for DirectoryBrowser {
    fn focus_handle(&self, cx: &App) -> FocusHandle {
        self.picker.focus_handle(cx)
    }
}

impl Render for DirectoryBrowser {
    fn render(&mut self, _window: &mut Window, cx: &mut Context<Self>) -> impl IntoElement {
        v_flex()
            .w(rems(34.))
            .on_action(cx.listener(Self::handle_navigate_to_parent))
            .on_action(cx.listener(Self::handle_filter_toggle_menu))
            .on_action(cx.listener(Self::handle_toggle_show_hidden))
            .on_action(cx.listener(Self::handle_split_toggle_menu))
            .on_action(cx.listener(Self::go_to_file_split_left))
            .on_action(cx.listener(Self::go_to_file_split_right))
            .on_action(cx.listener(Self::go_to_file_split_up))
            .on_action(cx.listener(Self::go_to_file_split_down))
            .child(self.picker.clone())
    }
}

#[derive(Debug, Clone)]
enum DirectoryBrowserEntry {
    ParentDirectory,
    Entry(Entry),
}

impl DirectoryBrowserEntry {
    fn display_name(&self) -> String {
        match self {
            DirectoryBrowserEntry::ParentDirectory => "..".to_string(),
            DirectoryBrowserEntry::Entry(e) => e
                .path
                .file_name()
                .map(|s| s.to_string())
                .unwrap_or_else(|| ".".to_string()),
        }
    }
}

pub struct DirectoryBrowserDelegate {
    directory_browser: WeakEntity<DirectoryBrowser>,
    workspace: WeakEntity<Workspace>,
    project: Entity<Project>,
    worktree_id: WorktreeId,
    current_path: Arc<RelPath>,
    all_entries: Vec<DirectoryBrowserEntry>,
    filtered_entries: Vec<DirectoryBrowserEntry>,
    selected_index: usize,
    show_hidden_files: bool,
    filter_popover_menu_handle: PopoverMenuHandle<ContextMenu>,
    split_popover_menu_handle: PopoverMenuHandle<ContextMenu>,
    focus_handle: FocusHandle,
    /// The path of the file to initially select (if any)
    initial_selected_path: Option<String>,
    /// The pane that was active when the picker opened
    original_pane: WeakEntity<Pane>,
    /// The item that was active when the picker opened (to restore on cancel)
    original_active_item_id: Option<EntityId>,
    /// Whether the user confirmed a selection (vs cancelled)
    confirmed: bool,
}

impl DirectoryBrowserDelegate {
    fn new(
        directory_browser: WeakEntity<DirectoryBrowser>,
        workspace: WeakEntity<Workspace>,
        project: Entity<Project>,
        worktree_id: WorktreeId,
        current_path: Arc<RelPath>,
        initial_selected_path: Option<String>,
        original_pane: WeakEntity<Pane>,
        original_active_item_id: Option<EntityId>,
        focus_handle: FocusHandle,
    ) -> Self {
        Self {
            directory_browser,
            workspace,
            project,
            worktree_id,
            current_path,
            all_entries: Vec::new(),
            filtered_entries: Vec::new(),
            selected_index: 0,
            show_hidden_files: false,
            filter_popover_menu_handle: PopoverMenuHandle::default(),
            split_popover_menu_handle: PopoverMenuHandle::default(),
            focus_handle,
            initial_selected_path,
            original_pane,
            original_active_item_id,
            confirmed: false,
        }
    }

    fn load_entries(&mut self, cx: &Context<Picker<Self>>) {
        self.all_entries.clear();

        let project = self.project.read(cx);
        let Some(worktree) = project.worktree_for_id(self.worktree_id, cx) else {
            return;
        };
        let worktree = worktree.read(cx);

        // Only show parent directory if we're not at the worktree root
        if !self.current_path.as_ref().as_unix_str().is_empty() {
            self.all_entries
                .push(DirectoryBrowserEntry::ParentDirectory);
        }

        let mut dirs: Vec<Entry> = Vec::new();
        let mut files: Vec<Entry> = Vec::new();

        for entry in worktree.child_entries(&self.current_path) {
            let file_name = entry.path.file_name().unwrap_or_default();
            let is_hidden = file_name.starts_with(".");

            if !self.show_hidden_files && is_hidden {
                continue;
            }

            if entry.is_dir() {
                dirs.push(entry.clone());
            } else {
                files.push(entry.clone());
            }
        }

        dirs.sort_by(|a, b| a.path.cmp(&b.path));
        files.sort_by(|a, b| a.path.cmp(&b.path));

        for dir in dirs {
            self.all_entries.push(DirectoryBrowserEntry::Entry(dir));
        }
        for file in files {
            self.all_entries.push(DirectoryBrowserEntry::Entry(file));
        }

        self.filtered_entries = self.all_entries.clone();

        self.selected_index = self
            .initial_selected_path
            .take()
            .and_then(|target_path| {
                self.filtered_entries.iter().position(|entry| match entry {
                    DirectoryBrowserEntry::ParentDirectory => false,
                    DirectoryBrowserEntry::Entry(e) => {
                        e.path.file_name().map(|s| s.to_string()).as_deref() == Some(&target_path)
                    }
                })
            })
            .unwrap_or(0);
    }

    fn navigate_to_parent(&mut self, window: &mut Window, cx: &mut Context<Picker<Self>>) {
        if let Some(parent) = self.current_path.parent() {
            self.current_path = parent.to_owned().into();
            self.load_entries(cx);
            cx.notify();

            cx.defer_in(window, |picker, window, cx| {
                picker.set_query("", window, cx);
                picker.refresh_placeholder(window, cx);
                picker.refresh(window, cx);
            });
        }
    }

    fn navigate_to_entry(
        &mut self,
        entry: &DirectoryBrowserEntry,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) {
        if let DirectoryBrowserEntry::Entry(e) = entry {
            self.current_path = e.path.clone();
            self.load_entries(cx);

            cx.notify();
            cx.defer_in(window, |picker, window, cx| {
                picker.set_query("", window, cx);
                picker.refresh_placeholder(window, cx);
                picker.refresh(window, cx);
            });
        }
    }

    fn display_path(&self, cx: &App) -> String {
        self.current_path
            .file_name()
            .map(|s| s.to_string())
            .unwrap_or_else(|| {
                self.project
                    .read(cx)
                    .worktree_for_id(self.worktree_id, cx)
                    .map(|wt| wt.read(cx).root_name().display(PathStyle::local()).into_owned())
                    .unwrap_or_else(|| ".".to_string())
            })
    }

    fn preview_entry_at_index(&self, ix: usize) -> Option<Box<dyn Fn(&mut Window, &mut App) + 'static>> {
        let entry = self.filtered_entries.get(ix)?;

        let project_path = match entry {
            DirectoryBrowserEntry::ParentDirectory => return None,
            DirectoryBrowserEntry::Entry(e) => {
                if e.is_dir() {
                    return None;
                }
                ProjectPath {
                    worktree_id: self.worktree_id,
                    path: e.path.clone(),
                }
            }
        };

        let workspace = self.workspace.clone();
        let pane = self.original_pane.clone();

        Some(Box::new(move |window, cx| {
            let settings = PreviewTabsSettings::get_global(cx);
            if !settings.enabled || !settings.enable_live_preview_in_directory_browser {
                return;
            }

            let Some(workspace) = workspace.upgrade() else {
                return;
            };
            let Some(pane) = pane.upgrade() else {
                return;
            };

            let existing_index = pane
                .read(cx)
                .items()
                .position(|item| item.project_path(cx).is_some_and(|path| path == project_path));

            if let Some(index) = existing_index {
                pane.update(cx, |pane, cx| {
                    pane.activate_item(index, false, false, window, cx);
                });
            } else {
                let project_path = project_path.clone();
                workspace.update(cx, |workspace, cx| {
                    workspace
                        .open_path_preview(
                            project_path,
                            Some(pane.downgrade()),
                            false,
                            true,
                            true,
                            window,
                            cx,
                        )
                        .detach_and_log_err(cx);
                });
            }
        }))
    }
}

impl PickerDelegate for DirectoryBrowserDelegate {
    type ListItem = ListItem;

    fn placeholder_text(&self, _window: &mut Window, cx: &mut App) -> Arc<str> {
        format!("Search in {}/", self.display_path(cx)).into()
    }

    fn match_count(&self) -> usize {
        self.filtered_entries.len()
    }

    fn selected_index(&self) -> usize {
        self.selected_index
    }

    fn separators_after_indices(&self) -> Vec<usize> {
        if self
            .filtered_entries
            .first()
            .is_some_and(|e| matches!(e, DirectoryBrowserEntry::ParentDirectory))
        {
            vec![0]
        } else {
            Vec::new()
        }
    }

    fn set_selected_index(&mut self, ix: usize, _: &mut Window, cx: &mut Context<Picker<Self>>) {
        self.selected_index = ix;
        cx.notify();
    }

    fn selected_index_changed(
        &self,
        ix: usize,
        _window: &mut Window,
        _cx: &mut Context<Picker<Self>>,
    ) -> Option<Box<dyn Fn(&mut Window, &mut App) + 'static>> {
        self.preview_entry_at_index(ix)
    }

    fn update_matches(
        &mut self,
        query: String,
        window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Task<()> {
        let query = query.to_lowercase();

        if query.is_empty() {
            self.filtered_entries = self.all_entries.clone();
        } else {
            self.filtered_entries = self
                .all_entries
                .iter()
                .filter(|entry| {
                    let name = entry.display_name().to_lowercase();
                    name.contains(&query)
                })
                .cloned()
                .collect();
        }

        if self.selected_index >= self.filtered_entries.len() {
            self.selected_index = self.filtered_entries.len().saturating_sub(1);
        }

        if let Some(preview_callback) = self.preview_entry_at_index(self.selected_index) {
            cx.defer_in(window, move |_, window, cx| {
                preview_callback(window, cx);
            });
        }

        cx.notify();
        Task::ready(())
    }

    fn confirm(&mut self, secondary: bool, window: &mut Window, cx: &mut Context<Picker<Self>>) {
        let Some(entry) = self.filtered_entries.get(self.selected_index).cloned() else {
            return;
        };

        match entry {
            DirectoryBrowserEntry::ParentDirectory => {
                self.navigate_to_parent(window, cx);
            }
            DirectoryBrowserEntry::Entry(ref e) if e.is_dir() => {
                self.navigate_to_entry(&entry, window, cx);
            }
            DirectoryBrowserEntry::Entry(e) => {
                self.confirmed = true;

                let project_path = ProjectPath {
                    worktree_id: self.worktree_id,
                    path: e.path,
                };

                if let Some(workspace) = self.workspace.upgrade() {
                    let allow_preview =
                        PreviewTabsSettings::get_global(cx).enable_preview_from_directory_browser;

                    let open_task = workspace.update(cx, |workspace, cx| {
                        if secondary {
                            workspace.split_path_preview(
                                project_path,
                                allow_preview,
                                None,
                                window,
                                cx,
                            )
                        } else {
                            workspace.open_path_preview(
                                project_path,
                                None,
                                true,
                                allow_preview,
                                true,
                                window,
                                cx,
                            )
                        }
                    });

                    let directory_browser = self.directory_browser.clone();
                    cx.spawn_in(window, async move |_, cx| {
                        let _ = open_task.await;
                        directory_browser.update(cx, |_, cx| cx.emit(DismissEvent)).ok();
                    })
                    .detach();
                }
            }
        }
    }

    fn dismissed(&mut self, window: &mut Window, cx: &mut Context<Picker<Self>>) {
        let settings = PreviewTabsSettings::get_global(cx);
        let live_preview_was_enabled =
            settings.enabled && settings.enable_live_preview_in_directory_browser;

        if !self.confirmed && live_preview_was_enabled {
            if let Some(pane) = self.original_pane.upgrade() {
                let preview_to_close = pane.read(cx).preview_item_id().and_then(|preview_id| {
                    let is_original = self
                        .original_active_item_id
                        .is_some_and(|id| id == preview_id);
                    if !is_original {
                        Some(preview_id)
                    } else {
                        None
                    }
                });

                let original_index = self.original_active_item_id.and_then(|original_id| {
                    pane.read(cx)
                        .items()
                        .position(|item| item.item_id() == original_id)
                });

                pane.update(cx, |pane, cx| {
                    if let Some(preview_id) = preview_to_close {
                        pane.close_item_by_id(preview_id, pane::SaveIntent::Skip, window, cx)
                            .detach_and_log_err(cx);
                    }

                    if let Some(index) = original_index {
                        pane.activate_item(index, false, false, window, cx);
                    }
                });
            }
        }

        self.directory_browser
            .update(cx, |_, cx| cx.emit(DismissEvent))
            .log_err();
    }

    fn render_match(
        &self,
        ix: usize,
        selected: bool,
        _window: &mut Window,
        cx: &mut Context<Picker<Self>>,
    ) -> Option<Self::ListItem> {
        let entry = self.filtered_entries.get(ix)?;

        let (icon_name, file_icon) = match entry {
            DirectoryBrowserEntry::ParentDirectory => (Some(IconName::ArrowUp), None),
            DirectoryBrowserEntry::Entry(e) => {
                if e.is_dir() {
                    (Some(IconName::Folder), None)
                } else {
                    let name = e.path.file_name().unwrap_or("");
                    let icon = FileIcons::get_icon(Path::new(name), cx)
                        .map(|path| Icon::from_path(path).color(Color::Muted));
                    (None, icon)
                }
            }
        };

        let (display_name, suffix) = match entry {
            DirectoryBrowserEntry::ParentDirectory => ("parent directory".to_string(), ""),
            DirectoryBrowserEntry::Entry(e) => {
                let name = e
                    .path
                    .file_name()
                    .map(|s| s.to_string())
                    .unwrap_or_else(|| ".".to_string());
                let suffix = if e.is_dir() { "/" } else { "" };
                (name, suffix)
            }
        };

        let start_icon = file_icon.or_else(|| icon_name.map(|n| Icon::new(n).color(Color::Muted)));

        Some(
            ListItem::new(ix)
                .spacing(ListItemSpacing::Sparse)
                .start_slot::<Icon>(start_icon)
                .inset(true)
                .toggle_state(selected)
                .child(Label::new(format!("{}{}", display_name, suffix))),
        )
    }

    fn render_footer(&self, _: &mut Window, cx: &mut Context<Picker<Self>>) -> Option<AnyElement> {
        let focus_handle = self.focus_handle.clone();
        let show_hidden = self.show_hidden_files;

        Some(
            h_flex()
                .w_full()
                .p_1p5()
                .justify_between()
                .border_t_1()
                .border_color(cx.theme().colors().border_variant)
                .child(
                    PopoverMenu::new("filter-menu-popover")
                        .with_handle(self.filter_popover_menu_handle.clone())
                        .attach(gpui::Corner::BottomRight)
                        .anchor(gpui::Corner::BottomLeft)
                        .offset(gpui::Point {
                            x: px(1.0),
                            y: px(1.0),
                        })
                        .trigger_with_tooltip(
                            IconButton::new("filter-trigger", IconName::Sliders)
                                .icon_size(IconSize::Small)
                                .toggle_state(show_hidden)
                                .when(show_hidden, |this| {
                                    this.indicator(Indicator::dot().color(Color::Info))
                                }),
                            {
                                let focus_handle = focus_handle.clone();
                                move |_window, cx| {
                                    Tooltip::for_action_in(
                                        "Filter Options",
                                        &ToggleFilterMenu,
                                        &focus_handle,
                                        cx,
                                    )
                                }
                            },
                        )
                        .menu({
                            let focus_handle = focus_handle.clone();

                            move |window, cx| {
                                Some(ContextMenu::build(window, cx, {
                                    let focus_handle = focus_handle.clone();
                                    move |menu, _, _| {
                                        menu.context(focus_handle.clone())
                                            .header("Filter Options")
                                            .toggleable_entry(
                                                "Show Hidden Files",
                                                show_hidden,
                                                ui::IconPosition::End,
                                                Some(ToggleShowHiddenFiles.boxed_clone()),
                                                move |window, cx| {
                                                    window.focus(&focus_handle);
                                                    window.dispatch_action(
                                                        ToggleShowHiddenFiles.boxed_clone(),
                                                        cx,
                                                    );
                                                },
                                            )
                                    }
                                }))
                            }
                        }),
                )
                .child(
                    h_flex()
                        .gap_0p5()
                        .child(
                            PopoverMenu::new("split-menu-popover")
                                .with_handle(self.split_popover_menu_handle.clone())
                                .attach(gpui::Corner::BottomRight)
                                .anchor(gpui::Corner::BottomLeft)
                                .offset(gpui::Point {
                                    x: px(1.0),
                                    y: px(1.0),
                                })
                                .trigger(
                                    ui::ButtonLike::new("split-trigger")
                                        .child(Label::new("Split…"))
                                        .selected_style(ui::ButtonStyle::Tinted(TintColor::Accent))
                                        .child(KeyBinding::for_action_in(
                                            &ToggleSplitMenu,
                                            &focus_handle,
                                            cx,
                                        )),
                                )
                                .menu({
                                    let focus_handle = focus_handle.clone();

                                    move |window, cx| {
                                        Some(ContextMenu::build(window, cx, {
                                            let focus_handle = focus_handle.clone();
                                            move |menu, _, _| {
                                                menu.context(focus_handle)
                                                    .action("Split Left", pane::SplitLeft.boxed_clone())
                                                    .action("Split Right", pane::SplitRight.boxed_clone())
                                                    .action("Split Up", pane::SplitUp.boxed_clone())
                                                    .action("Split Down", pane::SplitDown.boxed_clone())
                                            }
                                        }))
                                    }
                                }),
                        )
                        .child(
                            Button::new("open-selection", "Open")
                                .key_binding(KeyBinding::for_action_in(
                                    &menu::Confirm,
                                    &focus_handle,
                                    cx,
                                ))
                                .on_click(|_, window, cx| {
                                    window.dispatch_action(menu::Confirm.boxed_clone(), cx)
                                }),
                        ),
                )
                .into_any(),
        )
    }
}
