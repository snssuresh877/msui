-- ============================================================
-- NATIVE BUFFER TABLINE
-- ============================================================

vim.opt.showtabline = 2

vim.opt.tabline = "%!v:lua.BufferTabline()"

function _G.BufferTabline()
    local result = {}
    local current = vim.api.nvim_get_current_buf()

    for _, bufnr in ipairs(vim.api.nvim_list_bufs()) do
        if vim.api.nvim_buf_is_valid(bufnr)
            and vim.bo[bufnr].buflisted then

            local name = vim.api.nvim_buf_get_name(bufnr)

            if name == "" then
                name = "[No Name]"
            else
                name = vim.fn.fnamemodify(name, ":t")
            end

            if bufnr == current then
                table.insert(
                    result,
                    "%#TabLineSel#  " .. name .. "  "
                )
            else
                table.insert(
                    result,
                    "%#TabLine#  " .. name .. "  "
                )
            end
        end
    end

    table.insert(result, "%#TabLineFill#")

    return table.concat(result)
end
