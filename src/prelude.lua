local function variant(v, p)
  return {
    ['$variant'] = v,
    ['$payload'] = p,
  }
end

local function cons(i, r)
  return {
    ['$item'] = i,
    ['$rest'] = r,
  }
end

local extern = {
  ["io::print"] = function(x)
    return function()
      io.write(x)
    end
  end,
  ["fs::read"] = function(x)
    return function()
      local file = io.open(x, "r")

      if file == nil then
        return variant("err", variant("not-found"))
      end

      local contents = file:read("*a")
      file:close()

      if contents == nil then
        return variant("err", variant("not-found"))
      end

      return variant("ok", contents)
    end
  end,
  ["string::ansi-escape"] = "\x1b",
}

local function lazy(f)
  local value = nil

  return function()
    if value == nil then
      value = f()
    end

    return value
  end
end

local function copy(x)
  local output = {}

  for k, v in pairs(x) do
    output[k] = v
  end

  return output
end

local function dynamic(x)
  if type(x) == "table" and x['$item'] ~= nil then
    local function list(y)
      if y ~= nil then
        return cons(dynamic(y['$item']), list(y['$rest']))
      else
        return nil
      end
    end

    return variant("list", list(x))
  elseif type(x) == "nil" then
    return variant("list", nil)
  elseif type(x) == "table" and x['$variant'] ~= nil then
    local payload

    if x['$payload'] ~= nil then
      payload = variant("some", dynamic(x['$payload']))
    else
      payload = variant("none")
    end

    return variant("variant", { x['$variant'], payload })
  elseif type(x) == "table" and x[1] ~= nil then
    local fields = nil

    for i = #x, 1, -1 do
      fields = cons(dynamic(x[i]), fields)
    end

    return variant("tuple", fields)
  elseif type(x) == "table" then
    local fields = nil

    for k, v in pairs(x) do
      fields = cons({ k, dynamic(v) }, fields)
    end

    return variant("record", fields)
  elseif type(x) == "boolean" then
    if x then
      return variant("variant", { "true", variant("none") })
    else
      return variant("variant", { "false", variant("none") })
    end
  elseif type(x) == "number" then
    return variant("real'", x)
  elseif type(x) == "string" then
    return variant("str'", x)
  elseif type(x) == "function" then
    return variant("action")
  end
end

local function trace(message, x)
  io.write(message)
  return x
end

local function hashstr(x)
  local hash = 5381

  for i = 1, #x do
    hash = (hash * 33) + string.byte(x, i)
    hash = math.fmod(hash, 4294967296)
  end

  return hash
end

local function hashnum(x)
  return x
end

local function utf8_chars(x)
  local i = 1
  local len = #x

  return function()
    if i > len then
      return nil
    end

    local byte = string.byte(x, i)
    local n = 3

    if byte < 128 then
      n = 0
    elseif byte < 224 then
      n = 1
    elseif byte < 240 then
      n = 2
    end

    local c = string.sub(x, i, i + n)
    i = i + n + 1

    return c
  end
end

local function utf8_to_byte(x, i)
  local b = 1

  for c in utf8_chars(x) do
    if i == 0 then break end

    i = i - 1
    b = b + #c
  end

  return b
end

local function byte_to_utf8(x, b)
  local i = 0

  for _ in utf8_chars(string.sub(x, 1, b)) do
    i = i + 1
  end

  return i
end

local function strlength(x)
  -- local length = 0

  -- for _ in utf8_chars(x) do
  --   length = length + 1
  -- end

  return #x
end

local function strsplitat(x, i)
  local b = utf8_to_byte(x, i)

  return { string.sub(x, 1, b - 1), string.sub(x, b) }
end

local function strfind(haystack, needle)
  needle = needle:gsub("([%(%)%.%%%+%-%*%?%[%^%$])", "%%%1")
  local b = string.find(haystack, needle)

  if b == nil then
    return variant("none")
  end

  return variant("some", byte_to_utf8(haystack, b) - 1)
end

local function eq(a, b)
  if type(a) == "table" then
    for k, v in pairs(a) do
      if v ~= b[k] then
        return false
      end
    end

    return true
  else
    return a == b
  end
end

local function ne(a, b)
  return not eq(a, b)
end
