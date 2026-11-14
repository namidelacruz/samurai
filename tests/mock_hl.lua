-- Runs a generated Hyprland config against a stand-in `hl` that accepts any call, and counts what
-- was called. Checks syntax and that the file runs top to bottom; it is not Hyprland.
local calls = {}
local function node(path)
  return setmetatable({}, {
    __index = function(_, k) return node(path .. "." .. k) end,
    __call = function(_, ...)
      calls[path] = (calls[path] or 0) + 1
      local a = ...
      if path == "hl.on" then select(2, ...)() end -- run the autostart body too
      return node(path .. "()")
    end,
  })
end
hl = node("hl")
dofile(arg[1])
local keys = {}
for k in pairs(calls) do keys[#keys + 1] = k end
table.sort(keys)
for _, k in ipairs(keys) do print(string.format("%4d  %s", calls[k], k)) end
