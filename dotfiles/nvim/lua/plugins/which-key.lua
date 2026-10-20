return {
    {
        "folke/which-key.nvim",

        event = "VeryLazy",

        opts = {
            preset = "helix",

            delay = 300,

            spec = {
                { "<leader>e", desc = "File Explorer" },

                { "<leader>b", desc = "List Buffers" },
                { "<leader>n", desc = "Next Buffer" },
                { "<leader>p", desc = "Previous Buffer" },

                { "<leader>x", desc = "Close Window" },

                { "<leader>k", desc = "Keymaps" },

                { "<leader>rn", desc = "Rename Symbol" },
                { "<leader>ca", desc = "Code Action" },
                { "<leader>f", desc = "Format Buffer" },
                { "<leader>h", desc = "Signature Help" },
                { "<leader>d", desc = "Show Diagnostic" },

                { "<leader>t", desc = "Open Terminal" },
                { "<leader>th", desc = "Toggle Inlay Hints" },

                { "<leader>g", group = "Git" },
                { "<leader>s", group = "Search" },
                { "<leader>q", group = "Quit" },
                { "<leader>w", group = "Windows" },
            },

            win = {
                border = "rounded",
            },

            layout = {
                width = {
                    min = 30,
                    max = 50,
                },
                spacing = 3,
                align = "left",
            },

            sort = {
                "local",
                "order",
                "group",
                "alphanum",
                "mod",
            },
        },

        keys = {
            {
                "<leader>k",
                function()
                    require("which-key").show({
                        keys = "<leader>",
                        loop = true,
                    })
                end,
                desc = "Show Keymaps",
            },
        },
    },
}
