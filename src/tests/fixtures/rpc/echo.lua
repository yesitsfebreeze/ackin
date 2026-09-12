return {
	provide = {"lua"},
	apply = function(ctx)
		ctx:provide("lua", function(value)
			if value == "error" then error("contract service failed") end
			return value
		end)
	end,
}
