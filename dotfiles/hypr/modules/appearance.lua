-- ~/.config/hypr/modules/appearance.lua
--
-- Hyprland 0.55+ appearance & system performance configuration
--
-- Design:
--   Clean, minimal, 14px rounding, small gaps
--   Battery & CPU optimized with VFR (Variable Frame Rate)

-- ────────────────────────────────────────────────────────
-- DYNAMIC TIME-OF-DAY BORDER COLORS
-- ────────────────────────────────────────────────────────
local active_color = "rgba(5294e2ff)"   -- Default Day: Bright Blue
local inactive_color = "rgba(2a2a37ff)" -- Default Day: Greyish Blue

local hour = tonumber(os.date("%H"))
if hour >= 18 or hour < 6 then
	-- Evening/Night: Shift to a soft, muted indigo/purple
	active_color = "rgba(5e5c64ff)"
	inactive_color = "rgba(1e1e2eff)"
end

hl.config({

	--------------------------------------------------------
	-- GENERAL
	--------------------------------------------------------

	general = {

		-- Compact spacing between tiled windows.
		gaps_in = 3,

		-- Small screen-edge spacing.
		gaps_out = 6,

		-- Thin window border.
		border_size = 2,

		-- Allow resizing directly from window borders.
		resize_on_border = true,

		-- Prevent tearing.
		allow_tearing = false,

		-- Primary tiling layout.
		layout = "dwindle",
		
		-- Dynamic Borders
		["col.active_border"] = active_color,
		["col.inactive_border"] = inactive_color,
	},

	--------------------------------------------------------
	-- DECORATION
	--------------------------------------------------------

	decoration = {

		-- Modern universal corner rounding.
		rounding = 14,

		----------------------------------------------------
		-- BLUR (Hyprland 0.57+ Blur Shaders & Variants)
		----------------------------------------------------
		-- Available variants:
		--   "kawase"       - Standard dual-kawase blur (default, clean & lightweight)
		--   "frost"        - Frosted glass effect with surface micro-texture
		--   "ripple"       - Interactive ripple distortion wave effect
		--   "drops"        - Animated rain / liquid droplets on glass
		--   "water"        - Animated water waves and fluid surface displacement
		--   "fluid_jar"    - Viscous fluid physics container
		--   "prism"        - Rainbow chromatic dispersion & prism refraction
		--   "heat_shimmer" - Animated heat waves & mirage shimmer
		--   "acrylic"      - Modern acrylic with chromatic aberration & light diffusion
		--   "aurora"       - Animated northern lights / aurora borealis ambient color glow
		--   "haze"         - Iridescent atmospheric color haze
		-- Live testing: run 'hypr-blur next' or 'hypr-blur set <variant>' in terminal!
		blur = {
			enabled = true,
			variant = "acrylic", -- Active: modern acrylic glass effect with light diffusion & aberration
			size = 5,
			passes = 2,
			proportional = true, -- scale blur proportionally with display resolution
			ignore_opacity = true,
			new_optimizations = true,
		},

		----------------------------------------------------
		-- SHADOW
		----------------------------------------------------

		shadow = {
			enabled = false,
		},
	},

	--------------------------------------------------------
	-- MISCELLANEOUS & BATTERY EFFICIENCY
	--------------------------------------------------------

	misc = {
		-- Disable logos and splash screen
		disable_hyprland_logo = true,
		disable_splash_rendering = true,
		disable_scale_notification = true,

		-- Auto-focus windows that request attention
		focus_on_activate = true,

		-- Power management: wake display on mouse or keypress
		mouse_move_enables_dpms = true,
		key_press_enables_dpms = true,

		-- Allow session lock restore on recovery
		allow_session_lock_restore = true,
	},

	--------------------------------------------------------
	-- XWAYLAND CRISPNESS
	--------------------------------------------------------

	xwayland = {
		force_zero_scaling = true,
	},

	--------------------------------------------------------
	-- CURSOR
	--------------------------------------------------------

	cursor = {
		zoom_factor = 1.0,
		hide_on_key_press = true,
		warp_on_change_workspace = 1,
	},
})
