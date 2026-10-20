-- ============================================================
-- THEME / UI
-- ============================================================

vim.cmd.colorscheme("habamax")

-- Editor
vim.opt.cursorline = true
vim.opt.termguicolors = true

-- Make the current line subtle
vim.api.nvim_set_hl(0, "CursorLine", {
    bg = "#1c1f26",
})

-- Line numbers
vim.api.nvim_set_hl(0, "LineNr", {
    fg = "#5c6370",
})

vim.api.nvim_set_hl(0, "CursorLineNr", {
    fg = "#61afef",
    bold = true,
})

-- Sign column
vim.api.nvim_set_hl(0, "SignColumn", {
    bg = "NONE",
})

-- Statusline
vim.api.nvim_set_hl(0, "StatusLine", {
    fg = "#abb2bf",
    bg = "#1c1f26",
})

vim.api.nvim_set_hl(0, "StatusLineNC", {
    fg = "#5c6370",
    bg = "#15171c",
})

-- LSP Document Symbol Highlighting
vim.api.nvim_set_hl(0, "LspReferenceText", {
    bg = "#2e3440",
})

vim.api.nvim_set_hl(0, "LspReferenceRead", {
    bg = "#2e3440",
})

vim.api.nvim_set_hl(0, "LspReferenceWrite", {
    bg = "#3b4252",
    underline = true,
})

