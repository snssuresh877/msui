return {
    {
        "ibhagwan/fzf-lua",
        dependencies = { "nvim-tree/nvim-web-devicons" },
        cmd = "FzfLua",
        keys = {
            { "<leader>sf", function() require("fzf-lua").files() end, desc = "Find Files" },
            { "<leader>sg", function() require("fzf-lua").live_grep() end, desc = "Live Grep" },
            { "<leader>sb", function() require("fzf-lua").buffers() end, desc = "Search Buffers" },
            { "<leader>sh", function() require("fzf-lua").help_tags() end, desc = "Help Tags" },
            { "<leader>sr", function() require("fzf-lua").lsp_references() end, desc = "LSP References" },
            { "<leader>sd", function() require("fzf-lua").diagnostics_workspace() end, desc = "Workspace Diagnostics" },
        },
        opts = {
            winopts = {
                border = "rounded",
                preview = {
                    border = "rounded",
                },
            },
        },
    },
}
