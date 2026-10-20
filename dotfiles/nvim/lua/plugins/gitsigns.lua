return {
    {
        "lewis6991/gitsigns.nvim",
        event = { "BufReadPre", "BufNewFile" },
        opts = {
            signs = {
                add = { text = "▎" },
                change = { text = "▎" },
                delete = { text = " " },
                topdelete = { text = "▔" },
                changedelete = { text = "▎" },
                untracked = { text = "▎" },
            },
            on_attach = function(bufnr)
                local gs = package.loaded.gitsigns

                local function map(mode, l, r, opts)
                    opts = opts or {}
                    opts.buffer = bufnr
                    vim.keymap.set(mode, l, r, opts)
                end

                -- Hunk Navigation
                map("n", "]h", function()
                    if vim.wo.diff then
                        return "]c"
                    end
                    vim.schedule(function()
                        gs.next_hunk()
                    end)
                    return "<Ignore>"
                end, { expr = true, desc = "Next Git Hunk" })

                map("n", "[h", function()
                    if vim.wo.diff then
                        return "[c"
                    end
                    vim.schedule(function()
                        gs.prev_hunk()
                    end)
                    return "<Ignore>"
                end, { expr = true, desc = "Previous Git Hunk" })

                -- Git Actions
                map("n", "<leader>gp", gs.preview_hunk, { desc = "Preview Hunk" })
                map("n", "<leader>gb", function()
                    gs.blame_line({ full = true })
                end, { desc = "Blame Line" })
                map("n", "<leader>gd", gs.diffthis, { desc = "Diff Against Index" })
                map("n", "<leader>gs", gs.stage_hunk, { desc = "Stage Hunk" })
                map("n", "<leader>gr", gs.reset_hunk, { desc = "Reset Hunk" })
                map("v", "<leader>gs", function()
                    gs.stage_hunk({ vim.fn.line("."), vim.fn.line("v") })
                end, { desc = "Stage Selected Hunk" })
                map("v", "<leader>gr", function()
                    gs.reset_hunk({ vim.fn.line("."), vim.fn.line("v") })
                end, { desc = "Reset Selected Hunk" })
            end,
        },
    },
}
