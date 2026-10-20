return {
    {
        "nvim-neo-tree/neo-tree.nvim",
        branch = "v3.x",

        dependencies = {
            "nvim-lua/plenary.nvim",
            "MunifTanjim/nui.nvim",
            "nvim-tree/nvim-web-devicons",
        },

        lazy = false,

        opts = {
            close_if_last_window = true,

            filesystem = {
                filtered_items = {
                    hide_dotfiles = false,
                    hide_gitignored = false,
                    hide_hidden = false,
                },

                follow_current_file = {
                    enabled = true,
                    leave_dirs_open = true,
                },

                hijack_netrw_behavior = "open_default",

                window = {
                    mappings = {
                        ["<space>"] = "none",
                    },
                },
            },

            window = {
                width = 28,
                mappings = {
                    ["<space>"] = "none",
                },
            },
        },
    },
}
