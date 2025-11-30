# Directory Browser

A modal file browser for navigating directories and opening files in Zed.

## Overview

The directory browser provides hierarchical file navigation as an alternative to the fuzzy file finder. It displays the contents of a single directory at a time, allowing users to navigate up and down the directory tree within a worktree.

## Architecture

### Main Components

- **`DirectoryBrowser`** - The modal view that wraps the picker
- **`DirectoryBrowserDelegate`** - Implements `PickerDelegate` to provide directory listing and navigation logic
- **`DirectoryBrowserEntry`** - Enum representing `ParentDirectory` or `Entry` (worktree entry)

### Entry Point

```rust
pub fn init(cx: &mut App)
```

Registers the `Toggle` action on all workspaces via `cx.observe_new()`.

## Behavior

### Opening the Browser

When opened via `directory_browser::Toggle`:

1. If the active item is a file within a worktree, opens to that file's parent directory with the file pre-selected
2. If the active item is an external file (outside any worktree), does nothing
3. If no item is active but worktrees exist, opens to the root of the first worktree
4. If no worktrees exist, does nothing

### Navigation

- **Enter on directory**: Navigates into that directory
- **Enter on file**: Opens the file and closes the browser
- **Enter on "Parent Directory"**: Navigates to parent (only shown in subdirectories, not at worktree root)
- **`NavigateToParent` action**: Navigates to parent directory (no-op at worktree root)
- **Typing**: Filters entries by case-insensitive substring match

### Entry Display

Entries are sorted with directories first, then files, each group sorted alphabetically. Hidden files (starting with `.`) are excluded by default but can be shown via `ToggleShowHiddenFiles`.

### Live Preview

When enabled via settings, selecting a file entry (via arrow keys or filtering) opens a preview tab. On cancel, any preview opened during browsing is closed and the original tab is restored.

### Split Support

Files can be opened in split panes via the split menu or `pane::Split*` actions.

## Settings

Located under `preview_tabs` in settings:

| Setting | Default | Description |
|---------|---------|-------------|
| `enable_preview_from_directory_browser` | `true` | Open confirmed files in preview mode |
| `enable_live_preview_in_directory_browser` | `true` | Show preview while navigating entries |

## Actions

| Action | Description |
|--------|-------------|
| `Toggle` | Open/close the directory browser |
| `NavigateToParent` | Go to parent directory |
| `ToggleShowHiddenFiles` | Show/hide dotfiles |
| `ToggleFilterMenu` | Toggle filter options popover |
| `ToggleSplitMenu` | Toggle split pane popover |

## UI Structure

```
┌─────────────────────────────────────┐
│ [Search input: "Search in dir/"]   │
├─────────────────────────────────────┤
│ 📁 Parent Directory                 │
│ 📁 src/                             │
│ 📄 README.md                        │
│ 📄 Cargo.toml                       │
├─────────────────────────────────────┤
│ [Filter ⚙️]          [Split…] [Open]│
└─────────────────────────────────────┘
```

## Key Implementation Details

### Focus Handling

The picker's focus handle is passed to the delegate for use in tooltips and context menus. The modal checks if submenus are focused before allowing dismiss.

### Dismiss Behavior

On dismiss without confirmation:
1. Check if live preview was enabled
2. Close any preview tab that wasn't the original active item
3. Restore the original active item

### Worktree Integration

Uses `worktree.child_entries()` to get directory contents. The `worktree_id` is tracked to construct proper `ProjectPath` values for opening files. Navigation is limited to within the worktree - the parent directory entry is not shown at the worktree root.
