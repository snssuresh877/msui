vim.g.mapleader = " "
vim.g.maplocalleader = " "

-- ============================================================
-- FILE EXPLORER
-- ============================================================

vim.keymap.set("n", "<leader>e", "<cmd>Neotree toggle<cr>", {
    desc = "File Explorer",
})

-- ============================================================
-- BUFFERS
-- ============================================================

vim.keymap.set("n", "<leader>b", "<cmd>buffers<cr>", {
    desc = "List Buffers",
})

vim.keymap.set("n", "<leader>n", "<cmd>bnext<cr>", {
    desc = "Next Buffer",
})

vim.keymap.set("n", "<leader>p", "<cmd>bprev<cr>", {
    desc = "Previous Buffer",
})

vim.keymap.set("n", "<Tab>", "<cmd>bnext<cr>", {
    desc = "Next Buffer",
})

vim.keymap.set("n", "<S-Tab>", "<cmd>bprev<cr>", {
    desc = "Previous Buffer",
})

-- ============================================================
-- FILE / EDITOR
-- ============================================================

vim.keymap.set("n", "<C-s>", "<cmd>write<cr>", {
    desc = "Save File",
})

vim.keymap.set("n", "<C-q>", "<cmd>quit<cr>", {
    desc = "Quit Window",
})

vim.keymap.set("n", "<leader>x", "<cmd>close<cr>", {
    desc = "Close Window",
})

-- ============================================================
-- WINDOW NAVIGATION
-- ============================================================

vim.keymap.set("n", "<C-h>", "<C-w>h", {
    desc = "Move Left",
})

vim.keymap.set("n", "<C-j>", "<C-w>j", {
    desc = "Move Down",
})

vim.keymap.set("n", "<C-k>", "<C-w>k", {
    desc = "Move Up",
})

vim.keymap.set("n", "<C-l>", "<C-w>l", {
    desc = "Move Right",
})

-- ============================================================
-- LSP
-- Installed only when an LSP attaches
-- ============================================================

vim.api.nvim_create_autocmd("LspAttach", {
    callback = function(event)

        -- Native LSP completion
        vim.lsp.completion.enable(
            true,
            event.data.client_id,
            event.buf,
            {
                autotrigger = true,
            }
        )

        -- Navigation
        vim.keymap.set("n", "gd", vim.lsp.buf.definition, {
            buffer = event.buf,
            desc = "Go to Definition",
        })

        vim.keymap.set("n", "gD", vim.lsp.buf.declaration, {
            buffer = event.buf,
            desc = "Go to Declaration",
        })

        vim.keymap.set("n", "gr", vim.lsp.buf.references, {
            buffer = event.buf,
            desc = "Find References",
        })

        vim.keymap.set("n", "K", vim.lsp.buf.hover, {
            buffer = event.buf,
            desc = "Hover Documentation",
        })

        -- Code actions
        vim.keymap.set("n", "<leader>rn", vim.lsp.buf.rename, {
            buffer = event.buf,
            desc = "Rename Symbol",
        })

        vim.keymap.set("n", "<leader>ca", vim.lsp.buf.code_action, {
            buffer = event.buf,
            desc = "Code Action",
        })

        vim.keymap.set("n", "<leader>h", vim.lsp.buf.signature_help, {
            buffer = event.buf,
            desc = "Signature Help",
        })

        -- Diagnostics
        vim.keymap.set("n", "[d", vim.diagnostic.goto_prev, {
            buffer = event.buf,
            desc = "Previous Diagnostic",
        })

        vim.keymap.set("n", "]d", vim.diagnostic.goto_next, {
            buffer = event.buf,
            desc = "Next Diagnostic",
        })

        vim.keymap.set("n", "<leader>d", vim.diagnostic.open_float, {
            buffer = event.buf,
            desc = "Show Diagnostic",
        })

        -- Formatting
        vim.keymap.set("n", "<leader>f", function()
            local formatting_clients = vim.lsp.get_clients({
                bufnr = event.buf,
                method = "textDocument/formatting",
            })
            if #formatting_clients == 0 then
                vim.notify("No LSP client supports formatting for this buffer", vim.log.levels.WARN)
                return
            end

            local ok, err = pcall(function()
                vim.lsp.buf.format({
                    bufnr = event.buf,
                    async = false,
                    timeout_ms = 2000,
                    filter = function(c)
                        return c:supports_method("textDocument/formatting", event.buf)
                    end,
                })
            end)

            if not ok and err then
                vim.notify("Formatting failed: " .. tostring(err), vim.log.levels.WARN)
            end
        end, {
            buffer = event.buf,
            desc = "Format Buffer",
        })

        local client = vim.lsp.get_client_by_id(event.data.client_id)

        -- Auto-formatting on save
        if client and client:supports_method("textDocument/formatting", event.buf) then
            local format_group = vim.api.nvim_create_augroup("LspFormatting_" .. event.buf, { clear = true })
            vim.api.nvim_create_autocmd("BufWritePre", {
                group = format_group,
                buffer = event.buf,
                callback = function()
                    local formatting_clients = vim.lsp.get_clients({
                        bufnr = event.buf,
                        method = "textDocument/formatting",
                    })
                    if #formatting_clients == 0 then
                        return
                    end

                    local ok, err = pcall(function()
                        vim.lsp.buf.format({
                            bufnr = event.buf,
                            async = false,
                            timeout_ms = 2000,
                            filter = function(c)
                                return c:supports_method("textDocument/formatting", event.buf)
                            end,
                        })
                    end)

                    if not ok and err then
                        vim.notify("Formatting on save failed: " .. tostring(err), vim.log.levels.WARN)
                    end
                end,
            })
        end

        -- Inlay Hints (Space+th toggle & auto-enable)
        if client and client:supports_method("textDocument/inlayHint", event.buf) then
            vim.lsp.inlay_hint.enable(true, { bufnr = event.buf })
        end

        vim.keymap.set("n", "<leader>th", function()
            local hint_clients = vim.lsp.get_clients({
                bufnr = event.buf,
                method = "textDocument/inlayHint",
            })
            if #hint_clients == 0 then
                vim.notify("Inlay hints not supported by active LSP clients on this buffer", vim.log.levels.WARN)
                return
            end

            local current = vim.lsp.inlay_hint.is_enabled({ bufnr = event.buf })
            vim.lsp.inlay_hint.enable(not current, { bufnr = event.buf })
            local state = not current and "enabled" or "disabled"
            vim.notify("Inlay hints " .. state, vim.log.levels.INFO)
        end, {
            buffer = event.buf,
            desc = "Toggle Inlay Hints",
        })

        -- Document Symbol Highlighting
        if client and client:supports_method("textDocument/documentHighlight", event.buf) then
            local highlight_group = vim.api.nvim_create_augroup("LspDocHighlight_" .. event.buf, { clear = true })
            vim.api.nvim_create_autocmd({ "CursorHold", "CursorHoldI" }, {
                group = highlight_group,
                buffer = event.buf,
                callback = vim.lsp.buf.document_highlight,
            })
            vim.api.nvim_create_autocmd({ "CursorMoved", "CursorMovedI" }, {
                group = highlight_group,
                buffer = event.buf,
                callback = vim.lsp.buf.clear_references,
            })
        end
    end,
})
