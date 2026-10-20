-- ============================================================
-- Native Neovim Statusline
-- ============================================================

vim.opt.laststatus = 3
vim.opt.showmode = false

vim.opt.statusline = table.concat({
    " ",
    "%#StatusLine#",

    -- Mode
    "%{toupper(mode())}",

    " │ ",

    -- File
    "%f",

    " │ ",

    -- File type
    "%y",

    " │ ",

    -- LSP
    "%{v:lua.LspStatus()}",

    "%=",

    -- Git / modified
    "%m",

    " │ ",

    -- Position
    "Ln %l, Col %c",

    " │ ",

    -- Percentage
    "%p%% ",

    " ",
})

function _G.LspStatus()
    local clients = vim.lsp.get_clients({
        bufnr = 0,
    })

    if #clients == 0 then
        return "LSP: -"
    end

    local names = {}

    for _, client in ipairs(clients) do
        table.insert(names, client.name)
    end

    return "LSP: " .. table.concat(names, ", ")
end
