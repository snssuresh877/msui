return {
    {
        "nvim-treesitter/nvim-treesitter",
        lazy = false,
        build = ":TSUpdate",

        config = function()
            require("nvim-treesitter").install({
                "c",
                "cpp",
                "rust",
                "lua",
                "bash",
                "toml",
                "vim",
                "vimdoc",
                "query",
            })

            vim.api.nvim_create_autocmd("FileType", {
                pattern = {
                    "c",
                    "cpp",
                    "rust",
                    "lua",
                    "bash",
                    "toml",
                    "vim",
                    "vimdoc",
                    "query",
                },
                callback = function()
                    vim.treesitter.start()
                end,
            })
        end,
    },
}
