//! Noctyrn UI theme — dark minimal, built around the brand background
//! #141018 with purple/magenta accents from the logo.

use bevy::prelude::*;

// ── Surfaces ──

/// Main screen background (#141018).
pub const BG_BASE: Color = Color::srgba(0.0784, 0.0627, 0.0941, 1.0);
/// Panels, cards, menus (#1C1627).
pub const BG_PANEL: Color = Color::srgba(0.1098, 0.0863, 0.1529, 1.0);
/// Inputs, list items, buttons (#241C32).
pub const BG_ELEVATED: Color = Color::srgba(0.1412, 0.1098, 0.1961, 1.0);
/// Hover state (#2D2340).
pub const BG_HOVER: Color = Color::srgba(0.1765, 0.1373, 0.2471, 1.0);

// ── Borders ──

/// Subtle border (translucent purple-tinted white).
pub const BORDER: Color = Color::srgba(0.5882, 0.5451, 0.7216, 0.18);
/// Stronger border for emphasis.
pub const BORDER_STRONG: Color = Color::srgba(0.4431, 0.3529, 0.6157, 0.55);

// ── Accents (brand) ──

/// Brand purple (#9262DE).
pub const ACCENT: Color = Color::srgba(0.5725, 0.3843, 0.8706, 1.0);
/// Hover variant of the brand purple (#A67DE8).
pub const ACCENT_HOVER: Color = Color::srgba(0.6510, 0.4902, 0.9098, 1.0);
/// Brand magenta (#D468E0).
pub const ACCENT_MAGENTA: Color = Color::srgba(0.8314, 0.4078, 0.8784, 1.0);

// ── Text ──

/// Primary text (#F0ECF7).
pub const TEXT: Color = Color::srgba(0.9412, 0.9255, 0.9686, 1.0);
/// Secondary text (#9D94B0).
pub const TEXT_MUTED: Color = Color::srgba(0.6157, 0.5804, 0.6902, 1.0);
/// Faint/hint text (#675D7C).
pub const TEXT_FAINT: Color = Color::srgba(0.4039, 0.3647, 0.4863, 1.0);

// ── Semantic ──

/// Success / positive (#5FD68A).
pub const SUCCESS: Color = Color::srgba(0.3725, 0.8392, 0.5412, 1.0);
/// Warning / currency (#E6C45C).
pub const WARNING: Color = Color::srgba(0.9020, 0.7686, 0.3608, 1.0);
/// Danger / negative (#E05E5E).
pub const DANGER: Color = Color::srgba(0.8784, 0.3686, 0.3686, 1.0);

/// Standard rounded-corner radius for panels and buttons.
pub const RADIUS: BorderRadius = BorderRadius::all(Val::Px(8.0));
/// Smaller radius for compact elements.
pub const RADIUS_SM: BorderRadius = BorderRadius::all(Val::Px(4.0));
