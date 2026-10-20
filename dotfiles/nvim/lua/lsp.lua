-- ============================================================
-- NATIVE LSP
-- ============================================================

vim.lsp.config("rust_analyzer", {
    cmd = { "rust-analyzer" },

    filetypes = {
        "rust",
    },
})

vim.lsp.config("clangd", {
    cmd = { "clangd" },

    filetypes = {
        "c",
        "cpp",
        "objc",
        "objcpp",
    },
})

vim.lsp.config("lua_ls", {
    cmd = { "lua-language-server" },

    filetypes = {
        "lua",
    },

    settings = {
        Lua = {
            runtime = {
                version = "LuaJIT",
            },

            diagnostics = {
                globals = {
                    "vim",
                },
            },

            workspace = {
                checkThirdParty = false,
            },

            telemetry = {
                enable = false,
            },
        },
    },
})

vim.lsp.config("bashls", {
    cmd = {
        "bash-language-server",
        "start",
    },

    filetypes = {
        "sh",
        "bash",
    },
})

vim.lsp.config("taplo", {
    cmd = {
        "taplo",
        "lsp",
        "stdio",
    },

    filetypes = {
        "toml",
    },
})

vim.lsp.enable({
    "rust_analyzer",
    "clangd",
    "lua_ls",
    "bashls",
    "taplo",
})
