# 🦅 Yazi Keybindings & Quick Reference

This document provides a comprehensive cheat sheet for Yazi keybindings in msUI, including customized shortcuts, plugin integrations, and the full built-in command palette.

---

## 🌟 1. Custom & Overridden Keybindings (`keymap.toml`)

These custom bindings are defined in [`keymap.toml`](file:///home/m/Projects/msui/dotfiles/yazi/keymap.toml) under `[mgr]` and override/prepend default behaviors:

| Key | Action | Description |
| :--- | :--- | :--- |
| `<C-d>` / `<C-n>` | `plugin drag` | Drag & drop hovered/selected files into external GUI apps (`ripdrag`) |
| `c m` | `plugin chmod` | Interactive chmod modal on selected files |
| `e` | `shell --block 'nvim %s'` | Open hovered file or directory in Neovim |
| `E` | `shell --block 'nvim .'` | Open current working directory in Neovim |
| `<Enter>` | `plugin smart-enter` | Smart open: enters directories, opens files according to rules in `yazi.toml` |
| `l` | `plugin smart-enter` | Same as `<Enter>` (Vim-style right/enter) |
| `<Right>` | `plugin smart-enter` | Same as `<Enter>` (Arrow key right/enter) |

---

## 🧭 2. Navigation & Movement

| Key | Action |
| :--- | :--- |
| `k` / `<Up>` | Move cursor up |
| `j` / `<Down>` | Move cursor down |
| `h` / `<Left>` | Leave current directory (go to parent directory) |
| `l` / `<Right>` / `<Enter>` | Enter directory / open file (*customized via `smart-enter`*) |
| `K` | Scroll up 5 items |
| `J` | Scroll down 5 items |
| `g g` / `<Home>` | Jump to the very first item in the directory |
| `G` / `<End>` | Jump to the very last item in the directory |
| `<PageUp>` | Move cursor one full page up |
| `<PageDown>` | Move cursor one full page down |
| `H` | Jump to the top of the visible screen (or history backward) |
| `L` | Jump to the bottom of the visible screen (or history forward) |
| `[` / `]` | Seek up / down parent directory |

---

## 🚀 3. Fast Directory Jump (`g` prefix & Fuzzy Finders)

| Key | Action |
| :--- | :--- |
| `g h` | Jump to Home directory (`~`) |
| `g c` | Jump to Config directory (`~/.config`) |
| `g d` | Jump to Downloads directory (`~/Downloads`) |
| `g t` | Jump to Temporary directory (`/tmp`) |
| `g <Space>` | Interactive jump prompt (type directory path) |
| `z` | Jump via **zoxide** (fuzzy jump to recent directories) |
| `Z` | Jump via **fzf** |

---

## ✅ 4. Selection & Visual Mode

| Key | Action |
| :--- | :--- |
| `<Space>` | Toggle select current item and move down |
| `v` | Enter visual selection mode (select a range with `j`/`k`) |
| `V` | Invert / unset visual selection |
| `<C-a>` | Select all files in directory |
| `<C-r>` | Invert selection |
| `<Esc>` | Clear selection / exit visual mode |

---

## 📂 5. File Operations (Yank, Cut, Paste, Delete, Rename)

| Key | Action |
| :--- | :--- |
| `y` | Yank (copy) selected files |
| `x` / `d` | Cut selected files |
| `p` | Paste copied/cut files |
| `P` | Paste files (force overwrite) |
| `Y` | Copy absolute path to system clipboard |
| `X` | Copy filename to system clipboard |
| `d` | Move selected files to Trash (safe delete) |
| `D` | Permanently delete selected files |
| `a` | Create a new file (append `/` to create a directory) |
| `r` | Rename hovered file |
| `R` | Batch rename selected files |
| `c m` | Interactive file permission change (*custom `chmod` plugin*) |
| `<C-d>` / `<C-n>` | Drag & drop files to GUI apps (*custom `drag` plugin with `ripdrag`*) |

---

## 🎯 6. Opening & Editing Files

Configured file opener associations from `yazi.toml`:

| Key | Action |
| :--- | :--- |
| `e` | Open hovered file or directory in **Neovim** (*custom*) |
| `E` | Open current directory in **Neovim** (*custom*) |
| `o` | Open with system default application (`xdg-open`) |
| `O` | Open with interactive opener menu (`open --interactive`): choose between Neovim, Thunar, MPV, Inkscape, Krita, Evince, etc. |

### Configured Opener Fallbacks:
* **Video / Audio**: `mpv`
* **Images / SVGs**: `imv`, `inkscape`, `gimp`
* **Illustrator (`.ai`)**: `inkscape`, `evince`, `krita`
* **PDFs**: `evince`
* **Code / Configs**: `nvim`
* **Reveal in GUI**: `thunar`

---

## 🔍 7. Searching & Filtering

| Key | Action |
| :--- | :--- |
| `/` | Search within the current directory |
| `?` | Search backwards |
| `s` | Recursive search for files using `fd` |
| `S` | Recursive content search using `ripgrep` (`rg`) |
| `f` | Filter current directory (dynamically hides non-matching items) |
| `n` | Jump to next search result |
| `N` | Jump to previous search result |

---

## 📑 8. Tabs Management

| Key | Action |
| :--- | :--- |
| `t` | Create a new tab |
| `1` – `9` | Switch directly to tab 1 through 9 |
| `w` | Close current tab |
| `W` | Switch to last active tab |

---

## ⚙️ 9. Sorting & Linemode

| Key | Action |
| :--- | :--- |
| `.` | Toggle hidden dotfiles (on/off) |
| `, a` | Sort alphabetically by name |
| `, m` | Sort by modified time |
| `, c` | Sort by creation time |
| `, e` | Sort by file extension |
| `, s` | Sort by file size |
| `, n` | Sort natural (e.g., 2 before 10) |
| `, r` | Reverse current sort order |
| `m s` | Display line mode: File size |
| `m p` | Display line mode: File permissions |
| `m m` | Display line mode: Modified time |
| `m o` | Display line mode: File owner |
| `m n` | Display line mode: None |

---

## 💻 10. Shell, Tasks & System

| Key | Action |
| :--- | :--- |
| `w` | Open background task manager (view running file transfers/previews) |
| `;` | Execute shell command (interactive) |
| `:` | Execute blocking shell command |
| `~` or `F1` | **Interactive Help Menu** (search all active keybindings live inside Yazi) |
| `q` | Quit Yazi |
| `Q` | Quit Yazi without saving working directory |

---

## 🪟 11. Modal Contexts

### Task Manager (`w`):
* `j` / `k`: Navigate through tasks
* `x`: Cancel selected background task
* `<Enter>`: Inspect task details
* `q`: Close tasks modal

### Input Prompt (e.g., `a` for new file, `r` for rename):
* `<Enter>`: Confirm input
* `<Esc>`: Cancel
* `<C-u>`: Delete to start of line
* `<C-k>`: Delete to end of line
* `<C-w>`: Delete previous word
* `<Backspace>`: Delete previous character
