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

local empty = { ['$empty'] = true }

local extern = {
  ["io::print"] = function(x)
    return function()
      print(x)
    end
  end,
  ["fs::read"] = function(path)
    return function()
      local output, success = read(path)

      if success then
        return variant("ok", output)
      else
        return variant("err", variant("not-found"))
      end
    end
  end,
  ["fs::write"] = function(path)
    return function(x)
      return function()
        local success = write(path, x)

        if success then
          return variant("ok", {})
        else
          return variant("err", variant("not-found"))
        end
      end
    end
  end,
  ["fs::read-dir"] = function(path)
    return function()
      local entries = empty

      local output, success = readdir(path)

      if not success then
        return variant("err", variant("not-found"))
      end

      for _, entry in string.split(output, "\n") do
        entries = cons(entry, entries)
      end

      return variant("ok", entries)
    end
  end,
  ["fs::is-dir"] = function(path)
    return function()
      return isdir(path)
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

local function with(x, fields)
  x = copy(x)

  for k, v in pairs(fields) do
    x[k] = v
  end

  return x
end

local function pure(x)
  return function() return x end
end

local function bind(x, y)
  return y(x())
end

local function numcast(x)
  return x
end

local function panic(message)
  print("explicit panic: " .. message .. "\n")
  os.exit(1)
end

local function dynamic(x)
  if type(x) == "table" and (x['$item'] ~= nil or x['$empty'] ~= nil) then
    local function list(y)
      if y['$empty'] == nil then
        return cons(dynamic(y['$item']), list(y['$rest']))
      else
        return y
      end
    end

    return variant("list", list(x))
  elseif type(x) == "table" and x['$variant'] ~= nil then
    local payload

    if x['$payload'] ~= nil then
      payload = variant("some", dynamic(x['$payload']))
    else
      payload = variant("none")
    end

    return variant("variant", { x['$variant'], payload })
  elseif type(x) == "table" and x[1] ~= nil then
    local fields = empty

    for i = #x, 1, -1 do
      fields = cons(dynamic(x[i]), fields)
    end

    return variant("tuple", fields)
  elseif type(x) == "table" then
    local fields = empty

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
  print(message)
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
  return (x * 2654435761) % 4294967296
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
      if b[k] == nil or not eq(v, b[k]) then
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
 v48 = function(v312) return dynamic(v312) end v47 = function(v314) local scrutinee = (v48)(v314) local result2957554391 if false then elseif scrutinee["$variant"] == "str'" then local v317 = scrutinee["$payload"] result2957554391 = (extern["io::print"])(v317) elseif true then result2957554391 = (extern["io::print"])((v1)(v314)) else panic("not arm reached") end return result2957554391 end v46 = function(v318) return bind((v47)(v318), function(v319) return (v47)("\n") end) end v45 = lazy(function()  return (v46)("hello") end) v11 = function(v422) return function(v423) local scrutinee = v423 local result2090772500 if false then elseif not scrutinee["$empty"] then local v424 = scrutinee["$item"] local v425 = scrutinee["$rest"] result2090772500 = cons((v422)(v424), ((v11)(v422))(v425)) elseif scrutinee["$empty"] then result2090772500 = empty else panic("not arm reached") end return result2090772500 end end v10 = function(v426) return function(v427) return strfind(v427, v426) end end v9 = function(v429) return strlength(v429) end v8 = function(v431) return function(v432) local scrutinee = eq((((v7)((v9)(v431)))(v432))[1], v431) local result1963303583 if false then elseif scrutinee == false then result1963303583 = v432 elseif scrutinee == true then result1963303583 = (((v7)((v9)(v431)))(v432))[2] else panic("not arm reached") end return result1963303583 end end v7 = function(v434) return function(v435) return strsplitat(v435, v434) end end v6 = function(v437) return function(v438) return function(v439) local scrutinee = ((v10)(v437))(v439) local result2642660426 if false then elseif scrutinee["$variant"] == "none" then result2642660426 = v439 elseif scrutinee["$variant"] == "some" then local v440 = scrutinee["$payload"] result2642660426 = ((v5)((((v7)(v440))(v439))[1]))(((v5)(v438))((((v6)(v437))(v438))(((v8)(v437))((((v7)(v440))(v439))[2])))) else panic("not arm reached") end return result2642660426 end end end v5 = function(v442) return function(v443) return (v442 .. v443) end end v4 = function(v445) return function(v446) return ((v5)(v446))(v445) end end v3 = function(v447) return function(v448) local scrutinee = v448 local result2090772571 if false then elseif not scrutinee["$empty"] then local v449 = scrutinee["$item"] local v450 = scrutinee["$rest"] local scrutinee = v450 local result2090772596 if false then elseif not scrutinee["$empty"] then local v451 = scrutinee["$item"] local v452 = scrutinee["$rest"] result2090772596 = ((v4)(((v3)(v447))(v450)))(((v4)(v447))(v449)) elseif scrutinee["$empty"] then result2090772596 = v449 else panic("not arm reached") end result2090772571 = result2090772596 elseif scrutinee["$empty"] then result2090772571 = "" else panic("not arm reached") end return result2090772571 end end v2 = function(v453) return function(v454) local scrutinee = v454 local result2090772600 if false then elseif scrutinee["$variant"] == "real'" then local v471 = scrutinee["$payload"] result2090772600 = tostring(v471) elseif scrutinee["$variant"] == "lambda" then result2090772600 = "lambda" elseif scrutinee["$variant"] == "tuple" then local v470 = scrutinee["$payload"] local scrutinee = (v453 > 0) local result4013632182 if false then elseif scrutinee == true then v483 = function(v423) local scrutinee = v423 local result2090772500 if false then elseif not scrutinee["$empty"] then local v424 = scrutinee["$item"] local v425 = scrutinee["$rest"] result2090772500 = cons(((v2)(1))(v424), (v483)(v425)) elseif scrutinee["$empty"] then result2090772500 = empty else panic("not arm reached") end return result2090772500 end result4013632182 = ((v4)(")"))(((v5)("("))(((v3)(", "))((v483)(v470)))) elseif scrutinee == false then v483 = function(v423) local scrutinee = v423 local result2090772500 if false then elseif not scrutinee["$empty"] then local v424 = scrutinee["$item"] local v425 = scrutinee["$rest"] result2090772500 = cons(((v2)(1))(v424), (v483)(v425)) elseif scrutinee["$empty"] then result2090772500 = empty else panic("not arm reached") end return result2090772500 end result4013632182 = ((v3)(", "))((v483)(v470)) else panic("not arm reached") end result2090772600 = result4013632182 elseif scrutinee["$variant"] == "record" then local v466 = scrutinee["$payload"] local scrutinee = v466 local result2090772635 if false then elseif not scrutinee["$empty"] then local v467 = scrutinee["$item"] local v468 = scrutinee["$rest"] v482 = function(v423) local scrutinee = v423 local result2090772500 if false then elseif not scrutinee["$empty"] then local v424 = scrutinee["$item"] local v425 = scrutinee["$rest"] result2090772500 = cons(((v4)(((v2)(0))((v424)[2])))(((v4)(": "))((v424)[1])), (v482)(v425)) elseif scrutinee["$empty"] then result2090772500 = empty else panic("not arm reached") end return result2090772500 end result2090772635 = ((v4)(" }"))(((v5)("{ "))(((v3)("; "))((v482)(v466)))) elseif scrutinee["$empty"] then result2090772635 = "{}" else panic("not arm reached") end result2090772600 = result2090772635 elseif scrutinee["$variant"] == "int'" then local v464 = scrutinee["$payload"] result2090772600 = tostring(v464) elseif scrutinee["$variant"] == "str'" then local v463 = scrutinee["$payload"] result2090772600 = ((v4)("\""))(((v5)("\""))((((v6)("\0"))("\\0"))((((v6)("\t"))("\\t"))((((v6)("\r"))("\\r"))((((v6)("\n"))("\\n"))((((v6)("\""))("\\\""))((((v6)("\\"))("\\\\"))(v463)))))))) elseif scrutinee["$variant"] == "nat'" then local v461 = scrutinee["$payload"] result2090772600 = tostring(v461) elseif scrutinee["$variant"] == "action" then result2090772600 = "action" elseif scrutinee["$variant"] == "list" then local v460 = scrutinee["$payload"] v481 = function(v423) local scrutinee = v423 local result2090772500 if false then elseif not scrutinee["$empty"] then local v424 = scrutinee["$item"] local v425 = scrutinee["$rest"] result2090772500 = cons(((v2)(0))(v424), (v481)(v425)) elseif scrutinee["$empty"] then result2090772500 = empty else panic("not arm reached") end return result2090772500 end result2090772600 = ((v4)("]"))(((v5)("["))(((v3)("; "))((v481)(v460)))) elseif scrutinee["$variant"] == "variant" then local v458 = scrutinee["$payload"] local scrutinee = (v458)[2] local result778663095 if false then elseif scrutinee["$variant"] == "none" then result778663095 = ((v5)(":"))((v458)[1]) elseif scrutinee["$variant"] == "some" then local v459 = scrutinee["$payload"] local scrutinee = (v453 > 1) local result4013632215 if false then elseif scrutinee == true then result4013632215 = ((v4)(")"))(((v5)("("))(((v4)(((v2)(1))(v459)))(((v4)(" "))(((v5)(":"))((v458)[1]))))) elseif scrutinee == false then result4013632215 = ((v4)(((v2)(1))(v459)))(((v4)(" "))(((v5)(":"))((v458)[1]))) else panic("not arm reached") end result778663095 = result4013632215 else panic("not arm reached") end result2090772600 = result778663095 else panic("not arm reached") end return result2090772600 end end v1 = function(v473) return ((v2)(0))(dynamic(v473)) end v45()()