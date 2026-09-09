local function variant(v, p)
	return {
		["$variant"] = v,
		["$payload"] = p,
	}
end

local function cons(i, r)
	return {
		["$item"] = i,
		["$rest"] = r,
	}
end

local empty = { ["$empty"] = true }

local extern = {
	["io::print"] = function(x)
		return function()
			io.write(x)
		end
	end,
	["fs::read"] = function(path)
		return function()
			local file = io.open(path, "r")

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
	["fs::write"] = function(path)
		return function(x)
			return function()
				local file = io.open(path, "w")

				if file == nil then
					return variant("err", variant("not-found"))
				end

				local _, err = file:write(x)

				if err ~= nil then
					return variant("err", variant("not-found"))
				end

				file:close()

				return variant("ok", {})
			end
		end
	end,
	["os::execute"] = function(cmd)
		return function()
			local file = io.popen(cmd, "r")

			if not file then
				return variant("err", variant("not-found"))
			end

			local output = file:read("*a")

			local _, _, code = file:close()

			return variant("ok", { code, output })
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
	return function()
		return x
	end
end

local function bind(x, y)
	return y(x())
end

local function panic(message)
	print("explicit panic: " .. message)
	os.exit(1)
end

local function dynamic(x)
	if type(x) == "table" and (x["$item"] ~= nil or x["$empty"] ~= nil) then
		local function list(y)
			if y["$empty"] == nil then
				return cons(dynamic(y["$item"]), list(y["$rest"]))
			else
				return y
			end
		end

		return variant("list", list(x))
	elseif type(x) == "table" and x["$variant"] ~= nil then
		local payload

		if x["$payload"] ~= nil then
			payload = variant("some", dynamic(x["$payload"]))
		else
			payload = variant("none")
		end

		return variant("variant", { x["$variant"], payload })
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
		if i == 0 then
			break
		end

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
v0 = lazy(function()
	return function(v459)
		return function(v460)
			local scrutinee = (v460 > 0)
			local result892726676
			if false then
			elseif scrutinee == true then
				result892726676 = cons(v459, ((v0())((v459 + 1)))((v460 - 1)))
			elseif scrutinee == false then
				result892726676 = empty
			end
			return result892726676
		end
	end
end)
v1 = lazy(function()
	return function(v457)
		local scrutinee = (v457)["lhs"]
		local result3798884879
		if false then
		elseif scrutinee["$variant"] == "none" then
			result3798884879 = { (v457)["hash"], (v457)["key"], (v457)["value"] }
		elseif scrutinee["$variant"] == "some" then
			local v458 = scrutinee["$payload"]
			result3798884879 = (v1())(v458)
		end
		return result3798884879
	end
end)
v2 = lazy(function()
	return function(v456)
		return eq(v456, "")
	end
end)
v3 = lazy(function()
	return function(v453)
		return function(v454)
			return (v4())(((v5())(function(v455)
				local scrutinee = (v453)(v455)
				local result843185165
				if false then
				elseif scrutinee == true then
					result843185165 = false
				elseif scrutinee == false then
					result843185165 = true
				end
				return result843185165
			end))(v454))
		end
	end
end)
v4 = lazy(function()
	return function(v451)
		local scrutinee = v451
		local result2090772597
		if false then
		elseif scrutinee["$variant"] == "some" then
			local v452 = scrutinee["$payload"]
			result2090772597 = false
		elseif scrutinee["$variant"] == "none" then
			result2090772597 = true
		end
		return result2090772597
	end
end)
v5 = lazy(function()
	return function(v447)
		return function(v448)
			local scrutinee = v448
			local result2090772571
			if false then
			elseif not scrutinee["$empty"] then
				local v449 = scrutinee["$first"]
				local v450 = scrutinee["$rest"]
				local scrutinee = (v447)(v449)
				local result1858919123
				if false then
				elseif scrutinee == true then
					result1858919123 = variant("some", v449)
				elseif scrutinee == false then
					result1858919123 = ((v5())(v447))(v450)
				end
				result2090772571 = result1858919123
			elseif scrutinee["$empty"] then
				result2090772571 = variant("none")
			end
			return result2090772571
		end
	end
end)
v6 = lazy(function()
	return function(v444)
		local scrutinee = v444
		local result2090772567
		if false then
		elseif not scrutinee["$empty"] then
			local v445 = scrutinee["$first"]
			local v446 = scrutinee["$rest"]
			result2090772567 = variant("some", v445)
		elseif scrutinee["$empty"] then
			result2090772567 = variant("none")
		end
		return result2090772567
	end
end)
v7 = lazy(function()
	return function(v441)
		return function(v442)
			local scrutinee = v441
			local result2090772564
			if false then
			elseif scrutinee["$variant"] == "none" then
				result2090772564 = v442
			elseif scrutinee["$variant"] == "some" then
				local v443 = scrutinee["$payload"]
				result2090772564 = ((v7())((v443)["lhs"]))(
					((v7())((v443)["rhs"]))(((v8())({ (v443)["key"], (v443)["value"] }))(v442))
				)
			end
			return result2090772564
		end
	end
end)
v8 = lazy(function()
	return function(v439)
		return function(v440)
			return cons(v439, v440)
		end
	end
end)
v9 = lazy(function()
	return function(v437)
		return function(v438)
			return ((v10())(v437))(v438)
		end
	end
end)
v10 = lazy(function()
	return function(v435)
		return function(v436)
			return (((v16())((v11())(v435)))(v435))(v436)
		end
	end
end)
v11 = lazy(function()
	return function(v433)
		return (v12())((function(v434)
			return dynamic(v434)
		end)(v433))
	end
end)
v12 = lazy(function()
	return function(v414)
		local scrutinee = v414
		local result2090772468
		if false then
		elseif scrutinee["$variant"] == "lambda" then
			result2090772468 = (function(v432)
				return hashnum(v432)
			end)(0)
		elseif scrutinee["$variant"] == "real'" then
			local v430 = scrutinee["$payload"]
			result2090772468 = (function(v431)
				return hashnum(v431)
			end)(v430)
		elseif scrutinee["$variant"] == "tuple" then
			local v429 = scrutinee["$payload"]
			result2090772468 = (((v14())(0))(v13()))(((v15())(v12()))(v429))
		elseif scrutinee["$variant"] == "record" then
			local v427 = scrutinee["$payload"]
			result2090772468 = (((v14())(0))(v13()))(((v15())(function(v428)
				return (v12())((v428)[1])
			end))(v427))
		elseif scrutinee["$variant"] == "int'" then
			local v425 = scrutinee["$payload"]
			result2090772468 = (function(v426)
				return hashnum(v426)
			end)(v425)
		elseif scrutinee["$variant"] == "action" then
			result2090772468 = (function(v424)
				return hashnum(v424)
			end)(1)
		elseif scrutinee["$variant"] == "str'" then
			local v422 = scrutinee["$payload"]
			result2090772468 = (function(v423)
				return hashstr(v423)
			end)(v422)
		elseif scrutinee["$variant"] == "nat'" then
			local v420 = scrutinee["$payload"]
			result2090772468 = (function(v421)
				return hashnum(v421)
			end)(v420)
		elseif scrutinee["$variant"] == "list" then
			local v419 = scrutinee["$payload"]
			result2090772468 = (((v14())(0))(v13()))(((v15())(v12()))(v419))
		elseif scrutinee["$variant"] == "variant" then
			local v415 = scrutinee["$payload"]
			local scrutinee = (v415)[1]
			local result618563727
			if false then
			elseif scrutinee["$variant"] == "some" then
				local v417 = scrutinee["$payload"]
				result618563727 = ((v13())((function(v418)
					return hashstr(v418)
				end)((v415)[0])))((v12())(v417))
			elseif scrutinee["$variant"] == "none" then
				result618563727 = (function(v416)
					return hashstr(v416)
				end)((v415)[0])
			end
			result2090772468 = result618563727
		end
		return result2090772468
	end
end)
v13 = lazy(function()
	return function(v411)
		return function(v412)
			return (function(v413)
				return bit.bxor((v413)[0], (v413)[1])
			end)({ v411, v412 })
		end
	end
end)
v14 = lazy(function()
	return function(v406)
		return function(v407)
			return function(v408)
				local scrutinee = v408
				local result2090772439
				if false then
				elseif not scrutinee["$empty"] then
					local v409 = scrutinee["$first"]
					local v410 = scrutinee["$rest"]
					result2090772439 = (((v14())(((v407)(v406))(v409)))(v407))(v410)
				elseif scrutinee["$empty"] then
					result2090772439 = v406
				end
				return result2090772439
			end
		end
	end
end)
v15 = lazy(function()
	return function(v402)
		return function(v403)
			local scrutinee = v403
			local result2090772434
			if false then
			elseif not scrutinee["$empty"] then
				local v404 = scrutinee["$first"]
				local v405 = scrutinee["$rest"]
				result2090772434 = cons((v402)(v404), ((v15())(v402))(v405))
			elseif scrutinee["$empty"] then
				result2090772434 = empty
			end
			return result2090772434
		end
	end
end)
v16 = lazy(function()
	return function(v394)
		return function(v395)
			return function(v396)
				return ((v17())(function(v397)
					local scrutinee = (eq((v397)["hash"], v394) and eq((v397)["key"], v395))
					local result694641813
					if false then
					elseif scrutinee == true then
						local scrutinee = ({ (v397)["lhs"], (v397)["rhs"] })[0]
						local result1853267298
						if false then
						elseif scrutinee["$variant"] == "none" then
							local scrutinee = ({ (v397)["lhs"], (v397)["rhs"] })[1]
							local result1853267331
							if false then
							elseif scrutinee["$variant"] == "none" then
								result1853267331 = variant("none")
							elseif scrutinee["$variant"] == "some" then
								local v401 = scrutinee["$payload"]
								result1853267331 = variant("some", v401)
							end
							result1853267298 = result1853267331
						elseif scrutinee["$variant"] == "some" then
							local v398 = scrutinee["$payload"]
							local scrutinee = ({ (v397)["lhs"], (v397)["rhs"] })[1]
							local result1853267331
							if false then
							elseif scrutinee["$variant"] == "none" then
								result1853267331 = variant("some", v398)
							elseif scrutinee["$variant"] == "some" then
								local v399 = scrutinee["$payload"]
								local v400 = (v1())(v399)
								result1853267331 = variant(
									"some",
									with(
										v397,
										{
											["rhs"] = (((v16())((v400)[0]))((v400)[1]))(variant("some", v399)),
											["value"] = (v400)[2],
											["key"] = (v400)[1],
											["hash"] = (v400)[0],
										}
									)
								)
							end
							result1853267298 = result1853267331
						end
						result694641813 = result1853267298
					elseif scrutinee == false then
						local scrutinee = (v394 > (v397)["hash"])
						local result2226485364
						if false then
						elseif scrutinee == true then
							result2226485364 =
								variant("some", with(v397, { ["rhs"] = (((v16())(v394))(v395))((v397)["rhs"]) }))
						elseif scrutinee == false then
							result2226485364 =
								variant("some", with(v397, { ["lhs"] = (((v16())(v394))(v395))((v397)["lhs"]) }))
						end
						result694641813 = result2226485364
					end
					return result694641813
				end))(v396)
			end
		end
	end
end)
v17 = lazy(function()
	return function(v391)
		return function(v392)
			local scrutinee = v392
			local result2090771641
			if false then
			elseif scrutinee["$variant"] == "some" then
				local v393 = scrutinee["$payload"]
				result2090771641 = (v391)(v393)
			elseif scrutinee["$variant"] == "none" then
				result2090771641 = variant("none")
			end
			return result2090771641
		end
	end
end)
v18 = lazy(function()
	return function(v389)
		return (function(v390)
			return dynamic(v390)
		end)(v389)
	end
end)
v19 = lazy(function()
	return function(v387)
		return function(v388)
			return ((v20())(v388))(v387)
		end
	end
end)
v20 = lazy(function()
	return function(v383)
		return function(v384)
			local scrutinee = v384
			local result2090771610
			if false then
			elseif not scrutinee["$empty"] then
				local v385 = scrutinee["$first"]
				local v386 = scrutinee["$rest"]
				result2090771610 = cons(v385, ((v20())(v383))(v386))
			elseif scrutinee["$empty"] then
				result2090771610 = v383
			end
			return result2090771610
		end
	end
end)
v21 = lazy(function()
	return function(v379)
		local v380 = extern["io::print"]
		local scrutinee = (v18())(v379)
		local result2430817584
		if false then
		elseif scrutinee["$variant"] == "str'" then
			local v382 = scrutinee["$payload"]
			result2430817584 = (v380)(v382)
		elseif true then
			result2430817584 = (v380)((v22())(v379))
		end
		return result2430817584
	end
end)
v22 = lazy(function()
	return function(v377)
		return ((v23())(0))((function(v378)
			return dynamic(v378)
		end)(v377))
	end
end)
v23 = lazy(function()
	return function(v358)
		return function(v359)
			local v362 = function(v360)
				return function(v361)
					local scrutinee = (v358 > v360)
					local result3335606745
					if false then
					elseif scrutinee == true then
						result3335606745 = ((v25())(")"))(((v26())("("))(v361))
					elseif scrutinee == false then
						result3335606745 = v361
					end
					return result3335606745
				end
			end
			local scrutinee = v359
			local result2090771516
			if false then
			elseif scrutinee["$variant"] == "real'" then
				local v375 = scrutinee["$payload"]
				result2090771516 = (function(v376)
					return tostring(v376)
				end)(v375)
			elseif scrutinee["$variant"] == "lambda" then
				result2090771516 = "lambda"
			elseif scrutinee["$variant"] == "tuple" then
				local v374 = scrutinee["$payload"]
				result2090771516 = ((v362)(0))(((v24())(", "))(((v15())((v23())(1)))(v374)))
			elseif scrutinee["$variant"] == "record" then
				local v371 = scrutinee["$payload"]
				local scrutinee = v371
				local result2090771574
				if false then
				elseif scrutinee["$empty"] then
					result2090771574 = "{}"
				elseif true then
					result2090771574 = ((v25())(" }"))(((v26())("{ "))(((v24())("; "))(((v15())(function(v373)
						return ((v25())(((v23())(0))((v373)[1])))(((v25())(": "))((v373)[0]))
					end))(v371))))
				end
				result2090771516 = result2090771574
			elseif scrutinee["$variant"] == "int'" then
				local v369 = scrutinee["$payload"]
				result2090771516 = (function(v370)
					return tostring(v370)
				end)(v369)
			elseif scrutinee["$variant"] == "str'" then
				local v368 = scrutinee["$payload"]
				result2090771516 = ((v25())('"'))(
					((v26())('"'))(
						(((v27())("\0"))("\\0"))(
							(((v27())("\t"))("\\t"))(
								(((v27())("\r"))("\\r"))(
									(((v27())("\n"))("\\n"))((((v27())('"'))('\\"'))((((v27())("\\"))("\\\\"))(v368)))
								)
							)
						)
					)
				)
			elseif scrutinee["$variant"] == "nat'" then
				local v366 = scrutinee["$payload"]
				result2090771516 = (function(v367)
					return tostring(v367)
				end)(v366)
			elseif scrutinee["$variant"] == "action" then
				result2090771516 = "action"
			elseif scrutinee["$variant"] == "list" then
				local v365 = scrutinee["$payload"]
				result2090771516 = ((v25())("]"))(((v26())("["))(((v24())("; "))(((v15())((v23())(0)))(v365))))
			elseif scrutinee["$variant"] == "variant" then
				local v363 = scrutinee["$payload"]
				local scrutinee = (v363)[1]
				local result3815368177
				if false then
				elseif scrutinee["$variant"] == "none" then
					result3815368177 = ((v26())(":"))((v363)[0])
				elseif scrutinee["$variant"] == "some" then
					local v364 = scrutinee["$payload"]
					result3815368177 = ((v362)(1))(
						((v25())(((v23())(1))(v364)))(((v25())(" "))(((v26())(":"))((v363)[0])))
					)
				end
				result2090771516 = result3815368177
			end
			return result2090771516
		end
	end
end)
v24 = lazy(function()
	return function(v353)
		return function(v354)
			local scrutinee = v354
			local result2090771511
			if false then
			elseif not scrutinee["$empty"] then
				local v355 = scrutinee["$first"]
				local v356 = scrutinee["$rest"]
				local scrutinee = v356
				local result2090771513
				if false then
				elseif scrutinee["$empty"] then
					result2090771513 = v355
				elseif true then
					result2090771513 = ((v25())(((v24())(v353))(v356)))(((v25())(v353))(v355))
				end
				result2090771511 = result2090771513
			elseif scrutinee["$empty"] then
				result2090771511 = ""
			end
			return result2090771511
		end
	end
end)
v25 = lazy(function()
	return function(v351)
		return function(v352)
			return ((v26())(v352))(v351)
		end
	end
end)
v26 = lazy(function()
	return function(v348)
		return function(v349)
			return (function(v350)
				return ((v350)[0] .. (v350)[1])
			end)({ v348, v349 })
		end
	end
end)
v27 = lazy(function()
	return function(v343)
		return function(v344)
			return function(v345)
				local scrutinee = ((v31())(v343))(v345)
				local result2856512470
				if false then
				elseif scrutinee["$variant"] == "none" then
					result2856512470 = v345
				elseif scrutinee["$variant"] == "some" then
					local v346 = scrutinee["$payload"]
					local v347 = ((v30())(v346))(v345)
					result2856512470 = ((v26())((v347)[0]))(
						((v26())(v344))((((v27())(v343))(v344))(((v28())(v343))((v347)[1])))
					)
				end
				return result2856512470
			end
		end
	end
end)
v28 = lazy(function()
	return function(v340)
		return function(v341)
			local v342 = ((v30())((v29())(v340)))(v341)
			local scrutinee = eq((v342)[0], v340)
			local result2958986573
			if false then
			elseif scrutinee == false then
				result2958986573 = v341
			elseif scrutinee == true then
				result2958986573 = (v342)[1]
			end
			return result2958986573
		end
	end
end)
v29 = lazy(function()
	return function(v338)
		return (function(v339)
			return strlength(v339)
		end)(v338)
	end
end)
v30 = lazy(function()
	return function(v335)
		return function(v336)
			return (function(v337)
				return strsplitat((v337)[0], (v337)[1])
			end)({ v336, v335 })
		end
	end
end)
v31 = lazy(function()
	return function(v332)
		return function(v333)
			return (function(v334)
				return strfind((v334)[0], (v334)[1])
			end)({ v333, v332 })
		end
	end
end)
v32 = lazy(function()
	return v33()
end)
v33 = lazy(function()
	return variant("none")
end)
v34 = lazy(function()
	return function(v329)
		local scrutinee = v329
		local result2090771417
		if false then
		elseif not scrutinee["$empty"] then
			local v330 = scrutinee["$first"]
			local v331 = scrutinee["$rest"]
			result2090771417 = ((v20())((v34())(v331)))(v330)
		elseif scrutinee["$empty"] then
			result2090771417 = empty
		end
		return result2090771417
	end
end)
v35 = lazy(function()
	return (v37())((v36())(cons(1, cons(2, cons(3, empty)))))
end)
v36 = lazy(function()
	return function(v326)
		return (((v14())(0))(function(v327)
			return function(v328)
				return (v327 + 1)
			end
		end))(v326)
	end
end)
v37 = lazy(function()
	return function(v324)
		return bind((v21())(v324), function(v325)
			return (v21())("\n")
		end)
	end
end)
v38 = lazy(function()
	return function(v320)
		return function(v321)
			local scrutinee = v321
			local result2090771409
			if false then
			elseif not scrutinee["$empty"] then
				local v322 = scrutinee["$first"]
				local v323 = scrutinee["$rest"]
				local scrutinee = (v320 > 0)
				local result353052079
				if false then
				elseif scrutinee == true then
					result353052079 = ((v38())((v320 - 1)))(v323)
				elseif scrutinee == false then
					result353052079 = v321
				end
				result2090771409 = result353052079
			elseif scrutinee["$empty"] then
				result2090771409 = empty
			end
			return result2090771409
		end
	end
end)
v39 = lazy(function()
	return function(v317)
		return function(v318)
			return function(v319)
				return ((((v40())((v11())(v317)))(v317))(v318))(v319)
			end
		end
	end
end)
v40 = lazy(function()
	return function(v312)
		return function(v313)
			return function(v314)
				return function(v315)
					local scrutinee = v315
					local result2090771380
					if false then
					elseif scrutinee["$variant"] == "none" then
						result2090771380 = variant(
							"some",
							{
								["hash"] = v312,
								["key"] = v313,
								["value"] = v314,
								["lhs"] = variant("none"),
								["rhs"] = variant("none"),
							}
						)
					elseif scrutinee["$variant"] == "some" then
						local v316 = scrutinee["$payload"]
						local scrutinee = (eq((v316)["hash"], v312) and eq((v316)["key"], v313))
						local result556800975
						if false then
						elseif scrutinee == true then
							result556800975 = variant("some", with(v316, { ["value"] = v314 }))
						elseif scrutinee == false then
							local scrutinee = (v312 > (v316)["hash"])
							local result3033503905
							if false then
							elseif scrutinee == true then
								result3033503905 = variant(
									"some",
									with(v316, { ["rhs"] = ((((v40())(v312))(v313))(v314))((v316)["rhs"]) })
								)
							elseif scrutinee == false then
								result3033503905 = variant(
									"some",
									with(v316, { ["lhs"] = ((((v40())(v312))(v313))(v314))((v316)["lhs"]) })
								)
							end
							result556800975 = result3033503905
						end
						result2090771380 = result556800975
					end
					return result2090771380
				end
			end
		end
	end
end)
v41 = lazy(function()
	return function(v308)
		return function(v309)
			local scrutinee = v309
			local result2090771351
			if false then
			elseif scrutinee["$variant"] == "ok" then
				local v311 = scrutinee["$payload"]
				result2090771351 = (v308)(v311)
			elseif scrutinee["$variant"] == "err" then
				local v310 = scrutinee["$payload"]
				result2090771351 = variant("err", v310)
			end
			return result2090771351
		end
	end
end)
v42 = lazy(function()
	return function(v302)
		local scrutinee = v302
		local result2090771344
		if false then
		elseif not scrutinee["$empty"] then
			local v304 = scrutinee["$first"]
			local v305 = scrutinee["$rest"]
			local scrutinee = v304
			local result2090771346
			if false then
			elseif scrutinee["$variant"] == "some" then
				local v306 = scrutinee["$payload"]
				local scrutinee = (v42())(v305)
				local result146403298
				if false then
				elseif scrutinee["$variant"] == "some" then
					local v307 = scrutinee["$payload"]
					result146403298 = variant("some", cons(v306, v307))
				elseif scrutinee["$variant"] == "none" then
					result146403298 = variant("none")
				end
				result2090771346 = result146403298
			end
			result2090771344 = result2090771346
		elseif scrutinee["$empty"] then
			result2090771344 = variant("some", empty)
		elseif true then
			result2090771344 = variant("none")
		end
		return result2090771344
	end
end)
v43 = lazy(function()
	return function(v296)
		return function(v297)
			return function(v298)
				local scrutinee = v298
				local result2090770558
				if false then
				elseif not scrutinee["$empty"] then
					local v299 = scrutinee["$first"]
					local v300 = scrutinee["$rest"]
					local scrutinee = ((v297)(v296))(v299)
					local result4033003826
					if false then
					elseif scrutinee["$variant"] == "some" then
						local v301 = scrutinee["$payload"]
						result4033003826 = (((v43())(v301))(v297))(v300)
					elseif scrutinee["$variant"] == "none" then
						result4033003826 = variant("none")
					end
					result2090770558 = result4033003826
				elseif scrutinee["$empty"] then
					result2090770558 = variant("some", v296)
				end
				return result2090770558
			end
		end
	end
end)
v44 = lazy(function()
	return function(v293)
		local scrutinee = v293
		local result2090770553
		if false then
		elseif not scrutinee["$empty"] then
			local v294 = scrutinee["$first"]
			local v295 = scrutinee["$rest"]
			result2090770553 = false
		elseif scrutinee["$empty"] then
			result2090770553 = true
		end
		return result2090770553
	end
end)
v45 = lazy(function()
	return function(v291)
		return function(v292)
			local scrutinee = (v291 < v292)
			local result2286080311
			if false then
			elseif scrutinee == true then
				result2286080311 = v291
			elseif scrutinee == false then
				result2286080311 = v292
			end
			return result2286080311
		end
	end
end)
v46 = lazy(function()
	return function(v286)
		return function(v287)
			return function(v288)
				local v289 = ((v30())(v286))(v288)
				local v290 = ((v30())(v287))((v289)[1])
				return (v290)[0]
			end
		end
	end
end)
v47 = lazy(function()
	return function(v284)
		return function(v285)
			local scrutinee = (v284 > 0)
			local result4294664152
			if false then
			elseif scrutinee == true then
				result4294664152 = ((v25())(v285))(((v47())((v284 - 1)))(v285))
			elseif scrutinee == false then
				result4294664152 = ""
			end
			return result4294664152
		end
	end
end)
v48 = lazy(function()
	return function(v281)
		return function(v282)
			return ((v49())(function(v283)
				return eq(v283, v281)
			end))(v282)
		end
	end
end)
v49 = lazy(function()
	return function(v279)
		return function(v280)
			return (v50())(((v5())(v279))(v280))
		end
	end
end)
v50 = lazy(function()
	return function(v277)
		local scrutinee = v277
		local result2090770491
		if false then
		elseif scrutinee["$variant"] == "some" then
			local v278 = scrutinee["$payload"]
			result2090770491 = true
		elseif scrutinee["$variant"] == "none" then
			result2090770491 = false
		end
		return result2090770491
	end
end)
v51 = lazy(function()
	return function(v274)
		local scrutinee = v274
		local result2090770488
		if false then
		elseif not scrutinee["$empty"] then
			local v275 = scrutinee["$first"]
			local v276 = scrutinee["$rest"]
			result2090770488 = variant("some", v276)
		elseif scrutinee["$empty"] then
			result2090770488 = variant("none")
		end
		return result2090770488
	end
end)
v52 = lazy(function()
	return function(v273)
		return (v53())(v273)
	end
end)
v53 = lazy(function()
	return function(v271)
		return ((v15())(function(v272)
			return (v272)[0]
		end))((v54())(v271))
	end
end)
v54 = lazy(function()
	return function(v270)
		return ((v7())(v270))(empty)
	end
end)
v55 = lazy(function()
	return function(v268)
		return function(v269)
			return (((v39())(v268))({}))(v269)
		end
	end
end)
v56 = lazy(function()
	return function(v261)
		return function(v262)
			local scrutinee = ({ v262, v261 })[0]
			local result2987567745
			if false then
			elseif not scrutinee["$empty"] then
				local v264 = scrutinee["$first"]
				local v265 = scrutinee["$rest"]
				local scrutinee = ({ v262, v261 })[1]
				local result2987567778
				if false then
				elseif not scrutinee["$empty"] then
					local v266 = scrutinee["$first"]
					local v267 = scrutinee["$rest"]
					result2987567778 = cons({ v264, v266 }, ((v56())(v267))(v265))
				end
				result2987567745 = result2987567778
			elseif true then
				result2987567745 = empty
			end
			return result2987567745
		end
	end
end)
v57 = lazy(function()
	return function(v259)
		return (function(v260)
			return panic(v260)
		end)(v259)
	end
end)
v58 = lazy(function()
	return function(v255)
		return function(v256)
			local scrutinee = v256
			local result2090770424
			if false then
			elseif not scrutinee["$empty"] then
				local v257 = scrutinee["$first"]
				local v258 = scrutinee["$rest"]
				local scrutinee = (v255 > 0)
				local result459395638
				if false then
				elseif scrutinee == true then
					result459395638 = cons(v257, ((v58())((v255 - 1)))(v258))
				elseif scrutinee == false then
					result459395638 = empty
				end
				result2090770424 = result459395638
			elseif scrutinee["$empty"] then
				result2090770424 = empty
			end
			return result2090770424
		end
	end
end)
v59 = lazy(function()
	return function(v252)
		return function(v253)
			local scrutinee = v253
			local result2090770421
			if false then
			elseif scrutinee["$variant"] == "none" then
				result2090770421 = variant("none")
			elseif scrutinee["$variant"] == "some" then
				local v254 = scrutinee["$payload"]
				local scrutinee = (v252)(v254)
				local result1038740487
				if false then
				elseif scrutinee == true then
					result1038740487 = variant("some", v254)
				elseif scrutinee == false then
					result1038740487 = variant("none")
				end
				result2090770421 = result1038740487
			end
			return result2090770421
		end
	end
end)
v60 = lazy(function()
	return function(v250)
		return function(v251)
			return (((v61())((v11())(v250)))(v250))(v251)
		end
	end
end)
v61 = lazy(function()
	return function(v246)
		return function(v247)
			return function(v248)
				return ((v17())(function(v249)
					local scrutinee = eq((v249)["key"], v247)
					local result2239418294
					if false then
					elseif scrutinee == true then
						result2239418294 = variant("some", (v249)["value"])
					elseif scrutinee == false then
						local scrutinee = (v246 > (v249)["hash"])
						local result1597173932
						if false then
						elseif scrutinee == true then
							result1597173932 = (((v61())(v246))(v247))((v249)["rhs"])
						elseif scrutinee == false then
							result1597173932 = (((v61())(v246))(v247))((v249)["lhs"])
						end
						result2239418294 = result1597173932
					end
					return result2239418294
				end))(v248)
			end
		end
	end
end)
v62 = lazy(function()
	return function(v242)
		return function(v243)
			local scrutinee = v243
			local result2090770388
			if false then
			elseif scrutinee["$variant"] == "ok" then
				local v245 = scrutinee["$payload"]
				result2090770388 = (v242)(v245)
			elseif scrutinee["$variant"] == "err" then
				local v244 = scrutinee["$payload"]
				result2090770388 = pure(variant("err", v244))
			end
			return result2090770388
		end
	end
end)
v63 = lazy(function()
	return function(v238)
		return function(v239)
			local scrutinee = v239
			local result2090770361
			if false then
			elseif not scrutinee["$empty"] then
				local v240 = scrutinee["$first"]
				local v241 = scrutinee["$rest"]
				local scrutinee = (v238)(v240)
				local result3732669062
				if false then
				elseif scrutinee == false then
					result3732669062 = ((v63())(v238))(v241)
				elseif scrutinee == true then
					result3732669062 = cons(v240, ((v63())(v238))(v241))
				end
				result2090770361 = result3732669062
			elseif scrutinee["$empty"] then
				result2090770361 = empty
			end
			return result2090770361
		end
	end
end)
v64 = lazy(function()
	return function(v232)
		return function(v233)
			local scrutinee = v233
			local result2090770355
			if false then
			elseif not scrutinee["$empty"] then
				local v234 = scrutinee["$first"]
				local v235 = scrutinee["$rest"]
				result2090770355 = bind((v232)(v234), function(v236)
					return bind(((v64())(v232))(v235), function(v237)
						return pure(cons(v236, v237))
					end)
				end)
			elseif scrutinee["$empty"] then
				result2090770355 = pure(empty)
			end
			return result2090770355
		end
	end
end)
v65 = lazy(function()
	return function(v229)
		return function(v230)
			local scrutinee = v230
			local result2090770352
			if false then
			elseif scrutinee["$variant"] == "some" then
				local v231 = scrutinee["$payload"]
				result2090770352 = v231
			elseif scrutinee["$variant"] == "none" then
				result2090770352 = v229
			end
			return result2090770352
		end
	end
end)
v66 = lazy(function()
	return function(v228)
		local scrutinee = (v44())(v228)
		local result281972392
		if false then
		elseif scrutinee == true then
			result281972392 = false
		elseif scrutinee == false then
			result281972392 = true
		end
		return result281972392
	end
end)
v67 = lazy(function()
	return function(v226)
		return function(v227)
			local scrutinee = (v226 > v227)
			local result1281059509
			if false then
			elseif scrutinee == true then
				result1281059509 = v226
			elseif scrutinee == false then
				result1281059509 = v227
			end
			return result1281059509
		end
	end
end)
v68 = lazy(function()
	return function(v224)
		return (((v69())(1))(0))((function(v225)
			return dynamic(v225)
		end)(v224))
	end
end)
v69 = lazy(function()
	return function(v202)
		return function(v203)
			return function(v204)
				local v207 = function(v205)
					return function(v206)
						local scrutinee = (v203 > v205)
						local result3872451756
						if false then
						elseif scrutinee == true then
							result3872451756 = ((v25())(")"))(((v26())("("))(v206))
						elseif scrutinee == false then
							result3872451756 = v206
						end
						return result3872451756
					end
				end
				local v209 = function(v208)
					return ((v26())("\n"))(((v47())(v208))("  "))
				end
				local scrutinee = v204
				local result2090770257
				if false then
				elseif scrutinee["$variant"] == "real'" then
					local v222 = scrutinee["$payload"]
					result2090770257 = (function(v223)
						return tostring(v223)
					end)(v222)
				elseif scrutinee["$variant"] == "lambda" then
					result2090770257 = "lambda"
				elseif scrutinee["$variant"] == "tuple" then
					local v221 = scrutinee["$payload"]
					result2090770257 = ((v207)(0))(((v24())(", "))(((v15())(((v69())(v202))(1)))(v221)))
				elseif scrutinee["$variant"] == "record" then
					local v218 = scrutinee["$payload"]
					local scrutinee = v218
					local result2090770294
					if false then
					elseif scrutinee["$empty"] then
						result2090770294 = "{}"
					elseif true then
						result2090770294 = ((v25())("}"))(
							((v25())((v209)((v202 - 1))))(
								((v26())("{"))(((v26())((v209)(v202)))(((v24())((v209)(v202)))(((v15())(function(v220)
									return ((v25())((((v69())((v202 + 1)))(0))((v220)[1])))(((v25())(": "))((v220)[0]))
								end))(v218))))
							)
						)
					end
					result2090770257 = result2090770294
				elseif scrutinee["$variant"] == "int'" then
					local v216 = scrutinee["$payload"]
					result2090770257 = (function(v217)
						return tostring(v217)
					end)(v216)
				elseif scrutinee["$variant"] == "str'" then
					local v215 = scrutinee["$payload"]
					result2090770257 = ((v25())('"'))(
						((v26())('"'))(
							(((v27())("\0"))("\\0"))(
								(((v27())("\t"))("\\t"))(
									(((v27())("\r"))("\\r"))(
										(((v27())("\n"))("\\n"))(
											(((v27())('"'))('\\"'))((((v27())("\\"))("\\\\"))(v215))
										)
									)
								)
							)
						)
					)
				elseif scrutinee["$variant"] == "nat'" then
					local v213 = scrutinee["$payload"]
					result2090770257 = (function(v214)
						return tostring(v214)
					end)(v213)
				elseif scrutinee["$variant"] == "action" then
					result2090770257 = "action"
				elseif scrutinee["$variant"] == "list" then
					local v212 = scrutinee["$payload"]
					result2090770257 = ((v25())("]"))(
						((v25())((v209)((v202 - 1))))(
							((v26())("["))(
								((v26())((v209)(v202)))(
									((v24())((v209)(v202)))(((v15())(((v69())((v202 + 1)))(0)))(v212))
								)
							)
						)
					)
				elseif scrutinee["$variant"] == "variant" then
					local v210 = scrutinee["$payload"]
					local scrutinee = (v210)[1]
					local result2324665480
					if false then
					elseif scrutinee["$variant"] == "none" then
						result2324665480 = ((v26())(":"))((v210)[0])
					elseif scrutinee["$variant"] == "some" then
						local v211 = scrutinee["$payload"]
						result2324665480 = ((v207)(1))(
							((v25())((((v69())(v202))(1))(v211)))(((v25())(" "))(((v26())(":"))((v210)[0])))
						)
					end
					result2090770257 = result2324665480
				end
				return result2090770257
			end
		end
	end
end)
v70 = lazy(function()
	return function(v201)
		local scrutinee = (v2())(v201)
		local result88579433
		if false then
		elseif scrutinee == true then
			result88579433 = variant("none")
		elseif scrutinee == false then
			result88579433 = variant("some", ((v30())(1))(v201))
		end
		return result88579433
	end
end)
v71 = lazy(function()
	return function(v199)
		return function(v200)
			return (v50())(((v31())(v199))(v200))
		end
	end
end)
v72 = lazy(function()
	return function(v195)
		local scrutinee = v195
		local result2090769466
		if false then
		elseif not scrutinee["$empty"] then
			local v196 = scrutinee["$first"]
			local v197 = scrutinee["$rest"]
			local scrutinee = v197
			local result2090769468
			if false then
			elseif scrutinee["$empty"] then
				result2090769468 = variant("some", v196)
			elseif true then
				result2090769468 = (v72())(v197)
			end
			result2090769466 = result2090769468
		elseif scrutinee["$empty"] then
			result2090769466 = variant("none")
		end
		return result2090769466
	end
end)
v73 = lazy(function()
	return function(v191)
		return function(v192)
			return (((v14())(v192))(function(v193)
				return function(v194)
					return ((v55())(v194))(v193)
				end
			end))((v52())(v191))
		end
	end
end)
v74 = lazy(function()
	return function(v186)
		return function(v187)
			local scrutinee = v187
			local result2090769435
			if false then
			elseif not scrutinee["$empty"] then
				local v188 = scrutinee["$first"]
				local v189 = scrutinee["$rest"]
				local scrutinee = (v186)(v188)
				local result1213956691
				if false then
				elseif scrutinee["$variant"] == "some" then
					local v190 = scrutinee["$payload"]
					result1213956691 = variant("some", v190)
				elseif scrutinee["$variant"] == "none" then
					result1213956691 = ((v74())(v186))(v189)
				end
				result2090769435 = result1213956691
			elseif scrutinee["$empty"] then
				result2090769435 = variant("none")
			end
			return result2090769435
		end
	end
end)
v75 = lazy(function()
	return function(v184)
		return function(v185)
			return ((v76())(v184))(v185)
		end
	end
end)
v76 = lazy(function()
	return function(v182)
		return function(v183)
			return (v50())(((v60())(v182))(v183))
		end
	end
end)
v77 = lazy(function()
	return function(v181)
		return ((v78())(v181))(empty)
	end
end)
v78 = lazy(function()
	return function(v177)
		return function(v178)
			local scrutinee = v177
			local result2090769402
			if false then
			elseif not scrutinee["$empty"] then
				local v179 = scrutinee["$first"]
				local v180 = scrutinee["$rest"]
				result2090769402 = ((v78())(v180))(cons(v179, v178))
			elseif scrutinee["$empty"] then
				result2090769402 = v178
			end
			return result2090769402
		end
	end
end)
v79 = lazy(function()
	return function(v174)
		return function(v175)
			return function(v176)
				local scrutinee = v174
				local result2090769399
				if false then
				elseif scrutinee == true then
					result2090769399 = v176
				elseif scrutinee == false then
					result2090769399 = (v57())(v175)
				end
				return result2090769399
			end
		end
	end
end)
v80 = lazy(function()
	return function(v172)
		return function(v173)
			return (v6())(((v38())(v172))(v173))
		end
	end
end)
v81 = lazy(function()
	return function(v169)
		return function(v170)
			local scrutinee = v170
			local result2090769395
			if false then
			elseif scrutinee["$variant"] == "some" then
				local v171 = scrutinee["$payload"]
				result2090769395 = (v169)(v171)
			elseif scrutinee["$variant"] == "none" then
				result2090769395 = false
			end
			return result2090769395
		end
	end
end)
v82 = lazy(function()
	return function(v166)
		local scrutinee = v166
		local result2090769368
		if false then
		elseif scrutinee["$variant"] == "ok" then
			local v168 = scrutinee["$payload"]
			result2090769368 = v168
		elseif scrutinee["$variant"] == "err" then
			local v167 = scrutinee["$payload"]
			result2090769368 = (v57())((v22())(v167))
		end
		return result2090769368
	end
end)
v83 = lazy(function()
	return function(v162)
		local scrutinee = v162
		local result2090769364
		if false then
		elseif not scrutinee["$empty"] then
			local v163 = scrutinee["$first"]
			local v164 = scrutinee["$rest"]
			local scrutinee = v163
			local result2090769365
			if false then
			elseif scrutinee["$variant"] == "some" then
				local v165 = scrutinee["$payload"]
				result2090769365 = cons(v165, (v83())(v164))
			elseif scrutinee["$variant"] == "none" then
				result2090769365 = (v83())(v164)
			end
			result2090769364 = result2090769365
		elseif scrutinee["$empty"] then
			result2090769364 = empty
		end
		return result2090769364
	end
end)
v84 = lazy(function()
	return function(v157)
		return function(v158)
			return (((v14())(variant("none")))(function(v159)
				return function(v160)
					local scrutinee = v159
					local result2090769338
					if false then
					elseif scrutinee["$variant"] == "none" then
						result2090769338 = variant("some", v160)
					elseif scrutinee["$variant"] == "some" then
						local v161 = scrutinee["$payload"]
						result2090769338 = variant("some", ((v157)(v160))(v161))
					end
					return result2090769338
				end
			end))(v158)
		end
	end
end)
v85 = lazy(function()
	return function(v154)
		return function(v155)
			local scrutinee = v155
			local result2090769334
			if false then
			elseif scrutinee["$variant"] == "some" then
				local v156 = scrutinee["$payload"]
				result2090769334 = variant("some", v156)
			elseif scrutinee["$variant"] == "none" then
				result2090769334 = v154
			end
			return result2090769334
		end
	end
end)
v86 = lazy(function()
	return function(v150)
		return function(v151)
			local scrutinee = v151
			local result2090769330
			if false then
			elseif scrutinee["$variant"] == "ok" then
				local v153 = scrutinee["$payload"]
				result2090769330 = v153
			elseif scrutinee["$variant"] == "err" then
				local v152 = scrutinee["$payload"]
				result2090769330 = v150
			end
			return result2090769330
		end
	end
end)
v87 = lazy(function()
	return function(v148)
		return function(v149)
			local scrutinee = (v148 > 0)
			local result3911531127
			if false then
			elseif scrutinee == true then
				result3911531127 = cons(v149, ((v87())((v148 - 1)))(v149))
			elseif scrutinee == false then
				result3911531127 = empty
			end
			return result3911531127
		end
	end
end)
v88 = lazy(function()
	return function(v143)
		return function(v144)
			return function(v145)
				local scrutinee = v145
				local result2090769301
				if false then
				elseif not scrutinee["$empty"] then
					local v146 = scrutinee["$first"]
					local v147 = scrutinee["$rest"]
					result2090769301 = ((v144)((((v88())(v143))(v144))(v147)))(v146)
				elseif scrutinee["$empty"] then
					result2090769301 = v143
				end
				return result2090769301
			end
		end
	end
end)
v89 = lazy(function()
	return function(v139)
		local scrutinee = v139
		local result2090769272
		if false then
		elseif not scrutinee["$empty"] then
			local v140 = scrutinee["$first"]
			local v141 = scrutinee["$rest"]
			local scrutinee = (v89())(v141)
			local result980792459
			if false then
			elseif scrutinee["$variant"] == "some" then
				local v142 = scrutinee["$payload"]
				result980792459 = variant("some", { cons(v140, (v142)[0]), (v142)[1] })
			elseif scrutinee["$variant"] == "none" then
				result980792459 = variant("some", { empty, v140 })
			end
			result2090769272 = result980792459
		elseif scrutinee["$empty"] then
			result2090769272 = variant("none")
		end
		return result2090769272
	end
end)
v90 = lazy(function()
	return function(v137)
		return ((v91())(function(v138)
			return (v138)[0]
		end))((v70())(v137))
	end
end)
v91 = lazy(function()
	return function(v134)
		return function(v135)
			local scrutinee = v135
			local result2090769268
			if false then
			elseif scrutinee["$variant"] == "some" then
				local v136 = scrutinee["$payload"]
				result2090769268 = variant("some", (v134)(v136))
			elseif scrutinee["$variant"] == "none" then
				result2090769268 = variant("none")
			end
			return result2090769268
		end
	end
end)
v92 = lazy(function()
	return function(v131)
		local scrutinee = v131
		local result2090769264
		if false then
		elseif not scrutinee["$empty"] then
			local v132 = scrutinee["$first"]
			local v133 = scrutinee["$rest"]
			result2090769264 = variant("some", { v132, v133 })
		elseif scrutinee["$empty"] then
			result2090769264 = variant("none")
		end
		return result2090769264
	end
end)
v93 = lazy(function()
	return function(v129)
		return ((v15())(function(v130)
			return (v130)[1]
		end))((v54())(v129))
	end
end)
v94 = lazy(function()
	return function(v128)
		return (v95())(v128)
	end
end)
v95 = lazy(function()
	return function(v126)
		local scrutinee = v126
		local result2090769236
		if false then
		elseif scrutinee["$variant"] == "some" then
			local v127 = scrutinee["$payload"]
			result2090769236 = (1 + ((v95())((v127)["lhs"]) + (v95())((v127)["rhs"])))
		elseif scrutinee["$variant"] == "none" then
			result2090769236 = 0
		end
		return result2090769236
	end
end)
v96 = lazy(function()
	return extern["io::readln"]
end)
v97 = lazy(function()
	return function(v124)
		return function(v125)
			return (v83())(((v15())(v124))(v125))
		end
	end
end)
v98 = lazy(function()
	return ((v14())(0))(function(v122)
		return function(v123)
			return (v122 + v123)
		end
	end)
end)
v99 = lazy(function()
	return function(v118)
		return function(v119)
			local scrutinee = v119
			local result2090769206
			if false then
			elseif scrutinee["$variant"] == "ok" then
				local v121 = scrutinee["$payload"]
				result2090769206 = variant("ok", (v118)(v121))
			elseif scrutinee["$variant"] == "err" then
				local v120 = scrutinee["$payload"]
				result2090769206 = variant("err", v120)
			end
			return result2090769206
		end
	end
end)
v100 = lazy(function()
	return function(v112)
		return function(v113)
			return function(v114)
				local scrutinee = v114
				local result2090769201
				if false then
				elseif not scrutinee["$empty"] then
					local v115 = scrutinee["$first"]
					local v116 = scrutinee["$rest"]
					result2090769201 = bind(((v113)(v112))(v115), function(v117)
						return (((v100())(v117))(v113))(v116)
					end)
				elseif scrutinee["$empty"] then
					result2090769201 = pure(v112)
				end
				return result2090769201
			end
		end
	end
end)
v101 = lazy(function()
	return function(v110)
		local scrutinee = (v70())(v110)
		local result2428098621
		if false then
		elseif scrutinee["$variant"] == "some" then
			local v111 = scrutinee["$payload"]
			result2428098621 = cons((v111)[0], (v101())((v111)[1]))
		elseif scrutinee["$variant"] == "none" then
			result2428098621 = empty
		end
		return result2428098621
	end
end)
v102 = lazy(function()
	return function(v108)
		return function(v109)
			return ne(((v28())(v108))(v109), v109)
		end
	end
end)
v103 = lazy(function()
	return function(v104)
		return function(v105)
			return ((v65())(cons(v105, empty)))(((v91())(function(v106)
				return ((v8())((v106)[0]))(((v103())(v104))(((v28())(v104))((v106)[1])))
			end))(((v91())(function(v107)
				return ((v30())(v107))(v105)
			end))(((v31())(v104))(v105))))
		end
	end
end)
v35()()
