-- ============================================================
-- NATIVE TERMINAL
-- ============================================================

vim.keymap.set("n", "<leader>t", function()
    vim.cmd("botright split")
    vim.cmd("resize 12")
    vim.cmd("terminal")
    vim.cmd("startinsert")
end, {
    desc = "Open Terminal",
})

-- Leave terminal mode
vim.keymap.set("t", "<Esc>", [[<C-\><C-n>]], {
    desc = "Terminal Normal Mode",
})

-- Move between terminal and editor
vim.keymap.set("t", "<C-h>", [[<C-\><C-n><C-w>h]], {
    desc = "Move Left",
})

vim.keymap.set("t", "<C-j>", [[<C-\><C-n><C-w>j]], {
    desc = "Move Down",
})

vim.keymap.set("t", "<C-k>", [[<C-\><C-n><C-w>k]], {
    desc = "Move Up",
})

vim.keymap.set("t", "<C-l>", [[<C-\><C-n><C-w>l]], {
    desc = "Move Right",
})
