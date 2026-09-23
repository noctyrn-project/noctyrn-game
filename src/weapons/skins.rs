//! Data-driven weapon skins.
//!
//! A skin is a JSON file under `assets/cosmetics/weapons/skins/` describing a
//! `finish` recipe (composable: solid color, texture, emissive pulse, UV
//! scroll, pattern atlas, procedural galaxy) plus a rarity tier. Rarity is a
//! *label* (drop weight + sell value + colour), deliberately decoupled from
//! the finish: any tier may use any combination of finish features.
//!
//! Every owned copy also carries `wear` (0.00 = factory new, 1.00 = heavily
//! worn) and, when the finish has a `pattern`, a `pattern_seed`. Both live on
//! the inventory item and are handed to the weapon material so duplicates
//! look different.
//!
//! The system is still client-local (`savestate.json`), but `Inventory` /
//! `ItemKind` are shaped so they can move to a server table unchanged.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::material::{WeaponMaterial, WeaponSkinExt};

/// Directory scanned at runtime for skin definitions.
pub const SKIN_DIR: &str = "assets/cosmetics/weapons/skins";

/// The rarity ladder. Order matters: it drives display sorting and the
/// "better than" comparisons. Nothing about a finish is tied to a tier.
#[derive(
    Debug, Clone, Copy, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize,
)]
#[serde(rename_all = "snake_case")]
pub enum Rarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Mythic,
}

impl Rarity {
    pub const ALL: [Rarity; 6] = [
        Rarity::Common,
        Rarity::Uncommon,
        Rarity::Rare,
        Rarity::Epic,
        Rarity::Legendary,
        Rarity::Mythic,
    ];

    pub fn display_name(&self) -> &'static str {
        match self {
            Rarity::Common => "Common",
            Rarity::Uncommon => "Uncommon",
            Rarity::Rare => "Rare",
            Rarity::Epic => "Epic",
            Rarity::Legendary => "Legendary",
            Rarity::Mythic => "Mythic",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            Rarity::Common => Color::srgb(0.6, 0.6, 0.6),
            Rarity::Uncommon => Color::srgb(0.3, 0.7, 0.3),
            Rarity::Rare => Color::srgb(0.2, 0.4, 0.9),
            Rarity::Epic => Color::srgb(0.6, 0.2, 0.8),
            Rarity::Legendary => Color::srgb(0.9, 0.7, 0.1),
            Rarity::Mythic => Color::srgb(0.9, 0.15, 0.15),
        }
    }

    /// Baseline drop weight (out of ~1000) when a crate does not override it.
    pub fn drop_weight(&self) -> u32 {
        match self {
            Rarity::Common => 500,
            Rarity::Uncommon => 250,
            Rarity::Rare => 150,
            Rarity::Epic => 50,
            Rarity::Legendary => 9,
            Rarity::Mythic => 1,
        }
    }

    pub fn sell_value(&self) -> u64 {
        match self {
            Rarity::Common => 10,
            Rarity::Uncommon => 25,
            Rarity::Rare => 50,
            Rarity::Epic => 100,
            Rarity::Legendary => 250,
            Rarity::Mythic => 1000,
        }
    }
}

/// Stable skin identifier (file id, e.g. `solid_red`, `galaxy_deep_field`).
#[derive(Debug, Clone, PartialEq, Eq, Hash, PartialOrd, Ord, Serialize, Deserialize)]
#[serde(transparent)]
pub struct SkinId(pub String);

impl SkinId {
    /// Built-in fallback skin — always present, never droppable.
    pub const DEFAULT: &'static str = "default";

    pub fn new(id: impl Into<String>) -> Self {
        Self(id.into())
    }

    pub fn default_id() -> Self {
        Self::new(Self::DEFAULT)
    }

    pub fn as_str(&self) -> &str {
        &self.0
    }

    pub fn is_default(&self) -> bool {
        self.0 == Self::DEFAULT
    }
}

impl std::fmt::Display for SkinId {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        f.write_str(&self.0)
    }
}

impl Default for SkinId {
    fn default() -> Self {
        Self::default_id()
    }
}

// ── Finish recipe ────────────────────────────────────────────────────────

fn default_base_color() -> [f32; 4] {
    // #555566 — double-nibble so the built-in default follows the colour rule.
    [85.0 / 255.0, 85.0 / 255.0, 102.0 / 255.0, 1.0]
}

fn default_metallic() -> f32 {
    0.8
}

fn default_roughness() -> f32 {
    0.35
}

fn default_one() -> u32 {
    1
}

fn default_seeded_mode() -> String {
    "seeded".to_string()
}

fn default_strength() -> f32 {
    1.0
}

/// Animated emissive glow. `pulse_hz = 0` means steady.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Emissive {
    #[serde(default = "default_base_color")]
    pub color: [f32; 4],
    #[serde(default = "default_strength")]
    pub strength: f32,
    #[serde(default)]
    pub pulse_hz: f32,
}

/// A crop ("stamp") taken out of a grayscale motif sheet. The sheet is
/// tinted by the finish's `base_color`; the crop position comes from the
/// skin's optional fixed `offset`, otherwise from the per-copy pattern seed
/// so duplicates differ (CSGO-style patterns). The stamp size is fixed in
/// the shader so it isn't another authoring knob.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Pattern {
    pub sheet: String,
    /// Fixed crop origin in sheet UV space (0..1-STAMP). Omitted = seeded.
    #[serde(default)]
    pub offset: Option<[f32; 2]>,
}

/// Fraction of the motif sheet a single gun shows. Mirrors `STAMP` in
/// `assets/shaders/weapon_skin.wgsl`.
pub const PATTERN_STAMP: f32 = 0.5;

/// Pixel size the motif sheets are generated at (`noctyrn-extras
/// --gen-pattern`). Used to crop matching thumbnail bands on the CPU.
pub const PATTERN_SHEET_PX: f32 = 2048.0;

/// WGSL-compatible `hash21` from `weapon_skin.wgsl` (f32 math, same
/// operations). Used to pick the same pattern crop on the CPU.
fn hash21(x: f32, y: f32) -> f32 {
    let fract = |v: f32| v - v.floor();
    let mut p3 = [fract(x * 0.1031), fract(y * 0.1031), fract(x * 0.1031)];
    let d = p3[0] * (p3[1] + 33.33) + p3[1] * (p3[2] + 33.33) + p3[2] * (p3[0] + 33.33);
    for v in p3.iter_mut() {
        *v += d;
    }
    fract((p3[0] + p3[1]) * p3[2])
}

/// Two-tone gradient blended along one UV axis. The `color` is mixed in on
/// top of `base_color`.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Gradient {
    #[serde(default = "default_base_color")]
    pub color: [f32; 4],
    /// `x` (default) or `y`.
    #[serde(default = "default_axis")]
    pub axis: String,
}

fn default_axis() -> String {
    "x".to_string()
}

/// Procedural galaxy finish: a view-ray window into layered parallax nebula
/// and star fields. Reusable: palette/density/speed/stars/layers produce
/// visibly different galaxies from one shader.
#[derive(Debug, Clone, Default, Serialize, Deserialize)]
pub struct Galaxy {
    #[serde(default = "default_palette")]
    pub palette: Vec<[f32; 3]>,
    #[serde(default = "default_density")]
    pub density: f32,
    #[serde(default = "default_speed")]
    pub speed: f32,
    #[serde(default = "default_stars")]
    pub stars: f32,
    /// Star-field layers at increasing depth (1..4). More = deeper field.
    #[serde(default = "default_layers")]
    pub layers: u32,
    /// How strongly the layers separate as the view moves (0 = flat).
    #[serde(default = "default_parallax")]
    pub parallax: f32,
    /// Star field density in cells per unit.
    #[serde(default = "default_star_density")]
    pub star_density: f32,
    /// Star twinkle amount (0 = steady).
    #[serde(default = "default_twinkle")]
    pub twinkle: f32,
    /// Galactic plane + core glow strength.
    #[serde(default = "default_core")]
    pub core: f32,
}

fn default_palette() -> Vec<[f32; 3]> {
    vec![[0.35, 0.1, 0.6], [0.1, 0.25, 0.7], [0.9, 0.4, 0.8]]
}

fn default_density() -> f32 {
    1.0
}

fn default_speed() -> f32 {
    0.05
}

fn default_stars() -> f32 {
    1.0
}

fn default_layers() -> u32 {
    3
}

fn default_parallax() -> f32 {
    1.0
}

fn default_star_density() -> f32 {
    26.0
}

fn default_twinkle() -> f32 {
    1.0
}

fn default_core() -> f32 {
    0.35
}

/// A composable appearance recipe. Every field is optional; a skin's
/// "uniqueness" is simply which features it turns on.
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct Finish {
    #[serde(default = "default_base_color")]
    pub base_color: [f32; 4],
    #[serde(default)]
    pub texture: Option<String>,
    #[serde(default)]
    pub normal_map: Option<String>,
    #[serde(default = "default_metallic")]
    pub metallic: f32,
    #[serde(default = "default_roughness")]
    pub roughness: f32,
    #[serde(default)]
    pub emissive: Option<Emissive>,
    #[serde(default)]
    pub uv_scroll: Option<[f32; 2]>,
    #[serde(default)]
    pub pattern: Option<Pattern>,
    #[serde(default)]
    pub gradient: Option<Gradient>,
    #[serde(default)]
    pub galaxy: Option<Galaxy>,
}

impl Default for Finish {
    fn default() -> Self {
        Self {
            base_color: default_base_color(),
            texture: None,
            normal_map: None,
            metallic: default_metallic(),
            roughness: default_roughness(),
            emissive: None,
            uv_scroll: None,
            pattern: None,
            gradient: None,
            galaxy: None,
        }
    }
}

impl Finish {
    /// Whether the finish animates (drives the "is this special?" feel).
    pub fn is_animated(&self) -> bool {
        self.galaxy.is_some()
            || self.uv_scroll.is_some()
            || self
                .emissive
                .as_ref()
                .is_some_and(|e| e.pulse_hz.abs() > f32::EPSILON)
    }

    pub fn has_pattern(&self) -> bool {
        self.pattern.is_some()
    }

    /// Where this copy's crop of the motif sheet starts, in sheet UV space.
    /// Mirrors the WGSL in `weapon_skin.wgsl` so card thumbnails show the
    /// same piece of the motif the gun does.
    pub fn pattern_crop_origin(&self, seed: u32) -> [f32; 2] {
        match self.pattern.as_ref().and_then(|p| p.offset) {
            Some(offset) => offset,
            None => [
                hash21(seed as f32, 1.3) * (1.0 - PATTERN_STAMP),
                hash21(seed as f32, 7.7) * (1.0 - PATTERN_STAMP),
            ],
        }
    }

    /// Build the shader extension for one owned copy.
    pub fn to_extension(&self, instance: &SkinInstance) -> WeaponSkinExt {
        let mut ext = WeaponSkinExt::default();
        ext.params = Vec4::new(
            instance.wear.clamp(0.0, 1.0),
            0.0,
            (instance.seed % 1_000_000) as f32,
            if self.galaxy.is_some() { 1.0 } else { 0.0 },
        );

        let (pulse, strength) = self
            .emissive
            .as_ref()
            .map(|e| (e.pulse_hz, e.strength))
            .unwrap_or((0.0, 0.0));
        let scroll = self.uv_scroll.unwrap_or([0.0, 0.0]);
        ext.anim = Vec4::new(pulse, strength, scroll[0], scroll[1]);

        if let Some(galaxy) = &self.galaxy {
            let color = |i: usize| {
                galaxy
                    .palette
                    .get(i)
                    .map(|c| Vec4::new(c[0], c[1], c[2], 1.0))
                    .unwrap_or(Vec4::ONE)
            };
            ext.palette_a = color(0);
            ext.palette_b = color(1);
            ext.palette_c = color(2);
            ext.galaxy = Vec4::new(galaxy.density, galaxy.speed, galaxy.stars, galaxy.parallax);
            ext.galaxy_b = Vec4::new(
                galaxy.star_density,
                galaxy.twinkle,
                galaxy.core,
                galaxy.layers as f32,
            );
        }

        if let Some(pattern) = &self.pattern {
            let (offset_x, offset_y, fixed) = match pattern.offset {
                Some([x, y]) => (x, y, 1.0),
                None => (0.0, 0.0, 0.0),
            };
            // z = strength, w = enabled; xy = fixed crop origin (0,0 when
            // the seed picks it).
            ext.pattern = Vec4::new(offset_x, offset_y, 1.0, 1.0);
            ext.extra.y = fixed;
        }

        if let Some(gradient) = &self.gradient {
            let [r, g, b, _] = gradient.color;
            let axis = if gradient.axis == "y" { 1.0 } else { 0.0 };
            ext.gradient = Vec4::new(r, g, b, axis);
            ext.extra.x = 1.0;
        }
        ext
    }
}

/// One skin definition (one JSON file).
#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct SkinDef {
    pub id: SkinId,
    pub name: String,
    pub rarity: Rarity,
    /// Weapon ids this skin applies to. Empty = universal (any weapon).
    /// Legacy skins migrated from the old enum are universal.
    #[serde(default)]
    pub weapons: Vec<String>,
    #[serde(default)]
    pub finish: Finish,
}

impl SkinDef {
    pub fn is_universal(&self) -> bool {
        self.weapons.is_empty()
    }

    /// Applies to a specific weapon (universal skins always do).
    pub fn fits_weapon(&self, weapon_id: &str) -> bool {
        self.is_universal() || self.weapons.iter().any(|w| w == weapon_id)
    }

    /// Representative colour for UI swatches.
    pub fn swatch_color(&self) -> Color {
        if let Some(galaxy) = &self.finish.galaxy {
            if let Some(c) = galaxy.palette.first() {
                return Color::srgb(c[0], c[1], c[2]);
            }
        }
        if let Some(emissive) = &self.finish.emissive {
            let [r, g, b, _] = emissive.color;
            return Color::srgb(r, g, b);
        }
        let [r, g, b, _] = self.finish.base_color;
        Color::srgb(r, g, b)
    }

    /// Build the static (StandardMaterial) core. Textures are loaded through
    /// the asset server; animation/wear live in the shader extension.
    pub fn to_material(&self, asset_server: &AssetServer) -> StandardMaterial {
        let f = &self.finish;
        let [r, g, b, a] = f.base_color;
        let emissive = f
            .emissive
            .as_ref()
            .map(|e| {
                let [er, eg, eb, _] = e.color;
                LinearRgba::rgb(er * e.strength, eg * e.strength, eb * e.strength)
            })
            .unwrap_or_default();
        StandardMaterial {
            base_color: Color::srgba(r, g, b, a),
            metallic: f.metallic,
            perceptual_roughness: f.roughness,
            emissive,
            base_color_texture: f.texture.as_ref().map(|p| asset_server.load(p)),
            normal_map_texture: f.normal_map.as_ref().map(|p| asset_server.load(p)),
            ..default()
        }
    }
}

/// One owned/equipped copy of a skin: which skin plus the per-copy wear and
/// pattern seed that make duplicates distinct.
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
pub struct SkinInstance {
    pub skin: SkinId,
    #[serde(default)]
    pub wear: f32,
    #[serde(default)]
    pub seed: u32,
}

impl SkinInstance {
    pub fn factory(skin: SkinId) -> Self {
        Self {
            skin,
            wear: 0.0,
            seed: 0,
        }
    }

    pub fn with_wear(mut self, wear: f32) -> Self {
        self.wear = wear.clamp(0.0, 1.0);
        self
    }

    pub fn with_seed(mut self, seed: u32) -> Self {
        self.seed = seed;
        self
    }

    /// Roll a fresh drop. Wear is biased toward factory-new, but only
    /// gently: `roll^1.5` lands near a 0.40 mean instead of the old 0.33,
    /// so Battle-Scarred copies actually show up. Seeds are fully random so
    /// patterned duplicates differ.
    pub fn roll(skin: SkinId) -> Self {
        use rand::Rng;
        let mut rng = rand::rng();
        let roll: f32 = rng.random_range(0.0..1.0);
        Self {
            skin,
            wear: roll.powf(1.5).clamp(0.0, 1.0),
            seed: rng.random::<u32>(),
        }
    }

    pub fn is_default(&self) -> bool {
        self.skin.is_default()
    }

    /// Human-readable wear band, CSGO-style.
    pub fn wear_name(&self) -> &'static str {
        match self.wear {
            w if w < 0.07 => "Factory New",
            w if w < 0.15 => "Minimal Wear",
            w if w < 0.38 => "Field-Tested",
            w if w < 0.45 => "Well-Worn",
            _ => "Battle-Scarred",
        }
    }
}

impl Default for SkinInstance {
    fn default() -> Self {
        Self::factory(SkinId::default_id())
    }
}

// ── Registry ─────────────────────────────────────────────────────────────

#[derive(Resource, Default)]
pub struct SkinRegistry {
    pub skins: HashMap<SkinId, SkinDef>,
    /// weapon id -> skins bound to it (excludes universal ones).
    pub by_weapon: HashMap<String, Vec<SkinId>>,
    /// Skins that fit any weapon.
    pub universal: Vec<SkinId>,
}

impl SkinRegistry {
    /// Load every `.json` in [`SKIN_DIR`], plus the built-in default skin.
    /// Missing directory is not an error (tests, headless tooling).
    pub fn load() -> Self {
        Self::load_from(SKIN_DIR)
    }

    pub fn load_from(dir: &str) -> Self {
        let mut registry = Self::default();
        registry.skins.insert(
            SkinId::default_id(),
            SkinDef {
                id: SkinId::default_id(),
                name: "Default".to_string(),
                rarity: Rarity::Common,
                weapons: Vec::new(),
                finish: Finish::default(),
            },
        );

        match std::fs::read_dir(dir) {
            Ok(entries) => {
                for entry in entries.flatten() {
                    let path = entry.path();
                    if path.extension().and_then(|e| e.to_str()) != Some("json") {
                        continue;
                    }
                    let text = match std::fs::read_to_string(&path) {
                        Ok(t) => t,
                        Err(e) => {
                            warn!("skin {} unreadable: {e}", path.display());
                            continue;
                        }
                    };
                    match serde_json::from_str::<SkinDef>(&text) {
                        Ok(def) => {
                            registry.index(def);
                        }
                        Err(e) => warn!("skin {} failed to parse: {e}", path.display()),
                    }
                }
            }
            Err(_) => {
                debug!("no skin directory at {dir}; using built-in skins only");
            }
        }
        registry
    }

    fn index(&mut self, def: SkinDef) {
        let id = def.id.clone();
        if def.is_universal() {
            if !self.universal.contains(&id) {
                self.universal.push(id.clone());
            }
        } else {
            for weapon in &def.weapons {
                let list = self.by_weapon.entry(weapon.clone()).or_default();
                if !list.contains(&id) {
                    list.push(id.clone());
                }
            }
        }
        self.skins.insert(id, def);
    }

    pub fn get(&self, id: &SkinId) -> Option<&SkinDef> {
        self.skins.get(id)
    }

    pub fn def_or_default(&self, id: &SkinId) -> &SkinDef {
        self.skins
            .get(id)
            .or_else(|| self.skins.get(&SkinId::default_id()))
            .expect("default skin is always registered")
    }

    pub fn rarity_of(&self, id: &SkinId) -> Rarity {
        self.get(id).map(|d| d.rarity).unwrap_or(Rarity::Common)
    }

    pub fn display_name(&self, id: &SkinId) -> String {
        self.def_or_default(id).name.clone()
    }

    pub fn swatch_color(&self, id: &SkinId) -> Color {
        self.def_or_default(id).swatch_color()
    }

    /// Skins usable on `weapon_id`, sorted by rarity then name.
    pub fn for_weapon(&self, weapon_id: &str) -> Vec<SkinId> {
        let mut ids: Vec<SkinId> = self.universal.clone();
        if let Some(specific) = self.by_weapon.get(weapon_id) {
            ids.extend(specific.iter().cloned());
        }
        ids.sort_by(|a, b| {
            let ra = self.rarity_of(a);
            let rb = self.rarity_of(b);
            ra.cmp(&rb).then_with(|| {
                let na = self.get(a).map(|d| &d.name);
                let nb = self.get(b).map(|d| &d.name);
                na.cmp(&nb)
            })
        });
        ids
    }

    /// All droppable skins (everything except the built-in default).
    pub fn droppable(&self) -> Vec<SkinId> {
        let mut ids: Vec<SkinId> = self
            .skins
            .keys()
            .filter(|id| !id.is_default())
            .cloned()
            .collect();
        ids.sort_by(|a, b| self.rarity_of(a).cmp(&self.rarity_of(b)));
        ids
    }

    /// Skins of one rarity, droppable.
    pub fn droppable_of_rarity(&self, rarity: Rarity) -> Vec<SkinId> {
        self.droppable()
            .into_iter()
            .filter(|id| self.rarity_of(id) == rarity)
            .collect()
    }

    /// Build the full render material for one owned copy.
    pub fn material_for(
        &self,
        instance: &SkinInstance,
        asset_server: &AssetServer,
    ) -> WeaponMaterial {
        let def = self.def_or_default(&instance.skin);
        let mut extension = def.finish.to_extension(instance);
        if let Some(pattern) = &def.finish.pattern {
            extension.pattern_atlas = Some(asset_server.load(&pattern.sheet));
        }
        WeaponMaterial {
            base: def.to_material(asset_server),
            extension,
        }
    }

    /// Whether the given instance's finish needs per-frame uniforms.
    pub fn is_animated(&self, instance: &SkinInstance) -> bool {
        self.get(&instance.skin)
            .map(|d| d.finish.is_animated())
            .unwrap_or(false)
    }
}

// ── Inventory ────────────────────────────────────────────────────────────

pub const INVENTORY_VERSION: u32 = 2;

/// An owned thing. Flat and serializable so it maps 1:1 to a future server
/// row (`kind` discriminator + columns).
#[derive(Debug, Clone, PartialEq, Serialize, Deserialize)]
#[serde(tag = "kind", rename_all = "snake_case")]
pub enum ItemKind {
    GunSkin {
        weapon_id: String,
        skin: SkinId,
        #[serde(default)]
        wear: f32,
        #[serde(default)]
        seed: u32,
    },
    MeleeUnlock {
        weapon_id: String,
    },
}

impl ItemKind {
    pub fn weapon_id(&self) -> &str {
        match self {
            ItemKind::GunSkin { weapon_id, .. } => weapon_id,
            ItemKind::MeleeUnlock { weapon_id } => weapon_id,
        }
    }
}

/// Everything the player owns. Stored at `savestate.json` under `inventory`.
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct Inventory {
    #[serde(default = "inventory_version")]
    pub version: u32,
    #[serde(default)]
    pub items: Vec<ItemKind>,
}

fn inventory_version() -> u32 {
    INVENTORY_VERSION
}

impl Default for Inventory {
    fn default() -> Self {
        Self {
            version: INVENTORY_VERSION,
            items: Vec::new(),
        }
    }
}

impl Inventory {
    pub fn save(&self) {
        crate::storage::save_section("savestate.json", "inventory", self);
    }

    /// Load v2, migrating the v1 `{ owned: { weapon: { SkinName: count } } }`
    /// shape when found. Unknown/removed skins are dropped.
    pub fn load() -> Self {
        let raw: serde_json::Value = crate::storage::load_section("savestate.json", "inventory");
        if raw.is_null() {
            return Self::default();
        }
        if raw.get("version").is_some() {
            return serde_json::from_value(raw).unwrap_or_default();
        }
        if let Some(owned) = raw.get("owned").and_then(|v| v.as_object()) {
            return Self::migrate_v1(owned);
        }
        Self::default()
    }

    /// Drop gun skins whose id no longer exists in the registry (renamed or
    /// removed finishes). Returns how many items were dropped.
    pub fn prune_unknown(&mut self, registry: &SkinRegistry) -> u32 {
        let before = self.items.len();
        self.items.retain(|item| match item {
            ItemKind::GunSkin { skin, .. } => registry.get(skin).is_some(),
            ItemKind::MeleeUnlock { .. } => true,
        });
        (before - self.items.len()) as u32
    }

    fn migrate_v1(owned: &serde_json::Map<String, serde_json::Value>) -> Self {
        let mut inv = Self::default();
        for (weapon_id, skins) in owned {
            let Some(skins) = skins.as_object() else {
                continue;
            };
            for (legacy_name, count) in skins {
                let Some(skin) = legacy_skin_id(legacy_name) else {
                    continue;
                };
                let count = count.as_u64().unwrap_or(0).min(10_000) as u32;
                for _ in 0..count {
                    inv.add_gun_skin(weapon_id, skin.clone(), 0.0, 0);
                }
            }
        }
        info!(
            "Migrated v1 skin inventory: {} items ({} weapons)",
            inv.items.len(),
            owned.len()
        );
        inv
    }

    pub fn add_gun_skin(&mut self, weapon_id: &str, skin: SkinId, wear: f32, seed: u32) {
        self.items.push(ItemKind::GunSkin {
            weapon_id: weapon_id.to_string(),
            skin,
            wear: wear.clamp(0.0, 1.0),
            seed,
        });
    }

    /// Returns false when the melee mesh was already unlocked.
    pub fn add_melee_unlock(&mut self, weapon_id: &str) -> bool {
        if self.has_melee(weapon_id) {
            return false;
        }
        self.items.push(ItemKind::MeleeUnlock {
            weapon_id: weapon_id.to_string(),
        });
        true
    }

    pub fn has_melee(&self, weapon_id: &str) -> bool {
        self.items.iter().any(|item| {
            matches!(item, ItemKind::MeleeUnlock { weapon_id: w } if w == weapon_id)
        })
    }

    pub fn owned_melee(&self) -> Vec<String> {
        self.items
            .iter()
            .filter_map(|item| match item {
                ItemKind::MeleeUnlock { weapon_id } => Some(weapon_id.clone()),
                _ => None,
            })
            .collect()
    }

    /// One row per unique (weapon, skin) with its copy count, sorted by
    /// weapon. Used by the cosmetics browser.
    pub fn listing(&self) -> Vec<(String, SkinId, u32)> {
        let mut counts: HashMap<(String, SkinId), u32> = HashMap::new();
        for item in &self.items {
            if let ItemKind::GunSkin {
                weapon_id, skin, ..
            } = item
            {
                *counts.entry((weapon_id.clone(), skin.clone())).or_insert(0) += 1;
            }
        }
        let mut rows: Vec<(String, SkinId, u32)> = counts
            .into_iter()
            .map(|((weapon_id, skin), count)| (weapon_id, skin, count))
            .collect();
        rows.sort_by(|a, b| a.0.cmp(&b.0));
        rows
    }

    pub fn skins_for(&self, weapon_id: &str) -> Vec<SkinId> {
        let mut out: Vec<SkinId> = Vec::new();
        for item in &self.items {
            if let ItemKind::GunSkin {
                weapon_id: w,
                skin,
                ..
            } = item
            {
                if w == weapon_id && !out.contains(skin) {
                    out.push(skin.clone());
                }
            }
        }
        out
    }

    /// Owned copies of one skin on one weapon (wear/seed included).
    pub fn copies_of(&self, weapon_id: &str, skin: &SkinId) -> Vec<SkinInstance> {
        self.items
            .iter()
            .filter_map(|item| match item {
                ItemKind::GunSkin {
                    weapon_id: w,
                    skin: s,
                    wear,
                    seed,
                } if w == weapon_id && s == skin => Some(SkinInstance {
                    skin: s.clone(),
                    wear: *wear,
                    seed: *seed,
                }),
                _ => None,
            })
            .collect()
    }

    pub fn skin_count(&self, weapon_id: &str, skin: &SkinId) -> u32 {
        self.items
            .iter()
            .filter(|item| {
                matches!(item, ItemKind::GunSkin { weapon_id: w, skin: s, .. } if w == weapon_id && s == skin)
            })
            .count() as u32
    }

    pub fn duplicate_count(&self, weapon_id: &str, skin: &SkinId) -> u32 {
        self.skin_count(weapon_id, skin).saturating_sub(1)
    }

    /// Total duplicates (copies beyond the first of each skin).
    pub fn total_duplicates(&self) -> u32 {
        let mut counts: HashMap<(&str, &SkinId), u32> = HashMap::new();
        for item in &self.items {
            if let ItemKind::GunSkin {
                weapon_id, skin, ..
            } = item
            {
                *counts.entry((weapon_id.as_str(), skin)).or_insert(0) += 1;
            }
        }
        counts.values().map(|c| c.saturating_sub(1)).sum()
    }

    /// Remove one copy of a skin (used by the cosmetics sell dialog).
    pub fn sell_skin(&mut self, weapon_id: &str, skin: &SkinId) -> bool {
        if let Some(pos) = self.items.iter().position(|item| {
            matches!(item, ItemKind::GunSkin { weapon_id: w, skin: s, .. } if w == weapon_id && s == skin)
        }) {
            self.items.remove(pos);
            true
        } else {
            false
        }
    }

    /// Sell one copy but keep at least one (used by "sell duplicates").
    pub fn sell_duplicate(&mut self, weapon_id: &str, skin: &SkinId) -> bool {
        if self.skin_count(weapon_id, skin) > 1 {
            self.sell_skin(weapon_id, skin)
        } else {
            false
        }
    }

    /// Sell every duplicate, keeping one of each. Returns
    /// `(weapon, skin, sold_count)`; callers price the sales through the
    /// registry (sell value depends on rarity).
    pub fn sell_all_duplicates(&mut self) -> Vec<(String, SkinId, u32)> {
        let mut total: HashMap<(String, SkinId), u32> = HashMap::new();
        for item in &self.items {
            if let ItemKind::GunSkin {
                weapon_id, skin, ..
            } = item
            {
                *total.entry((weapon_id.clone(), skin.clone())).or_insert(0) += 1;
            }
        }

        // Keep the first copy of each (weapon, skin); drop the rest.
        let mut kept: HashMap<(String, SkinId), u32> = HashMap::new();
        let mut items = Vec::with_capacity(self.items.len());
        for item in self.items.drain(..) {
            if let ItemKind::GunSkin {
                weapon_id, skin, ..
            } = &item
            {
                let n = kept
                    .entry((weapon_id.clone(), skin.clone()))
                    .or_insert(0);
                if *n >= 1 {
                    *n += 1;
                    continue;
                }
                *n += 1;
            }
            items.push(item);
        }
        self.items = items;

        total
            .into_iter()
            .filter_map(|((weapon_id, skin), count)| {
                (count > 1).then_some((weapon_id, skin, count - 1))
            })
            .collect()
    }
}

/// Map old enum variant names (`SolidRed`) to new skin ids (`solid_red`).
pub fn legacy_skin_id(name: &str) -> Option<SkinId> {
    let snake = camel_to_snake(name);
    match snake.as_str() {
        "default" => Some(SkinId::default_id()),
        "solid_red" | "solid_blue" | "solid_green" | "solid_black" | "solid_white"
        | "camo_woodland" | "camo_desert" | "carbon_fiber" | "tiger_stripe"
        | "solid_gold" | "arctic_white" | "neon_pink" | "dragon_scale"
        | "chrome_forge" | "void_walker" | "supernova" => Some(SkinId::new(snake)),
        _ => None,
    }
}

// ── Colour rules ─────────────────────────────────────────────────────────

/// Every skin colour must use "double nibble" channels: each 8-bit channel
/// is a multiple of 0x11 (`0x00, 0x11, 0x22, … 0xFF`). That is 16 levels per
/// channel — 4096 colours — which removes near-duplicate shades and makes
/// extremes like pure white realistically rollable.
#[allow(dead_code)] // enforced by `shipped_skins_use_valid_colours`
pub fn is_valid_skin_color(color: [f32; 4]) -> bool {
    color.iter().take(3).all(|c| color_byte(*c) % 17 == 0)
}

pub fn color_byte(value: f32) -> u8 {
    (value.clamp(0.0, 1.0) * 255.0).round() as u8
}

/// `#A1F` — compact form of a double-nibble colour.
pub fn color_hex_compact(color: [f32; 4]) -> String {
    format!(
        "#{:X}{:X}{:X}",
        color_byte(color[0]) / 17,
        color_byte(color[1]) / 17,
        color_byte(color[2]) / 17
    )
}

/// `#AA11FF` — six-digit form.
#[allow(dead_code)] // UI shows the compact form; kept for editors/debug
pub fn color_hex(color: [f32; 4]) -> String {
    format!(
        "#{:02X}{:02X}{:02X}",
        color_byte(color[0]),
        color_byte(color[1]),
        color_byte(color[2])
    )
}

/// Parse a compact or six-digit hex colour into linear-ish sRGB floats.
/// Returns None when a channel isn't double-nibble.
#[allow(dead_code)] // round-trip tested; used by the colour picker work
pub fn color_from_hex(hex: &str) -> Option<[f32; 4]> {
    let hex = hex.trim_start_matches('#');
    let (r, g, b) = match hex.len() {
        3 => (
            u8::from_str_radix(&hex[0..1], 16).ok()? * 17,
            u8::from_str_radix(&hex[1..2], 16).ok()? * 17,
            u8::from_str_radix(&hex[2..3], 16).ok()? * 17,
        ),
        6 => (
            u8::from_str_radix(&hex[0..2], 16).ok()?,
            u8::from_str_radix(&hex[2..4], 16).ok()?,
            u8::from_str_radix(&hex[4..6], 16).ok()?,
        ),
        _ => return None,
    };
    let color = [r as f32 / 255.0, g as f32 / 255.0, b as f32 / 255.0, 1.0];
    is_valid_skin_color(color).then_some(color)
}

fn camel_to_snake(name: &str) -> String {
    let mut out = String::with_capacity(name.len() + 4);
    for (i, c) in name.chars().enumerate() {
        if c.is_ascii_uppercase() {
            if i > 0 {
                out.push('_');
            }
            out.push(c.to_ascii_lowercase());
        } else {
            out.push(c);
        }
    }
    out
}

#[cfg(test)]
mod tests {
    use super::*;

    fn def(id: &str, rarity: Rarity, weapons: &[&str]) -> SkinDef {
        SkinDef {
            id: SkinId::new(id),
            name: id.to_string(),
            rarity,
            weapons: weapons.iter().map(|w| w.to_string()).collect(),
            finish: Finish::default(),
        }
    }

    #[test]
    fn registry_separates_universal_and_bound_skins() {
        let mut reg = SkinRegistry::default();
        reg.skins
            .insert(SkinId::default_id(), def("default", Rarity::Common, &[]));
        reg.index(def("any_red", Rarity::Common, &[]));
        reg.index(def("m4_gold", Rarity::Rare, &["colt_m4a1"]));

        assert_eq!(reg.for_weapon("colt_m4a1").len(), 2);
        assert_eq!(reg.for_weapon("g17").len(), 1);
        assert!(reg.get(&SkinId::new("m4_gold")).unwrap().fits_weapon("colt_m4a1"));
        assert!(!reg.get(&SkinId::new("m4_gold")).unwrap().fits_weapon("g17"));
    }

    #[test]
    fn droppable_excludes_default_and_filters_rarity() {
        let mut reg = SkinRegistry::default();
        reg.skins
            .insert(SkinId::default_id(), def("default", Rarity::Common, &[]));
        reg.index(def("a", Rarity::Common, &[]));
        reg.index(def("b", Rarity::Mythic, &[]));
        assert_eq!(reg.droppable().len(), 2);
        assert_eq!(reg.droppable_of_rarity(Rarity::Mythic).len(), 1);
    }

    #[test]
    fn inventory_counts_duplicates_and_sells() {
        let mut inv = Inventory::default();
        let red = SkinId::new("solid_red");
        inv.add_gun_skin("g17", red.clone(), 0.1, 0);
        inv.add_gun_skin("g17", red.clone(), 0.9, 7);
        inv.add_gun_skin("g17", SkinId::new("solid_blue"), 0.0, 0);

        assert_eq!(inv.skin_count("g17", &red), 2);
        assert_eq!(inv.duplicate_count("g17", &red), 1);
        assert_eq!(inv.total_duplicates(), 1);
        assert_eq!(inv.skins_for("g17").len(), 2);

        // Wear/seed survive per copy.
        let copies = inv.copies_of("g17", &red);
        assert_eq!(copies[1].wear, 0.9);
        assert_eq!(copies[1].seed, 7);

        assert!(inv.sell_duplicate("g17", &red));
        assert_eq!(inv.skin_count("g17", &red), 1);
        assert!(!inv.sell_duplicate("g17", &red));
        assert!(inv.sell_skin("g17", &red));
        assert_eq!(inv.skin_count("g17", &red), 0);
    }

    #[test]
    fn sell_all_duplicates_keeps_one_copy() {
        let mut inv = Inventory::default();
        let red = SkinId::new("solid_red");
        for _ in 0..4 {
            inv.add_gun_skin("g17", red.clone(), 0.0, 0);
        }
        inv.add_gun_skin("g17", SkinId::new("solid_blue"), 0.0, 0);
        let sold = inv.sell_all_duplicates();
        assert_eq!(inv.skin_count("g17", &red), 1);
        assert_eq!(inv.skin_count("g17", &SkinId::new("solid_blue")), 1);
        assert_eq!(sold.len(), 1);
        assert_eq!(sold[0].2, 3);
    }

    #[test]
    fn melee_unlocks_are_idempotent() {
        let mut inv = Inventory::default();
        assert!(inv.add_melee_unlock("m9_bayonet"));
        assert!(!inv.add_melee_unlock("m9_bayonet"));
        assert!(inv.has_melee("m9_bayonet"));
        assert_eq!(inv.owned_melee(), vec!["m9_bayonet".to_string()]);
    }

    #[test]
    fn legacy_names_map_to_new_ids() {
        assert_eq!(
            legacy_skin_id("SolidRed"),
            Some(SkinId::new("solid_red"))
        );
        assert_eq!(
            legacy_skin_id("CamoWoodland"),
            Some(SkinId::new("camo_woodland"))
        );
        assert_eq!(legacy_skin_id("Nonsense"), None);
    }

    #[test]
    fn v1_inventory_migrates_to_items() {
        let raw: serde_json::Value = serde_json::json!({
            "owned": {
                "g17": { "SolidRed": 2, "Nonsense": 5 },
                "colt_m4a1": { "ChromeForge": 1 }
            }
        });
        let owned = raw.get("owned").unwrap().as_object().unwrap();
        let inv = Inventory::migrate_v1(owned);
        assert_eq!(inv.version, INVENTORY_VERSION);
        assert_eq!(inv.skin_count("g17", &SkinId::new("solid_red")), 2);
        assert_eq!(inv.skin_count("colt_m4a1", &SkinId::new("chrome_forge")), 1);
        // Unknown legacy skins are dropped, not panicked on.
        assert_eq!(inv.items.len(), 3);
        assert!(inv.copies_of("g17", &SkinId::new("solid_red")).iter().all(|c| c.wear == 0.0));
    }

    #[test]
    fn skin_instance_wear_bands() {
        let mut inst = SkinInstance::factory(SkinId::new("solid_red"));
        inst.wear = 0.0;
        assert_eq!(inst.wear_name(), "Factory New");
        inst.wear = 0.5;
        assert_eq!(inst.wear_name(), "Battle-Scarred");
    }

    #[test]
    fn finish_features_are_independent_of_rarity() {
        let mut common = def("common_galaxy", Rarity::Common, &[]);
        common.finish.galaxy = Some(Galaxy {
            palette: default_palette(),
            density: 1.0,
            speed: 0.05,
            stars: 1.0,
            layers: 3,
            parallax: 1.0,
            star_density: 26.0,
            twinkle: 1.0,
            core: 0.35,
        });
        let mythic = def("mythic_flat", Rarity::Mythic, &[]);
        assert!(common.finish.is_animated());
        assert!(!mythic.finish.is_animated());
    }

    #[test]
    fn extension_carries_wear_seed_and_features() {
        let instance = SkinInstance {
            skin: SkinId::new("supernova"),
            wear: 0.42,
            seed: 777,
        };

        let mut galaxy = def("g", Rarity::Mythic, &[]);
        galaxy.finish.galaxy = Some(Galaxy {
            palette: vec![[1.0, 0.0, 0.0], [0.0, 1.0, 0.0], [0.0, 0.0, 1.0]],
            density: 2.0,
            speed: 0.1,
            stars: 3.0,
            layers: 4,
            parallax: 1.5,
            star_density: 32.0,
            twinkle: 0.5,
            core: 0.8,
        });
        galaxy.finish.emissive = Some(Emissive {
            color: [1.0, 1.0, 1.0, 1.0],
            strength: 2.5,
            pulse_hz: 0.75,
        });
        let ext = galaxy.finish.to_extension(&instance);
        assert!((ext.params.x - 0.42).abs() < 1e-6, "wear");
        assert_eq!(ext.params.z, 777.0, "seed");
        assert_eq!(ext.params.w, 1.0, "galaxy on");
        assert!((ext.anim.x - 0.75).abs() < 1e-6, "pulse");
        assert!((ext.anim.y - 2.5).abs() < 1e-6, "emissive strength");
        assert_eq!(ext.palette_a, Vec4::new(1.0, 0.0, 0.0, 1.0));
        assert_eq!(ext.palette_c, Vec4::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(ext.galaxy, Vec4::new(2.0, 0.1, 3.0, 1.5), "galaxy params");
        assert_eq!(
            ext.galaxy_b,
            Vec4::new(32.0, 0.5, 0.8, 4.0),
            "galaxy layer params"
        );
        assert_eq!(ext.pattern.w, 0.0, "no pattern");

        let mut patterned = def("p", Rarity::Rare, &[]);
        patterned.finish.pattern = Some(Pattern {
            sheet: "cosmetics/weapons/skins/tex/pattern_camo.png".to_string(),
            offset: None,
        });
        let ext = patterned.finish.to_extension(&instance);
        assert_eq!(ext.pattern, Vec4::new(0.0, 0.0, 1.0, 1.0));
        assert_eq!(ext.extra.y, 0.0, "seeded crop, not a fixed offset");
        assert!(!patterned.finish.is_animated());

        // A fixed offset overrides the seed.
        patterned.finish.pattern = Some(Pattern {
            sheet: "cosmetics/weapons/skins/tex/pattern_camo.png".to_string(),
            offset: Some([0.25, 0.5]),
        });
        let ext = patterned.finish.to_extension(&instance);
        assert_eq!(ext.pattern.x, 0.25);
        assert_eq!(ext.pattern.y, 0.5);
        assert_eq!(ext.extra.y, 1.0, "fixed crop origin");
    }

    #[test]
    fn gradient_extension_carries_colour_and_axis() {
        let instance = SkinInstance::factory(SkinId::new("x"));
        let mut def = def("g", Rarity::Uncommon, &[]);
        def.finish.gradient = Some(Gradient {
            color: [1.0, 0.5, 0.0, 1.0],
            axis: "y".to_string(),
        });
        let ext = def.finish.to_extension(&instance);
        assert_eq!(ext.extra.x, 1.0, "gradient on");
        assert_eq!(ext.gradient, Vec4::new(1.0, 0.5, 0.0, 1.0));
        assert!(!def.finish.is_animated());
    }

    /// Seeded pattern crops are deterministic and stay inside the sheet;
    /// an authored fixed offset wins over the seed.
    #[test]
    fn pattern_crop_origin_is_seeded_and_in_range() {
        let mut def = def("p", Rarity::Uncommon, &[]);
        def.finish.pattern = Some(Pattern {
            sheet: "tex/pattern_camo.png".to_string(),
            offset: None,
        });
        let a = def.finish.pattern_crop_origin(7);
        let b = def.finish.pattern_crop_origin(7);
        assert_eq!(a, b, "same seed, same crop");
        assert_ne!(a, def.finish.pattern_crop_origin(8), "seeds differ");
        for value in a {
            assert!((0.0..=1.0 - PATTERN_STAMP).contains(&value), "{value}");
        }

        def.finish.pattern = Some(Pattern {
            sheet: "tex/pattern_camo.png".to_string(),
            offset: Some([0.25, 0.125]),
        });
        assert_eq!(def.finish.pattern_crop_origin(7), [0.25, 0.125]);
    }

    #[test]
    fn hex_round_trips_and_rejects_non_double_nibble() {
        let color = color_from_hex("#AA11FF").expect("valid");
        assert!((color[0] - 0xAA as f32 / 255.0).abs() < 1e-6);
        assert_eq!(color_hex(color), "#AA11FF");
        assert_eq!(color_hex_compact(color), "#A1F");
        // Compact form parses back to the same colour.
        assert_eq!(color_from_hex("#A1F"), Some(color));
        assert!(color_from_hex("#123ABC").is_none());
        assert!(color_from_hex("#12345").is_none());
    }

    /// Every shipped skin must use double-nibble colours (base, emissive,
    /// gradient and galaxy palette).
    #[test]
    fn shipped_skins_use_valid_colours() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/cosmetics/weapons/skins");
        let registry = SkinRegistry::load_from(dir);
        for def in registry.skins.values() {
            assert!(
                is_valid_skin_color(def.finish.base_color),
                "{} base_color {} is not double-nibble",
                def.id,
                color_hex(def.finish.base_color)
            );
            if let Some(emissive) = &def.finish.emissive {
                assert!(
                    is_valid_skin_color(emissive.color),
                    "{} emissive {} is not double-nibble",
                    def.id,
                    color_hex(emissive.color)
                );
            }
            if let Some(gradient) = &def.finish.gradient {
                assert!(
                    is_valid_skin_color(gradient.color),
                    "{} gradient {} is not double-nibble",
                    def.id,
                    color_hex(gradient.color)
                );
            }
            if let Some(galaxy) = &def.finish.galaxy {
                for c in &galaxy.palette {
                    let color = [c[0], c[1], c[2], 1.0];
                    assert!(
                        is_valid_skin_color(color),
                        "{} galaxy palette {} is not double-nibble",
                        def.id,
                        color_hex(color)
                    );
                }
            }
        }
    }

    /// Rarity is a data label, but the shipped set follows the taxonomy:
    /// solids are common, patterns uncommon/rare, galaxy mythic.
    #[test]
    fn shipped_skins_follow_the_rarity_taxonomy() {
        let dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/cosmetics/weapons/skins");
        let registry = SkinRegistry::load_from(dir);
        for def in registry.skins.values() {
            if def.id.is_default() {
                continue;
            }
            let f = &def.finish;
            // Patterns sit in uncommon/rare unless the finish animates,
            // which pushes them up the ladder (epic+).
            if f.pattern.is_some() && !f.is_animated() {
                assert!(
                    matches!(def.rarity, Rarity::Uncommon | Rarity::Rare),
                    "{} has a static pattern but rarity {:?}",
                    def.id,
                    def.rarity
                );
            }
            if f.galaxy.is_some() {
                assert_eq!(def.rarity, Rarity::Mythic, "{} galaxy rarity", def.id);
            }
            let is_plain_solid = f.pattern.is_none()
                && f.gradient.is_none()
                && f.galaxy.is_none()
                && f.emissive.is_none()
                && f.uv_scroll.is_none()
                && f.texture.is_none();
            if is_plain_solid {
                assert_eq!(def.rarity, Rarity::Common, "{} solid rarity", def.id);
            }
        }
    }

    #[test]
    fn wear_roll_biases_toward_factory_new() {
        // Gentle bias: `roll^1.5` lands near a 0.40 mean, between the old
        // squared curve (0.33) and uniform (0.5).
        let samples: Vec<f32> = (0..2000)
            .map(|_| SkinInstance::roll(SkinId::new("solid_red")).wear)
            .collect();
        let mean = samples.iter().sum::<f32>() / samples.len() as f32;
        assert!((0.35..0.5).contains(&mean), "mean wear {mean}");
        assert!(samples.iter().all(|w| (0.0..=1.0).contains(w)));
    }
}
