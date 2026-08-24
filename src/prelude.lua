local global = {}
local extern = {
  ["io::print"] = function(x)
    io.write(x)
  end
}

local function copy(x)
  local output = {}

  for k, v in pairs(x) do
    output[k] = v
  end

  return output
end

local function dynamic(x)
  if type(x) == "table" and x.variant ~= nil then
    local payload

    if x.payload ~= nil then
      payload = {
        variant = "some",
        payload = dynamic(x.payload)
      }
    else
      payload = { variant = "none" }
    end

    return {
      variant = "variant",
      payload = { x.variant, payload }
    }
  elseif type(x) == "table" and x[1] ~= nil then
    local fields = { variant = "none" }

    for i = #x, 1, -1 do
      fields = {
        variant = "some",
        payload = { dynamic(x[i]), fields },
      }
    end

    return { variant = "tuple", payload = fields }
  elseif type(x) == "table" then
    local fields = { variant = "none" }

    for k, v in pairs(x) do
      fields = {
        variant = "some",
        payload = { { k, dynamic(v) }, fields },
      }
    end

    return { variant = "record", payload = fields }
  elseif type(x) == "boolean" then
  elseif type(x) == "number" then
    return { variant = "real'", payload = x }
  elseif type(x) == "string" then
    return { variant = "str'", payload = x }
  end
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
  local length = 0

  for _ in utf8_chars(x) do
    length = length + 1
  end

  return length
end

local function strsplitat(x, i)
  local b = utf8_to_byte(x, i)

  return { string.sub(x, 1, b - 1), string.sub(x, b) }
end

local function strfind(haystack, needle)
  local b = string.find(haystack, needle)

  if b == nil then
    return { variant = "none" }
  end

  return {
    variant = "some",
    payload = byte_to_utf8(haystack, b) - 1,
  }
end

local function eq(a, b)
  return a == b
end
