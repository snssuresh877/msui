# Neovim Navigation & Action Keymaps Reference

> **Leader Key:** `<Space>`  
> To view an interactive cheat sheet inside Neovim at any time, press `<Space>k` or press `<Space>` and wait briefly.

---

## 1. Search & File Navigation

| Keybinding | Mode | Action | Description |
| :--- | :---: | :--- | :--- |
| `<Space>e` | Normal | Toggle Neo-tree | Opens or closes the project file explorer tree |
| `<Space>sf` | Normal | Find Files | Fuzzy search for files across the project (`fzf-lua`) |
| `<Space>sg` | Normal | Live Grep | Search for text/code across the entire codebase |
| `<Space>sb` | Normal | Search Buffers | Fuzzy find across currently open buffers |
| `<Space>sh` | Normal | Search Help | Fuzzy search Neovim `:help` documentation tags |
| `<Space>sr` | Normal | LSP References | Fuzzy search all references of symbol under cursor |
| `<Space>sd` | Normal | Diagnostics | Fuzzy list all workspace errors and warnings |

---

## 2. Window & Buffer Navigation

### Window Navigation (Splits)
| Keybinding | Mode | Action | Description |
| :--- | :---: | :--- | :--- |
| `Ctrl + h` | Normal | Move Left | Focus window to the left (`<C-w>h`) |
| `Ctrl + j` | Normal | Move Down | Focus window below (`<C-w>j`) |
| `Ctrl + k` | Normal | Move Up | Focus window above (`<C-w>k`) |
| `Ctrl + l` | Normal | Move Right | Focus window to the right (`<C-w>l`) |
| `<Space>x` | Normal | Close Window | Closes active window / split (`:close`) |

### Buffer Cycling & Management
| Keybinding | Mode | Action | Description |
| :--- | :---: | :--- | :--- |
| `<Tab>` or `<Space>n` | Normal | Next Buffer | Switch to next open buffer (`:bnext`) |
| `<S-Tab>` or `<Space>p` | Normal | Prev Buffer | Switch to previous open buffer (`:bprev`) |
| `<Space>b` | Normal | List Buffers | Display list of loaded buffers |
| `Ctrl + s` | Normal | Save File | Writes active buffer to disk (`:write`) |
| `Ctrl + q` | Normal | Quit Window | Quits window / buffer (`:quit`) |

---

## 3. Code Intelligence & LSP Navigation

> *These keymaps become active when a language server (Rust, C/C++, Lua, Bash, TOML) attaches.*

| Keybinding | Mode | Action | Description |
| :--- | :---: | :--- | :--- |
| `gd` | Normal | Go to Definition | Jumps to source definition of symbol |
| `gD` | Normal | Go to Declaration | Jumps to symbol declaration |
| `gr` | Normal | Find References | Lists all occurrences/references across project |
| `K` | Normal | Hover Docs | Shows documentation, types, and function signature |
| `<Space>h` | Normal | Signature Help | Displays parameter hints for active function call |
| `<Space>th` | Normal | Toggle Inlay Hints | Toggles parameter & inferred type hints in the buffer |

---

## 4. Code Actions & Editing

| Keybinding | Mode | Action | Description |
| :--- | :---: | :--- | :--- |
| `<Space>f` | Normal | Format Buffer | Formats current buffer synchronously via LSP |
| *(auto on save)* | Normal | Auto-Format | Formats code automatically whenever you `:w` |
| `<Space>rn` | Normal | Rename Symbol | Project-wide symbol rename via LSP |
| `<Space>ca` | Normal | Code Action | Shows available LSP code fixes and refactorings |
| `gcc` | Normal | Toggle Comment | Comments or uncomments current line (native) |
| `gc` | Visual | Toggle Comment | Comments or uncomments selected lines (native) |
| `(` `[` `{` `"` `'` | Insert | Auto-Pair | Automatically inserts matching closing pair |
| `<BS>` | Insert | Auto-Pair Delete | Deletes both opening and closing character when empty |

---

## 5. Diagnostic Navigation

| Keybinding | Mode | Action | Description |
| :--- | :---: | :--- | :--- |
| `[d` | Normal | Prev Diagnostic | Jumps to previous error / warning in buffer |
| `]d` | Normal | Next Diagnostic | Jumps to next error / warning in buffer |
| `<Space>d` | Normal | Show Diagnostic | Opens rounded floating window with error details |

---

## 6. Git Hunks Navigation & Actions (Gitsigns)

| Keybinding | Mode | Action | Description |
| :--- | :---: | :--- | :--- |
| `]h` | Normal | Next Git Hunk | Jumps cursor to the next modified/added Git hunk |
| `[h` | Normal | Prev Git Hunk | Jumps cursor to previous modified/added Git hunk |
| `<Space>gp` | Normal | Preview Hunk | Displays inline popup previewing the diff |
| `<Space>gb` | Normal | Blame Line | Shows detailed commit author and date for current line |
| `<Space>gd` | Normal | Diff Against Index | Opens split showing full buffer diff |
| `<Space>gs` | Normal / Visual | Stage Hunk | Stages the current hunk or visual selection |
| `<Space>gr` | Normal / Visual | Reset Hunk | Discards changes in the current hunk or selection |

---

## 7. Integrated Terminal

| Keybinding | Mode | Action | Description |
| :--- | :---: | :--- | :--- |
| `<Space>t` | Normal | Open Terminal | Opens a 12-line drawer terminal at the bottom |
| `<Esc>` | Terminal | Normal Mode | Leaves terminal input mode into Neovim normal mode |
| `Ctrl + h` | Terminal | Move Left | Jump directly out of terminal to left window |
| `Ctrl + j` | Terminal | Move Down | Jump directly out of terminal to window below |
| `Ctrl + k` | Terminal | Move Up | Jump directly out of terminal to window above |
| `Ctrl + l` | Terminal | Move Right | Jump directly out of terminal to right window |
