----------------------------------------------------------------
-- REQUIRED
----------------------------------------------------------------
require("hs.ipc")

hs.allowAppleScript(true)

local log = hs.logger.new("keyremap", "debug")

----------------------------------------------------------------
-- CONFIG
----------------------------------------------------------------
local RCTRL = 62
local TAP_TIMEOUT = 0.20 -- seconds (tap vs hold threshold)

----------------------------------------------------------------
-- GLOBAL TAP REGISTRY (prevent garbage collection)
----------------------------------------------------------------
_G._taps = {}

----------------------------------------------------------------
-- STATE
----------------------------------------------------------------
local rctrlPressedAt = nil
local rctrlUsedAsModifier = false
local secureInputPreviously = false

----------------------------------------------------------------
-- RIGHT CTRL → ESC (bulletproof)
----------------------------------------------------------------

-- flags watcher (press/release detection)
local ctrlFlagWatcher = hs.eventtap.new(
  { hs.eventtap.event.types.flagsChanged },
  function(event)
    if event:getKeyCode() ~= RCTRL then return false end

    local flags = event:getFlags()
    local ctrlActive = flags.ctrl

    ------------------------------------------------------------
    -- PRESSED
    ------------------------------------------------------------
    if ctrlActive then
      rctrlPressedAt = hs.timer.secondsSinceEpoch()
      rctrlUsedAsModifier = false
      return false
    end

    ------------------------------------------------------------
    -- RELEASED
    ------------------------------------------------------------
    if not rctrlPressedAt then return false end

    local heldTime = hs.timer.secondsSinceEpoch() - rctrlPressedAt
    local used = rctrlUsedAsModifier
    rctrlPressedAt = nil

    if not used and heldTime < TAP_TIMEOUT then
      log.d("rctrl tap → escape")

      return false, {
        hs.eventtap.event.newKeyEvent({}, "escape", true),
        hs.eventtap.event.newKeyEvent({}, "escape", false),
      }
    end

    return false
  end
)

-- key watcher (detect modifier usage)
local ctrlKeyWatcher = hs.eventtap.new(
  { hs.eventtap.event.types.keyDown },
  function(_)
    if rctrlPressedAt then
      rctrlUsedAsModifier = true
    end
    return false
  end
)

----------------------------------------------------------------
-- ALT + HJKL → ARROWS (robust)
----------------------------------------------------------------
local altArrowMap = {
  [4]  = "left",   -- h
  [38] = "down",   -- j
  [40] = "up",     -- k
  [37] = "right",  -- l
}

local altArrowRemap = hs.eventtap.new(
  { hs.eventtap.event.types.keyDown, hs.eventtap.event.types.keyUp },
  function(event)
    local flags = event:getFlags()
    local arrow = altArrowMap[event:getKeyCode()]
    if not flags.alt or not arrow then return false end

    local isDown = event:getType() == hs.eventtap.event.types.keyDown

    local mods = {}
    if flags.shift then table.insert(mods, "shift") end
    if flags.ctrl  then table.insert(mods, "ctrl")  end
    if flags.cmd   then table.insert(mods, "cmd")   end

    return true, { hs.eventtap.event.newKeyEvent(mods, arrow, isDown) }
  end
)

----------------------------------------------------------------
-- START TAPS
----------------------------------------------------------------
_G._taps.ctrlFlagWatcher = ctrlFlagWatcher
_G._taps.ctrlKeyWatcher = ctrlKeyWatcher
_G._taps.altArrowRemap = altArrowRemap

ctrlFlagWatcher:start()
ctrlKeyWatcher:start()
altArrowRemap:start()

----------------------------------------------------------------
-- 🔥 SELF-HEALING WATCHDOG (VERY IMPORTANT)
----------------------------------------------------------------
local function keepAlive(name, tap)
  hs.timer.doEvery(10, function()
    if not tap:isEnabled() then
      log.e(name .. " died → restarting")
      tap:start()
    end
  end)
end

keepAlive("ctrlFlagWatcher", ctrlFlagWatcher)
keepAlive("ctrlKeyWatcher", ctrlKeyWatcher)
keepAlive("altArrowRemap", altArrowRemap)

----------------------------------------------------------------
-- 🔒 SECURE INPUT MONITOR (explains “random” failures)
----------------------------------------------------------------
hs.timer.doEvery(2, function()
  local secure = hs.eventtap.isSecureInputEnabled()

  if secure and not secureInputPreviously then
    log.w("Secure Input ENABLED (some apps block key capture)")
  elseif not secure and secureInputPreviously then
    log.i("Secure Input disabled")
  end

  secureInputPreviously = secure
end)

----------------------------------------------------------------
-- OPTIONAL: reload alert
----------------------------------------------------------------
hs.alert.closeAll()
hs.alert.show("Hammerspoon keyremap loaded")

----------------------------------------------------------------
-- 🔍 PERIODIC DEBUG STATUS
----------------------------------------------------------------
local DEBUG_INTERVAL = 5 -- seconds (adjust if noisy)

hs.timer.doEvery(DEBUG_INTERVAL, function()
  local secure = hs.eventtap.isSecureInputEnabled()

  local function tapStatus(name, tap)
    local ok, enabled = pcall(function() return tap:isEnabled() end)
    if not ok then
      return name .. "=ERROR"
    end
    return string.format("%s=%s", name, enabled and "ON" or "OFF")
  end

  log.df(
    "STATUS | secure=%s | rctrlDown=%s | usedAsMod=%s | %s | %s | %s",
    tostring(secure),
    tostring(rctrlPressedAt ~= nil),
    tostring(rctrlUsedAsModifier),
    tapStatus("flagTap", ctrlFlagWatcher),
    tapStatus("keyTap", ctrlKeyWatcher),
    tapStatus("altTap", altArrowRemap)
  )
end)
