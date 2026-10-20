-- ============================================================
-- BOOTSTRAP LAZY.NVIM
-- ============================================================

local lazypath = vim.fn.stdpath("data") .. "/lazy/lazy.nvim"

vim.opt.rtp:prepend(lazypath)

-- ============================================================
-- LEADER
-- ============================================================

vim.g.mapleader = " "
vim.g.maplocalleader = " "

-- ============================================================
-- PLUGINS
-- ============================================================

require("lazy").setup({
    { import = "plugins" },
})

-- ============================================================
-- CONFIGURATION
-- ============================================================

require("options")
require("keymaps")
require("lsp")
require("statusline")
require("startup")
require("diagnostics")
require("theme")
require("tabline")
require("terminal")
