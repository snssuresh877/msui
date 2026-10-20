-- ============================================================
-- STARTUP
-- ============================================================

local function is_project()
    local cwd = vim.fn.getcwd()

    -- Git repository
    if vim.fn.isdirectory(cwd .. "/.git") == 1 then
        return true
    end

    -- Common project markers
    local markers = {
        "Cargo.toml",
        "CMakeLists.txt",
        "Makefile",
        "package.json",
        "pyproject.toml",
        "go.mod",
        "build.gradle",
        ".git",
    }

    for _, marker in ipairs(markers) do
        if vim.fn.filereadable(cwd .. "/" .. marker) == 1
            or vim.fn.isdirectory(cwd .. "/" .. marker) == 1 then
            return true
        end
    end

    return false
end

local function find_entry_file()
    local cwd = vim.fn.getcwd()

    local files = {
        "src/main.rs",
        "main.rs",

        "src/main.cpp",
        "main.cpp",

        "src/main.c",
        "main.c",

        "init.lua",
        "main.lua",

        "main.py",

        "main.sh",
    }

    for _, file in ipairs(files) do
        if vim.fn.filereadable(cwd .. "/" .. file) == 1 then
            return cwd .. "/" .. file
        end
    end

    return nil
end

vim.api.nvim_create_autocmd("VimEnter", {
    callback = function()
        -- Plain `nvim`
        if vim.fn.argc() == 0 and not is_project() then
            return
        end

        -- Project directory
        if vim.fn.argc() == 0 and is_project() then
            vim.schedule(function()
                local entry = find_entry_file()

                if entry then
                    vim.cmd("edit " .. vim.fn.fnameescape(entry))
                else
                    vim.cmd("enew")
                end

                vim.cmd("Neotree filesystem reveal left")
                vim.cmd("wincmd l")
            end)
        end
    end,
})
