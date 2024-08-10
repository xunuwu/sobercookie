#!/usr/bin/env lua

local lgi = require("lgi")
local Gtk = lgi.require("Gtk", "3.0")

local sobercookie_command = "sobercookie" -- uses sobercookie from $PATH

local app = Gtk.Application({ application_id = "io.github.xunuwu.sobercookie_launcher" })

local function get_accounts()
	local handle = io.popen(sobercookie_command .. " list")
	if handle == nil then
		print("failed run command")
		return
	end
	local result = handle:read("*a")
	local lines = {}
	for s in result:gmatch("[^\r\n]+") do
		table.insert(lines, s)
	end
	return lines
end

function app:on_startup()
	local window = Gtk.ApplicationWindow.new(self)
	window:set_title("Sobercookie Launcher")
	window:set_default_size(500, 500)
	local scrolled = Gtk.ScrolledWindow.new()
	local box = Gtk.Box.new(Gtk.Orientation.VERTICAL, 10)

	local accounts = get_accounts()
	if accounts ~= nil then
		for _, v in ipairs(accounts) do
			local button = Gtk.Button.new_with_label(v)
			box:pack_start(button, false, true, 0)
			function button:on_clicked()
				os.execute(sobercookie_command .. " launch " .. v .. " " .. table.concat(arg, " "))
				os.exit()
			end
		end
	end

	box:show_all()
	scrolled:add(box)
	scrolled:show()
	window:add(scrolled)

	window:set_resizable(false)
end

function app:on_activate()
	self.active_window:present()
end

return app:run(arg)
