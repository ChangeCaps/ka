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

--starts-with
v0 = lazy(function()  return function(v477) return function(v478) local scrutinee = eq((strsplitat(v478, strlength(v477)))[1], v477) local result2006336190 if false then elseif scrutinee == false then result2006336190 = v478 elseif scrutinee == true then result2006336190 = (strsplitat(v478, strlength(v477)))[2] else panic("not arm reached") end return ne(result2006336190, v478) end end end)

--trim
v1 = lazy(function()  return function(v474) return function(v475) local scrutinee = eq((strsplitat(v475, strlength(v474)))[1], v474) local result4178150037 if false then elseif scrutinee == false then result4178150037 = v475 elseif scrutinee == true then result4178150037 = (strsplitat(v475, strlength(v474)))[2] else panic("not arm reached") end return result4178150037 end end end)

--length
v2 = lazy(function()  return function(v472) return strlength(v472) end end)

--split-at
v3 = lazy(function()  return function(v469) return function(v470) return strsplitat(v470, v469) end end end)

--is-dir
v4 = lazy(function()  return extern["fs::is-dir"] end)

--is-not-empty
v5 = lazy(function()  return function(v468) local scrutinee = v468 local result2090772637 if false then elseif not scrutinee["$empty"] then local v466 = scrutinee["$item"] local v467 = scrutinee["$rest"] result2090772637 = false elseif scrutinee["$empty"] then result2090772637 = true else panic("not arm reached") end local scrutinee = result2090772637 local result3235910831 if false then elseif scrutinee == true then result3235910831 = false elseif scrutinee == false then result3235910831 = true else panic("not arm reached") end return result3235910831 end end)

--is-empty
v6 = lazy(function()  return function(v465) local scrutinee = v465 local result2090772634 if false then elseif not scrutinee["$empty"] then local v466 = scrutinee["$item"] local v467 = scrutinee["$rest"] result2090772634 = false elseif scrutinee["$empty"] then result2090772634 = true else panic("not arm reached") end return result2090772634 end end)

--map-ok
v7 = lazy(function()  return function(v461) return function(v462) local scrutinee = v462 local result2090772631 if false then elseif scrutinee["$variant"] == "ok" then local v464 = scrutinee["$payload"] result2090772631 = variant("ok", (v461)(v464)) elseif scrutinee["$variant"] == "err" then local v463 = scrutinee["$payload"] result2090772631 = variant("err", v463) else panic("not arm reached") end return result2090772631 end end end)

--fold
v8 = lazy(function()  return function(v456) return function(v457) return function(v458) local scrutinee = v458 local result2090772604 if false then elseif not scrutinee["$empty"] then local v459 = scrutinee["$item"] local v460 = scrutinee["$rest"] local scrutinee = v460 local result2090772629 if false then elseif not scrutinee["$empty"] then local v459 = scrutinee["$item"] local v460 = scrutinee["$rest"] result2090772629 = (((v8())(((v10000001)(((v10000001)(v10000000))(v459)))(v459)))(v10000001))(v460) elseif scrutinee["$empty"] then result2090772629 = ((v10000001)(v10000000))(v459) else panic("not arm reached") end result2090772604 = result2090772629 elseif scrutinee["$empty"] then result2090772604 = v456 else panic("not arm reached") end return result2090772604 end end end end)

--graphemes
v9 = lazy(function()  return function(v454) local scrutinee = eq(v454, "") local result1128123119 if false then elseif scrutinee == true then result1128123119 = variant("none") elseif scrutinee == false then result1128123119 = variant("some", strsplitat(v454, 1)) else panic("not arm reached") end local scrutinee = result1128123119 local result352194817 if false then elseif scrutinee["$variant"] == "some" then local v455 = scrutinee["$payload"] local scrutinee = (v10())((v455)[2]) local result2596153918 if false then elseif scrutinee["$variant"] == "some" then local v455 = scrutinee["$payload"] result2596153918 = cons((v455)[1], (v9())((v455)[2])) elseif scrutinee["$variant"] == "none" then result2596153918 = empty else panic("not arm reached") end result352194817 = cons((v455)[1], result2596153918) elseif scrutinee["$variant"] == "none" then result352194817 = empty else panic("not arm reached") end return result352194817 end end)

--pop
v10 = lazy(function()  return function(v453) local scrutinee = eq(v453, "") local result1088987726 if false then elseif scrutinee == true then result1088987726 = variant("none") elseif scrutinee == false then result1088987726 = variant("some", strsplitat(v453, 1)) else panic("not arm reached") end return result1088987726 end end)

--is-empty
v11 = lazy(function()  return function(v452) return eq(v452, "") end end)

--last
v12 = lazy(function()  return function(v447) local scrutinee = v447 local result2090772570 if false then elseif not scrutinee["$empty"] then local v448 = scrutinee["$item"] local v449 = scrutinee["$rest"] local scrutinee = v449 local result2090772572 if false then elseif not scrutinee["$empty"] then local v450 = scrutinee["$item"] local v451 = scrutinee["$rest"] local scrutinee = v449 local result2090772572 if false then elseif not scrutinee["$empty"] then local v448 = scrutinee["$item"] local v449 = scrutinee["$rest"] local scrutinee = v449 local result2090772572 if false then elseif not scrutinee["$empty"] then local v450 = scrutinee["$item"] local v451 = scrutinee["$rest"] result2090772572 = (v12())(v449) elseif scrutinee["$empty"] then result2090772572 = variant("some", v448) else panic("not arm reached") end result2090772572 = result2090772572 elseif scrutinee["$empty"] then result2090772572 = variant("none") else panic("not arm reached") end result2090772572 = result2090772572 elseif scrutinee["$empty"] then result2090772572 = variant("some", v448) else panic("not arm reached") end result2090772570 = result2090772572 elseif scrutinee["$empty"] then result2090772570 = variant("none") else panic("not arm reached") end return result2090772570 end end)

--is-none
v13 = lazy(function()  return function(v445) local scrutinee = v445 local result2090772568 if false then elseif scrutinee["$variant"] == "some" then local v446 = scrutinee["$payload"] result2090772568 = false elseif scrutinee["$variant"] == "none" then result2090772568 = true else panic("not arm reached") end return result2090772568 end end)

--find-map
v14 = lazy(function()  return function(v440) return function(v441) local scrutinee = v441 local result2090772564 if false then elseif not scrutinee["$empty"] then local v442 = scrutinee["$item"] local v443 = scrutinee["$rest"] local scrutinee = (v440)(v442) local result4177528773 if false then elseif scrutinee["$variant"] == "some" then local v444 = scrutinee["$payload"] result4177528773 = variant("some", v444) elseif scrutinee["$variant"] == "none" then local scrutinee = v443 local result2090772566 if false then elseif not scrutinee["$empty"] then local v442 = scrutinee["$item"] local v443 = scrutinee["$rest"] local scrutinee = (v10000000)(v442) local result382817230 if false then elseif scrutinee["$variant"] == "some" then local v444 = scrutinee["$payload"] result382817230 = variant("some", v444) elseif scrutinee["$variant"] == "none" then result382817230 = ((v14())(v10000000))(v443) else panic("not arm reached") end result2090772566 = result382817230 elseif scrutinee["$empty"] then result2090772566 = variant("none") else panic("not arm reached") end result4177528773 = result2090772566 else panic("not arm reached") end result2090772564 = result4177528773 elseif scrutinee["$empty"] then result2090772564 = variant("none") else panic("not arm reached") end return result2090772564 end end end)

--push
v15 = lazy(function()  return function(v438) return function(v439) return cons(v438, v439) end end end)

--main
v16 = lazy(function()  return cons("", ((v17())(function(v437) return v437 end))(cons("", cons("", empty)))) end)

--map
v17 = lazy(function()  return function(v433) return function(v434) local scrutinee = v434 local result2090772534 if false then elseif not scrutinee["$empty"] then local v435 = scrutinee["$item"] local v436 = scrutinee["$rest"] local scrutinee = v436 local result2090772536 if false then elseif not scrutinee["$empty"] then local v435 = scrutinee["$item"] local v436 = scrutinee["$rest"] result2090772536 = cons((v10000000)(v435), ((v17())(v10000000))(v436)) elseif scrutinee["$empty"] then result2090772536 = empty else panic("not arm reached") end result2090772534 = cons((v433)(v435), result2090772536) elseif scrutinee["$empty"] then result2090772534 = empty else panic("not arm reached") end return result2090772534 end end end)

--reverse
v18 = lazy(function()  return function(v432) local scrutinee = v432 local result2090772532 if false then elseif not scrutinee["$empty"] then local v430 = scrutinee["$item"] local v431 = scrutinee["$rest"] result2090772532 = ((v19())(v431))(cons(v430, empty)) elseif scrutinee["$empty"] then result2090772532 = empty else panic("not arm reached") end return result2090772532 end end)

--extend-reversed
v19 = lazy(function()  return function(v428) return function(v429) local scrutinee = v428 local result2090772505 if false then elseif not scrutinee["$empty"] then local v430 = scrutinee["$item"] local v431 = scrutinee["$rest"] local scrutinee = v431 local result2090772531 if false then elseif not scrutinee["$empty"] then local v430 = scrutinee["$item"] local v431 = scrutinee["$rest"] result2090772531 = ((v19())(v431))(cons(v430, cons(v430, v10000001))) elseif scrutinee["$empty"] then result2090772531 = cons(v430, v10000001) else panic("not arm reached") end result2090772505 = result2090772531 elseif scrutinee["$empty"] then result2090772505 = v429 else panic("not arm reached") end return result2090772505 end end end)

--try-map-some
v20 = lazy(function()  return function(v425) return function(v426) local scrutinee = v426 local result2090772503 if false then elseif scrutinee["$variant"] == "some" then local v427 = scrutinee["$payload"] result2090772503 = (v425)(v427) elseif scrutinee["$variant"] == "none" then result2090772503 = variant("none") else panic("not arm reached") end return result2090772503 end end end)

--insert
v21 = lazy(function()  return function(v422) return function(v423) return function(v424) return ((((v25())((v23())(dynamic(v422))))(v422))(v423))(v424) end end end end)

--hash
v22 = lazy(function()  return function(v420) return (v23())(dynamic(v420)) end end)

--hash-dynamic
v23 = lazy(function()  return function(v401) local scrutinee = v401 local result2090772432 if false then elseif scrutinee["$variant"] == "lambda" then result2090772432 = hashnum(0) elseif scrutinee["$variant"] == "real'" then local v417 = scrutinee["$payload"] result2090772432 = hashnum(v417) elseif scrutinee["$variant"] == "tuple" then local v416 = scrutinee["$payload"] result2090772432 = (((v8())(0))(v24()))(((v17())(v23()))(v416)) elseif scrutinee["$variant"] == "record" then local v414 = scrutinee["$payload"] result2090772432 = (((v8())(0))(v24()))(((v17())(function(v415) return (v23())((v415)[2]) end))(v414)) elseif scrutinee["$variant"] == "int'" then local v412 = scrutinee["$payload"] result2090772432 = hashnum(v412) elseif scrutinee["$variant"] == "action" then result2090772432 = hashnum(1) elseif scrutinee["$variant"] == "str'" then local v409 = scrutinee["$payload"] result2090772432 = hashstr(v409) elseif scrutinee["$variant"] == "nat'" then local v407 = scrutinee["$payload"] result2090772432 = hashnum(v407) elseif scrutinee["$variant"] == "list" then local v406 = scrutinee["$payload"] local scrutinee = v406 local result2090772437 if false then elseif not scrutinee["$empty"] then local v435 = scrutinee["$item"] local v436 = scrutinee["$rest"] result2090772437 = cons((v23())(v435), ((v17())(v23()))(v436)) elseif scrutinee["$empty"] then result2090772437 = empty else panic("not arm reached") end local scrutinee = result2090772437 local result3235908653 if false then elseif not scrutinee["$empty"] then local v459 = scrutinee["$item"] local v460 = scrutinee["$rest"] result3235908653 = (((v8())(((v24())(0))(v459)))(v24()))(v460) elseif scrutinee["$empty"] then result3235908653 = 0 else panic("not arm reached") end result2090772432 = result3235908653 elseif scrutinee["$variant"] == "variant" then local v402 = scrutinee["$payload"] local scrutinee = (v402)[2] local result575870604 if false then elseif scrutinee["$variant"] == "some" then local v404 = scrutinee["$payload"] result575870604 = bit32.bxor(hashstr((v402)[1]), (v23())(v404)) elseif scrutinee["$variant"] == "none" then result575870604 = hashstr((v402)[1]) else panic("not arm reached") end result2090772432 = result575870604 else panic("not arm reached") end return result2090772432 end end)

--xor
v24 = lazy(function()  return function(v398) return function(v399) return bit32.bxor(v398, v399) end end end)

--insert-hashed
v25 = lazy(function()  return function(v393) return function(v394) return function(v395) return function(v396) local scrutinee = v396 local result2090771645 if false then elseif scrutinee["$variant"] == "none" then result2090771645 = variant("some", { ["hash"] = v393, ["key"] = v394, ["value"] = v395, ["lhs"] = variant("none"), ["rhs"] = variant("none") }) elseif scrutinee["$variant"] == "some" then local v397 = scrutinee["$payload"] local scrutinee = (eq((v397)["hash"], v393) and eq((v397)["key"], v394)) local result1554157715 if false then elseif scrutinee == true then result1554157715 = variant("some", with(v397, { ["value"] = v395 })) elseif scrutinee == false then local scrutinee = (v393 > (v397)["hash"]) local result3554349619 if false then elseif scrutinee == true then result3554349619 = variant("some", with(v397, { ["rhs"] = ((((v25())(v393))(v394))(v395))((v397)["rhs"]) })) elseif scrutinee == false then result3554349619 = variant("some", with(v397, { ["lhs"] = ((((v25())(v393))(v394))(v395))((v397)["lhs"]) })) else panic("not arm reached") end result1554157715 = result3554349619 else panic("not arm reached") end result2090771645 = result1554157715 else panic("not arm reached") end return result2090771645 end end end end end)

--remove
v26 = lazy(function()  return function(v391) return function(v392) return (((v27())((v23())(dynamic(v391))))(v391))(v392) end end end)

--remove-hashed
v27 = lazy(function()  return function(v383) return function(v384) return function(v385) local scrutinee = v385 local result2090771611 if false then elseif scrutinee["$variant"] == "some" then local v427 = scrutinee["$payload"] local scrutinee = (eq((v427)["hash"], v383) and eq((v427)["key"], v384)) local result3919823173 if false then elseif scrutinee == true then local scrutinee = (v427)["lhs"] local result3595480748 if false then elseif scrutinee["$variant"] == "none" then local scrutinee = (v427)["rhs"] local result3602596274 if false then elseif scrutinee["$variant"] == "none" then result3602596274 = variant("none") elseif scrutinee["$variant"] == "some" then local v390 = scrutinee["$payload"] result3602596274 = variant("some", v390) else panic("not arm reached") end result3595480748 = result3602596274 elseif scrutinee["$variant"] == "some" then local v387 = scrutinee["$payload"] local scrutinee = (v427)["rhs"] local result3602596274 if false then elseif scrutinee["$variant"] == "none" then result3602596274 = variant("some", v387) elseif scrutinee["$variant"] == "some" then local v388 = scrutinee["$payload"] local scrutinee = (v388)["lhs"] local result3719156018 if false then elseif scrutinee["$variant"] == "none" then result3719156018 = { (v388)["hash"], (v388)["key"], (v388)["value"] } elseif scrutinee["$variant"] == "some" then local v382 = scrutinee["$payload"] result3719156018 = (v28())(v382) else panic("not arm reached") end local scrutinee = (v388)["lhs"] local result3719156018 if false then elseif scrutinee["$variant"] == "none" then result3719156018 = { (v388)["hash"], (v388)["key"], (v388)["value"] } elseif scrutinee["$variant"] == "some" then local v382 = scrutinee["$payload"] result3719156018 = (v28())(v382) else panic("not arm reached") end local scrutinee = (v388)["lhs"] local result3719156018 if false then elseif scrutinee["$variant"] == "none" then result3719156018 = { (v388)["hash"], (v388)["key"], (v388)["value"] } elseif scrutinee["$variant"] == "some" then local v382 = scrutinee["$payload"] result3719156018 = (v28())(v382) else panic("not arm reached") end local scrutinee = (v388)["lhs"] local result3719156018 if false then elseif scrutinee["$variant"] == "none" then result3719156018 = { (v388)["hash"], (v388)["key"], (v388)["value"] } elseif scrutinee["$variant"] == "some" then local v382 = scrutinee["$payload"] result3719156018 = (v28())(v382) else panic("not arm reached") end local scrutinee = (v388)["lhs"] local result3719156018 if false then elseif scrutinee["$variant"] == "none" then result3719156018 = { (v388)["hash"], (v388)["key"], (v388)["value"] } elseif scrutinee["$variant"] == "some" then local v382 = scrutinee["$payload"] result3719156018 = (v28())(v382) else panic("not arm reached") end result3602596274 = variant("some", with(v427, { ["rhs"] = (((v27())((result3719156018)[1]))((result3719156018)[2]))(variant("some", v388)), ["value"] = (result3719156018)[3], ["key"] = (result3719156018)[2], ["hash"] = (result3719156018)[1] })) else panic("not arm reached") end result3595480748 = result3602596274 else panic("not arm reached") end result3919823173 = result3595480748 elseif scrutinee == false then local scrutinee = (v383 > (v427)["hash"]) local result7036844 if false then elseif scrutinee == true then result7036844 = variant("some", with(v427, { ["rhs"] = (((v27())(v383))(v384))((v427)["rhs"]) })) elseif scrutinee == false then result7036844 = variant("some", with(v427, { ["lhs"] = (((v27())(v383))(v384))((v427)["lhs"]) })) else panic("not arm reached") end result3919823173 = result7036844 else panic("not arm reached") end result2090771611 = result3919823173 elseif scrutinee["$variant"] == "none" then result2090771611 = variant("none") else panic("not arm reached") end return result2090771611 end end end end)

--smallest
v28 = lazy(function()  return function(v381) local scrutinee = (v381)["lhs"] local result2923870763 if false then elseif scrutinee["$variant"] == "none" then result2923870763 = { (v381)["hash"], (v381)["key"], (v381)["value"] } elseif scrutinee["$variant"] == "some" then local v382 = scrutinee["$payload"] local scrutinee = (v382)["lhs"] local result583215916 if false then elseif scrutinee["$variant"] == "none" then result583215916 = { (v382)["hash"], (v382)["key"], (v382)["value"] } elseif scrutinee["$variant"] == "some" then local v382 = scrutinee["$payload"] result583215916 = (v28())(v382) else panic("not arm reached") end result2923870763 = result583215916 else panic("not arm reached") end return result2923870763 end end)

--remove
v29 = lazy(function()  return function(v379) return function(v380) return (((v27())((v23())(dynamic(v379))))(v379))(v380) end end end)

--join
v30 = lazy(function()  return function(v373) return function(v374) local scrutinee = v374 local result2090771577 if false then elseif not scrutinee["$empty"] then local v375 = scrutinee["$item"] local v376 = scrutinee["$rest"] local scrutinee = v376 local result2090771579 if false then elseif not scrutinee["$empty"] then local v377 = scrutinee["$item"] local v378 = scrutinee["$rest"] local scrutinee = v376 local result2090771579 if false then elseif not scrutinee["$empty"] then local v375 = scrutinee["$item"] local v376 = scrutinee["$rest"] local scrutinee = v376 local result2090771579 if false then elseif not scrutinee["$empty"] then local v377 = scrutinee["$item"] local v378 = scrutinee["$rest"] result2090771579 = (((v31())(v10000005))(v375) .. ((v30())(v10000005))(v376)) elseif scrutinee["$empty"] then result2090771579 = v375 else panic("not arm reached") end result2090771579 = result2090771579 elseif scrutinee["$empty"] then result2090771579 = "" else panic("not arm reached") end result2090771579 = ((v31())(result2090771579))(((v31())(v373))(v375)) elseif scrutinee["$empty"] then result2090771579 = v375 else panic("not arm reached") end result2090771577 = result2090771579 elseif scrutinee["$empty"] then result2090771577 = "" else panic("not arm reached") end return result2090771577 end end end)

--append
v31 = lazy(function()  return function(v371) return function(v372) return (v372 .. v371) end end end)

--prepend
v32 = lazy(function()  return function(v368) return function(v369) return (v368 .. v369) end end end)

--println
v33 = lazy(function()  return function(v366) local scrutinee = dynamic(v366) local result293438192 if false then elseif scrutinee["$variant"] == "str'" then local v365 = scrutinee["$payload"] result293438192 = (extern["io::print"])(v365) elseif true then result293438192 = (extern["io::print"])(((v36())(0))(dynamic(v366))) else panic("not arm reached") end return bind(result293438192, function(v367) return (v34())("\n") end) end end)

--print
v34 = lazy(function()  return function(v362) local scrutinee = dynamic(v362) local result293438060 if false then elseif scrutinee["$variant"] == "str'" then local v365 = scrutinee["$payload"] result293438060 = (extern["io::print"])(v365) elseif true then result293438060 = (extern["io::print"])(((v36())(0))(dynamic(v362))) else panic("not arm reached") end return result293438060 end end)

--format
v35 = lazy(function()  return function(v360) return ((v36())(0))(dynamic(v360)) end end)

--format-dynamic
v36 = lazy(function()  return function(v340) return function(v341) local scrutinee = v341 local result2090771475 if false then elseif scrutinee["$variant"] == "real'" then local v358 = scrutinee["$payload"] result2090771475 = tostring(v358) elseif scrutinee["$variant"] == "lambda" then result2090771475 = "lambda" elseif scrutinee["$variant"] == "tuple" then local v357 = scrutinee["$payload"] local scrutinee = (v340 > 0) local result2935988017 if false then elseif scrutinee == true then result2935988017 = ((v31())(")"))(((v32())("("))(((v30())(", "))(((v17())((v36())(1)))(v357)))) elseif scrutinee == false then result2935988017 = ((v30())(", "))(((v17())((v36())(1)))(v357)) else panic("not arm reached") end result2090771475 = result2935988017 elseif scrutinee["$variant"] == "record" then local v353 = scrutinee["$payload"] local scrutinee = v353 local result2090771510 if false then elseif not scrutinee["$empty"] then local v354 = scrutinee["$item"] local v355 = scrutinee["$rest"] result2090771510 = ((v31())(" }"))(((v32())("{ "))(((v30())("; "))(((v17())(function(v356) return ((v31())(((v36())(0))((v356)[2])))(((v31())(": "))((v356)[1])) end))(v353)))) elseif scrutinee["$empty"] then result2090771510 = "{}" else panic("not arm reached") end result2090771475 = result2090771510 elseif scrutinee["$variant"] == "int'" then local v351 = scrutinee["$payload"] result2090771475 = tostring(v351) elseif scrutinee["$variant"] == "str'" then local v350 = scrutinee["$payload"] local v10000022 = (((v37())("\t"))("\\t"))((((v37())("\r"))("\\r"))((((v37())("\n"))("\\n"))((((v37())("\""))("\\\""))((((v37())("\\"))("\\\\"))(v350))))) local scrutinee = strfind(v10000022, "\0") local result1281827623 if false then elseif scrutinee["$variant"] == "none" then result1281827623 = v10000022 elseif scrutinee["$variant"] == "some" then local v338 = scrutinee["$payload"] local scrutinee = eq((strsplitat((((v3())(v338))(v10000022))[2], strlength("\0")))[1], "\0") local result3955363709 if false then elseif scrutinee == false then result3955363709 = (((v3())(v338))(v10000022))[2] elseif scrutinee == true then result3955363709 = (strsplitat((((v3())(v338))(v10000022))[2], strlength("\0")))[2] else panic("not arm reached") end result1281827623 = ((v32())((((v3())(v338))(v10000022))[1]))(((v32())("\\0"))((((v37())("\0"))("\\0"))(result3955363709))) else panic("not arm reached") end result2090771475 = ((v31())("\""))(((v32())("\""))(result1281827623)) elseif scrutinee["$variant"] == "nat'" then local v348 = scrutinee["$payload"] result2090771475 = tostring(v348) elseif scrutinee["$variant"] == "action" then result2090771475 = "action" elseif scrutinee["$variant"] == "list" then local v347 = scrutinee["$payload"] local scrutinee = v347 local result2090771481 if false then elseif not scrutinee["$empty"] then local v435 = scrutinee["$item"] local v436 = scrutinee["$rest"] result2090771481 = cons(((v36())(0))(v435), ((v17())((v36())(0)))(v436)) elseif scrutinee["$empty"] then result2090771481 = empty else panic("not arm reached") end local scrutinee = result2090771481 local result3235872875 if false then elseif not scrutinee["$empty"] then local v375 = scrutinee["$item"] local v376 = scrutinee["$rest"] local scrutinee = v376 local result2090771579 if false then elseif not scrutinee["$empty"] then local v377 = scrutinee["$item"] local v378 = scrutinee["$rest"] result2090771579 = ((v31())(((v30())("; "))(v376)))(((v31())("; "))(v375)) elseif scrutinee["$empty"] then result2090771579 = v375 else panic("not arm reached") end result3235872875 = result2090771579 elseif scrutinee["$empty"] then result3235872875 = "" else panic("not arm reached") end result2090771475 = ((v31())("]"))(((v32())("["))(result3235872875)) elseif scrutinee["$variant"] == "variant" then local v345 = scrutinee["$payload"] local scrutinee = (v345)[2] local result3739469266 if false then elseif scrutinee["$variant"] == "none" then result3739469266 = ((v32())(":"))((v345)[1]) elseif scrutinee["$variant"] == "some" then local v346 = scrutinee["$payload"] local scrutinee = (v340 > 1) local result2935988050 if false then elseif scrutinee == true then result2935988050 = ((v31())(")"))(((v32())("("))((((v31())(" "))(((v32())(":"))((v345)[1])) .. ((v36())(1))(v346)))) elseif scrutinee == false then result2935988050 = (((v31())(" "))(((v32())(":"))((v345)[1])) .. ((v36())(1))(v346)) else panic("not arm reached") end result3739469266 = result2935988050 else panic("not arm reached") end result2090771475 = result3739469266 else panic("not arm reached") end return result2090771475 end end end)

--replace
v37 = lazy(function()  return function(v335) return function(v336) return function(v337) local scrutinee = strfind(v337, v335) local result2866114848 if false then elseif scrutinee["$variant"] == "none" then result2866114848 = v337 elseif scrutinee["$variant"] == "some" then local v338 = scrutinee["$payload"] local scrutinee = ((v38())(v10000012))(((v1())(v335))((((v3())(v338))(v337))[2])) local result324490978 if false then elseif scrutinee["$variant"] == "none" then result324490978 = ((v1())(v335))((((v3())(v338))(v337))[2]) elseif scrutinee["$variant"] == "some" then local v338 = scrutinee["$payload"] local scrutinee = eq((strsplitat((((v3())(v338))(((v1())(v335))((((v3())(v338))(v337))[2])))[2], strlength(v10000012)))[1], v10000012) local result4023327811 if false then elseif scrutinee == false then result4023327811 = (((v3())(v338))(((v1())(v335))((((v3())(v338))(v337))[2])))[2] elseif scrutinee == true then result4023327811 = (strsplitat((((v3())(v338))(((v1())(v335))((((v3())(v338))(v337))[2])))[2], strlength(v10000012)))[2] else panic("not arm reached") end result324490978 = ((v32())((((v3())(v338))(((v1())(v335))((((v3())(v338))(v337))[2])))[1]))(((v32())(v10000013))((((v37())(v10000012))(v10000013))(result4023327811))) else panic("not arm reached") end result2866114848 = ((((v3())(v338))(v337))[1] .. ((v32())(v336))(result324490978)) else panic("not arm reached") end return result2866114848 end end end end)

--find
v38 = lazy(function()  return function(v332) return function(v333) return strfind(v333, v332) end end end)

--dynamic
v39 = lazy(function()  return function(v330) return dynamic(v330) end end)

--filter-some
v40 = lazy(function()  return function(v326) local scrutinee = v326 local result2090771414 if false then elseif not scrutinee["$empty"] then local v327 = scrutinee["$item"] local v328 = scrutinee["$rest"] local scrutinee = v327 local result2090771415 if false then elseif scrutinee["$variant"] == "some" then local v329 = scrutinee["$payload"] result2090771415 = cons(v329, (v40())(v328)) elseif scrutinee["$variant"] == "none" then local scrutinee = v328 local result2090771416 if false then elseif not scrutinee["$empty"] then local v327 = scrutinee["$item"] local v328 = scrutinee["$rest"] local scrutinee = v327 local result2090771415 if false then elseif scrutinee["$variant"] == "some" then local v329 = scrutinee["$payload"] result2090771415 = cons(v329, (v40())(v328)) elseif scrutinee["$variant"] == "none" then result2090771415 = (v40())(v328) else panic("not arm reached") end result2090771416 = result2090771415 elseif scrutinee["$empty"] then result2090771416 = empty else panic("not arm reached") end result2090771415 = result2090771416 else panic("not arm reached") end result2090771414 = result2090771415 elseif scrutinee["$empty"] then result2090771414 = empty else panic("not arm reached") end return result2090771414 end end)

--nth
v41 = lazy(function()  return function(v324) return function(v325) local scrutinee = v325 local result2090771413 if false then elseif not scrutinee["$empty"] then local v319 = scrutinee["$item"] local v320 = scrutinee["$rest"] local scrutinee = (v324 > 0) local result509593651 if false then elseif scrutinee == true then result509593651 = ((v43())((v324 - 1)))(v320) elseif scrutinee == false then result509593651 = v325 else panic("not arm reached") end result2090771413 = result509593651 elseif scrutinee["$empty"] then result2090771413 = empty else panic("not arm reached") end local scrutinee = result2090771413 local result3235872646 if false then elseif not scrutinee["$empty"] then local v322 = scrutinee["$item"] local v323 = scrutinee["$rest"] result3235872646 = variant("some", v322) elseif scrutinee["$empty"] then result3235872646 = variant("none") else panic("not arm reached") end return result3235872646 end end end)

--first
v42 = lazy(function()  return function(v321) local scrutinee = v321 local result2090771409 if false then elseif not scrutinee["$empty"] then local v322 = scrutinee["$item"] local v323 = scrutinee["$rest"] result2090771409 = variant("some", v322) elseif scrutinee["$empty"] then result2090771409 = variant("none") else panic("not arm reached") end return result2090771409 end end)

--skip
v43 = lazy(function()  return function(v317) return function(v318) local scrutinee = v318 local result2090771383 if false then elseif not scrutinee["$empty"] then local v319 = scrutinee["$item"] local v320 = scrutinee["$rest"] local scrutinee = (v317 > 0) local result3630499157 if false then elseif scrutinee == true then local scrutinee = v320 local result2090771408 if false then elseif not scrutinee["$empty"] then local v319 = scrutinee["$item"] local v320 = scrutinee["$rest"] local scrutinee = ((v10000000 - 1) > 0) local result1157380426 if false then elseif scrutinee == true then result1157380426 = ((v43())(((v10000000 - 1) - 1)))(v320) elseif scrutinee == false then result1157380426 = v320 else panic("not arm reached") end result2090771408 = result1157380426 elseif scrutinee["$empty"] then result2090771408 = empty else panic("not arm reached") end result3630499157 = result2090771408 elseif scrutinee == false then result3630499157 = v318 else panic("not arm reached") end result2090771383 = result3630499157 elseif scrutinee["$empty"] then result2090771383 = empty else panic("not arm reached") end return result2090771383 end end end)

--empty
v44 = lazy(function()  return v45() end)

--empty
v45 = lazy(function()  return variant("none") end)

--repeat
v46 = lazy(function()  return function(v315) return function(v316) local scrutinee = (v315 > 0) local result3552228371 if false then elseif scrutinee == true then local scrutinee = ((v10000004 - 1) > 0) local result1517227598 if false then elseif scrutinee == true then result1517227598 = ((v31())(v10000005))(((v46())(((v10000004 - 1) - 1)))(v10000005)) elseif scrutinee == false then result1517227598 = "" else panic("not arm reached") end result3552228371 = (result1517227598 .. v316) elseif scrutinee == false then result3552228371 = "" else panic("not arm reached") end return result3552228371 end end end)

--read-dir
v47 = lazy(function()  return extern["fs::read-dir"] end)

--repeat
v48 = lazy(function()  return function(v313) return function(v314) local scrutinee = (v313 > 0) local result3473957585 if false then elseif scrutinee == true then local scrutinee = ((v10000000 - 1) > 0) local result1157380426 if false then elseif scrutinee == true then result1157380426 = cons(v10000001, ((v48())(((v10000000 - 1) - 1)))(v10000001)) elseif scrutinee == false then result1157380426 = empty else panic("not arm reached") end result3473957585 = cons(v314, result1157380426) elseif scrutinee == false then result3473957585 = empty else panic("not arm reached") end return result3473957585 end end end)

--reduce
v49 = lazy(function()  return function(v308) return function(v309) local scrutinee = v309 local result2090771351 if false then elseif not scrutinee["$empty"] then local v459 = scrutinee["$item"] local v460 = scrutinee["$rest"] result2090771351 = (((v8())(variant("some", v459)))(function(v310) return function(v311) local scrutinee = v310 local result2090771375 if false then elseif scrutinee["$variant"] == "none" then result2090771375 = variant("some", v311) elseif scrutinee["$variant"] == "some" then local v312 = scrutinee["$payload"] result2090771375 = variant("some", ((v308)(v311))(v312)) else panic("not arm reached") end return result2090771375 end end))(v460) elseif scrutinee["$empty"] then result2090771351 = variant("none") else panic("not arm reached") end return result2090771351 end end end)

--try
v50 = lazy(function()  return function(v304) return function(v305) local scrutinee = v305 local result2090771347 if false then elseif scrutinee["$variant"] == "ok" then local v307 = scrutinee["$payload"] result2090771347 = (v304)(v307) elseif scrutinee["$variant"] == "err" then local v306 = scrutinee["$payload"] result2090771347 = variant("err", v306) else panic("not arm reached") end return result2090771347 end end end)

--rfold
v51 = lazy(function()  return function(v299) return function(v300) return function(v301) local scrutinee = v301 local result2090771343 if false then elseif not scrutinee["$empty"] then local v302 = scrutinee["$item"] local v303 = scrutinee["$rest"] local scrutinee = v303 local result2090771345 if false then elseif not scrutinee["$empty"] then local v302 = scrutinee["$item"] local v303 = scrutinee["$rest"] result2090771345 = ((v10000001)((((v51())(v10000000))(v10000001))(v303)))(v302) elseif scrutinee["$empty"] then result2090771345 = v10000000 else panic("not arm reached") end result2090771343 = ((v300)(result2090771345))(v302) elseif scrutinee["$empty"] then result2090771343 = v299 else panic("not arm reached") end return result2090771343 end end end end)

--pop-last
v52 = lazy(function()  return function(v295) local scrutinee = v295 local result2090770555 if false then elseif not scrutinee["$empty"] then local v296 = scrutinee["$item"] local v297 = scrutinee["$rest"] local scrutinee = v297 local result2090770557 if false then elseif not scrutinee["$empty"] then local v296 = scrutinee["$item"] local v297 = scrutinee["$rest"] local scrutinee = (v52())(v297) local result2383822669 if false then elseif scrutinee["$variant"] == "some" then local v298 = scrutinee["$payload"] result2383822669 = variant("some", { cons(v296, (v298)[1]), (v298)[2] }) elseif scrutinee["$variant"] == "none" then result2383822669 = variant("some", { empty, v296 }) else panic("not arm reached") end result2090770557 = result2383822669 elseif scrutinee["$empty"] then result2090770557 = variant("none") else panic("not arm reached") end local scrutinee = result2090770557 local result3235837934 if false then elseif scrutinee["$variant"] == "some" then local v298 = scrutinee["$payload"] result3235837934 = variant("some", { cons(v296, (v298)[1]), (v298)[2] }) elseif scrutinee["$variant"] == "none" then result3235837934 = variant("some", { empty, v296 }) else panic("not arm reached") end result2090770555 = result3235837934 elseif scrutinee["$empty"] then result2090770555 = variant("none") else panic("not arm reached") end return result2090770555 end end)

--some-if
v53 = lazy(function()  return function(v292) return function(v293) local scrutinee = v293 local result2090770553 if false then elseif scrutinee["$variant"] == "none" then result2090770553 = variant("none") elseif scrutinee["$variant"] == "some" then local v294 = scrutinee["$payload"] local scrutinee = (v292)(v294) local result266060047 if false then elseif scrutinee == true then result266060047 = variant("some", v294) elseif scrutinee == false then result266060047 = variant("none") else panic("not arm reached") end result2090770553 = result266060047 else panic("not arm reached") end return result2090770553 end end end)

--any
v54 = lazy(function()  return function(v290) return function(v291) local scrutinee = v291 local result2090770551 if false then elseif not scrutinee["$empty"] then local v286 = scrutinee["$item"] local v287 = scrutinee["$rest"] local scrutinee = (v290)(v286) local result928518990 if false then elseif scrutinee == true then result928518990 = variant("some", v286) elseif scrutinee == false then result928518990 = ((v56())(v290))(v287) else panic("not arm reached") end result2090770551 = result928518990 elseif scrutinee["$empty"] then result2090770551 = variant("none") else panic("not arm reached") end local scrutinee = result2090770551 local result3235837928 if false then elseif scrutinee["$variant"] == "some" then local v289 = scrutinee["$payload"] result3235837928 = true elseif scrutinee["$variant"] == "none" then result3235837928 = false else panic("not arm reached") end return result3235837928 end end end)

--is-some
v55 = lazy(function()  return function(v288) local scrutinee = v288 local result2090770525 if false then elseif scrutinee["$variant"] == "some" then local v289 = scrutinee["$payload"] result2090770525 = true elseif scrutinee["$variant"] == "none" then result2090770525 = false else panic("not arm reached") end return result2090770525 end end)

--find
v56 = lazy(function()  return function(v284) return function(v285) local scrutinee = v285 local result2090770522 if false then elseif not scrutinee["$empty"] then local v286 = scrutinee["$item"] local v287 = scrutinee["$rest"] local scrutinee = (v284)(v286) local result1944253905 if false then elseif scrutinee == true then result1944253905 = variant("some", v286) elseif scrutinee == false then local scrutinee = v287 local result2090770524 if false then elseif not scrutinee["$empty"] then local v286 = scrutinee["$item"] local v287 = scrutinee["$rest"] local scrutinee = (v10000000)(v286) local result382749844 if false then elseif scrutinee == true then result382749844 = variant("some", v286) elseif scrutinee == false then result382749844 = ((v56())(v10000000))(v287) else panic("not arm reached") end result2090770524 = result382749844 elseif scrutinee["$empty"] then result2090770524 = variant("none") else panic("not arm reached") end result1944253905 = result2090770524 else panic("not arm reached") end result2090770522 = result1944253905 elseif scrutinee["$empty"] then result2090770522 = variant("none") else panic("not arm reached") end return result2090770522 end end end)

--sub
v57 = lazy(function()  return function(v279) return function(v280) return function(v281) return (strsplitat((((v3())(v279))(v281))[2], v280))[1] end end end end)

--pop
v58 = lazy(function()  return function(v276) local scrutinee = v276 local result2090770490 if false then elseif not scrutinee["$empty"] then local v277 = scrutinee["$item"] local v278 = scrutinee["$rest"] result2090770490 = variant("some", { v277, v278 }) elseif scrutinee["$empty"] then result2090770490 = variant("none") else panic("not arm reached") end return result2090770490 end end)

--panic
v59 = lazy(function()  return function(v274) return panic(v274) end end)

--append
v60 = lazy(function()  return function(v270) return function(v271) local scrutinee = v271 local result2090770485 if false then elseif not scrutinee["$empty"] then local v272 = scrutinee["$item"] local v273 = scrutinee["$rest"] local scrutinee = v273 local result2090770487 if false then elseif not scrutinee["$empty"] then local v272 = scrutinee["$item"] local v273 = scrutinee["$rest"] result2090770487 = cons(v272, ((v60())(v10000000))(v273)) elseif scrutinee["$empty"] then result2090770487 = v10000000 else panic("not arm reached") end result2090770485 = cons(v272, result2090770487) elseif scrutinee["$empty"] then result2090770485 = v270 else panic("not arm reached") end return result2090770485 end end end)

--some-or
v61 = lazy(function()  return function(v267) return function(v268) local scrutinee = v268 local result2090770459 if false then elseif scrutinee["$variant"] == "some" then local v269 = scrutinee["$payload"] result2090770459 = v269 elseif scrutinee["$variant"] == "none" then result2090770459 = v267 else panic("not arm reached") end return result2090770459 end end end)

--get
v62 = lazy(function()  return function(v265) return function(v266) local scrutinee = v266 local result2090770457 if false then elseif scrutinee["$variant"] == "some" then local v427 = scrutinee["$payload"] local scrutinee = eq((v427)["key"], v265) local result596184180 if false then elseif scrutinee == true then result596184180 = variant("some", (v427)["value"]) elseif scrutinee == false then local scrutinee = ((v23())(dynamic(v265)) > (v427)["hash"]) local result1160988207 if false then elseif scrutinee == true then result1160988207 = (((v63())((v23())(dynamic(v265))))(v265))((v427)["rhs"]) elseif scrutinee == false then result1160988207 = (((v63())((v23())(dynamic(v265))))(v265))((v427)["lhs"]) else panic("not arm reached") end result596184180 = result1160988207 else panic("not arm reached") end result2090770457 = result596184180 elseif scrutinee["$variant"] == "none" then result2090770457 = variant("none") else panic("not arm reached") end return result2090770457 end end end)

--get-hashed
v63 = lazy(function()  return function(v261) return function(v262) return function(v263) return ((v20())(function(v264) local scrutinee = eq((v264)["key"], v262) local result1847909936 if false then elseif scrutinee == true then result1847909936 = variant("some", (v264)["value"]) elseif scrutinee == false then local scrutinee = (v261 > (v264)["hash"]) local result1247051430 if false then elseif scrutinee == true then result1247051430 = (((v63())(v261))(v262))((v264)["rhs"]) elseif scrutinee == false then local scrutinee = (v264)["lhs"] local result2118792619 if false then elseif scrutinee["$variant"] == "some" then local v427 = scrutinee["$payload"] local scrutinee = eq((v427)["key"], v10000004) local result1653616828 if false then elseif scrutinee == true then result1653616828 = variant("some", (v427)["value"]) elseif scrutinee == false then local scrutinee = (v10000003 > (v427)["hash"]) local result1381217554 if false then elseif scrutinee == true then result1381217554 = (((v63())(v10000003))(v10000004))((v427)["rhs"]) elseif scrutinee == false then result1381217554 = (((v63())(v10000003))(v10000004))((v427)["lhs"]) else panic("not arm reached") end result1653616828 = result1381217554 else panic("not arm reached") end result2118792619 = result1653616828 elseif scrutinee["$variant"] == "none" then result2118792619 = variant("none") else panic("not arm reached") end result1247051430 = result2118792619 else panic("not arm reached") end result1847909936 = result1247051430 else panic("not arm reached") end return result1847909936 end))(v263) end end end end)

--pairs
v64 = lazy(function()  return function(v260) local scrutinee = v260 local result2090770451 if false then elseif scrutinee["$variant"] == "none" then result2090770451 = empty elseif scrutinee["$variant"] == "some" then local v259 = scrutinee["$payload"] result2090770451 = ((v65())((v259)["lhs"]))(((v65())((v259)["rhs"]))(cons({ (v259)["key"], (v259)["value"] }, empty))) else panic("not arm reached") end return result2090770451 end end)

--extend-list
v65 = lazy(function()  return function(v257) return function(v258) local scrutinee = v257 local result2090770425 if false then elseif scrutinee["$variant"] == "none" then result2090770425 = v258 elseif scrutinee["$variant"] == "some" then local v259 = scrutinee["$payload"] local scrutinee = (v259)["lhs"] local result3232618895 if false then elseif scrutinee["$variant"] == "none" then result3232618895 = ((v65())((v259)["rhs"]))(((v15())({ (v259)["key"], (v259)["value"] }))(v10000003)) elseif scrutinee["$variant"] == "some" then local v259 = scrutinee["$payload"] result3232618895 = ((v65())((v259)["lhs"]))(((v65())((v259)["rhs"]))(cons({ (v259)["key"], (v259)["value"] }, ((v65())((v259)["rhs"]))(((v15())({ (v259)["key"], (v259)["value"] }))(v10000003))))) else panic("not arm reached") end result2090770425 = result3232618895 else panic("not arm reached") end return result2090770425 end end end)

--trace
v66 = lazy(function()  return function(v254) return function(v255) return trace((v254 .. "\n"), v255) end end end)

--items
v67 = lazy(function()  return function(v253) local scrutinee = v253 local result2090770421 if false then elseif scrutinee["$variant"] == "none" then result2090770421 = empty elseif scrutinee["$variant"] == "some" then local v259 = scrutinee["$payload"] result2090770421 = ((v65())((v259)["lhs"]))(((v65())((v259)["rhs"]))(cons({ (v259)["key"], (v259)["value"] }, empty))) else panic("not arm reached") end local scrutinee = result2090770421 local result3235836740 if false then elseif not scrutinee["$empty"] then local v435 = scrutinee["$item"] local v436 = scrutinee["$rest"] result3235836740 = cons((v435)[1], ((v17())(function(v252) return (v252)[1] end))(v436)) elseif scrutinee["$empty"] then result3235836740 = empty else panic("not arm reached") end return result3235836740 end end)

--keys
v68 = lazy(function()  return function(v251) local scrutinee = v251 local result2090770419 if false then elseif scrutinee["$variant"] == "none" then result2090770419 = empty elseif scrutinee["$variant"] == "some" then local v259 = scrutinee["$payload"] result2090770419 = ((v65())((v259)["lhs"]))(((v65())((v259)["rhs"]))(cons({ (v259)["key"], (v259)["value"] }, empty))) else panic("not arm reached") end local scrutinee = result2090770419 local result3235836715 if false then elseif not scrutinee["$empty"] then local v435 = scrutinee["$item"] local v436 = scrutinee["$rest"] result3235836715 = cons((v435)[1], ((v17())(function(v252) return (v252)[1] end))(v436)) elseif scrutinee["$empty"] then result3235836715 = empty else panic("not arm reached") end return result3235836715 end end)

--filter-map
v69 = lazy(function()  return function(v249) return function(v250) local scrutinee = v250 local result2090770418 if false then elseif not scrutinee["$empty"] then local v435 = scrutinee["$item"] local v436 = scrutinee["$rest"] result2090770418 = cons((v249)(v435), ((v17())(v249))(v436)) elseif scrutinee["$empty"] then result2090770418 = empty else panic("not arm reached") end local scrutinee = result2090770418 local result3235836714 if false then elseif not scrutinee["$empty"] then local v327 = scrutinee["$item"] local v328 = scrutinee["$rest"] local scrutinee = v327 local result2090771415 if false then elseif scrutinee["$variant"] == "some" then local v329 = scrutinee["$payload"] result2090771415 = cons(v329, (v40())(v328)) elseif scrutinee["$variant"] == "none" then result2090771415 = (v40())(v328) else panic("not arm reached") end result3235836714 = result2090771415 elseif scrutinee["$empty"] then result3235836714 = empty else panic("not arm reached") end return result3235836714 end end end)

--insert
v70 = lazy(function()  return function(v247) return function(v248) return ((((v25())((v23())(dynamic(v247))))(v247))({  }))(v248) end end end)

--min
v71 = lazy(function()  return function(v245) return function(v246) local scrutinee = (v245 < v246) local result1174383317 if false then elseif scrutinee == true then result1174383317 = v245 elseif scrutinee == false then result1174383317 = v246 else panic("not arm reached") end return result1174383317 end end end)

--count
v72 = lazy(function()  return function(v242) local scrutinee = v242 local result2090770387 if false then elseif not scrutinee["$empty"] then local v459 = scrutinee["$item"] local v460 = scrutinee["$rest"] result2090770387 = (((v8())(1))(function(v243) return function(v244) return (v243 + 1) end end))(v460) elseif scrutinee["$empty"] then result2090770387 = 0 else panic("not arm reached") end return result2090770387 end end)

--try-then
v73 = lazy(function()  return function(v238) return function(v239) local scrutinee = v239 local result2090770361 if false then elseif scrutinee["$variant"] == "ok" then local v241 = scrutinee["$payload"] result2090770361 = (v238)(v241) elseif scrutinee["$variant"] == "err" then local v240 = scrutinee["$payload"] result2090770361 = pure(variant("err", v240)) else panic("not arm reached") end return result2090770361 end end end)

--mfold
v74 = lazy(function()  return function(v232) return function(v233) return function(v234) local scrutinee = v234 local result2090770356 if false then elseif not scrutinee["$empty"] then local v235 = scrutinee["$item"] local v236 = scrutinee["$rest"] result2090770356 = bind(((v233)(v232))(v235), function(v237) local scrutinee = v236 local result2090770358 if false then elseif not scrutinee["$empty"] then local v235 = scrutinee["$item"] local v236 = scrutinee["$rest"] result2090770358 = bind(((v10000001)(v237))(v235), function(v237) return (((v74())(v237))(v10000001))(v236) end) elseif scrutinee["$empty"] then result2090770358 = pure(v237) else panic("not arm reached") end return result2090770358 end) elseif scrutinee["$empty"] then result2090770356 = pure(v232) else panic("not arm reached") end return result2090770356 end end end end)

--contains
v75 = lazy(function()  return function(v230) return function(v231) local scrutinee = strfind(v231, v230) local result490689971 if false then elseif scrutinee["$variant"] == "some" then local v289 = scrutinee["$payload"] result490689971 = true elseif scrutinee["$variant"] == "none" then result490689971 = false else panic("not arm reached") end return result490689971 end end end)

--read
v76 = lazy(function()  return extern["fs::read"] end)

--range
v77 = lazy(function()  return function(v228) return function(v229) local scrutinee = (v229 > 0) local result1036500599 if false then elseif scrutinee == true then local scrutinee = ((v10000001 - 1) > 0) local result3394825867 if false then elseif scrutinee == true then result3394825867 = cons((v10000000 + 1), ((v77())(((v10000000 + 1) + 1)))(((v10000001 - 1) - 1))) elseif scrutinee == false then result3394825867 = empty else panic("not arm reached") end result1036500599 = cons(v228, result3394825867) elseif scrutinee == false then result1036500599 = empty else panic("not arm reached") end return result1036500599 end end end)

--is-some-and
v78 = lazy(function()  return function(v225) return function(v226) local scrutinee = v226 local result2090770325 if false then elseif scrutinee["$variant"] == "some" then local v227 = scrutinee["$payload"] result2090770325 = (v225)(v227) elseif scrutinee["$variant"] == "none" then result2090770325 = false else panic("not arm reached") end return result2090770325 end end end)

--all
v79 = lazy(function()  return function(v222) return function(v223) local scrutinee = v223 local result2090770322 if false then elseif not scrutinee["$empty"] then local v286 = scrutinee["$item"] local v287 = scrutinee["$rest"] local scrutinee = (v222)(v286) local result3765741065 if false then elseif scrutinee == true then result3765741065 = false elseif scrutinee == false then result3765741065 = true else panic("not arm reached") end local scrutinee = result3765741065 local result2956492528 if false then elseif scrutinee == true then result2956492528 = variant("some", v286) elseif scrutinee == false then result2956492528 = ((v56())(function(v224) local scrutinee = (v222)(v224) local result3765734465 if false then elseif scrutinee == true then result3765734465 = false elseif scrutinee == false then result3765734465 = true else panic("not arm reached") end return result3765734465 end))(v287) else panic("not arm reached") end result2090770322 = result2956492528 elseif scrutinee["$empty"] then result2090770322 = variant("none") else panic("not arm reached") end local scrutinee = result2090770322 local result3235835652 if false then elseif scrutinee["$variant"] == "some" then local v446 = scrutinee["$payload"] result3235835652 = false elseif scrutinee["$variant"] == "none" then result3235835652 = true else panic("not arm reached") end return result3235835652 end end end)

--assert
v80 = lazy(function()  return function(v219) return function(v220) return function(v221) local scrutinee = v219 local result2090770295 if false then elseif scrutinee == true then result2090770295 = v221 elseif scrutinee == false then result2090770295 = panic(v220) else panic("not arm reached") end return result2090770295 end end end end)

--prepend
v81 = lazy(function()  return function(v217) return function(v218) local scrutinee = v217 local result2090770293 if false then elseif not scrutinee["$empty"] then local v272 = scrutinee["$item"] local v273 = scrutinee["$rest"] result2090770293 = cons(v272, ((v60())(v218))(v273)) elseif scrutinee["$empty"] then result2090770293 = v218 else panic("not arm reached") end return result2090770293 end end end)

--some-else
v82 = lazy(function()  return function(v214) return function(v215) local scrutinee = v215 local result2090770291 if false then elseif scrutinee["$variant"] == "some" then local v216 = scrutinee["$payload"] result2090770291 = variant("some", v216) elseif scrutinee["$variant"] == "none" then result2090770291 = v214 else panic("not arm reached") end return result2090770291 end end end)

--has
v83 = lazy(function()  return function(v212) return function(v213) local scrutinee = v213 local result2090770289 if false then elseif scrutinee["$variant"] == "some" then local v427 = scrutinee["$payload"] local scrutinee = eq((v427)["key"], v212) local result596178636 if false then elseif scrutinee == true then result596178636 = variant("some", (v427)["value"]) elseif scrutinee == false then local scrutinee = ((v23())(dynamic(v212)) > (v427)["hash"]) local result347005319 if false then elseif scrutinee == true then result347005319 = (((v63())((v23())(dynamic(v212))))(v212))((v427)["rhs"]) elseif scrutinee == false then result347005319 = (((v63())((v23())(dynamic(v212))))(v212))((v427)["lhs"]) else panic("not arm reached") end result596178636 = result347005319 else panic("not arm reached") end result2090770289 = result596178636 elseif scrutinee["$variant"] == "none" then result2090770289 = variant("none") else panic("not arm reached") end local scrutinee = result2090770289 local result3235834768 if false then elseif scrutinee["$variant"] == "some" then local v289 = scrutinee["$payload"] result3235834768 = true elseif scrutinee["$variant"] == "none" then result3235834768 = false else panic("not arm reached") end return result3235834768 end end end)

--debug
v84 = lazy(function()  return function(v211) return trace((((v36())(0))(dynamic(v211)) .. "\n"), v211) end end)

--format-pretty
v85 = lazy(function()  return function(v209) return (((v86())(1))(0))(dynamic(v209)) end end)

--format-dynamic-pretty
v86 = lazy(function()  return function(v186) return function(v187) return function(v188) local scrutinee = v188 local result2090769436 if false then elseif scrutinee["$variant"] == "real'" then local v207 = scrutinee["$payload"] result2090769436 = tostring(v207) elseif scrutinee["$variant"] == "lambda" then result2090769436 = "lambda" elseif scrutinee["$variant"] == "tuple" then local v206 = scrutinee["$payload"] local scrutinee = (v187 > 0) local result448333018 if false then elseif scrutinee == true then result448333018 = ((v31())(")"))(((v32())("("))(((v30())(", "))(((v17())(((v86())(v186))(1)))(v206)))) elseif scrutinee == false then result448333018 = ((v30())(", "))(((v17())(((v86())(v186))(1)))(v206)) else panic("not arm reached") end result2090769436 = result448333018 elseif scrutinee["$variant"] == "record" then local v202 = scrutinee["$payload"] local scrutinee = v202 local result2090770255 if false then elseif not scrutinee["$empty"] then local v203 = scrutinee["$item"] local v204 = scrutinee["$rest"] local scrutinee = ((v186 - 1) > 0) local result573831528 if false then elseif scrutinee == true then result573831528 = ((v31())("  "))(((v46())(((v186 - 1) - 1)))("  ")) elseif scrutinee == false then result573831528 = "" else panic("not arm reached") end local scrutinee = (v186 > 0) local result409197625 if false then elseif scrutinee == true then result409197625 = ((v31())("  "))(((v46())((v186 - 1)))("  ")) elseif scrutinee == false then result409197625 = "" else panic("not arm reached") end local scrutinee = (v186 > 0) local result409197625 if false then elseif scrutinee == true then result409197625 = ((v31())("  "))(((v46())((v186 - 1)))("  ")) elseif scrutinee == false then result409197625 = "" else panic("not arm reached") end result2090770255 = ((v31())("}"))(((v31())(((v32())("\n"))(result573831528)))(((v32())("{"))(((v32())(((v32())("\n"))(result409197625)))(((v30())(((v32())("\n"))(result409197625)))(((v17())(function(v205) return ((v31())((((v86())((v186 + 1)))(0))((v205)[2])))(((v31())(": "))((v205)[1])) end))(v202)))))) elseif scrutinee["$empty"] then result2090770255 = "{}" else panic("not arm reached") end result2090769436 = result2090770255 elseif scrutinee["$variant"] == "int'" then local v200 = scrutinee["$payload"] result2090769436 = tostring(v200) elseif scrutinee["$variant"] == "str'" then local v199 = scrutinee["$payload"] local v10000022 = (((v37())("\t"))("\\t"))((((v37())("\r"))("\\r"))((((v37())("\n"))("\\n"))((((v37())("\""))("\\\""))((((v37())("\\"))("\\\\"))(v199))))) local scrutinee = strfind(v10000022, "\0") local result1281827623 if false then elseif scrutinee["$variant"] == "none" then result1281827623 = v10000022 elseif scrutinee["$variant"] == "some" then local v338 = scrutinee["$payload"] local scrutinee = eq((strsplitat((((v3())(v338))(v10000022))[2], strlength("\0")))[1], "\0") local result3955363709 if false then elseif scrutinee == false then result3955363709 = (((v3())(v338))(v10000022))[2] elseif scrutinee == true then result3955363709 = (strsplitat((((v3())(v338))(v10000022))[2], strlength("\0")))[2] else panic("not arm reached") end result1281827623 = ((v32())((((v3())(v338))(v10000022))[1]))(((v32())("\\0"))((((v37())("\0"))("\\0"))(result3955363709))) else panic("not arm reached") end result2090769436 = ((v31())("\""))(((v32())("\""))(result1281827623)) elseif scrutinee["$variant"] == "nat'" then local v197 = scrutinee["$payload"] result2090769436 = tostring(v197) elseif scrutinee["$variant"] == "action" then result2090769436 = "action" elseif scrutinee["$variant"] == "list" then local v196 = scrutinee["$payload"] local scrutinee = ((v186 - 1) > 0) local result573831528 if false then elseif scrutinee == true then result573831528 = ((v31())("  "))(((v46())(((v186 - 1) - 1)))("  ")) elseif scrutinee == false then result573831528 = "" else panic("not arm reached") end local scrutinee = (v186 > 0) local result409197625 if false then elseif scrutinee == true then result409197625 = ((v31())("  "))(((v46())((v186 - 1)))("  ")) elseif scrutinee == false then result409197625 = "" else panic("not arm reached") end local scrutinee = v196 local result2090769467 if false then elseif not scrutinee["$empty"] then local v435 = scrutinee["$item"] local v436 = scrutinee["$rest"] result2090769467 = cons((((v86())((v186 + 1)))(0))(v435), ((v17())(((v86())((v186 + 1)))(0)))(v436)) elseif scrutinee["$empty"] then result2090769467 = empty else panic("not arm reached") end local scrutinee = result2090769467 local result3234974390 if false then elseif not scrutinee["$empty"] then local v375 = scrutinee["$item"] local v376 = scrutinee["$rest"] local scrutinee = v376 local result2090771579 if false then elseif not scrutinee["$empty"] then local v377 = scrutinee["$item"] local v378 = scrutinee["$rest"] local scrutinee = (v186 > 0) local result409197625 if false then elseif scrutinee == true then result409197625 = ((v31())("  "))(((v46())((v186 - 1)))("  ")) elseif scrutinee == false then result409197625 = "" else panic("not arm reached") end local scrutinee = (v186 > 0) local result409197625 if false then elseif scrutinee == true then result409197625 = ((v31())("  "))(((v46())((v186 - 1)))("  ")) elseif scrutinee == false then result409197625 = "" else panic("not arm reached") end result2090771579 = ((v31())(((v30())(((v32())("\n"))(result409197625)))(v376)))(((v31())(((v32())("\n"))(result409197625)))(v375)) elseif scrutinee["$empty"] then result2090771579 = v375 else panic("not arm reached") end result3234974390 = result2090771579 elseif scrutinee["$empty"] then result3234974390 = "" else panic("not arm reached") end result2090769436 = ((v31())("]"))(((v31())(((v32())("\n"))(result573831528)))(((v32())("["))(((v32())(((v32())("\n"))(result409197625)))(result3234974390)))) elseif scrutinee["$variant"] == "variant" then local v194 = scrutinee["$payload"] local scrutinee = (v194)[2] local result1351024372 if false then elseif scrutinee["$variant"] == "none" then result1351024372 = ((v32())(":"))((v194)[1]) elseif scrutinee["$variant"] == "some" then local v195 = scrutinee["$payload"] local scrutinee = (v187 > 1) local result448333051 if false then elseif scrutinee == true then result448333051 = ((v31())(")"))(((v32())("("))((((v31())(" "))(((v32())(":"))((v194)[1])) .. (((v86())(v186))(1))(v195)))) elseif scrutinee == false then result448333051 = (((v31())(" "))(((v32())(":"))((v194)[1])) .. (((v86())(v186))(1))(v195)) else panic("not arm reached") end result1351024372 = result448333051 else panic("not arm reached") end result2090769436 = result1351024372 else panic("not arm reached") end return result2090769436 end end end end)

--union
v87 = lazy(function()  return function(v182) return function(v183) local scrutinee = v182 local result2090769430 if false then elseif scrutinee["$variant"] == "none" then result2090769430 = empty elseif scrutinee["$variant"] == "some" then local v259 = scrutinee["$payload"] result2090769430 = ((v65())((v259)["lhs"]))(((v65())((v259)["rhs"]))(cons({ (v259)["key"], (v259)["value"] }, empty))) else panic("not arm reached") end local scrutinee = result2090769430 local result3234974284 if false then elseif not scrutinee["$empty"] then local v435 = scrutinee["$item"] local v436 = scrutinee["$rest"] result3234974284 = cons((v435)[1], ((v17())(function(v252) return (v252)[1] end))(v436)) elseif scrutinee["$empty"] then result3234974284 = empty else panic("not arm reached") end local scrutinee = result3234974284 local result1559025458 if false then elseif not scrutinee["$empty"] then local v459 = scrutinee["$item"] local v460 = scrutinee["$rest"] result1559025458 = (((v8())(((((v25())((v23())(dynamic(v459))))(v459))({  }))(v183)))(function(v184) return function(v185) return ((((v25())((v23())(dynamic(v185))))(v185))({  }))(v184) end end))(v460) elseif scrutinee["$empty"] then result1559025458 = v183 else panic("not arm reached") end return result1559025458 end end end)

--flatten
v88 = lazy(function()  return function(v179) local scrutinee = v179 local result2090769404 if false then elseif not scrutinee["$empty"] then local v180 = scrutinee["$item"] local v181 = scrutinee["$rest"] local scrutinee = v181 local result2090769429 if false then elseif not scrutinee["$empty"] then local v180 = scrutinee["$item"] local v181 = scrutinee["$rest"] local scrutinee = v180 local result2090769428 if false then elseif not scrutinee["$empty"] then local v272 = scrutinee["$item"] local v273 = scrutinee["$rest"] result2090769428 = cons(v272, ((v60())((v88())(v181)))(v273)) elseif scrutinee["$empty"] then result2090769428 = (v88())(v181) else panic("not arm reached") end result2090769429 = result2090769428 elseif scrutinee["$empty"] then result2090769429 = empty else panic("not arm reached") end result2090769404 = ((v60())(result2090769429))(v180) elseif scrutinee["$empty"] then result2090769404 = empty else panic("not arm reached") end return result2090769404 end end)

--has
v89 = lazy(function()  return function(v177) return function(v178) local scrutinee = v178 local result2090769403 if false then elseif scrutinee["$variant"] == "some" then local v427 = scrutinee["$payload"] local scrutinee = eq((v427)["key"], v177) local result596149398 if false then elseif scrutinee == true then result596149398 = variant("some", (v427)["value"]) elseif scrutinee == false then local scrutinee = ((v23())(dynamic(v177)) > (v427)["hash"]) local result2445534993 if false then elseif scrutinee == true then result2445534993 = (((v63())((v23())(dynamic(v177))))(v177))((v427)["rhs"]) elseif scrutinee == false then result2445534993 = (((v63())((v23())(dynamic(v177))))(v177))((v427)["lhs"]) else panic("not arm reached") end result596149398 = result2445534993 else panic("not arm reached") end result2090769403 = result596149398 elseif scrutinee["$variant"] == "none" then result2090769403 = variant("none") else panic("not arm reached") end local scrutinee = result2090769403 local result3234974188 if false then elseif scrutinee["$variant"] == "some" then local v289 = scrutinee["$payload"] result3234974188 = true elseif scrutinee["$variant"] == "none" then result3234974188 = false else panic("not arm reached") end return result3234974188 end end end)

--ansi-escape
v90 = lazy(function()  return extern["string::ansi-escape"] end)

--max
v91 = lazy(function()  return function(v175) return function(v176) local scrutinee = (v175 > v176) local result1723245275 if false then elseif scrutinee == true then result1723245275 = v175 elseif scrutinee == false then result1723245275 = v176 else panic("not arm reached") end return result1723245275 end end end)

--readln
v92 = lazy(function()  return extern["io::readln"] end)

--maybe
v93 = lazy(function()  return function(v169) local scrutinee = v169 local result2090769371 if false then elseif not scrutinee["$empty"] then local v170 = scrutinee["$item"] local v171 = scrutinee["$rest"] local scrutinee = v170 local result2090769395 if false then elseif scrutinee["$variant"] == "some" then local v173 = scrutinee["$payload"] local scrutinee = v171 local result2090769396 if false then elseif not scrutinee["$empty"] then local v170 = scrutinee["$item"] local v171 = scrutinee["$rest"] local scrutinee = v170 local result2090769395 if false then elseif scrutinee["$variant"] == "some" then local v173 = scrutinee["$payload"] local scrutinee = (v93())(v171) local result2811432905 if false then elseif scrutinee["$variant"] == "some" then local v174 = scrutinee["$payload"] result2811432905 = variant("some", cons(v173, v174)) elseif scrutinee["$variant"] == "none" then result2811432905 = variant("none") else panic("not arm reached") end result2090769395 = result2811432905 elseif true then result2090769395 = variant("none") else panic("not arm reached") end result2090769396 = result2090769395 elseif scrutinee["$empty"] then result2090769396 = variant("some", empty) else panic("not arm reached") end local scrutinee = result2090769396 local result3234973399 if false then elseif scrutinee["$variant"] == "some" then local v174 = scrutinee["$payload"] result3234973399 = variant("some", cons(v173, v174)) elseif scrutinee["$variant"] == "none" then result3234973399 = variant("none") else panic("not arm reached") end result2090769395 = result3234973399 elseif true then result2090769395 = variant("none") else panic("not arm reached") end result2090769371 = result2090769395 elseif scrutinee["$empty"] then result2090769371 = variant("some", empty) else panic("not arm reached") end return result2090769371 end end)

--assert-ok
v94 = lazy(function()  return function(v166) local scrutinee = v166 local result2090769368 if false then elseif scrutinee["$variant"] == "ok" then local v168 = scrutinee["$payload"] result2090769368 = v168 elseif scrutinee["$variant"] == "err" then local v167 = scrutinee["$payload"] result2090769368 = panic(((v36())(0))(dynamic(v167))) else panic("not arm reached") end return result2090769368 end end)

--maybe-fold
v95 = lazy(function()  return function(v160) return function(v161) return function(v162) local scrutinee = v162 local result2090769364 if false then elseif not scrutinee["$empty"] then local v163 = scrutinee["$item"] local v164 = scrutinee["$rest"] local scrutinee = ((v161)(v160))(v163) local result416625268 if false then elseif scrutinee["$variant"] == "some" then local v165 = scrutinee["$payload"] local scrutinee = v164 local result2090769366 if false then elseif not scrutinee["$empty"] then local v163 = scrutinee["$item"] local v164 = scrutinee["$rest"] local scrutinee = ((v10000001)(v165))(v163) local result371435171 if false then elseif scrutinee["$variant"] == "some" then local v165 = scrutinee["$payload"] result371435171 = (((v95())(v165))(v10000001))(v164) elseif scrutinee["$variant"] == "none" then result371435171 = variant("none") else panic("not arm reached") end result2090769366 = result371435171 elseif scrutinee["$empty"] then result2090769366 = variant("some", v165) else panic("not arm reached") end result416625268 = result2090769366 elseif scrutinee["$variant"] == "none" then result416625268 = variant("none") else panic("not arm reached") end result2090769364 = result416625268 elseif scrutinee["$empty"] then result2090769364 = variant("some", v160) else panic("not arm reached") end return result2090769364 end end end end)

--write
v96 = lazy(function()  return extern["fs::write"] end)

--assert-some
v97 = lazy(function()  return function(v157) return function(v158) local scrutinee = v158 local result2090769337 if false then elseif scrutinee["$variant"] == "some" then local v159 = scrutinee["$payload"] result2090769337 = v159 elseif scrutinee["$variant"] == "none" then result2090769337 = panic(v157) else panic("not arm reached") end return result2090769337 end end end)

--ok-or
v98 = lazy(function()  return function(v153) return function(v154) local scrutinee = v154 local result2090769333 if false then elseif scrutinee["$variant"] == "ok" then local v156 = scrutinee["$payload"] result2090769333 = v156 elseif scrutinee["$variant"] == "err" then local v155 = scrutinee["$payload"] result2090769333 = v153 else panic("not arm reached") end return result2090769333 end end end)

--contains
v99 = lazy(function()  return function(v150) return function(v151) local scrutinee = v151 local result2090769330 if false then elseif not scrutinee["$empty"] then local v286 = scrutinee["$item"] local v287 = scrutinee["$rest"] local scrutinee = eq(v286, v150) local result1035508378 if false then elseif scrutinee == true then result1035508378 = variant("some", v286) elseif scrutinee == false then result1035508378 = ((v56())(function(v152) return eq(v152, v150) end))(v287) else panic("not arm reached") end result2090769330 = result1035508378 elseif scrutinee["$empty"] then result2090769330 = variant("none") else panic("not arm reached") end local scrutinee = result2090769330 local result3234973195 if false then elseif scrutinee["$variant"] == "some" then local v289 = scrutinee["$payload"] result3234973195 = true elseif scrutinee["$variant"] == "none" then result3234973195 = false else panic("not arm reached") end return result3234973195 end end end)

--first
v100 = lazy(function()  return function(v148) local scrutinee = eq(v148, "") local result986886671 if false then elseif scrutinee == true then result986886671 = variant("none") elseif scrutinee == false then result986886671 = variant("some", strsplitat(v148, 1)) else panic("not arm reached") end local scrutinee = result986886671 local result800411535 if false then elseif scrutinee["$variant"] == "some" then local v147 = scrutinee["$payload"] result800411535 = variant("some", (v147)[1]) elseif scrutinee["$variant"] == "none" then result800411535 = variant("none") else panic("not arm reached") end return result800411535 end end)

--map-some
v101 = lazy(function()  return function(v145) return function(v146) local scrutinee = v146 local result2090769302 if false then elseif scrutinee["$variant"] == "some" then local v147 = scrutinee["$payload"] result2090769302 = variant("some", (v145)(v147)) elseif scrutinee["$variant"] == "none" then result2090769302 = variant("none") else panic("not arm reached") end return result2090769302 end end end)

--tail
v102 = lazy(function()  return function(v142) local scrutinee = v142 local result2090769298 if false then elseif not scrutinee["$empty"] then local v143 = scrutinee["$item"] local v144 = scrutinee["$rest"] result2090769298 = variant("some", v144) elseif scrutinee["$empty"] then result2090769298 = variant("none") else panic("not arm reached") end return result2090769298 end end)

--zip
v103 = lazy(function()  return function(v136) return function(v137) local scrutinee = v137 local result2090769270 if false then elseif not scrutinee["$empty"] then local v138 = scrutinee["$item"] local v139 = scrutinee["$rest"] local scrutinee = v136 local result2090769269 if false then elseif not scrutinee["$empty"] then local v140 = scrutinee["$item"] local v141 = scrutinee["$rest"] local scrutinee = v139 local result2090769272 if false then elseif not scrutinee["$empty"] then local v138 = scrutinee["$item"] local v139 = scrutinee["$rest"] local scrutinee = v141 local result2090769297 if false then elseif not scrutinee["$empty"] then local v140 = scrutinee["$item"] local v141 = scrutinee["$rest"] result2090769297 = cons({ v138, v140 }, ((v103())(v141))(v139)) elseif scrutinee["$empty"] then result2090769297 = empty else panic("not arm reached") end result2090769272 = result2090769297 elseif scrutinee["$empty"] then result2090769272 = empty else panic("not arm reached") end result2090769269 = cons({ v138, v140 }, result2090769272) elseif scrutinee["$empty"] then result2090769269 = empty else panic("not arm reached") end result2090769270 = result2090769269 elseif scrutinee["$empty"] then result2090769270 = empty else panic("not arm reached") end return result2090769270 end end end)

--count
v104 = lazy(function()  return function(v134) local scrutinee = v134 local result2090769267 if false then elseif scrutinee["$variant"] == "some" then local v135 = scrutinee["$payload"] local scrutinee = (v135)["lhs"] local result1632255496 if false then elseif scrutinee["$variant"] == "some" then local v135 = scrutinee["$payload"] result1632255496 = (1 + ((v104())((v135)["lhs"]) + (v104())((v135)["rhs"]))) elseif scrutinee["$variant"] == "none" then result1632255496 = 0 else panic("not arm reached") end result2090769267 = (1 + (result1632255496 + (v104())((v135)["rhs"]))) elseif scrutinee["$variant"] == "none" then result2090769267 = 0 else panic("not arm reached") end return result2090769267 end end)

--values
v105 = lazy(function()  return function(v132) local scrutinee = v132 local result2090769265 if false then elseif scrutinee["$variant"] == "none" then result2090769265 = empty elseif scrutinee["$variant"] == "some" then local v259 = scrutinee["$payload"] result2090769265 = ((v65())((v259)["lhs"]))(((v65())((v259)["rhs"]))(cons({ (v259)["key"], (v259)["value"] }, empty))) else panic("not arm reached") end local scrutinee = result2090769265 local result3234972210 if false then elseif not scrutinee["$empty"] then local v435 = scrutinee["$item"] local v436 = scrutinee["$rest"] result3234972210 = cons((v435)[2], ((v17())(function(v133) return (v133)[2] end))(v436)) elseif scrutinee["$empty"] then result3234972210 = empty else panic("not arm reached") end return result3234972210 end end)

--count
v106 = lazy(function()  return function(v131) local scrutinee = v131 local result2090769264 if false then elseif scrutinee["$variant"] == "some" then local v135 = scrutinee["$payload"] result2090769264 = (1 + ((v104())((v135)["lhs"]) + (v104())((v135)["rhs"]))) elseif scrutinee["$variant"] == "none" then result2090769264 = 0 else panic("not arm reached") end return result2090769264 end end)

--split
v107 = lazy(function()  return function(v127) return function(v128) local scrutinee = ((v101())(function(v129) local scrutinee = ((v101())(function(v130) return ((v3())(v130))(((v1())(v127))((v129)[2])) end))(strfind(((v1())(v127))((v129)[2]), v10000015)) local result3646351048 if false then elseif scrutinee["$variant"] == "some" then local v147 = scrutinee["$payload"] local scrutinee = eq((strsplitat((v147)[2], strlength(v10000015)))[1], v10000015) local result3759663228 if false then elseif scrutinee == false then result3759663228 = (v147)[2] elseif scrutinee == true then result3759663228 = (strsplitat((v147)[2], strlength(v10000015)))[2] else panic("not arm reached") end result3646351048 = variant("some", ((v15())((v147)[1]))(((v107())(v10000015))(result3759663228))) elseif scrutinee["$variant"] == "none" then result3646351048 = variant("none") else panic("not arm reached") end return cons((v129)[1], ((v61())(cons(((v1())(v127))((v129)[2]), empty)))(result3646351048)) end))(((v101())(function(v130) return ((v3())(v130))(v128) end))(((v38())(v127))(v128))) local result1636629518 if false then elseif scrutinee["$variant"] == "some" then local v269 = scrutinee["$payload"] result1636629518 = v269 elseif scrutinee["$variant"] == "none" then result1636629518 = cons(v128, empty) else panic("not arm reached") end return result1636629518 end end end)

--filter
v108 = lazy(function()  return function(v123) return function(v124) local scrutinee = v124 local result2090769234 if false then elseif not scrutinee["$empty"] then local v125 = scrutinee["$item"] local v126 = scrutinee["$rest"] local scrutinee = (v123)(v125) local result3366667201 if false then elseif scrutinee == false then result3366667201 = ((v108())(v123))(v126) elseif scrutinee == true then local scrutinee = v126 local result2090769236 if false then elseif not scrutinee["$empty"] then local v125 = scrutinee["$item"] local v126 = scrutinee["$rest"] local scrutinee = (v10000000)(v125) local result382707340 if false then elseif scrutinee == false then result382707340 = ((v108())(v10000000))(v126) elseif scrutinee == true then result382707340 = cons(v125, ((v108())(v10000000))(v126)) else panic("not arm reached") end result2090769236 = result382707340 elseif scrutinee["$empty"] then result2090769236 = empty else panic("not arm reached") end result3366667201 = cons(v125, result2090769236) else panic("not arm reached") end result2090769234 = result3366667201 elseif scrutinee["$empty"] then result2090769234 = empty else panic("not arm reached") end return result2090769234 end end end)

--take
v109 = lazy(function()  return function(v119) return function(v120) local scrutinee = v120 local result2090769230 if false then elseif not scrutinee["$empty"] then local v121 = scrutinee["$item"] local v122 = scrutinee["$rest"] local scrutinee = (v119 > 0) local result76262613 if false then elseif scrutinee == true then local scrutinee = v122 local result2090769232 if false then elseif not scrutinee["$empty"] then local v121 = scrutinee["$item"] local v122 = scrutinee["$rest"] local scrutinee = ((v10000000 - 1) > 0) local result1157380426 if false then elseif scrutinee == true then result1157380426 = cons(v121, ((v109())(((v10000000 - 1) - 1)))(v122)) elseif scrutinee == false then result1157380426 = empty else panic("not arm reached") end result2090769232 = result1157380426 elseif scrutinee["$empty"] then result2090769232 = empty else panic("not arm reached") end result76262613 = cons(v121, result2090769232) elseif scrutinee == false then result76262613 = empty else panic("not arm reached") end result2090769230 = result76262613 elseif scrutinee["$empty"] then result2090769230 = empty else panic("not arm reached") end return result2090769230 end end end)

--each
v110 = lazy(function()  return function(v113) return function(v114) local scrutinee = v114 local result2090769201 if false then elseif not scrutinee["$empty"] then local v115 = scrutinee["$item"] local v116 = scrutinee["$rest"] result2090769201 = bind((v113)(v115), function(v117) local scrutinee = v116 local result2090769203 if false then elseif not scrutinee["$empty"] then local v115 = scrutinee["$item"] local v116 = scrutinee["$rest"] result2090769203 = bind((v10000000)(v115), function(v117) return bind(((v110())(v10000000))(v116), function(v118) return pure(cons(v117, v118)) end) end) elseif scrutinee["$empty"] then result2090769203 = pure(empty) else panic("not arm reached") end return bind(result2090769203, function(v118) return pure(cons(v117, v118)) end) end) elseif scrutinee["$empty"] then result2090769201 = pure(empty) else panic("not arm reached") end return result2090769201 end end end)

--debug-pretty
v111 = lazy(function()  return function(v112) return trace(((((v86())(1))(0))(dynamic(v112)) .. "\n"), v112) end end)
v16()()