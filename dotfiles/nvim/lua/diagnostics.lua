-- ============================================================
-- DIAGNOSTICS
-- ============================================================

vim.diagnostic.config({
    signs = {
        text = {
            [vim.diagnostic.severity.ERROR] = "✘",
            [vim.diagnostic.severity.WARN] = "▲",
            [vim.diagnostic.severity.INFO] = "ℹ",
            [vim.diagnostic.severity.HINT] = "◆",
        },
    },

    virtual_text = {
        spacing = 4,
        prefix = "●",
    },

    underline = true,

    update_in_insert = false,

    severity_sort = true,

    float = {
        border = "rounded",
        source = true,
    },
})
