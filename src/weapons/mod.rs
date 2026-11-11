use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

pub mod crates;
pub mod material;
pub mod melee;
pub mod skins;

pub use crates::{CrateDef, CrateRegistry, CrateReward};
#[allow(unused_imports)]
pub use material::{WeaponMaterial, WeaponSkinExt};
#[allow(unused_imports)]
pub use melee::{MeleeArchetypes, DEFAULT_MELEE};

#[allow(unused_imports)]
pub use skins::{
    color_hex, color_hex_compact, color_from_hex, is_valid_skin_color, Emissive, Finish, Galaxy,
    Inventory, ItemKind, Pattern, Rarity, SkinDef, SkinId, SkinInstance, SkinRegistry,
    PATTERN_SHEET_PX, PATTERN_STAMP,
};


#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum WeaponSlot {
    #[default]
    Primary,
    Secondary,
    Melee,
    Equipment,
}

impl std::fmt::Display for WeaponSlot {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        match self {
            WeaponSlot::Primary => write!(f, "Primary"),
            WeaponSlot::Secondary => write!(f, "Secondary"),
            WeaponSlot::Melee => write!(f, "Melee"),
            WeaponSlot::Equipment => write!(f, "Equipment"),
        }
    }
}

#[derive(Component)]
pub struct BaseWeaponTransform(pub Transform);

#[derive(Component, Default)]
pub struct WeaponRecoil {
    pub current_offset: Vec3,
    pub current_rotation: Vec3,
    pub target_offset: Vec3,
    pub target_rotation: Vec3,
    pub sway_offset: Vec3,
    pub sway_rotation: Vec3,
    pub sway_phase: f32,
    pub aim_offset: Vec3,
    pub switch_offset: Vec3,
    pub switch_rotation: Vec3,
    pub melee_rotation: Vec3,
    pub sprint_blend: f32,
}

/// Tag to mark a weapon entity so we can apply its skin material to all mesh descendants.
#[derive(Component)]
pub struct WeaponSkinTag {
    pub skin: SkinInstance,
    pub applied: bool,
    /// How many mesh descendants were skinned last pass. glTF scenes stream
    /// their children in over several frames, so a changed count means new
    /// meshes appeared and the material pass has to run again — otherwise
    /// only the first-loaded parts get the skin.
    pub meshes_seen: usize,
}

impl WeaponSkinTag {
    pub fn new(skin: SkinInstance) -> Self {
        Self {
            skin,
            applied: false,
            meshes_seen: 0,
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum FireMode {
    Auto,
    Semi,
    Burst(u32),
}

#[derive(Debug, Deserialize, Clone)]
pub struct WeaponInfo {
    pub name: String,
    pub description: String,
    pub manufacturer: String,
    pub year_introduced: u32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WeaponMeta {
    pub weapon_type: String,
    pub model_path: String,
    #[serde(default)]
    pub category: String,
    /// Cartridge this gun chambers (e.g. "5.56x45mm"). Drives ammo options.
    #[serde(default)]
    pub caliber: String,
    /// Gun family for attachment fit rules (ar15, ak, glock, ...).
    #[serde(default)]
    pub family: String,
    /// Melee stat archetype id (`1h_blade`, `2h_blunt`, ...). Empty for guns.
    #[serde(default)]
    pub archetype: String,
    /// Mount standards this gun accepts (stanag556, ak762, ...). Shared
    /// with a part's `interfaces`: any overlap fits, alongside the fit rule.
    #[serde(default)]
    pub accepted_interfaces: Vec<String>,
    #[serde(default)]
    pub icon_path: String,
    pub position_offset: [f32; 3],
    pub rotation_offset: [f32; 3],
    pub scale: f32,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct WeaponAttributes {
    #[serde(default)]
    pub fire_rate: f32,
    #[serde(default)]
    pub reload_speed: f32,
    #[serde(default)]
    pub accuracy: f32,
    #[serde(default)]
    pub mobility: f32,
    #[serde(default)]
    pub stability: f32,
    #[serde(default)]
    pub horizontal_recoil: f32,
    #[serde(default)]
    pub vertical_recoil: f32,
    #[serde(default)]
    pub ads_speed: f32,
    #[serde(default)]
    pub fire_modes: Vec<String>,
    #[serde(default)]
    pub attack_speed: f32,
    #[serde(default)]
    pub equip_speed: f32,
    #[serde(default)]
    pub stab_damage: f32,
    #[serde(default)]
    pub slash_damage: f32,
    #[serde(default)]
    pub back_stab_multiplier: f32,
    #[serde(default)]
    pub reach: f32,
    #[serde(default)]
    pub detonation_time: f32,
    #[serde(default)]
    pub blast_radius: f32,
    #[serde(default)]
    pub blast_damage: f32,
    #[serde(default)]
    pub weight: f32,
    #[serde(default)]
    pub amount: u32,
    #[serde(default)]
    pub special_effects: Vec<String>,

    // ── Shotgun-specific ──
    /// Number of pellets per shot (0 = single projectile weapon)
    #[serde(default)]
    pub pellet_count: u32,
    /// Spread cone angle in degrees for pellet scatter
    #[serde(default)]
    pub spread_cone: f32,
    /// Time per shell for shell-by-shell reload (0 = magazine reload)
    #[serde(default)]
    pub shell_reload_time: f32,

    // ── Melee-specific (driven by the weapon's archetype) ──
    /// Wider (lower) dot-product cone for swings.
    #[serde(default = "default_slash_cone")]
    pub slash_cone: f32,
    /// Narrower (higher) cone for thrusts.
    #[serde(default = "default_stab_cone")]
    pub stab_cone: f32,
}

fn default_slash_cone() -> f32 {
    0.2
}

fn default_stab_cone() -> f32 {
    0.8
}

#[derive(Debug, Deserialize, Clone)]
pub struct AttachmentMeta {
    #[serde(default)]
    pub model_path: String,
    #[serde(default)]
    pub mesh_path: String,
    #[serde(default)]
    pub aim_offset: Option<[f32; 3]>,
    #[serde(default)]
    pub muzzle_flash_offset: Option<[f32; 3]>,
    #[serde(default)]
    pub position_offset: Option<[f32; 3]>,
    #[serde(default)]
    pub rotation_offset: Option<[f32; 3]>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpticAttachment {
    pub name: String,
    #[serde(rename = "type")]
    pub optic_type: String,
    pub zoom_level: f32,
    pub zoom_speed: f32,
    pub sway_modifier: f32,
    pub stability_modifier: f32,
    #[serde(default)]
    pub special_effects: Vec<String>,
    pub meta: Option<AttachmentMeta>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BarrelAttachment {
    pub name: String,
    pub range_modifier: f32,
    pub accuracy_modifier: f32,
    pub horizontal_recoil_modifier: f32,
    pub vertical_recoil_modifier: f32,
    #[serde(default)]
    pub special_effects: Vec<String>,
    pub meta: Option<AttachmentMeta>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MagazineAttachment {
    pub name: String,
    pub capacity: u32,
    pub carry_capacity: u32,
    pub reload_speed_modifier: f32,
    pub meta: Option<AttachmentMeta>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AmmoAttachment {
    pub name: String,
    pub damage: f32,
    pub penetration: f32,
    pub velocity: f32,
    /// Max range in meters (catalog-filled; legacy gun JSONs omit it).
    #[serde(default)]
    pub range: f32,
    /// Gravity response multiplier, 1.0 = standard (catalog-filled).
    #[serde(default = "default_drop_factor")]
    pub drop_factor: f32,
    pub meta: Option<AttachmentMeta>,
}

fn default_drop_factor() -> f32 {
    1.0
}

#[derive(Debug, Deserialize, Clone)]
pub struct UnderbarrelAttachment {
    pub name: String,
    #[serde(default)]
    pub stability_modifier: f32,
    #[serde(default)]
    pub mobility_modifier: f32,
    #[serde(default)]
    pub horizontal_recoil_modifier: f32,
    #[serde(default)]
    pub vertical_recoil_modifier: f32,
    #[serde(default)]
    pub ads_speed_modifier: f32,
    #[serde(default)]
    pub equip_speed_modifier: f32,
    #[serde(default)]
    pub special_effects: Vec<String>,
    pub meta: Option<AttachmentMeta>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct SideAttachment {
    pub name: String,
    #[serde(default)]
    pub stability_modifier: f32,
    #[serde(default)]
    pub mobility_modifier: f32,
    #[serde(default)]
    pub special_effects: Vec<String>,
    pub meta: Option<AttachmentMeta>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct StockAttachment {
    pub name: String,
    #[serde(default)]
    pub mobility_modifier: f32,
    #[serde(default)]
    pub stability_modifier: f32,
    #[serde(default)]
    pub horizontal_recoil_modifier: f32,
    #[serde(default)]
    pub vertical_recoil_modifier: f32,
    #[serde(default)]
    pub ads_speed_modifier: f32,
    #[serde(default)]
    pub equip_speed_modifier: f32,
    #[serde(default)]
    pub special_effects: Vec<String>,
    pub meta: Option<AttachmentMeta>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct SocketDef {
    #[serde(default)]
    pub socket_type: String,
    #[serde(default)]
    pub position: [f32; 3],
    #[serde(default)]
    pub rotation: [f32; 3],
}

#[derive(Debug, Deserialize, Clone)]
pub struct MuzzleAttachment {
    pub name: String,
    #[serde(default)]
    pub range_modifier: f32,
    #[serde(default)]
    pub horizontal_recoil_modifier: f32,
    #[serde(default)]
    pub vertical_recoil_modifier: f32,
    #[serde(default)]
    pub special_effects: Vec<String>,
    pub meta: Option<AttachmentMeta>,
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct WeaponAttachments {
    pub optic: Option<OpticAttachment>,
    pub barrel: Option<BarrelAttachment>,
    pub muzzle: Option<MuzzleAttachment>,
    pub underbarrel: Option<UnderbarrelAttachment>,
    pub side: Option<SideAttachment>,
    pub magazine: Option<MagazineAttachment>,
    pub ammo: Option<AmmoAttachment>,
    pub stock: Option<StockAttachment>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WeaponConfig {
    pub info: WeaponInfo,
    pub meta: WeaponMeta,
    /// Guns carry full stats; melee configs omit them and inherit from
    /// their archetype at load time.
    #[serde(default)]
    pub attributes: WeaponAttributes,
    #[serde(default)]
    pub attachments: WeaponAttachments,
    /// Receiver socket transforms in the gun's model frame. Guns without a
    /// socket for a slot fall back to the attachment's own position_offset
    /// (legacy behavior).
    #[serde(default)]
    pub sockets: HashMap<String, SocketDef>,
    /// Curated attachment pools per slot: explicit allow-lists of registry
    /// ids. Absent/empty pool falls back to fit-rule matching. May contain
    /// "__none__" to offer explicit removal on removable slots.
    #[serde(default)]
    pub pools: HashMap<String, Vec<String>>,
}

/// Explicit-removal sentinel: equipping this means "nothing mounted".
pub const ATTACHMENT_NONE: &str = "__none__";

/// Whether an attachment id may equip on a slot: pool allow-list when the
/// gun curates one, else allowed (fit rules decide listing upstream).
pub fn pool_allows(config: &WeaponConfig, slot_key: &str, att_id: &str) -> bool {
    match config.pools.get(slot_key) {
        Some(pool) if !pool.is_empty() => pool.iter().any(|id| id == att_id),
        _ => true,
    }
}

/// Whether the slot offers explicit removal (pool lists "__none__").
pub fn slot_allows_none(config: &WeaponConfig, slot_key: &str) -> bool {
    config
        .pools
        .get(slot_key)
        .is_some_and(|pool| pool.iter().any(|id| id == ATTACHMENT_NONE))
}

pub type WeaponId = String;

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct PlayerLoadout {
    pub primary: WeaponId,
    pub secondary: WeaponId,
    pub melee: WeaponId,
    pub equipment: WeaponId,
    pub skins: HashMap<WeaponSlot, SkinInstance>,
}

impl Default for PlayerLoadout {
    fn default() -> Self {
        Self {
            primary: "colt_m4a1".to_string(),
            secondary: "g17".to_string(),
            melee: "km2000".to_string(),
            equipment: "rgd-5".to_string(),
            skins: HashMap::new(),
        }
    }
}

impl PlayerLoadout {
    pub fn get_id_for_slot(&self, slot: WeaponSlot) -> &str {
        match slot {
            WeaponSlot::Primary => &self.primary,
            WeaponSlot::Secondary => &self.secondary,
            WeaponSlot::Melee => &self.melee,
            WeaponSlot::Equipment => &self.equipment,
        }
    }

    pub fn set_id_for_slot(&mut self, slot: WeaponSlot, id: String) {
        match slot {
            WeaponSlot::Primary => self.primary = id,
            WeaponSlot::Secondary => self.secondary = id,
            WeaponSlot::Melee => self.melee = id,
            WeaponSlot::Equipment => self.equipment = id,
        }
    }

    pub fn get_skin(&self, slot: WeaponSlot) -> SkinInstance {
        self.skins.get(&slot).cloned().unwrap_or_default()
    }

    pub fn set_skin(&mut self, slot: WeaponSlot, skin: SkinInstance) {
        self.skins.insert(slot, skin);
    }

    pub fn save(&self) {
        crate::storage::save_section("savestate.json", "loadout", self);
    }

    pub fn load() -> Self {
        crate::storage::load_section("savestate.json", "loadout")
    }
}

/// Persistent credits currency. Earned by selling duplicate skins.
/// Stored at `settings/savestate.json`.
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct PlayerCredits {
    pub balance: u64,
}

impl Default for PlayerCredits {
    fn default() -> Self {
        Self { balance: 500 } // Start with 500 credits
    }
}

impl PlayerCredits {
    pub fn save(&self) {
        crate::storage::save_section("savestate.json", "credits", self);
    }

    pub fn load() -> Self {
        crate::storage::load_section("savestate.json", "credits")
    }

    /// Credit value for selling a skin of a given rarity
    pub fn sell_value(rarity: Rarity) -> u64 {
        rarity.sell_value()
    }

    /// Cost to open each crate tier
    pub fn crate_cost(crate_tier: u8) -> u64 {
        match crate_tier {
            0 => 50,   // Standard
            1 => 100,  // Tactical
            2 => 250,  // Elite
            3 => 500,  // Legendary
            _ => 50,
        }
    }
}


#[derive(Resource, Default)]
pub struct WeaponRegistry {
    pub weapons: HashMap<WeaponId, WeaponConfig>,
    pub by_slot: HashMap<WeaponSlot, Vec<WeaponId>>,
    pub by_category: HashMap<String, Vec<WeaponId>>,
    pub configs: HashMap<WeaponSlot, WeaponConfig>,
}

/// A swappable attachment part (parsed from `assets/weapons/attachments/`).
/// The gun's own JSON carries the *default* part per slot; these are the
/// alternatives the player can equip.
#[derive(Debug, Deserialize, Clone, Default)]
pub struct AttachmentDefMeta {
    #[serde(default)]
    pub mesh_path: String,
    #[serde(default)]
    pub offset: [f32; 3],
    #[serde(default)]
    pub rotation: [f32; 3],
}

#[derive(Debug, Deserialize, Clone)]
pub struct ProvidedSocket {
    #[serde(default)]
    pub position: [f32; 3],
    #[serde(default)]
    pub rotation: [f32; 3],
    #[serde(default)]
    pub heuristic: String,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AttachmentDef {
    pub id: String,
    pub name: String,
    /// Slot key this plugs into (`optic`, `barrel`, `muzzle`, ...).
    pub slot: String,
    pub socket_type: String,
    /// Fit rule: which guns this goes on. Empty = universal.
    #[serde(default)]
    pub fit: FitRule,
    /// Mount standards this part speaks (stanag556, ak762, ...). Any overlap
    /// with the gun's `accepted_interfaces` fits, alongside the fit rule.
    #[serde(default)]
    pub interfaces: Vec<String>,
    /// Model author credit. Split parts default to D_U.
    #[serde(default = "default_unknown_artist")]
    pub artist: String,
    #[serde(default)]
    pub meta: AttachmentDefMeta,
    #[serde(default)]
    pub provides_sockets: HashMap<String, ProvidedSocket>,
    #[serde(default)]
    pub modifiers: HashMap<String, f32>,
    /// Magazine capacity override (mag slots only). None = keep gun default.
    #[serde(default)]
    pub capacity: Option<u32>,
    /// Reserve ammo override (mag slots only). None = keep gun default.
    #[serde(default)]
    pub carry_capacity: Option<u32>,
    #[serde(default)]
    pub special_effects: Vec<String>,
}

#[derive(Resource, Default)]
pub struct AttachmentRegistry {
    pub by_id: HashMap<String, AttachmentDef>,
}

/// A swappable ammunition type from `assets/weapons/attachments/ammo/`.
/// Stat-only: no mesh, no socket. Listed per gun by caliber match.
/// Shotgun shells may override the gun's pellet scatter: `pellet_count`
/// (`None` = keep the gun default) and `spread_cone` in degrees.
#[derive(Debug, Deserialize, Clone)]
pub struct AmmoDef {
    pub id: String,
    pub name: String,
    pub caliber: String,
    pub damage: f32,
    pub velocity: f32,
    pub penetration: f32,
    pub range: f32,
    #[serde(default = "default_drop_factor")]
    pub drop_factor: f32,
    /// Pellets per shot override (shotguns). None = gun default.
    #[serde(default)]
    pub pellet_count: Option<u32>,
    /// Spread cone override in degrees. None = gun default.
    #[serde(default)]
    pub spread_cone: Option<f32>,
    #[serde(default)]
    pub special_effects: Vec<String>,
}

#[derive(Resource, Default)]
pub struct AmmoRegistry {
    pub by_id: HashMap<String, AmmoDef>,
}

/// Ammo options for a caliber, sorted by name. Empty caliber offers none.
pub fn compatible_ammo<'a>(caliber: &str, registry: &'a AmmoRegistry) -> Vec<&'a AmmoDef> {
    if caliber.is_empty() {
        return Vec::new();
    }
    let mut out: Vec<&AmmoDef> = registry
        .by_id
        .values()
        .filter(|a| a.caliber == caliber)
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Slot key aliases kept readable forever: the pre-rename `sidebarrel`
/// key and the legacy `other` socket both mean the `side` slot now.
/// `other` itself stays reserved (never surfaced) for later use.
pub fn canonical_slot_key(slot: &str) -> &str {
    match slot {
        "sidebarrel" | "other" => "side",
        s => s,
    }
}

/// The player's equipped attachments: gun id -> slot key -> attachment id.
/// A missing entry means the gun's built-in default. Everything is unlocked;
/// this only records choices. Persisted in `savestate.json`.
#[derive(Resource, Clone, Debug, Serialize, Deserialize, Default)]
pub struct PlayerAttachments {
    pub selected: HashMap<String, HashMap<String, String>>,
}

impl PlayerAttachments {
    pub fn get(&self, gun_id: &str, slot: &str) -> Option<&str> {
        self.selected
            .get(gun_id)
            .and_then(|slots| slots.get(canonical_slot_key(slot)))
            .map(|s| s.as_str())
    }

    pub fn set(&mut self, gun_id: String, slot: String, attachment_id: String) {
        // `other` stays reserved for later: anything still stored under it
        // (or the old `sidebarrel` key) reads as the `side` slot.
        let slot = canonical_slot_key(&slot).to_string();
        self.selected
            .entry(gun_id)
            .or_default()
            .insert(slot, attachment_id);
    }

    /// Back to the gun's built-in default.
    pub fn clear(&mut self, gun_id: &str, slot: &str) {
        if let Some(slots) = self.selected.get_mut(gun_id) {
            slots.remove(canonical_slot_key(slot));
        }
    }

    pub fn save(&self) {
        crate::storage::save_section("savestate.json", "attachments", self);
    }

    pub fn load() -> Self {
        crate::storage::load_section("savestate.json", "attachments")
    }
}

/// Effective visual override for one slot: replacement mesh plus a
/// fine-tune transform applied on top of the slot's socket transform.
#[derive(Debug, Clone, Default)]
pub struct ResolvedMesh {
    pub mesh_path: String,
    pub offset: [f32; 3],
    pub rotation: [f32; 3],
}

/// Socket keys consulted (in order) for a slot key.
pub fn socket_keys_for_slot(slot_key: &str) -> &'static [&'static str] {
    match slot_key {
        "optic" => &["optic"],
        "barrel" => &["barrel"],
        "muzzle" => &["muzzle"],
        "underbarrel" => &["underbarrel"],
        "side" => &["side", "sidebarrel", "other"],
        "magazine" => &["magazine"],
        "stock" => &["stock"],
        _ => &[],
    }
}

/// Slot label (as shown in UI / used by the spawner) for a slot key.
pub fn slot_key_to_label(slot_key: &str) -> Option<&'static str> {
    match slot_key {
        "optic" => Some("Optic"),
        "barrel" => Some("Barrel"),
        "muzzle" => Some("Muzzle"),
        "underbarrel" => Some("Underbarrel"),
        "side" => Some("Side"),
        "magazine" => Some("Magazine"),
        "stock" => Some("Stock"),
        _ => None,
    }
}

/// Canonical socket type for a slot key. Used as a fallback when a gun
/// socket omits `socket_type`, so a missing field can never silently empty
/// an options list again.
pub fn default_socket_type(slot_key: &str) -> Option<&'static str> {
    match slot_key {
        "optic" => Some("optic_rail"),
        "barrel" => Some("barrel_mount"),
        "muzzle" => Some("muzzle_thread"),
        "underbarrel" => Some("underbarrel_rail"),
        "side" => Some("side_rail"),
        "magazine" => Some("mag_well"),
        "stock" => Some("stock_mount"),
        _ => None,
    }
}

/// Effective socket type for a slot on a gun: the socket's own type when
/// set, else the slot default.
pub fn socket_type_for<'a>(config: &'a WeaponConfig, slot_key: &str) -> Option<&'a str> {
    socket_keys_for_slot(slot_key)
        .iter()
        .filter_map(|k| config.sockets.get(*k))
        .next()
        .map(|s| {
            if s.socket_type.is_empty() {
                default_socket_type(slot_key).unwrap_or("")
            } else {
                s.socket_type.as_str()
            }
        })
}

#[derive(Debug, Deserialize, Clone, Default)]
pub struct FitRule {
    /// Explicit gun ids this fits (PF-style exclusives).
    #[serde(default)]
    pub guns: Vec<String>,
    /// Gun families this fits (ar15, ak, glock, ...).
    #[serde(default)]
    pub families: Vec<String>,
    /// Weapon classes this fits (assault, smg, shotgun, ...).
    #[serde(default)]
    pub classes: Vec<String>,
}

fn default_unknown_artist() -> String {
    "Unknown".to_string()
}

impl FitRule {
    /// Empty rule = universal (fits any gun with the right socket type).
    /// A shared mount interface (part `interfaces` ∩ gun
    /// `accepted_interfaces`) also fits, alongside guns/families/classes.
    pub fn allows(
        &self,
        gun_id: &str,
        family: &str,
        class: &str,
        part_interfaces: &[String],
        gun_interfaces: &[String],
    ) -> bool {
        if !part_interfaces.is_empty()
            && !gun_interfaces.is_empty()
            && part_interfaces.iter().any(|i| gun_interfaces.contains(i))
        {
            return true;
        }
        if self.guns.is_empty()
            && self.families.is_empty()
            && self.classes.is_empty()
            && part_interfaces.is_empty()
        {
            return true;
        }
        self.guns.iter().any(|g| g == gun_id)
            || (!family.is_empty() && self.families.iter().any(|f| f == family))
            || (!class.is_empty() && self.classes.iter().any(|c| c == class))
    }
}

/// Registry alternatives that fit a gun: matching socket type AND fit
/// rule (universal, gun-exclusive, family- or class-scoped). Guns without
/// a socket for the slot offer none.
pub fn compatible_attachments<'a>(
    gun_id: &str,
    slot_key: &str,
    config: &WeaponConfig,
    registry: &'a AttachmentRegistry,
) -> Vec<&'a AttachmentDef> {
    let Some(want) = socket_type_for(config, slot_key) else {
        return Vec::new();
    };
    let family = config.meta.family.as_str();
    let class = config.meta.category.as_str();
    let gun_ifaces = config.meta.accepted_interfaces.as_slice();
    let mut out: Vec<&AttachmentDef> = registry
        .by_id
        .values()
        .filter(|a| {
            a.slot == slot_key
                && a.socket_type == want
                && a.fit.allows(gun_id, family, class, &a.interfaces, gun_ifaces)
                && pool_allows(config, slot_key, &a.id)
        })
        .collect();
    out.sort_by(|a, b| a.name.cmp(&b.name));
    out
}

/// Built-in default display name of a slot, if the gun defines one.
pub fn default_slot_name(slot_key: &str, attachments: &WeaponAttachments) -> Option<String> {
    match slot_key {
        "optic" => attachments.optic.as_ref().map(|a| a.name.clone()),
        "barrel" => attachments.barrel.as_ref().map(|a| a.name.clone()),
        "muzzle" => attachments.muzzle.as_ref().map(|a| a.name.clone()),
        "underbarrel" => attachments.underbarrel.as_ref().map(|a| a.name.clone()),
        "side" => attachments.side.as_ref().map(|a| a.name.clone()),
        "magazine" => attachments.magazine.as_ref().map(|a| a.name.clone()),
        "stock" => attachments.stock.as_ref().map(|a| a.name.clone()),
        "ammo" => attachments.ammo.as_ref().map(|a| a.name.clone()),
        _ => None,
    }
}

/// Built-in default mesh path of a slot, if the gun declares one. Used to
/// hide registry entries that merely duplicate the default.
pub fn default_slot_mesh(slot_key: &str, attachments: &WeaponAttachments) -> Option<String> {
    let meta = match slot_key {
        "optic" => attachments.optic.as_ref().and_then(|a| a.meta.as_ref()),
        "barrel" => attachments.barrel.as_ref().and_then(|a| a.meta.as_ref()),
        "muzzle" => attachments.muzzle.as_ref().and_then(|a| a.meta.as_ref()),
        "underbarrel" => attachments.underbarrel.as_ref().and_then(|a| a.meta.as_ref()),
        "side" => attachments.side.as_ref().and_then(|a| a.meta.as_ref()),
        "magazine" => attachments.magazine.as_ref().and_then(|a| a.meta.as_ref()),
        "stock" => attachments.stock.as_ref().and_then(|a| a.meta.as_ref()),
        _ => None,
    };
    attachment_mesh_path(meta).map(|s| s.to_owned())
}

/// Effective display name of a slot: the equipped alternative, else the
/// gun's built-in default name.
pub fn effective_attachment_name(
    gun_id: &str,
    slot_key: &str,
    default_name: &str,
    player: &PlayerAttachments,
    registry: &AttachmentRegistry,
) -> String {
    match player.get(gun_id, slot_key) {
        Some(ATTACHMENT_NONE) => "None".to_string(),
        Some(id) => registry
            .by_id
            .get(id)
            .map(|a| a.name.clone())
            .unwrap_or_else(|| default_name.to_string()),
        None => default_name.to_string(),
    }
}

/// Effective meshes per slot label: gun defaults, with equipped alternatives
/// (matching socket type) swapped in.
pub fn resolve_attachment_meshes(
    gun_id: &str,
    config: &WeaponConfig,
    player: &PlayerAttachments,
    registry: &AttachmentRegistry,
) -> HashMap<String, ResolvedMesh> {
    let mut out = HashMap::new();
    let slots = [
        ("optic", config.attachments.optic.as_ref().and_then(|a| attachment_mesh_path(a.meta.as_ref()))),
        ("barrel", config.attachments.barrel.as_ref().and_then(|a| attachment_mesh_path(a.meta.as_ref()))),
        ("muzzle", config.attachments.muzzle.as_ref().and_then(|a| attachment_mesh_path(a.meta.as_ref()))),
        ("underbarrel", config.attachments.underbarrel.as_ref().and_then(|a| attachment_mesh_path(a.meta.as_ref()))),
        ("side", config.attachments.side.as_ref().and_then(|a| attachment_mesh_path(a.meta.as_ref()))),
        ("magazine", config.attachments.magazine.as_ref().and_then(|a| attachment_mesh_path(a.meta.as_ref()))),
        ("stock", config.attachments.stock.as_ref().and_then(|a| attachment_mesh_path(a.meta.as_ref()))),
    ];
    for (slot_key, default_path) in slots {
        let Some(label) = slot_key_to_label(slot_key) else {
            continue;
        };
        // Explicit removal wins over everything: no mesh, no placeholder.
        if player.get(gun_id, slot_key) == Some(ATTACHMENT_NONE)
            && slot_allows_none(config, slot_key)
        {
            out.insert(label.to_string(), ResolvedMesh::default());
            continue;
        }
        // Equipped alternative wins when it exists, fits the socket, and
        // passes the curated pool (if any).
        let alt = player.get(gun_id, slot_key).and_then(|id| registry.by_id.get(id)).filter(|a| {
            a.slot == slot_key
                && socket_type_for(config, slot_key).is_some_and(|t| t == a.socket_type)
                && a.fit.allows(
                    gun_id,
                    &config.meta.family,
                    &config.meta.category,
                    &a.interfaces,
                    &config.meta.accepted_interfaces,
                )
                && pool_allows(config, slot_key, &a.id)
        });
        if let Some(a) = alt {
            if !a.meta.mesh_path.is_empty() {
                out.insert(
                    label.to_string(),
                    ResolvedMesh {
                        mesh_path: a.meta.mesh_path.clone(),
                        offset: a.meta.offset,
                        rotation: a.meta.rotation,
                    },
                );
            }
        } else if let Some(path) = default_path {
            out.insert(
                label.to_string(),
                ResolvedMesh {
                    mesh_path: path.to_owned(),
                    ..Default::default()
                },
            );
        }
    }
    out
}

pub struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponRegistry>();
        app.init_resource::<AttachmentRegistry>();
        app.init_resource::<AmmoRegistry>();
        app.insert_resource(PlayerLoadout::load());
        app.insert_resource(Inventory::load());
        app.insert_resource(PlayerCredits::load());
        app.insert_resource(PlayerAttachments::load());
        app.insert_resource(SkinMaterialCache::default());
        app.insert_resource(MeleeArchetypes::load());
        app.add_plugins(MaterialPlugin::<WeaponMaterial>::default());
        app.add_systems(
            PreStartup,
            (
                load_skin_registry,
                load_crate_registry,
                load_attachment_registry,
                load_ammo_registry,
                load_all_weapon_configs,
            )
                .chain(),
        );
        app.add_systems(Update, (apply_weapon_skins, animate_weapon_skins, warmup_weapon_material));
    }
}

/// One-shot warmup: render a subpixel quad with a `WeaponMaterial` for a few
/// frames so the skin shader pipeline is compiled at load instead of
/// hitching the first time a skinned weapon is equipped.
fn warmup_weapon_material(
    mut commands: Commands,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<WeaponMaterial>>,
    mut frames_alive: Local<u32>,
    mut warmup: Local<Option<Entity>>,
    mut done: Local<bool>,
    cameras: Query<Entity, With<Camera>>,
) {
    if *done {
        return;
    }
    if crate::weapons::material::skin_material_mode()
        != crate::weapons::material::SkinMaterialMode::Advanced
        || !crate::weapons::material::skin_warmup_enabled()
    {
        *done = true;
        return;
    }

    // Spawn once a camera exists, then despawn after a few rendered frames.
    if warmup.is_none() {
        let Some(camera) = cameras.iter().next() else {
            return;
        };
        let material = materials.add(WeaponMaterial {
            base: default_slot_material(WeaponSlot::Primary),
            extension: Default::default(),
        });
        let mesh = meshes.add(Rectangle::new(0.0002, 0.0002));
        let entity = commands
            .spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                Transform::from_xyz(0.0, 0.0, -0.1),
                SkinWarmupQuad,
            ))
            .id();
        commands.entity(camera).add_child(entity);
        *warmup = Some(entity);
        info!("Warmed up weapon skin material pipeline");
        return;
    }

    *frames_alive += 1;
    if *frames_alive >= 4 {
        if let Some(entity) = warmup.take() {
            commands.entity(entity).try_despawn();
        }
        *done = true;
    }
}

#[derive(Component)]
struct SkinWarmupQuad;

/// Cache of skin materials by (skin, wear, seed) so respawning a weapon does
/// not leak a new material each time. Materials with animated finishes are
/// tracked so `animate_weapon_skins` can advance their clock uniform.
#[derive(Resource, Default)]
pub struct SkinMaterialCache {
    map: HashMap<(SkinId, u32, u32), Handle<WeaponMaterial>>,
    animated: Vec<Handle<WeaponMaterial>>,
    plain: HashMap<SkinId, Handle<StandardMaterial>>,
}

impl SkinMaterialCache {
    pub fn get_or_create(
        &mut self,
        instance: &SkinInstance,
        registry: &SkinRegistry,
        asset_server: &AssetServer,
        materials: &mut Assets<WeaponMaterial>,
    ) -> Handle<WeaponMaterial> {
        let key = (
            instance.skin.clone(),
            (instance.wear.clamp(0.0, 1.0) * 1000.0).round() as u32,
            instance.seed,
        );
        if let Some(handle) = self.map.get(&key) {
            return handle.clone();
        }
        let handle = materials.add(registry.material_for(instance, asset_server));
        if registry.is_animated(instance) {
            self.animated.push(handle.clone());
        }
        self.map.insert(key, handle.clone());
        handle
    }

    /// Plain fallback material for a skin: base colour, texture, normal map
    /// and steady emissive only (no wear/galaxy/pattern/animation).
    pub fn plain_for(
        &mut self,
        instance: &SkinInstance,
        registry: &SkinRegistry,
        asset_server: &AssetServer,
        materials: &mut Assets<StandardMaterial>,
    ) -> Handle<StandardMaterial> {
        if let Some(handle) = self.plain.get(&instance.skin) {
            return handle.clone();
        }
        let def = registry.def_or_default(&instance.skin);
        let handle = materials.add(def.to_material(asset_server));
        self.plain.insert(instance.skin.clone(), handle.clone());
        handle
    }
}

/// Handle to a weapon-part material, whichever path built it.
pub enum SkinMatHandle {
    Advanced(Handle<WeaponMaterial>),
    Plain(Handle<StandardMaterial>),
}

impl SkinMatHandle {
    /// Set this material on an entity, removing the other material kind so
    /// the renderer never sees two `MeshMaterial3d` components at once.
    pub fn apply(&self, commands: &mut Commands, entity: Entity) {
        match self {
            SkinMatHandle::Advanced(handle) => {
                commands
                    .entity(entity)
                    .remove::<MeshMaterial3d<StandardMaterial>>()
                    .insert(MeshMaterial3d(handle.clone()));
            }
            SkinMatHandle::Plain(handle) => {
                commands
                    .entity(entity)
                    .remove::<MeshMaterial3d<WeaponMaterial>>()
                    .insert(MeshMaterial3d(handle.clone()));
            }
        }
    }
}

/// Advance the clock uniform on animated skin materials.
fn animate_weapon_skins(
    time: Res<Time>,
    mut cache: ResMut<SkinMaterialCache>,
    mut materials: ResMut<Assets<WeaponMaterial>>,
) {
    if cache.animated.is_empty() {
        return;
    }
    let now = time.elapsed_secs();
    cache.animated.retain(|handle| {
        if let Some(mut material) = materials.get_mut(handle) {
            material.extension.params.y = now;
            true
        } else {
            false
        }
    });
}

/// Load data-driven skins from `assets/cosmetics/weapons/skins/`.
fn load_skin_registry(mut commands: Commands, mut inventory: ResMut<Inventory>) {
    let registry = SkinRegistry::load();
    // A renamed or removed finish must not linger as a phantom inventory
    // card (renders default, sells for its stale rarity).
    let pruned = inventory.prune_unknown(&registry);
    if pruned > 0 {
        info!("Pruned {pruned} inventory items with unknown skins");
        inventory.save();
    }
    let mode = crate::weapons::material::skin_material_mode();
    let features = crate::weapons::material::skin_features();
    info!(
        "Loaded {} skins ({} universal, {} weapon-bound); material={:?} features(wear={} galaxy={} pattern={} gradient={} anim={}) warmup={}",
        registry.skins.len(),
        registry.universal.len(),
        registry.by_weapon.values().map(|v| v.len()).sum::<usize>(),
        mode,
        features.wear,
        features.galaxy,
        features.pattern,
        features.gradient,
        features.anim,
        crate::weapons::material::skin_warmup_enabled(),
    );
    commands.insert_resource(registry);
}

/// Load data-driven crates from `assets/cosmetics/crates/`.
fn load_crate_registry(mut commands: Commands) {
    let registry = CrateRegistry::load();
    info!("Loaded {} crates", registry.crates.len());
    commands.insert_resource(registry);
}

/// Load swappable attachment definitions from `assets/weapons/attachments/`.
/// Parse failures warn and skip (same policy as weapon configs).
fn load_attachment_registry(mut registry: ResMut<AttachmentRegistry>) {
    let attachment_files: Vec<(&str, &str)> = vec![
        ("aek-971_barrel", include_str!("../../assets/weapons/attachments/barrel/aek-971_barrel.json")),
        ("bcm_mk_12_a5_barrel", include_str!("../../assets/weapons/attachments/barrel/bcm_mk_12_a5_barrel.json")),
        ("colt_m1911_a1_barrel", include_str!("../../assets/weapons/attachments/barrel/colt_m1911_a1_barrel.json")),
        ("colt_m4a1_barrel", include_str!("../../assets/weapons/attachments/barrel/colt_m4a1_barrel.json")),
        ("colt_xm177_car-15_barrel", include_str!("../../assets/weapons/attachments/barrel/colt_xm177_car-15_barrel.json")),
        ("fn_scar_h_std_barrel", include_str!("../../assets/weapons/attachments/barrel/fn_scar_h_std_barrel.json")),
        ("fn_scar_l_std_barrel", include_str!("../../assets/weapons/attachments/barrel/fn_scar_l_std_barrel.json")),
        ("g17_barrel", include_str!("../../assets/weapons/attachments/barrel/g17_barrel.json")),
        ("hk416_barrel", include_str!("../../assets/weapons/attachments/barrel/hk416_barrel.json")),
        ("hk_mp5k_barrel", include_str!("../../assets/weapons/attachments/barrel/hk_mp5k_barrel.json")),
        ("hk_mp7_a1_barrel", include_str!("../../assets/weapons/attachments/barrel/hk_mp7_a1_barrel.json")),
        ("hk_ump_45_barrel", include_str!("../../assets/weapons/attachments/barrel/hk_ump_45_barrel.json")),
        ("ingram_mac_10_barrel", include_str!("../../assets/weapons/attachments/barrel/ingram_mac_10_barrel.json")),
        ("m60_machine_gun_barrel", include_str!("../../assets/weapons/attachments/barrel/m60_machine_gun_barrel.json")),
        ("mosin_nagant_189130_barrel", include_str!("../../assets/weapons/attachments/barrel/mosin_nagant_189130_barrel.json")),
        ("pgm_hecate_ii_barrel", include_str!("../../assets/weapons/attachments/barrel/pgm_hecate_ii_barrel.json")),
        ("remington_870_barrel", include_str!("../../assets/weapons/attachments/barrel/remington_870_barrel.json")),
        ("rpk_7.62_barrel", include_str!("../../assets/weapons/attachments/barrel/rpk_7.62_barrel.json")),
        ("toz-34_barrel", include_str!("../../assets/weapons/attachments/barrel/toz-34_barrel.json")),
        ("webley_mk_iv_barrel", include_str!("../../assets/weapons/attachments/barrel/webley_mk_iv_barrel.json")),
        ("barrett_m82_magazine", include_str!("../../assets/weapons/attachments/magazine/barrett_m82_magazine.json")),
        ("colt_m1911_a1_magazine", include_str!("../../assets/weapons/attachments/magazine/colt_m1911_a1_magazine.json")),
        ("fn_p90_magazine", include_str!("../../assets/weapons/attachments/magazine/fn_p90_magazine.json")),
        ("fn_scar_h_std_magazine", include_str!("../../assets/weapons/attachments/magazine/fn_scar_h_std_magazine.json")),
        ("g17_magazine", include_str!("../../assets/weapons/attachments/magazine/g17_magazine.json")),
        ("hk_mp7_a1_magazine", include_str!("../../assets/weapons/attachments/magazine/hk_mp7_a1_magazine.json")),
        ("hk_ump_45_magazine", include_str!("../../assets/weapons/attachments/magazine/hk_ump_45_magazine.json")),
        ("imi_uzi_magazine", include_str!("../../assets/weapons/attachments/magazine/imi_uzi_magazine.json")),
        ("ingram_mac_10_magazine", include_str!("../../assets/weapons/attachments/magazine/ingram_mac_10_magazine.json")),
        ("intratec_tec-9_magazine", include_str!("../../assets/weapons/attachments/magazine/intratec_tec-9_magazine.json")),
        ("m1_garand_magazine", include_str!("../../assets/weapons/attachments/magazine/m1_garand_magazine.json")),
        ("m60_machine_gun_magazine", include_str!("../../assets/weapons/attachments/magazine/m60_machine_gun_magazine.json")),
        ("mosin_nagant_189130_magazine", include_str!("../../assets/weapons/attachments/magazine/mosin_nagant_189130_magazine.json")),
        ("pgm_hecate_ii_magazine", include_str!("../../assets/weapons/attachments/magazine/pgm_hecate_ii_magazine.json")),
        ("ppsh-41_magazine", include_str!("../../assets/weapons/attachments/magazine/ppsh-41_magazine.json")),
        ("remington_700_magazine", include_str!("../../assets/weapons/attachments/magazine/remington_700_magazine.json")),
        ("rpd-44_magazine", include_str!("../../assets/weapons/attachments/magazine/rpd-44_magazine.json")),
        ("rpk_7.62_magazine", include_str!("../../assets/weapons/attachments/magazine/rpk_7.62_magazine.json")),
        ("saiga-12_ex._30_magazine", include_str!("../../assets/weapons/attachments/magazine/saiga-12_ex._30_magazine.json")),
        ("sig_sg-553_r_magazine", include_str!("../../assets/weapons/attachments/magazine/sig_sg-553_r_magazine.json")),
        ("tompson_magazine", include_str!("../../assets/weapons/attachments/magazine/tompson_magazine.json")),
        ("vss_vintorez_magazine", include_str!("../../assets/weapons/attachments/magazine/vss_vintorez_magazine.json")),
        ("zbroyar_z-008_magazine", include_str!("../../assets/weapons/attachments/magazine/zbroyar_z-008_magazine.json")),
        ("aek-971_muzzle", include_str!("../../assets/weapons/attachments/muzzle/aek-971_muzzle.json")),
        ("ak-12_muzzle", include_str!("../../assets/weapons/attachments/muzzle/ak-12_muzzle.json")),
        ("ak-47_muzzle", include_str!("../../assets/weapons/attachments/muzzle/ak-47_muzzle.json")),
        ("bcm_mk_12_a5_muzzle", include_str!("../../assets/weapons/attachments/muzzle/bcm_mk_12_a5_muzzle.json")),
        ("colt_m4a1_muzzle", include_str!("../../assets/weapons/attachments/muzzle/colt_m4a1_muzzle.json")),
        ("colt_xm177_car-15_muzzle", include_str!("../../assets/weapons/attachments/muzzle/colt_xm177_car-15_muzzle.json")),
        ("fn_p90_muzzle", include_str!("../../assets/weapons/attachments/muzzle/fn_p90_muzzle.json")),
        ("fn_scar_h_std_muzzle", include_str!("../../assets/weapons/attachments/muzzle/fn_scar_h_std_muzzle.json")),
        ("fn_scar_l_std_muzzle", include_str!("../../assets/weapons/attachments/muzzle/fn_scar_l_std_muzzle.json")),
        ("franchi_spas_12_muzzle", include_str!("../../assets/weapons/attachments/muzzle/franchi_spas_12_muzzle.json")),
        ("g17_muzzle", include_str!("../../assets/weapons/attachments/muzzle/g17_muzzle.json")),
        ("hk416_muzzle", include_str!("../../assets/weapons/attachments/muzzle/hk416_muzzle.json")),
        ("hk_mp7_a1_muzzle", include_str!("../../assets/weapons/attachments/muzzle/hk_mp7_a1_muzzle.json")),
        ("imi_uzi_muzzle", include_str!("../../assets/weapons/attachments/muzzle/imi_uzi_muzzle.json")),
        ("ingram_mac_10_muzzle", include_str!("../../assets/weapons/attachments/muzzle/ingram_mac_10_muzzle.json")),
        ("m60_machine_gun_muzzle", include_str!("../../assets/weapons/attachments/muzzle/m60_machine_gun_muzzle.json")),
        ("pgm_hecate_ii_muzzle", include_str!("../../assets/weapons/attachments/muzzle/pgm_hecate_ii_muzzle.json")),
        ("rpd-44_muzzle", include_str!("../../assets/weapons/attachments/muzzle/rpd-44_muzzle.json")),
        ("rpk_7.62_muzzle", include_str!("../../assets/weapons/attachments/muzzle/rpk_7.62_muzzle.json")),
        ("saiga-12_ex._30_muzzle", include_str!("../../assets/weapons/attachments/muzzle/saiga-12_ex._30_muzzle.json")),
        ("sig_sauer_mpx_muzzle", include_str!("../../assets/weapons/attachments/muzzle/sig_sauer_mpx_muzzle.json")),
        ("sig_sg-553_r_muzzle", include_str!("../../assets/weapons/attachments/muzzle/sig_sg-553_r_muzzle.json")),
        ("zbroyar_z-008_muzzle", include_str!("../../assets/weapons/attachments/muzzle/zbroyar_z-008_muzzle.json")),
        ("ak-12_optic", include_str!("../../assets/weapons/attachments/optic/ak-12_optic.json")),
        ("barrett_m82_optic", include_str!("../../assets/weapons/attachments/optic/barrett_m82_optic.json")),
        ("bcm_mk_12_a5_optic", include_str!("../../assets/weapons/attachments/optic/bcm_mk_12_a5_optic.json")),
        ("colt_m4a1_optic", include_str!("../../assets/weapons/attachments/optic/colt_m4a1_optic.json")),
        ("fn_scar_h_std_optic", include_str!("../../assets/weapons/attachments/optic/fn_scar_h_std_optic.json")),
        ("fn_scar_l_std_optic", include_str!("../../assets/weapons/attachments/optic/fn_scar_l_std_optic.json")),
        ("hk416_optic", include_str!("../../assets/weapons/attachments/optic/hk416_optic.json")),
        ("hk_mp7_a1_optic", include_str!("../../assets/weapons/attachments/optic/hk_mp7_a1_optic.json")),
        ("pgm_hecate_ii_optic", include_str!("../../assets/weapons/attachments/optic/pgm_hecate_ii_optic.json")),
        ("rpk_7.62_optic", include_str!("../../assets/weapons/attachments/optic/rpk_7.62_optic.json")),
        ("sig_sauer_mpx_optic", include_str!("../../assets/weapons/attachments/optic/sig_sauer_mpx_optic.json")),
        ("sig_sg-553_r_optic", include_str!("../../assets/weapons/attachments/optic/sig_sg-553_r_optic.json")),
        ("vortex_xm157", include_str!("../../assets/weapons/attachments/optic/vortex_xm157.json")),
        ("mag_556_stanag_30", include_str!("../../assets/weapons/attachments/magazine/mag_556_stanag_30.json")),
        ("mag_556_curved_30", include_str!("../../assets/weapons/attachments/magazine/mag_556_curved_30.json")),
        ("mag_556_stanag_20", include_str!("../../assets/weapons/attachments/magazine/mag_556_stanag_20.json")),
        ("mag_762ak_30", include_str!("../../assets/weapons/attachments/magazine/mag_762ak_30.json")),
        ("mag_545_30", include_str!("../../assets/weapons/attachments/magazine/mag_545_30.json")),
        ("mag_9mm_30", include_str!("../../assets/weapons/attachments/magazine/mag_9mm_30.json")),
        ("adar_2-15_barrel", include_str!("../../assets/weapons/attachments/barrel/adar_2-15_barrel.json")),
        ("adar_2-15_magazine", include_str!("../../assets/weapons/attachments/magazine/adar_2-15_magazine.json")),
        ("adar_2-15_muzzle", include_str!("../../assets/weapons/attachments/muzzle/adar_2-15_muzzle.json")),
        ("adar_2-15_optic", include_str!("../../assets/weapons/attachments/optic/adar_2-15_optic.json")),
        ("adar_2-15_stock", include_str!("../../assets/weapons/attachments/stock/adar_2-15_stock.json")),
        ("ak-15_k_barrel", include_str!("../../assets/weapons/attachments/barrel/ak-15_k_barrel.json")),
        ("ak-15_k_stock", include_str!("../../assets/weapons/attachments/stock/ak-15_k_stock.json")),
        ("car-15_barrel", include_str!("../../assets/weapons/attachments/barrel/car-15_barrel.json")),
        ("car-15_muzzle", include_str!("../../assets/weapons/attachments/muzzle/car-15_muzzle.json")),
        ("car-15_stock", include_str!("../../assets/weapons/attachments/stock/car-15_stock.json")),
        ("colt_m5_scw_barrel", include_str!("../../assets/weapons/attachments/barrel/colt_m5_scw_barrel.json")),
        ("colt_m5_scw_muzzle", include_str!("../../assets/weapons/attachments/muzzle/colt_m5_scw_muzzle.json")),
        ("colt_m5_scw_optic", include_str!("../../assets/weapons/attachments/optic/colt_m5_scw_optic.json")),
        ("colt_m5_scw_stock", include_str!("../../assets/weapons/attachments/stock/colt_m5_scw_stock.json")),
        ("dd_mk18_barrel", include_str!("../../assets/weapons/attachments/barrel/dd_mk18_barrel.json")),
        ("dd_mk18_magazine", include_str!("../../assets/weapons/attachments/magazine/dd_mk18_magazine.json")),
        ("dd_mk18_muzzle", include_str!("../../assets/weapons/attachments/muzzle/dd_mk18_muzzle.json")),
        ("dd_mk18_optic", include_str!("../../assets/weapons/attachments/optic/dd_mk18_optic.json")),
        ("dd_mk18_stock", include_str!("../../assets/weapons/attachments/stock/dd_mk18_stock.json")),
        ("diemaco_c7_ct_barrel", include_str!("../../assets/weapons/attachments/barrel/diemaco_c7_ct_barrel.json")),
        ("diemaco_c7_ct_muzzle", include_str!("../../assets/weapons/attachments/muzzle/diemaco_c7_ct_muzzle.json")),
        ("diemaco_c7_ct_optic", include_str!("../../assets/weapons/attachments/optic/diemaco_c7_ct_optic.json")),
        ("diemaco_c7_ct_stock", include_str!("../../assets/weapons/attachments/stock/diemaco_c7_ct_stock.json")),
        ("fn_m16a4_barrel", include_str!("../../assets/weapons/attachments/barrel/fn_m16a4_barrel.json")),
        ("fn_m16a4_muzzle", include_str!("../../assets/weapons/attachments/muzzle/fn_m16a4_muzzle.json")),
        ("fn_m16a4_optic", include_str!("../../assets/weapons/attachments/optic/fn_m16a4_optic.json")),
        ("fn_m16a4_stock", include_str!("../../assets/weapons/attachments/stock/fn_m16a4_stock.json")),
        ("g95_a1_muzzle", include_str!("../../assets/weapons/attachments/muzzle/g95_a1_muzzle.json")),
        ("g95_a1_optic", include_str!("../../assets/weapons/attachments/optic/g95_a1_optic.json")),
        ("g95_a1_stock", include_str!("../../assets/weapons/attachments/stock/g95_a1_stock.json")),
        ("sako_m23_5.56_barrel", include_str!("../../assets/weapons/attachments/barrel/sako_m23_5.56_barrel.json")),
        ("sako_m23_5.56_muzzle", include_str!("../../assets/weapons/attachments/muzzle/sako_m23_5.56_muzzle.json")),
        ("sako_m23_5.56_optic", include_str!("../../assets/weapons/attachments/optic/sako_m23_5.56_optic.json")),
        ("sako_m23_5.56_stock", include_str!("../../assets/weapons/attachments/stock/sako_m23_5.56_stock.json")),
        ("uar15_s1_barrel", include_str!("../../assets/weapons/attachments/barrel/uar15_s1_barrel.json")),
        ("uar15_s1_muzzle", include_str!("../../assets/weapons/attachments/muzzle/uar15_s1_muzzle.json")),
        ("uar15_s1_optic", include_str!("../../assets/weapons/attachments/optic/uar15_s1_optic.json")),
        ("uar15_s1_stock", include_str!("../../assets/weapons/attachments/stock/uar15_s1_stock.json")),
        ("vss_vintorez_optic", include_str!("../../assets/weapons/attachments/optic/vss_vintorez_optic.json")),
        ("zbroyar_z-008_optic", include_str!("../../assets/weapons/attachments/optic/zbroyar_z-008_optic.json")),
        ("fort-500_side", include_str!("../../assets/weapons/attachments/side/fort-500_side.json")),
        ("aek-971_stock", include_str!("../../assets/weapons/attachments/stock/aek-971_stock.json")),
        ("ak-12_stock", include_str!("../../assets/weapons/attachments/stock/ak-12_stock.json")),
        ("bcm_mk_12_a5_stock", include_str!("../../assets/weapons/attachments/stock/bcm_mk_12_a5_stock.json")),
        ("colt_m4a1_stock", include_str!("../../assets/weapons/attachments/stock/colt_m4a1_stock.json")),
        ("colt_xm177_car-15_stock", include_str!("../../assets/weapons/attachments/stock/colt_xm177_car-15_stock.json")),
        ("fn_scar_h_std_stock", include_str!("../../assets/weapons/attachments/stock/fn_scar_h_std_stock.json")),
        ("fn_scar_l_std_stock", include_str!("../../assets/weapons/attachments/stock/fn_scar_l_std_stock.json")),
        ("franchi_spas_12_stock", include_str!("../../assets/weapons/attachments/stock/franchi_spas_12_stock.json")),
        ("hk416_stock", include_str!("../../assets/weapons/attachments/stock/hk416_stock.json")),
        ("hk_mp5k_stock", include_str!("../../assets/weapons/attachments/stock/hk_mp5k_stock.json")),
        ("hk_mp7_a1_stock", include_str!("../../assets/weapons/attachments/stock/hk_mp7_a1_stock.json")),
        ("hk_ump_45_stock", include_str!("../../assets/weapons/attachments/stock/hk_ump_45_stock.json")),
        ("imi_uzi_stock", include_str!("../../assets/weapons/attachments/stock/imi_uzi_stock.json")),
        ("ingram_mac_10_stock", include_str!("../../assets/weapons/attachments/stock/ingram_mac_10_stock.json")),
        ("mosin_nagant_189130_stock", include_str!("../../assets/weapons/attachments/stock/mosin_nagant_189130_stock.json")),
        ("pgm_hecate_ii_stock", include_str!("../../assets/weapons/attachments/stock/pgm_hecate_ii_stock.json")),
        ("remington_870_stock", include_str!("../../assets/weapons/attachments/stock/remington_870_stock.json")),
        ("rpk_7.62_stock", include_str!("../../assets/weapons/attachments/stock/rpk_7.62_stock.json")),
        ("saiga-12_ex._30_stock", include_str!("../../assets/weapons/attachments/stock/saiga-12_ex._30_stock.json")),
        ("sig_sauer_mpx_stock", include_str!("../../assets/weapons/attachments/stock/sig_sauer_mpx_stock.json")),
        ("sig_sg-553_r_stock", include_str!("../../assets/weapons/attachments/stock/sig_sg-553_r_stock.json")),
        ("tompson_stock", include_str!("../../assets/weapons/attachments/stock/tompson_stock.json")),
        ("hk_mp7_a1_underbarrel", include_str!("../../assets/weapons/attachments/underbarrel/hk_mp7_a1_underbarrel.json")),
        ("m60_machine_gun_underbarrel", include_str!("../../assets/weapons/attachments/underbarrel/m60_machine_gun_underbarrel.json")),
        ("rpk_7.62_underbarrel", include_str!("../../assets/weapons/attachments/underbarrel/rpk_7.62_underbarrel.json")),
        // TODO batch 2 (mesh-deduped, canonicals shared by geometry hash)
        ("ak-104_barrel", include_str!("../../assets/weapons/attachments/barrel/ak-104_barrel.json")),
        ("ak-104_kochevnik_barrel", include_str!("../../assets/weapons/attachments/barrel/ak-104_kochevnik_barrel.json")),
        ("ak-108_barrel", include_str!("../../assets/weapons/attachments/barrel/ak-108_barrel.json")),
        ("ak-12_sk_barrel", include_str!("../../assets/weapons/attachments/barrel/ak-12_sk_barrel.json")),
        ("ak-19_barrel", include_str!("../../assets/weapons/attachments/barrel/ak-19_barrel.json")),
        ("ak-202_barrel", include_str!("../../assets/weapons/attachments/barrel/ak-202_barrel.json")),
        ("ak-74_barrel", include_str!("../../assets/weapons/attachments/barrel/ak-74_barrel.json")),
        ("ak-74m_barrel", include_str!("../../assets/weapons/attachments/barrel/ak-74m_barrel.json")),
        ("akm_barrel", include_str!("../../assets/weapons/attachments/barrel/akm_barrel.json")),
        ("akms_barrel", include_str!("../../assets/weapons/attachments/barrel/akms_barrel.json")),
        ("aks-74_barrel", include_str!("../../assets/weapons/attachments/barrel/aks-74_barrel.json")),
        ("aks-74u_barrel", include_str!("../../assets/weapons/attachments/barrel/aks-74u_barrel.json")),
        ("benelli_m4_barrel", include_str!("../../assets/weapons/attachments/barrel/benelli_m4_barrel.json")),
        ("c14_timberwolf_barrel", include_str!("../../assets/weapons/attachments/barrel/c14_timberwolf_barrel.json")),
        ("fn_arka_barrel", include_str!("../../assets/weapons/attachments/barrel/fn_arka_barrel.json")),
        ("fn_fal_barrel", include_str!("../../assets/weapons/attachments/barrel/fn_fal_barrel.json")),
        ("gepard_smg_barrel", include_str!("../../assets/weapons/attachments/barrel/gepard_smg_barrel.json")),
        ("hk_usp_9mm_barrel", include_str!("../../assets/weapons/attachments/barrel/hk_usp_9mm_barrel.json")),
        ("l85a1_barrel", include_str!("../../assets/weapons/attachments/barrel/l85a1_barrel.json")),
        ("l85a2_barrel", include_str!("../../assets/weapons/attachments/barrel/l85a2_barrel.json")),
        ("l85a3_barrel", include_str!("../../assets/weapons/attachments/barrel/l85a3_barrel.json")),
        ("m110_sass_barrel", include_str!("../../assets/weapons/attachments/barrel/m110_sass_barrel.json")),
        ("mpi-km-k_barrel", include_str!("../../assets/weapons/attachments/barrel/mpi-km-k_barrel.json")),
        ("sw_m1940_light_rifle_barrel", include_str!("../../assets/weapons/attachments/barrel/sw_m1940_light_rifle_barrel.json")),
        ("tabuk_sniper_rifle_barrel", include_str!("../../assets/weapons/attachments/barrel/tabuk_sniper_rifle_barrel.json")),
        ("type_98-3_barrel", include_str!("../../assets/weapons/attachments/barrel/type_98-3_barrel.json")),
        ("uar-10_barrel", include_str!("../../assets/weapons/attachments/barrel/uar-10_barrel.json")),
        ("vhs2_ct_barrel", include_str!("../../assets/weapons/attachments/barrel/vhs2_ct_barrel.json")),
        ("wac-47_barrel", include_str!("../../assets/weapons/attachments/barrel/wac-47_barrel.json")),
        ("zastava_m70_barrel", include_str!("../../assets/weapons/attachments/barrel/zastava_m70_barrel.json")),
        ("zastava_m70_ab2_barrel", include_str!("../../assets/weapons/attachments/barrel/zastava_m70_ab2_barrel.json")),
        ("barrel_ak200", include_str!("../../assets/weapons/attachments/barrel/barrel_ak200.json")),
        ("ak-104_kochevnik_magazine", include_str!("../../assets/weapons/attachments/magazine/ak-104_kochevnik_magazine.json")),
        ("ak-12_sk_magazine", include_str!("../../assets/weapons/attachments/magazine/ak-12_sk_magazine.json")),
        ("ak-19_magazine", include_str!("../../assets/weapons/attachments/magazine/ak-19_magazine.json")),
        ("ak-200_magazine", include_str!("../../assets/weapons/attachments/magazine/ak-200_magazine.json")),
        ("ak-74m_magazine", include_str!("../../assets/weapons/attachments/magazine/ak-74m_magazine.json")),
        ("akdas_sa-9_magazine", include_str!("../../assets/weapons/attachments/magazine/akdas_sa-9_magazine.json")),
        ("akms_magazine", include_str!("../../assets/weapons/attachments/magazine/akms_magazine.json")),
        ("aks-74_magazine", include_str!("../../assets/weapons/attachments/magazine/aks-74_magazine.json")),
        ("c14_timberwolf_magazine", include_str!("../../assets/weapons/attachments/magazine/c14_timberwolf_magazine.json")),
        ("fn_fal_magazine", include_str!("../../assets/weapons/attachments/magazine/fn_fal_magazine.json")),
        ("gepard_smg_magazine", include_str!("../../assets/weapons/attachments/magazine/gepard_smg_magazine.json")),
        ("hk_usp_9mm_magazine", include_str!("../../assets/weapons/attachments/magazine/hk_usp_9mm_magazine.json")),
        ("l85a2_magazine", include_str!("../../assets/weapons/attachments/magazine/l85a2_magazine.json")),
        ("l85a3_magazine", include_str!("../../assets/weapons/attachments/magazine/l85a3_magazine.json")),
        ("m110_sass_magazine", include_str!("../../assets/weapons/attachments/magazine/m110_sass_magazine.json")),
        ("pdr-c_magazine", include_str!("../../assets/weapons/attachments/magazine/pdr-c_magazine.json")),
        ("rpk-74_magazine", include_str!("../../assets/weapons/attachments/magazine/rpk-74_magazine.json")),
        ("sw_m1940_light_rifle_magazine", include_str!("../../assets/weapons/attachments/magazine/sw_m1940_light_rifle_magazine.json")),
        ("tabuk_sniper_rifle_magazine", include_str!("../../assets/weapons/attachments/magazine/tabuk_sniper_rifle_magazine.json")),
        ("type_98-3_magazine", include_str!("../../assets/weapons/attachments/magazine/type_98-3_magazine.json")),
        ("uar-10_magazine", include_str!("../../assets/weapons/attachments/magazine/uar-10_magazine.json")),
        ("vhs2_ct_magazine", include_str!("../../assets/weapons/attachments/magazine/vhs2_ct_magazine.json")),
        ("vp9_magazine", include_str!("../../assets/weapons/attachments/magazine/vp9_magazine.json")),
        ("wac-47_magazine", include_str!("../../assets/weapons/attachments/magazine/wac-47_magazine.json")),
        ("welrod_magazine", include_str!("../../assets/weapons/attachments/magazine/welrod_magazine.json")),
        ("zastava_m70_magazine", include_str!("../../assets/weapons/attachments/magazine/zastava_m70_magazine.json")),
        ("zastava_m70_ab2_magazine", include_str!("../../assets/weapons/attachments/magazine/zastava_m70_ab2_magazine.json")),
        ("fal_280_magazine", include_str!("../../assets/weapons/attachments/magazine/fal_280_magazine.json")),
        ("mag_545_bakelite_30", include_str!("../../assets/weapons/attachments/magazine/mag_545_bakelite_30.json")),
        ("mag_556_ak101_30", include_str!("../../assets/weapons/attachments/magazine/mag_556_ak101_30.json")),
        ("mag_762ak_poly_30", include_str!("../../assets/weapons/attachments/magazine/mag_762ak_poly_30.json")),
        ("mag_762ak_steel_30", include_str!("../../assets/weapons/attachments/magazine/mag_762ak_steel_30.json")),
        ("mag_556_steel_30", include_str!("../../assets/weapons/attachments/magazine/mag_556_steel_30.json")),
        ("ak-104_muzzle", include_str!("../../assets/weapons/attachments/muzzle/ak-104_muzzle.json")),
        ("ak-104_kochevnik_muzzle", include_str!("../../assets/weapons/attachments/muzzle/ak-104_kochevnik_muzzle.json")),
        ("ak-108_muzzle", include_str!("../../assets/weapons/attachments/muzzle/ak-108_muzzle.json")),
        ("ak-200_muzzle", include_str!("../../assets/weapons/attachments/muzzle/ak-200_muzzle.json")),
        ("ak-202_muzzle", include_str!("../../assets/weapons/attachments/muzzle/ak-202_muzzle.json")),
        ("ak-203_muzzle", include_str!("../../assets/weapons/attachments/muzzle/ak-203_muzzle.json")),
        ("ak-74_muzzle", include_str!("../../assets/weapons/attachments/muzzle/ak-74_muzzle.json")),
        ("akdas_sa-9_muzzle", include_str!("../../assets/weapons/attachments/muzzle/akdas_sa-9_muzzle.json")),
        ("aks-74u_muzzle", include_str!("../../assets/weapons/attachments/muzzle/aks-74u_muzzle.json")),
        ("c14_timberwolf_muzzle", include_str!("../../assets/weapons/attachments/muzzle/c14_timberwolf_muzzle.json")),
        ("fn_arka_muzzle", include_str!("../../assets/weapons/attachments/muzzle/fn_arka_muzzle.json")),
        ("fn_fal_muzzle", include_str!("../../assets/weapons/attachments/muzzle/fn_fal_muzzle.json")),
        ("gepard_smg_muzzle", include_str!("../../assets/weapons/attachments/muzzle/gepard_smg_muzzle.json")),
        ("ksg_410_muzzle", include_str!("../../assets/weapons/attachments/muzzle/ksg_410_muzzle.json")),
        ("m110_sass_muzzle", include_str!("../../assets/weapons/attachments/muzzle/m110_sass_muzzle.json")),
        ("pdr-c_muzzle", include_str!("../../assets/weapons/attachments/muzzle/pdr-c_muzzle.json")),
        ("tabuk_sniper_rifle_muzzle", include_str!("../../assets/weapons/attachments/muzzle/tabuk_sniper_rifle_muzzle.json")),
        ("type_98-3_muzzle", include_str!("../../assets/weapons/attachments/muzzle/type_98-3_muzzle.json")),
        ("uar-10_muzzle", include_str!("../../assets/weapons/attachments/muzzle/uar-10_muzzle.json")),
        ("vhs2_ct_muzzle", include_str!("../../assets/weapons/attachments/muzzle/vhs2_ct_muzzle.json")),
        ("vp9_muzzle", include_str!("../../assets/weapons/attachments/muzzle/vp9_muzzle.json")),
        ("wac-47_muzzle", include_str!("../../assets/weapons/attachments/muzzle/wac-47_muzzle.json")),
        ("welrod_muzzle", include_str!("../../assets/weapons/attachments/muzzle/welrod_muzzle.json")),
        ("fal_280_muzzle", include_str!("../../assets/weapons/attachments/muzzle/fal_280_muzzle.json")),
        ("muzzle_l85", include_str!("../../assets/weapons/attachments/muzzle/muzzle_l85.json")),
        ("muzzle_ak_rpk", include_str!("../../assets/weapons/attachments/muzzle/muzzle_ak_rpk.json")),
        ("muzzle_akm", include_str!("../../assets/weapons/attachments/muzzle/muzzle_akm.json")),
        ("muzzle_ak74", include_str!("../../assets/weapons/attachments/muzzle/muzzle_ak74.json")),
        ("ak-12_sk_optic", include_str!("../../assets/weapons/attachments/optic/ak-12_sk_optic.json")),
        ("akdas_sa-9_optic", include_str!("../../assets/weapons/attachments/optic/akdas_sa-9_optic.json")),
        ("c14_timberwolf_optic", include_str!("../../assets/weapons/attachments/optic/c14_timberwolf_optic.json")),
        ("fn_arka_optic", include_str!("../../assets/weapons/attachments/optic/fn_arka_optic.json")),
        ("fn_fal_optic", include_str!("../../assets/weapons/attachments/optic/fn_fal_optic.json")),
        ("ksg_410_optic", include_str!("../../assets/weapons/attachments/optic/ksg_410_optic.json")),
        ("l85a1_optic", include_str!("../../assets/weapons/attachments/optic/l85a1_optic.json")),
        ("l85a2_optic", include_str!("../../assets/weapons/attachments/optic/l85a2_optic.json")),
        ("l85a3_optic", include_str!("../../assets/weapons/attachments/optic/l85a3_optic.json")),
        ("m110_sass_optic", include_str!("../../assets/weapons/attachments/optic/m110_sass_optic.json")),
        ("tabuk_sniper_rifle_optic", include_str!("../../assets/weapons/attachments/optic/tabuk_sniper_rifle_optic.json")),
        ("uar-10_optic", include_str!("../../assets/weapons/attachments/optic/uar-10_optic.json")),
        ("wac-47_optic", include_str!("../../assets/weapons/attachments/optic/wac-47_optic.json")),
        ("ak-104_stock", include_str!("../../assets/weapons/attachments/stock/ak-104_stock.json")),
        ("ak-12_sk_stock", include_str!("../../assets/weapons/attachments/stock/ak-12_sk_stock.json")),
        ("ak-19_stock", include_str!("../../assets/weapons/attachments/stock/ak-19_stock.json")),
        ("ak-74_stock", include_str!("../../assets/weapons/attachments/stock/ak-74_stock.json")),
        ("akdas_sa-9_stock", include_str!("../../assets/weapons/attachments/stock/akdas_sa-9_stock.json")),
        ("akm_stock", include_str!("../../assets/weapons/attachments/stock/akm_stock.json")),
        ("akms_stock", include_str!("../../assets/weapons/attachments/stock/akms_stock.json")),
        ("aks-74_stock", include_str!("../../assets/weapons/attachments/stock/aks-74_stock.json")),
        ("aks-74u_stock", include_str!("../../assets/weapons/attachments/stock/aks-74u_stock.json")),
        ("benelli_m4_stock", include_str!("../../assets/weapons/attachments/stock/benelli_m4_stock.json")),
        ("c14_timberwolf_stock", include_str!("../../assets/weapons/attachments/stock/c14_timberwolf_stock.json")),
        ("fn_arka_stock", include_str!("../../assets/weapons/attachments/stock/fn_arka_stock.json")),
        ("fn_fal_stock", include_str!("../../assets/weapons/attachments/stock/fn_fal_stock.json")),
        ("gepard_smg_stock", include_str!("../../assets/weapons/attachments/stock/gepard_smg_stock.json")),
        ("ksg_410_stock", include_str!("../../assets/weapons/attachments/stock/ksg_410_stock.json")),
        ("m110_sass_stock", include_str!("../../assets/weapons/attachments/stock/m110_sass_stock.json")),
        ("mpi-km-k_stock", include_str!("../../assets/weapons/attachments/stock/mpi-km-k_stock.json")),
        ("sw_m1940_light_rifle_stock", include_str!("../../assets/weapons/attachments/stock/sw_m1940_light_rifle_stock.json")),
        ("tabuk_sniper_rifle_stock", include_str!("../../assets/weapons/attachments/stock/tabuk_sniper_rifle_stock.json")),
        ("type_98-3_stock", include_str!("../../assets/weapons/attachments/stock/type_98-3_stock.json")),
        ("uar-10_stock", include_str!("../../assets/weapons/attachments/stock/uar-10_stock.json")),
        ("vhs2_ct_stock", include_str!("../../assets/weapons/attachments/stock/vhs2_ct_stock.json")),
        ("wac-47_stock", include_str!("../../assets/weapons/attachments/stock/wac-47_stock.json")),
        ("zastava_m70_stock", include_str!("../../assets/weapons/attachments/stock/zastava_m70_stock.json")),
        ("zastava_m70_ab2_stock", include_str!("../../assets/weapons/attachments/stock/zastava_m70_ab2_stock.json")),
        ("stock_ak200_poly", include_str!("../../assets/weapons/attachments/stock/stock_ak200_poly.json")),
        ("stock_ak101_poly", include_str!("../../assets/weapons/attachments/stock/stock_ak101_poly.json")),
    ];

    for (id, json_str) in attachment_files {
        match serde_json::from_str::<AttachmentDef>(json_str) {
            Ok(def) => {
                registry.by_id.insert(id.to_string(), def);
            }
            Err(e) => {
                warn!("Failed to parse attachment '{}': {}", id, e);
            }
        }
    }

    info!("Loaded {} attachments", registry.by_id.len());
}

/// Load the ammo catalog from `assets/weapons/attachments/ammo/`.
fn load_ammo_registry(mut registry: ResMut<AmmoRegistry>) {
    let ammo_files: Vec<(&str, &str)> = vec![
        ("ammo_556nato", include_str!("../../assets/weapons/attachments/ammo/ammo_556nato.json")),
        ("ammo_556mk262", include_str!("../../assets/weapons/attachments/ammo/ammo_556mk262.json")),
        ("ammo_762x39", include_str!("../../assets/weapons/attachments/ammo/ammo_762x39.json")),
        ("ammo_762x25", include_str!("../../assets/weapons/attachments/ammo/ammo_762x25.json")),
        ("ammo_762x51", include_str!("../../assets/weapons/attachments/ammo/ammo_762x51.json")),
        ("ammo_762x54r", include_str!("../../assets/weapons/attachments/ammo/ammo_762x54r.json")),
        ("ammo_9mm", include_str!("../../assets/weapons/attachments/ammo/ammo_9mm.json")),
        ("ammo_9x39", include_str!("../../assets/weapons/attachments/ammo/ammo_9x39.json")),
        ("ammo_57x28", include_str!("../../assets/weapons/attachments/ammo/ammo_57x28.json")),
        ("ammo_12ga", include_str!("../../assets/weapons/attachments/ammo/ammo_12ga.json")),
        ("ammo_12ga_birdshot", include_str!("../../assets/weapons/attachments/ammo/ammo_12ga_birdshot.json")),
        ("ammo_12ga_slug", include_str!("../../assets/weapons/attachments/ammo/ammo_12ga_slug.json")),
        ("ammo_12ga_flechette", include_str!("../../assets/weapons/attachments/ammo/ammo_12ga_flechette.json")),
        ("ammo_12ga_magnum", include_str!("../../assets/weapons/attachments/ammo/ammo_12ga_magnum.json")),
        ("ammo_45acp", include_str!("../../assets/weapons/attachments/ammo/ammo_45acp.json")),
        ("ammo_357mag", include_str!("../../assets/weapons/attachments/ammo/ammo_357mag.json")),
        ("ammo_455w", include_str!("../../assets/weapons/attachments/ammo/ammo_455w.json")),
        ("ammo_338lm", include_str!("../../assets/weapons/attachments/ammo/ammo_338lm.json")),
        ("ammo_50bmg", include_str!("../../assets/weapons/attachments/ammo/ammo_50bmg.json")),
        ("ammo_energy", include_str!("../../assets/weapons/attachments/ammo/ammo_energy.json")),
        ("ammo_545x39", include_str!("../../assets/weapons/attachments/ammo/ammo_545x39.json")),
        ("ammo_46x30", include_str!("../../assets/weapons/attachments/ammo/ammo_46x30.json")),
        ("ammo_3006", include_str!("../../assets/weapons/attachments/ammo/ammo_3006.json")),
        ("ammo_32acp", include_str!("../../assets/weapons/attachments/ammo/ammo_32acp.json")),
        ("ammo_410", include_str!("../../assets/weapons/attachments/ammo/ammo_410.json")),
        ("ammo_410_birdshot", include_str!("../../assets/weapons/attachments/ammo/ammo_410_birdshot.json")),
        ("ammo_410_slug", include_str!("../../assets/weapons/attachments/ammo/ammo_410_slug.json")),
        ("ammo_280brit", include_str!("../../assets/weapons/attachments/ammo/ammo_280brit.json")),
    ];

    for (id, json_str) in ammo_files {
        match serde_json::from_str::<AmmoDef>(json_str) {
            Ok(def) => {
                registry.by_id.insert(id.to_string(), def);
            }
            Err(e) => {
                warn!("Failed to parse ammo '{}': {}", id, e);
            }
        }
    }

    info!("Loaded {} ammo types", registry.by_id.len());
}

fn load_all_weapon_configs(
    mut registry: ResMut<WeaponRegistry>,
    archetypes: Res<MeleeArchetypes>,
    loadout: Res<PlayerLoadout>,
    player_attachments: Res<PlayerAttachments>,
    ammo_registry: Res<AmmoRegistry>,
    attachment_registry: Res<AttachmentRegistry>,
) {
    let weapon_files: Vec<(&str, &str, WeaponSlot)> = vec![
        // Primary - Assault Rifles
        ("hk416", include_str!("../../assets/weapons/data/primary/assault/hk416.json"), WeaponSlot::Primary),
        ("ak-12", include_str!("../../assets/weapons/data/primary/assault/ak-12.json"), WeaponSlot::Primary),
        ("ak-47", include_str!("../../assets/weapons/data/primary/assault/ak-47.json"), WeaponSlot::Primary),
        ("fn_scar_h_std", include_str!("../../assets/weapons/data/primary/assault/fn_scar_h_std.json"), WeaponSlot::Primary),
        ("fn_scar_l_std", include_str!("../../assets/weapons/data/primary/assault/fn_scar_l_std.json"), WeaponSlot::Primary),
        ("sig_sg-553_r", include_str!("../../assets/weapons/data/primary/assault/sig_sg-553_r.json"), WeaponSlot::Primary),
        ("adar_2-15", include_str!("../../assets/weapons/data/primary/assault/adar_2-15.json"), WeaponSlot::Primary),
        ("ak-15_k", include_str!("../../assets/weapons/data/primary/assault/ak-15_k.json"), WeaponSlot::Primary),
        ("colt_m5_scw", include_str!("../../assets/weapons/data/primary/assault/colt_m5_scw.json"), WeaponSlot::Primary),
        ("diemaco_c7_ct", include_str!("../../assets/weapons/data/primary/assault/diemaco_c7_ct.json"), WeaponSlot::Primary),
        ("fn_m16a4", include_str!("../../assets/weapons/data/primary/assault/fn_m16a4.json"), WeaponSlot::Primary),
        ("g95_a1", include_str!("../../assets/weapons/data/primary/assault/g95_a1.json"), WeaponSlot::Primary),
        ("uar15_s1", include_str!("../../assets/weapons/data/primary/assault/uar15_s1.json"), WeaponSlot::Primary),
        // Primary - Carbines
        ("aek-971", include_str!("../../assets/weapons/data/primary/carbine/aek-971.json"), WeaponSlot::Primary),
        ("colt_m4a1", include_str!("../../assets/weapons/data/primary/carbine/colt_m4a1.json"), WeaponSlot::Primary),
        ("colt_xm177_car-15", include_str!("../../assets/weapons/data/primary/carbine/colt_xm177_car-15.json"), WeaponSlot::Primary),
        ("car-15", include_str!("../../assets/weapons/data/primary/carbine/car-15.json"), WeaponSlot::Primary),
        ("dd_mk18", include_str!("../../assets/weapons/data/primary/carbine/dd_mk18.json"), WeaponSlot::Primary),
        ("sako_m23_5.56", include_str!("../../assets/weapons/data/primary/carbine/sako_m23_5.56.json"), WeaponSlot::Primary),
        ("vss_vintorez", include_str!("../../assets/weapons/data/primary/carbine/vss_vintorez.json"), WeaponSlot::Primary),
        // Primary - DMRs
        ("bcm_mk_12_a5", include_str!("../../assets/weapons/data/primary/dmr/bcm_mk_12_a5.json"), WeaponSlot::Primary),
        // Primary - LMGs
        ("m60_machine_gun", include_str!("../../assets/weapons/data/primary/lmg/m60_machine_gun.json"), WeaponSlot::Primary),
        ("rpd-44", include_str!("../../assets/weapons/data/primary/lmg/rpd-44.json"), WeaponSlot::Primary),
        ("rpk_7.62", include_str!("../../assets/weapons/data/primary/lmg/rpk_7.62.json"), WeaponSlot::Primary),
        // Primary - PDWs
        ("fn_p90", include_str!("../../assets/weapons/data/primary/pdw/fn_p90.json"), WeaponSlot::Primary),
        ("hk_mp7_a1", include_str!("../../assets/weapons/data/primary/pdw/hk_mp7_a1.json"), WeaponSlot::Primary),
        ("imi_uzi", include_str!("../../assets/weapons/data/primary/pdw/imi_uzi.json"), WeaponSlot::Primary),
        ("sig_sauer_mpx", include_str!("../../assets/weapons/data/primary/pdw/sig_sauer_mpx.json"), WeaponSlot::Primary),
        // Primary - Rifles
        ("m1_garand", include_str!("../../assets/weapons/data/primary/rifle/m1_garand.json"), WeaponSlot::Primary),
        ("mosin_nagant_189130", include_str!("../../assets/weapons/data/primary/rifle/mosin_nagant_189130.json"), WeaponSlot::Primary),
        // Primary - Shotguns
        ("fort-500", include_str!("../../assets/weapons/data/primary/shotgun/fort-500.json"), WeaponSlot::Primary),
        ("franchi_spas_12", include_str!("../../assets/weapons/data/primary/shotgun/franchi_spas_12.json"), WeaponSlot::Primary),
        ("remington_870", include_str!("../../assets/weapons/data/primary/shotgun/remington_870.json"), WeaponSlot::Primary),
        ("saiga-12_ex._30", include_str!("../../assets/weapons/data/primary/shotgun/saiga-12_ex._30.json"), WeaponSlot::Primary),
        ("toz-34", include_str!("../../assets/weapons/data/primary/shotgun/toz-34.json"), WeaponSlot::Primary),
        // Primary - SMGs
        ("hk_mp5k", include_str!("../../assets/weapons/data/primary/smg/hk_mp5k.json"), WeaponSlot::Primary),
        ("hk_ump_45", include_str!("../../assets/weapons/data/primary/smg/hk_ump_45.json"), WeaponSlot::Primary),
        ("ppsh-41", include_str!("../../assets/weapons/data/primary/smg/ppsh-41.json"), WeaponSlot::Primary),
        ("tompson", include_str!("../../assets/weapons/data/primary/smg/tompson.json"), WeaponSlot::Primary),
        // Primary - Snipers
        ("barrett_m82", include_str!("../../assets/weapons/data/primary/sniper/barrett_m82.json"), WeaponSlot::Primary),
        ("pgm_hecate_ii", include_str!("../../assets/weapons/data/primary/sniper/pgm_hecate_ii.json"), WeaponSlot::Primary),
        ("remington_700", include_str!("../../assets/weapons/data/primary/sniper/remington_700.json"), WeaponSlot::Primary),
        ("zbroyar_z-008", include_str!("../../assets/weapons/data/primary/sniper/zbroyar_z-008.json"), WeaponSlot::Primary),
        // TODO batch 2 primaries (mesh-deduped)
        ("ak-104", include_str!("../../assets/weapons/data/primary/assault/ak-104.json"), WeaponSlot::Primary),
        ("ak-104_kochevnik", include_str!("../../assets/weapons/data/primary/assault/ak-104_kochevnik.json"), WeaponSlot::Primary),
        ("ak-108", include_str!("../../assets/weapons/data/primary/assault/ak-108.json"), WeaponSlot::Primary),
        ("ak-12_sk", include_str!("../../assets/weapons/data/primary/assault/ak-12_sk.json"), WeaponSlot::Primary),
        ("ak-19", include_str!("../../assets/weapons/data/primary/assault/ak-19.json"), WeaponSlot::Primary),
        ("ak-200", include_str!("../../assets/weapons/data/primary/assault/ak-200.json"), WeaponSlot::Primary),
        ("ak-202", include_str!("../../assets/weapons/data/primary/assault/ak-202.json"), WeaponSlot::Primary),
        ("ak-203", include_str!("../../assets/weapons/data/primary/assault/ak-203.json"), WeaponSlot::Primary),
        ("ak-74", include_str!("../../assets/weapons/data/primary/assault/ak-74.json"), WeaponSlot::Primary),
        ("ak-74m", include_str!("../../assets/weapons/data/primary/assault/ak-74m.json"), WeaponSlot::Primary),
        ("akdas_sa-9", include_str!("../../assets/weapons/data/primary/smg/akdas_sa-9.json"), WeaponSlot::Primary),
        ("akm", include_str!("../../assets/weapons/data/primary/assault/akm.json"), WeaponSlot::Primary),
        ("akms", include_str!("../../assets/weapons/data/primary/assault/akms.json"), WeaponSlot::Primary),
        ("aks-74", include_str!("../../assets/weapons/data/primary/assault/aks-74.json"), WeaponSlot::Primary),
        ("aks-74u", include_str!("../../assets/weapons/data/primary/assault/aks-74u.json"), WeaponSlot::Primary),
        ("benelli_m4", include_str!("../../assets/weapons/data/primary/shotgun/benelli_m4.json"), WeaponSlot::Primary),
        ("c14_timberwolf", include_str!("../../assets/weapons/data/primary/sniper/c14_timberwolf.json"), WeaponSlot::Primary),
        ("fal_280", include_str!("../../assets/weapons/data/primary/assault/fal_280.json"), WeaponSlot::Primary),
        ("fn_arka", include_str!("../../assets/weapons/data/primary/assault/fn_arka.json"), WeaponSlot::Primary),
        ("fn_fal", include_str!("../../assets/weapons/data/primary/assault/fn_fal.json"), WeaponSlot::Primary),
        ("gepard_smg", include_str!("../../assets/weapons/data/primary/smg/gepard_smg.json"), WeaponSlot::Primary),
        ("ksg_410", include_str!("../../assets/weapons/data/primary/shotgun/ksg_410.json"), WeaponSlot::Primary),
        ("l85a1", include_str!("../../assets/weapons/data/primary/assault/l85a1.json"), WeaponSlot::Primary),
        ("l85a2", include_str!("../../assets/weapons/data/primary/assault/l85a2.json"), WeaponSlot::Primary),
        ("l85a3", include_str!("../../assets/weapons/data/primary/assault/l85a3.json"), WeaponSlot::Primary),
        ("m110_sass", include_str!("../../assets/weapons/data/primary/dmr/m110_sass.json"), WeaponSlot::Primary),
        ("mpi-km-k", include_str!("../../assets/weapons/data/primary/assault/mpi-km-k.json"), WeaponSlot::Primary),
        ("pdr-c", include_str!("../../assets/weapons/data/primary/pdw/pdr-c.json"), WeaponSlot::Primary),
        ("rpk-74", include_str!("../../assets/weapons/data/primary/lmg/rpk-74.json"), WeaponSlot::Primary),
        ("sw_m1940_light_rifle", include_str!("../../assets/weapons/data/primary/smg/sw_m1940_light_rifle.json"), WeaponSlot::Primary),
        ("tabuk_sniper_rifle", include_str!("../../assets/weapons/data/primary/dmr/tabuk_sniper_rifle.json"), WeaponSlot::Primary),
        ("type_98-3", include_str!("../../assets/weapons/data/primary/assault/type_98-3.json"), WeaponSlot::Primary),
        ("uar-10", include_str!("../../assets/weapons/data/primary/dmr/uar-10.json"), WeaponSlot::Primary),
        ("vhs2_ct", include_str!("../../assets/weapons/data/primary/assault/vhs2_ct.json"), WeaponSlot::Primary),
        ("wac-47", include_str!("../../assets/weapons/data/primary/assault/wac-47.json"), WeaponSlot::Primary),
        ("zastava_m70", include_str!("../../assets/weapons/data/primary/assault/zastava_m70.json"), WeaponSlot::Primary),
        ("zastava_m70_ab2", include_str!("../../assets/weapons/data/primary/assault/zastava_m70_ab2.json"), WeaponSlot::Primary),
        // Secondary - Pistols
        ("g17", include_str!("../../assets/weapons/data/secondary/pistol/g17.json"), WeaponSlot::Secondary),
        ("colt_m1911_a1", include_str!("../../assets/weapons/data/secondary/pistol/colt_m1911_a1.json"), WeaponSlot::Secondary),
        ("hk_usp_9mm", include_str!("../../assets/weapons/data/secondary/pistol/hk_usp_9mm.json"), WeaponSlot::Secondary),
        ("vp9", include_str!("../../assets/weapons/data/secondary/pistol/vp9.json"), WeaponSlot::Secondary),
        ("welrod", include_str!("../../assets/weapons/data/secondary/pistol/welrod.json"), WeaponSlot::Secondary),
        // Secondary - Machine Pistols
        ("ingram_mac_10", include_str!("../../assets/weapons/data/secondary/mpistol/ingram_mac_10.json"), WeaponSlot::Secondary),
        ("intratec_tec-9", include_str!("../../assets/weapons/data/secondary/mpistol/intratec_tec-9.json"), WeaponSlot::Secondary),
        // Secondary - Other
        ("cerevo_dominator_eliminator", include_str!("../../assets/weapons/data/secondary/other/cerevo_dominator_eliminator.json"), WeaponSlot::Secondary),
        // Secondary - Revolvers
        ("colt_python", include_str!("../../assets/weapons/data/secondary/revolver/colt_python.json"), WeaponSlot::Secondary),
        ("webley_mk_iv", include_str!("../../assets/weapons/data/secondary/revolver/webley_mk_iv.json"), WeaponSlot::Secondary),
        // Melee
        ("msbs_grot_bayonet", include_str!("../../assets/weapons/data/melee/msbs_grot_bayonet.json"), WeaponSlot::Melee),
        ("axe", include_str!("../../assets/weapons/data/melee/axe.json"), WeaponSlot::Melee),
        ("6x4_bayonet", include_str!("../../assets/weapons/data/melee/6x4_bayonet.json"), WeaponSlot::Melee),
        ("bmk-405_brown_argus", include_str!("../../assets/weapons/data/melee/bmk-405_brown_argus.json"), WeaponSlot::Melee),
        ("cjrb_chord", include_str!("../../assets/weapons/data/melee/cjrb_chord.json"), WeaponSlot::Melee),
        ("extrema_ratio_mamba_sw_desert", include_str!("../../assets/weapons/data/melee/extrema_ratio_mamba_sw_desert.json"), WeaponSlot::Melee),
        ("fa-03_bayonet", include_str!("../../assets/weapons/data/melee/fa-03_bayonet.json"), WeaponSlot::Melee),
        ("km2000", include_str!("../../assets/weapons/data/melee/km2000.json"), WeaponSlot::Melee),
        ("m968_bayonet", include_str!("../../assets/weapons/data/melee/m968_bayonet.json"), WeaponSlot::Melee),
        ("m9_bayonet", include_str!("../../assets/weapons/data/melee/m9_bayonet.json"), WeaponSlot::Melee),
        ("mx-8054", include_str!("../../assets/weapons/data/melee/mx-8054.json"), WeaponSlot::Melee),
        ("sks_bayonet", include_str!("../../assets/weapons/data/melee/sks_bayonet.json"), WeaponSlot::Melee),
        // Equipment
        ("rgd-5", include_str!("../../assets/weapons/data/equipment/rgd-5.json"), WeaponSlot::Equipment),
        ("cerevo_dominator_paralyzer", include_str!("../../assets/weapons/data/equipment/cerevo_dominator_paralyzer.json"), WeaponSlot::Equipment),
    ];

    for (id, json_str, slot) in weapon_files {
        match serde_json::from_str::<WeaponConfig>(json_str) {
            Ok(config) => {
                let category = config.meta.category.clone();
                registry.weapons.insert(id.to_string(), config.clone());
                registry.by_slot.entry(slot).or_default().push(id.to_string());
                if !category.is_empty() {
                    registry.by_category.entry(category).or_default().push(id.to_string());
                }
            }
            Err(e) => {
                warn!("Failed to parse weapon config '{}': {}", id, e);
            }
        }
    }

    // Melee stats come from the four archetypes, not per-weapon JSON.
    melee::apply_archetypes(&mut registry, &archetypes);

    sync_loadout_to_configs(&mut registry, &loadout, &player_attachments, &ammo_registry, &attachment_registry);

    info!(
        "Loaded {} weapons ({} primary, {} secondary, {} melee, {} equipment)",
        registry.weapons.len(),
        registry.by_slot.get(&WeaponSlot::Primary).map(|v| v.len()).unwrap_or(0),
        registry.by_slot.get(&WeaponSlot::Secondary).map(|v| v.len()).unwrap_or(0),
        registry.by_slot.get(&WeaponSlot::Melee).map(|v| v.len()).unwrap_or(0),
        registry.by_slot.get(&WeaponSlot::Equipment).map(|v| v.len()).unwrap_or(0),
    );
}

/// Bundled attachment resources (keeps menu systems under bevy's
/// 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub struct AttachmentRes<'w> {
    pub player: Res<'w, PlayerAttachments>,
    pub ammo: Res<'w, AmmoRegistry>,
    pub parts: Res<'w, AttachmentRegistry>,
}

/// Bundled skin resources (same 16-param cap).
#[derive(bevy::ecs::system::SystemParam)]
pub struct SkinRes<'w> {
    pub registry: Res<'w, SkinRegistry>,
    pub inventory: Res<'w, Inventory>,
}

pub fn sync_loadout_to_configs(
    registry: &mut WeaponRegistry,
    loadout: &PlayerLoadout,
    player_attachments: &PlayerAttachments,
    ammo_registry: &AmmoRegistry,
    attachment_registry: &AttachmentRegistry,
) {
    for slot in [WeaponSlot::Primary, WeaponSlot::Secondary, WeaponSlot::Melee, WeaponSlot::Equipment] {
        let id = loadout.get_id_for_slot(slot);
        if let Some(config) = registry.weapons.get(id) {
            let mut config = config.clone();
            // Fold the equipped catalog ammo over the gun default so every
            // existing consumer (fire, HUD, dummies read base registry only
            // for dummies) sees effective ballistics. Caliber is re-checked
            // so stale selections can never leak across guns.
            if let Some(ammo_id) = player_attachments.get(id, "ammo") {
                if let Some(def) = ammo_registry.by_id.get(ammo_id) {
                    if !config.meta.caliber.is_empty() && def.caliber == config.meta.caliber {
                        if let Some(ammo) = config.attachments.ammo.as_mut() {
                            ammo.name = def.name.clone();
                            ammo.damage = def.damage;
                            ammo.velocity = def.velocity;
                            ammo.penetration = def.penetration;
                            ammo.range = def.range;
                            ammo.drop_factor = def.drop_factor;
                        }
                        // Shotgun shells reshape the spread itself: a slug
                        // collapses to one tight projectile, birdshot fans
                        // wide. None = keep the gun's default.
                        if let Some(pellets) = def.pellet_count {
                            config.attributes.pellet_count = pellets;
                        }
                        if let Some(cone) = def.spread_cone {
                            config.attributes.spread_cone = cone;
                        }
                    }
                }
            }
            // Fold equipped part modifiers + magazine capacity over the
            // defaults. The same fit check the UI enforces applies here, so
            // stale cross-gun selections stay inert.
            if let Some(slots) = player_attachments.selected.get(id) {
                for (slot_key, att_id) in slots {
                    if slot_key == "ammo" {
                        continue;
                    }
                    let Some(def) = attachment_registry.by_id.get(att_id) else {
                        continue;
                    };
                    if def.slot != *slot_key {
                        continue;
                    }
                    let fits = socket_type_for(&config, slot_key)
                        .is_some_and(|t| t == def.socket_type)
                        && def.fit.allows(
                            id,
                            &config.meta.family,
                            &config.meta.category,
                            &def.interfaces,
                            &config.meta.accepted_interfaces,
                        )
                        && pool_allows(&config, slot_key, &def.id);
                    if !fits {
                        continue;
                    }
                    apply_attachment_modifiers(&mut config.attributes, &def.modifiers);
                    if slot_key == "magazine" {
                        if let Some(mag) = config.attachments.magazine.as_mut() {
                            if let Some(cap) = def.capacity {
                                mag.capacity = cap;
                            }
                            if let Some(carry) = def.carry_capacity {
                                mag.carry_capacity = carry;
                            }
                        }
                    }
                }
            }
            registry.configs.insert(slot, config);
        }
    }
}

/// Human-readable effect line for one modifier entry: (label, is_good).
/// Direction depends on the attribute (lower recoil is good, higher
/// stability is good, ...). Unknown keys yield None.
pub fn modifier_effect_text(key: &str, delta: f32) -> Option<(String, bool)> {
    if delta == 0.0 {
        return None;
    }
    let (label, lower_is_better) = match key {
        "stability" => ("Stability", false),
        "ads_speed" => ("ADS speed", false),
        "mobility" => ("Mobility", false),
        "accuracy" => ("Accuracy", false),
        "horizontal_recoil" => ("Horizontal recoil", true),
        "vertical_recoil" => ("Vertical recoil", true),
        "reload_speed" => ("Reload speed", true),
        "equip_speed" => ("Equip speed", true),
        "fire_rate" => ("Fire rate", true),
        _ => return None,
    };
    let good = if lower_is_better { delta < 0.0 } else { delta > 0.0 };
    let sign = if delta > 0.0 { "+" } else { "-" };
    Some((format!("{label} {sign}{:.2}", delta.abs()), good))
}

/// Fold one alternative's generic modifiers into gun attributes.
fn apply_attachment_modifiers(attributes: &mut WeaponAttributes, modifiers: &HashMap<String, f32>) {
    for (key, delta) in modifiers {
        match key.as_str() {
            "stability" => attributes.stability += delta,
            "ads_speed" => attributes.ads_speed += delta,
            "mobility" => attributes.mobility += delta,
            "accuracy" => attributes.accuracy += delta,
            "horizontal_recoil" => attributes.horizontal_recoil += delta,
            "vertical_recoil" => attributes.vertical_recoil += delta,
            "reload_speed" => attributes.reload_speed += delta,
            "equip_speed" => attributes.equip_speed += delta,
            "fire_rate" => attributes.fire_rate += delta,
            _ => {}
        }
    }
}

/// Default (unskinned) material per slot.
pub fn default_slot_material(slot: WeaponSlot) -> StandardMaterial {
    match slot {
        WeaponSlot::Primary => StandardMaterial {
            base_color: Color::srgb(0.25, 0.25, 0.3),
            metallic: 0.7,
            perceptual_roughness: 0.3,
            ..default()
        },
        WeaponSlot::Secondary => StandardMaterial {
            base_color: Color::srgb(0.2, 0.2, 0.25),
            metallic: 0.7,
            perceptual_roughness: 0.3,
            ..default()
        },
        WeaponSlot::Melee => StandardMaterial {
            base_color: Color::srgb(0.5, 0.5, 0.55),
            metallic: 0.9,
            perceptual_roughness: 0.1,
            ..default()
        },
        WeaponSlot::Equipment => StandardMaterial {
            base_color: Color::srgb(0.3, 0.35, 0.2),
            metallic: 0.3,
            perceptual_roughness: 0.7,
            ..default()
        },
    }
}

/// Material for a non-default skin instance, honouring the safety mode.
pub fn skin_instance_material(
    skin: &SkinInstance,
    skin_registry: &SkinRegistry,
    asset_server: &AssetServer,
    cache: &mut SkinMaterialCache,
    advanced_materials: &mut Assets<WeaponMaterial>,
    materials: &mut Assets<StandardMaterial>,
) -> SkinMatHandle {
    match crate::weapons::material::skin_material_mode() {
        crate::weapons::material::SkinMaterialMode::Advanced => SkinMatHandle::Advanced(
            cache.get_or_create(skin, skin_registry, asset_server, advanced_materials),
        ),
        crate::weapons::material::SkinMaterialMode::Plain => SkinMatHandle::Plain(
            cache.plain_for(skin, skin_registry, asset_server, materials),
        ),
    }
}

/// Resolve one owned copy to a render material: the finish when a skin is
/// equipped, the slot's default gray otherwise. Uses the custom shader
/// material when advanced skins are enabled, plain `StandardMaterial`
/// otherwise.
pub fn skin_material(
    skin: &SkinInstance,
    slot: WeaponSlot,
    skin_registry: &SkinRegistry,
    asset_server: &AssetServer,
    cache: &mut SkinMaterialCache,
    advanced_materials: &mut Assets<WeaponMaterial>,
    materials: &mut Assets<StandardMaterial>,
) -> SkinMatHandle {
    if skin.is_default() {
        match crate::weapons::material::skin_material_mode() {
            crate::weapons::material::SkinMaterialMode::Advanced => {
                SkinMatHandle::Advanced(advanced_materials.add(WeaponMaterial {
                    base: default_slot_material(slot),
                    extension: Default::default(),
                }))
            }
            crate::weapons::material::SkinMaterialMode::Plain => {
                SkinMatHandle::Plain(materials.add(default_slot_material(slot)))
            }
        }
    } else {
        skin_instance_material(
            skin,
            skin_registry,
            asset_server,
            cache,
            advanced_materials,
            materials,
        )
    }
}

fn weapon_fallback_mesh(
    slot: WeaponSlot,
    skin: &SkinInstance,
    skin_registry: &SkinRegistry,
    asset_server: &AssetServer,
    cache: &mut SkinMaterialCache,
    meshes: &mut Assets<Mesh>,
    advanced_materials: &mut Assets<WeaponMaterial>,
    materials: &mut Assets<StandardMaterial>,
) -> (Handle<Mesh>, SkinMatHandle) {
    let mat = skin_material(
        skin,
        slot,
        skin_registry,
        asset_server,
        cache,
        advanced_materials,
        materials,
    );
    let mesh = match slot {
        WeaponSlot::Primary => meshes.add(Cuboid::new(0.08, 0.12, 0.6)),
        WeaponSlot::Secondary => meshes.add(Cuboid::new(0.06, 0.12, 0.3)),
        WeaponSlot::Melee => meshes.add(Cuboid::new(0.04, 0.04, 0.4)),
        WeaponSlot::Equipment => meshes.add(Cuboid::new(0.15, 0.15, 0.15)),
    };
    (mesh, mat)
}

/// Mesh overrides for a loadout slot: the equipped alternatives (if any)
/// resolved against the gun's config. Empty when everything is default.
pub fn overrides_for_slot(
    slot: WeaponSlot,
    loadout: &PlayerLoadout,
    registry: &WeaponRegistry,
    player: &PlayerAttachments,
    attachment_registry: &AttachmentRegistry,
) -> HashMap<String, ResolvedMesh> {
    let gun_id = loadout.get_id_for_slot(slot);
    registry
        .configs
        .get(&slot)
        .map(|c| resolve_attachment_meshes(gun_id, c, player, attachment_registry))
        .unwrap_or_default()
}

pub fn spawn_weapon_visual(
    commands: &mut Commands,
    slot: WeaponSlot,
    asset_server: &AssetServer,
    registry: &WeaponRegistry,
    skin_registry: &SkinRegistry,
    cache: &mut SkinMaterialCache,
    meshes: &mut Assets<Mesh>,
    weapon_materials: &mut Assets<WeaponMaterial>,
    materials: &mut Assets<StandardMaterial>,
    overrides: &HashMap<String, ResolvedMesh>,
    show_placeholders: bool,
) -> Entity {
    spawn_weapon_visual_skinned(
        commands,
        slot,
        SkinInstance::default(),
        asset_server,
        registry,
        skin_registry,
        cache,
        meshes,
        weapon_materials,
        materials,
        overrides,
        show_placeholders,
    )
}

pub fn spawn_weapon_visual_skinned(
    commands: &mut Commands,
    slot: WeaponSlot,
    skin: SkinInstance,
    asset_server: &AssetServer,
    registry: &WeaponRegistry,
    skin_registry: &SkinRegistry,
    cache: &mut SkinMaterialCache,
    meshes: &mut Assets<Mesh>,
    weapon_materials: &mut Assets<WeaponMaterial>,
    materials: &mut Assets<StandardMaterial>,
    overrides: &HashMap<String, ResolvedMesh>,
    show_placeholders: bool,
) -> Entity {
    if let Some(config) = registry.configs.get(&slot) {
        let transform = Transform::from_translation(Vec3::from(config.meta.position_offset))
            .with_rotation(Quat::from_euler(EulerRot::XYZ, config.meta.rotation_offset[0], config.meta.rotation_offset[1], config.meta.rotation_offset[2]))
            .with_scale(Vec3::splat(config.meta.scale));

        let model_file = config.meta.model_path.split('#').next().unwrap_or("");
        let model_exists = !model_file.is_empty()
            && std::path::Path::new(&format!("assets/{}", model_file)).exists();
        if !model_exists && !model_file.is_empty() {
            // CWD-relative check: if the game is launched from elsewhere this
            // fires for every gun and the player sees placeholder boxes.
            warn!(
                "weapon model '{}' not found relative to CWD; using placeholder for {slot:?}",
                model_file
            );
        }

        let entity = if model_exists {
            commands.spawn((
                WorldAssetRoot(asset_server.load(&config.meta.model_path)),
                transform,
                Visibility::default(),
                BaseWeaponTransform(transform),
                WeaponRecoil::default(),
                WeaponSkinTag::new(skin),
            )).id()
        } else {
            let (mesh, material) = weapon_fallback_mesh(
                slot, &skin, skin_registry, asset_server, cache, meshes, weapon_materials,
                materials,
            );
            let entity = commands.spawn((
                Mesh3d(mesh),
                transform,
                Visibility::default(),
                BaseWeaponTransform(transform),
                WeaponRecoil::default(),
                WeaponSkinTag { applied: true, ..WeaponSkinTag::new(skin) },
            )).id();
            material.apply(commands, entity);
            entity
        };

        // Assemble attachments: real split meshes where they exist (gray
        // placeholder blocks only when `show_placeholders` is set).
        spawn_weapon_attachments(commands, asset_server, entity, config, meshes, materials, overrides, show_placeholders);

        entity
    } else {
        let (mesh, material) = weapon_fallback_mesh(
            slot, &skin, skin_registry, asset_server, cache, meshes, weapon_materials,
            materials,
        );
        let transform = Transform::from_xyz(0.5, -0.5, -1.0);

        let entity = commands.spawn((
            Mesh3d(mesh),
            transform,
            BaseWeaponTransform(transform),
            WeaponRecoil::default(),
            WeaponSkinTag { applied: true, ..WeaponSkinTag::new(skin) },
        )).id();
        material.apply(commands, entity);
        entity
    }
}

/// Tag component for attachment children (meshes and placeholder blocks).
#[derive(Component)]
pub struct AttachmentPlaceholder {
    pub slot_name: String,
}

/// Resolve an attachment's local transform: the gun's socket when present,
/// else the attachment's own position_offset (legacy), else a per-slot
/// fallback. Returns None when there is nothing to anchor to.
fn attachment_transform(
    sockets: &HashMap<String, SocketDef>,
    slot_keys: &[&str],
    meta: Option<&AttachmentMeta>,
    fallback: [f32; 3],
) -> Option<Transform> {
    if let Some(socket) = slot_keys.iter().filter_map(|k| sockets.get(*k)).next() {
        return Some(
            Transform::from_translation(Vec3::from(socket.position)).with_rotation(
                Quat::from_euler(
                    EulerRot::XYZ,
                    socket.rotation[0],
                    socket.rotation[1],
                    socket.rotation[2],
                ),
            ),
        );
    }
    if let Some(meta) = meta {
        if let Some(pos) = meta.position_offset {
            let rot = meta.rotation_offset.unwrap_or([0.0, 0.0, 0.0]);
            return Some(
                Transform::from_translation(Vec3::from(pos)).with_rotation(
                    Quat::from_euler(EulerRot::XYZ, rot[0], rot[1], rot[2]),
                ),
            );
        }
    }
    Some(Transform::from_translation(Vec3::from(fallback)))
}

/// Mesh path of an attachment, if it declares one. Returns the config's
/// `String` by reference (not `&str`): `AssetServer::load` accepts `&String`
/// with a borrowed lifetime, while a `&str` argument demands `'static`.
fn attachment_mesh_path(meta: Option<&AttachmentMeta>) -> Option<&String> {
    meta.and_then(|m| {
        if !m.mesh_path.is_empty() {
            Some(&m.mesh_path)
        } else if !m.model_path.is_empty() {
            Some(&m.model_path)
        } else {
            None
        }
    })
}

fn mesh_file_exists(path: &str) -> bool {
    let file = path.split('#').next().unwrap_or("");
    !file.is_empty() && std::path::Path::new(&format!("assets/{}", file)).exists()
}

/// Assemble a weapon's attachments as children of the weapon entity: the
/// real split mesh when its file exists, else nothing (gray placeholder
/// blocks only when `show_placeholders` is set — a debug socket check).
/// Parts share the gun's model frame, so they inherit the weapon
/// entity's scale/position like the base model does.
///
/// `overrides` (slot label -> [`ResolvedMesh`], see
/// [`resolve_attachment_meshes`]) swaps an equipped alternative's mesh and
/// fine-tune transform in for the gun default.
pub fn spawn_weapon_attachments(
    commands: &mut Commands,
    asset_server: &AssetServer,
    weapon_entity: Entity,
    config: &WeaponConfig,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
    overrides: &HashMap<String, ResolvedMesh>,
    show_placeholders: bool,
) {
    let gray_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.4, 0.4, 0.45, 0.6),
        alpha_mode: AlphaMode::Blend,
        metallic: 0.3,
        perceptual_roughness: 0.6,
        ..default()
    });

    // (slot label, socket keys, legacy fallback, placeholder size). The
    // attachment meta + mesh path resolve per slot inside the loop so no
    // borrows are held across iterations.
    let slots: [(&str, &[&str], [f32; 3], Vec3); 7] = [
        ("Optic", &["optic"], [0.0, 0.08, 0.1], Vec3::new(0.15, 0.1, 0.15)),
        ("Barrel", &["barrel"], [0.0, 0.0, -0.3], Vec3::new(0.08, 0.08, 0.3)),
        ("Muzzle", &["muzzle"], [0.0, 0.0, 0.35], Vec3::new(0.07, 0.07, 0.15)),
        ("Underbarrel", &["underbarrel"], [0.0, -0.06, 0.1], Vec3::new(0.1, 0.08, 0.2)),
        ("Side", &["side", "sidebarrel", "other"], [0.06, 0.0, 0.0], Vec3::new(0.06, 0.06, 0.15)),
        ("Magazine", &["magazine"], [0.0, -0.1, 0.0], Vec3::new(0.08, 0.15, 0.1)),
        ("Stock", &["stock"], [-0.3, 0.0, 0.0], Vec3::new(0.1, 0.1, 0.25)),
    ];
    let attachments = &config.attachments;

    // Skip slots the gun doesn't define at all (no attachment block and no
    // socket) — matches the old placeholder behavior.
    let slot_defined = |label: &str| -> bool {
        match label {
            "Optic" => attachments.optic.is_some(),
            "Barrel" => attachments.barrel.is_some(),
            "Muzzle" => attachments.muzzle.is_some(),
            "Underbarrel" => attachments.underbarrel.is_some(),
            "Side" => attachments.side.is_some(),
            "Magazine" => attachments.magazine.is_some(),
            "Stock" => attachments.stock.is_some(),
            _ => false,
        }
    };

    for (label, socket_keys, fallback_default, block_size) in &slots {
        let has_socket = socket_keys.iter().any(|k| config.sockets.contains_key(*k));
        if !slot_defined(label) && !has_socket {
            continue;
        }
        let (meta, mesh_path) = match *label {
            "Optic" => {
                let meta = attachments.optic.as_ref().and_then(|a| a.meta.as_ref());
                (meta, attachment_mesh_path(meta))
            }
            "Barrel" => {
                let meta = attachments.barrel.as_ref().and_then(|a| a.meta.as_ref());
                (meta, attachment_mesh_path(meta))
            }
            "Muzzle" => {
                let meta = attachments.muzzle.as_ref().and_then(|a| a.meta.as_ref());
                (meta, attachment_mesh_path(meta))
            }
            "Underbarrel" => {
                let meta = attachments.underbarrel.as_ref().and_then(|a| a.meta.as_ref());
                (meta, attachment_mesh_path(meta))
            }
            "Side" => {
                let meta = attachments.side.as_ref().and_then(|a| a.meta.as_ref());
                (meta, attachment_mesh_path(meta))
            }
            "Magazine" => {
                let meta = attachments.magazine.as_ref().and_then(|a| a.meta.as_ref());
                (meta, attachment_mesh_path(meta))
            }
            _ => {
                let meta = attachments.stock.as_ref().and_then(|a| a.meta.as_ref());
                (meta, attachment_mesh_path(meta))
            }
        };
        // The optic keeps its aim-offset-derived anchor when neither a
        // socket nor an explicit offset exists (preserves ADS alignment).
        let mut fallback = *fallback_default;
        if *label == "Optic" {
            if let Some(aim) = attachments
                .optic
                .as_ref()
                .and_then(|o| o.meta.as_ref())
                .and_then(|m| m.aim_offset)
            {
                fallback = [aim[0], aim[1] + 0.05, aim[2]];
            }
        }
        // The barrel keeps its muzzle-flash-derived anchor likewise.
        if *label == "Barrel" {
            if let Some(mf) = attachments
                .barrel
                .as_ref()
                .and_then(|b| b.meta.as_ref())
                .and_then(|m| m.muzzle_flash_offset)
            {
                fallback = [mf[0] * 0.7, mf[1], mf[2]];
            }
        }
        let Some(transform) =
            attachment_transform(&config.sockets, socket_keys, meta, fallback)
        else {
            continue;
        };

        // Equipped alternative wins over the gun default: its mesh plus its
        // fine-tune transform composed on top of the slot transform.
        let resolved: Option<(String, Transform)> =
            if let Some(o) = overrides.get(*label) {
                if o.mesh_path.is_empty() {
                    None
                } else {
                    Some((
                        o.mesh_path.clone(),
                        Transform::from_translation(Vec3::from(o.offset)).with_rotation(
                            Quat::from_euler(
                                EulerRot::XYZ,
                                o.rotation[0],
                                o.rotation[1],
                                o.rotation[2],
                            ),
                        ),
                    ))
                }
            } else {
                mesh_path.map(|p| (p.to_owned(), Transform::IDENTITY))
            };

        let child = if let Some((path, fine)) = resolved {
            let transform = transform * fine;
            if mesh_file_exists(&path) {
                Some(commands.spawn((
                    WorldAssetRoot(asset_server.load(&path)),
                    transform,
                    Visibility::default(),
                    AttachmentPlaceholder {
                        slot_name: label.to_string(),
                    },
                )).id())
            } else if show_placeholders {
                Some(commands.spawn((
                    Mesh3d(meshes.add(Cuboid::new(
                        block_size.x,
                        block_size.y,
                        block_size.z,
                    ))),
                    MeshMaterial3d(gray_mat.clone()),
                    transform,
                    AttachmentPlaceholder {
                        slot_name: label.to_string(),
                    },
                )).id())
            } else {
                None
            }
        } else if show_placeholders {
            Some(commands.spawn((
                Mesh3d(meshes.add(Cuboid::new(
                    block_size.x,
                    block_size.y,
                    block_size.z,
                ))),
                MeshMaterial3d(gray_mat.clone()),
                transform,
                AttachmentPlaceholder {
                    slot_name: label.to_string(),
                },
            )).id())
        } else {
            None
        };
        if let Some(child) = child {
            commands.entity(weapon_entity).add_child(child);
        }
    }
}

pub fn slot_from_weapon_type(weapon_type: &str) -> WeaponSlot {
    match weapon_type {
        "Melee" | "2H Blade" | "Blade" | "Bayonet" | "Knife" => WeaponSlot::Melee,
        "Grenade" | "Equipment" => WeaponSlot::Equipment,
        "Pistol" | "Revolver" | "Machine Pistol" | "Secondary" => WeaponSlot::Secondary,
        // Melee configs spell out the grip: "1 Handed Sharp", "2 Handed Sharp".
        _ if weapon_type.contains("Handed") => WeaponSlot::Melee,
        _ => WeaponSlot::Primary,
    }
}

/// System that applies skin materials to all mesh descendants of a weapon
/// entity. Runs every frame but only processes entities where `applied` is
/// false. Swaps the glTF-loaded `StandardMaterial` on each mesh child for
/// the skin's `WeaponMaterial` (wear, pattern, animation).
fn apply_weapon_skins(
    mut commands: Commands,
    mut skin_query: Query<(Entity, &mut WeaponSkinTag)>,
    children_query: Query<&Children>,
    standard_meshes: Query<(), With<MeshMaterial3d<StandardMaterial>>>,
    advanced_meshes: Query<(), With<MeshMaterial3d<WeaponMaterial>>>,
    mut advanced_materials: ResMut<Assets<WeaponMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    skin_registry: Res<SkinRegistry>,
    asset_server: Res<AssetServer>,
    mut cache: ResMut<SkinMaterialCache>,
) {
    for (entity, mut skin_tag) in skin_query.iter_mut() {
        if skin_tag.skin.is_default() {
            skin_tag.applied = true;
            continue;
        }

        // Collect every mesh descendant. The scene streams children in, so
        // the count is the signal that another pass is needed.
        let mut meshes: Vec<Entity> = Vec::new();
        let mut stack: Vec<Entity> = vec![entity];
        while let Some(current) = stack.pop() {
            if standard_meshes.contains(current) || advanced_meshes.contains(current) {
                meshes.push(current);
            }
            if let Ok(children) = children_query.get(current) {
                for child in children.iter() {
                    stack.push(child);
                }
            }
        }

        if meshes.is_empty() {
            // Nothing loaded yet; retry next frame.
            continue;
        }
        if skin_tag.applied && meshes.len() == skin_tag.meshes_seen {
            continue;
        }

        let skin_mat = skin_instance_material(
            &skin_tag.skin,
            &skin_registry,
            &asset_server,
            &mut cache,
            &mut advanced_materials,
            &mut materials,
        );
        for mesh in &meshes {
            skin_mat.apply(&mut commands, *mesh);
        }
        skin_tag.meshes_seen = meshes.len();
        skin_tag.applied = true;
    }
}

#[cfg(test)]
mod dummy_weapon_tests {
    use super::*;

    /// The weapons the testing-grounds target dummies use must all resolve:
    /// valid models, scales, and (for guns) usable ammo stats.
    #[test]
    fn testing_grounds_dummy_weapons_are_valid() {
        let weapons = [
            ("hk416", include_str!("../../assets/weapons/data/primary/assault/hk416.json")),
            ("g17", include_str!("../../assets/weapons/data/secondary/pistol/g17.json")),
            ("remington_700", include_str!("../../assets/weapons/data/primary/sniper/remington_700.json")),
            ("franchi_spas_12", include_str!("../../assets/weapons/data/primary/shotgun/franchi_spas_12.json")),
            ("km2000", include_str!("../../assets/weapons/data/melee/km2000.json")),
        ];
        for (id, json) in weapons {
            let cfg: WeaponConfig =
                serde_json::from_str(json).unwrap_or_else(|e| panic!("{id} failed to parse: {e}"));
            assert!(!cfg.meta.model_path.is_empty(), "{id} has no model path");
            assert!(cfg.meta.scale > 0.0, "{id} has no scale");
            if id != "km2000" {
                let ammo = cfg
                    .attachments
                    .ammo
                    .as_ref()
                    .unwrap_or_else(|| panic!("{id} has no ammo"));
                assert!(ammo.velocity > 0.0 && ammo.damage > 0.0, "{id} has bad ammo stats");
                assert!(cfg.attributes.accuracy > 0.0, "{id} has no accuracy");
            }
        }
    }

    /// Pilot guns (hk416, ak-47) must parse with sockets, split-part mesh
    /// paths, and the new muzzle slot.
    #[test]
    fn pilot_guns_have_sockets_and_split_meshes() {
        for (id, json) in [
            ("hk416", include_str!("../../assets/weapons/data/primary/assault/hk416.json")),
            ("ak-47", include_str!("../../assets/weapons/data/primary/assault/ak-47.json")),
        ] {
            let cfg: WeaponConfig =
                serde_json::from_str(json).unwrap_or_else(|e| panic!("{id} failed to parse: {e}"));
            assert!(
                cfg.sockets.contains_key("magazine"),
                "{id} needs a magazine socket"
            );
            assert!(
                cfg.sockets.contains_key("muzzle"),
                "{id} needs a muzzle socket"
            );
            let muzzle = cfg
                .attachments
                .muzzle
                .as_ref()
                .unwrap_or_else(|| panic!("{id} has no muzzle attachment"));
            let mesh = muzzle
                .meta
                .as_ref()
                .and_then(|m| {
                    if !m.mesh_path.is_empty() {
                        Some(m.mesh_path.as_str())
                    } else {
                        None
                    }
                })
                .unwrap_or_else(|| panic!("{id} muzzle has no mesh path"));
            assert!(
                mesh.contains("attachments/muzzle/"),
                "{id} muzzle mesh path looks wrong: {mesh}"
            );
        }
        let hk: WeaponConfig = serde_json::from_str(include_str!(
            "../../assets/weapons/data/primary/assault/hk416.json"
        ))
        .unwrap();
        for slot in ["barrel", "optic", "magazine", "stock", "muzzle"] {
            assert!(
                hk.sockets.contains_key(slot),
                "hk416 needs a {slot} socket"
            );
        }
    }

    fn test_registry() -> AttachmentRegistry {        let files = [
            include_str!("../../assets/weapons/attachments/barrel/hk416_barrel.json"),
            include_str!("../../assets/weapons/attachments/muzzle/hk416_muzzle.json"),
            include_str!("../../assets/weapons/attachments/optic/hk416_optic.json"),
            include_str!("../../assets/weapons/attachments/stock/hk416_stock.json"),
            include_str!("../../assets/weapons/attachments/magazine/mag_556_curved_30.json"),
            include_str!("../../assets/weapons/attachments/muzzle/ak-47_muzzle.json"),
            include_str!("../../assets/weapons/attachments/magazine/mag_762ak_30.json"),
            include_str!("../../assets/weapons/attachments/optic/vortex_xm157.json"),
        ];
        let mut registry = AttachmentRegistry::default();
        for json in files {
            let def: AttachmentDef = serde_json::from_str(json).expect("stub must parse");
            registry.by_id.insert(def.id.clone(), def);
        }
        registry
    }

    fn hk416_config() -> WeaponConfig {
        serde_json::from_str(include_str!(
            "../../assets/weapons/data/primary/assault/hk416.json"
        ))
        .unwrap()
    }

    #[test]
    fn every_shipped_stub_parses_and_points_at_real_meshes() {
        // Walks the whole attachments catalog (not just the pilot stubs):
        // every def parses, ids are unique, socket types are known, and
        // every declared mesh file actually exists in assets.
        let known_sockets = [
            "barrel_mount",
            "muzzle_thread",
            "optic_rail",
            "stock_mount",
            "mag_well",
            "underbarrel_rail",
            "side_rail",
        ];
        let assets = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR")).join("assets");
        let mut ids = std::collections::HashSet::new();
        let mut count = 0;
        let mut stack = vec![assets.join("weapons/attachments")];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    // Skip the third-party model dirs (no JSONs there yet).
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "json")
                    && path.file_name().is_some_and(|n| n != "exampleAttachment.json")
                {
                    // Ammo entries live in the same tree with their own schema.
                    if path.parent().is_some_and(|p| p.ends_with("ammo")) {
                        let json = std::fs::read_to_string(&path).unwrap();
                        let def: AmmoDef = serde_json::from_str(&json)
                            .unwrap_or_else(|e| {
                                panic!("{} failed to parse: {e}", path.display())
                            });
                        assert!(ids.insert(format!("ammo:{}", def.id)), "duplicate ammo id {}", def.id);
                        assert!(!def.caliber.is_empty(), "{} needs a caliber", def.id);
                        assert!(
                            def.damage > 0.0 && def.velocity > 0.0,
                            "{} needs ballistics",
                            def.id
                        );
                        count += 1;
                        continue;
                    }
                    let json = std::fs::read_to_string(&path).unwrap();
                    let def: AttachmentDef = serde_json::from_str(&json)
                        .unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()));
                    assert!(ids.insert(def.id.clone()), "duplicate attachment id {}", def.id);
                    assert!(
                        known_sockets.contains(&def.socket_type.as_str()),
                        "{} has unknown socket type {}",
                        def.id,
                        def.socket_type
                    );
                    assert!(!def.artist.is_empty(), "{} needs an artist", def.id);
                    if !def.meta.mesh_path.is_empty() {
                        // Mesh paths are assets/-relative ("weapons/...").
                        let file = def.meta.mesh_path.split('#').next().unwrap_or("");
                        let full = assets.join(file);
                        assert!(
                            full.exists(),
                            "{} mesh missing: {}",
                            def.id,
                            full.display()
                        );
                    }
                    count += 1;
                }
            }
        }
        assert!(count > 100, "catalog should hold the batch stubs, got {count}");
    }

    #[test]
    fn fit_rule_matrix() {
        let no_iface: &[String] = &[];
        let open = FitRule::default();
        assert!(open.allows("hk416", "ar15", "assault", no_iface, no_iface));
        let exclusive = FitRule {
            guns: vec!["g17".to_string()],
            ..Default::default()
        };
        assert!(exclusive.allows("g17", "glock", "pistol", no_iface, no_iface));
        assert!(!exclusive.allows("hk416", "ar15", "assault", no_iface, no_iface));
        let family = FitRule {
            families: vec!["ak".to_string()],
            ..Default::default()
        };
        assert!(family.allows("ak-47", "ak", "assault", no_iface, no_iface));
        assert!(!family.allows("hk416", "ar15", "assault", no_iface, no_iface));
        // Empty family never matches a family rule (no silent cross-fit).
        assert!(!family.allows("mystery", "", "assault", no_iface, no_iface));
        let class = FitRule {
            classes: vec!["shotgun".to_string()],
            ..Default::default()
        };
        assert!(class.allows("remington_870", "", "shotgun", no_iface, no_iface));
        assert!(!class.allows("hk416", "ar15", "assault", no_iface, no_iface));
        // Shared mount interfaces fit across families (SCAR-L + STANAG-20).
        let stanag: &[String] = &["stanag556".to_string()];
        let ar_only = FitRule {
            families: vec!["ar15".to_string()],
            ..Default::default()
        };
        assert!(ar_only.allows("fn_scar_l_std", "scar", "assault", stanag, stanag));
        assert!(!ar_only.allows("ak-47", "ak", "assault", stanag, no_iface));
        // Interfaces alone never make a part universal: both sides need them.
        assert!(!ar_only.allows("fn_scar_l_std", "scar", "assault", stanag, no_iface));
        assert!(!ar_only.allows("fn_scar_l_std", "scar", "assault", no_iface, stanag));
    }

    #[test]
    fn registry_lists_compatible_alternatives() {
        let registry = test_registry();
        assert_eq!(registry.by_id.len(), 8, "all stubs must load");
        let hk = hk416_config();

        // The Vortex is universal: offered on hk416's optic rail.
        let optic_opts = compatible_attachments("hk416", "optic", &hk, &registry);
        let ids: Vec<&str> = optic_opts.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"vortex_xm157"), "vortex must fit, got {ids:?}");

        // Mounting respects fit: the AK mag must NOT list on the hk416
        // (ak family vs ar15), while the curved canonical does.
        let mag_opts = compatible_attachments("hk416", "magazine", &hk, &registry);
        let mag_ids: Vec<&str> = mag_opts.iter().map(|a| a.id.as_str()).collect();
        assert!(
            !mag_ids.contains(&"mag_762ak_30"),
            "ak mag must not fit hk416, got {mag_ids:?}"
        );
        assert!(
            mag_ids.contains(&"mag_556_curved_30"),
            "curved canonical must list, got {mag_ids:?}"
        );

        // Guns without a socket offer nothing (revolvers have no sockets).
        let python: WeaponConfig = serde_json::from_str(include_str!(
            "../../assets/weapons/data/secondary/revolver/colt_python.json"
        ))
        .unwrap();
        assert!(
            compatible_attachments("python", "optic", &python, &registry).is_empty(),
            "python has no optic socket"
        );
        // Socket expansion: the g17 takes universal optics now.
        let g17: WeaponConfig = serde_json::from_str(include_str!(
            "../../assets/weapons/data/secondary/pistol/g17.json"
        ))
        .unwrap();
        let g17_opts = compatible_attachments("g17", "optic", &g17, &registry);
        assert!(
            g17_opts.iter().any(|a| a.id == "vortex_xm157"),
            "vortex must fit g17's new optic socket"
        );
    }

    #[test]
    fn resolution_prefers_equipped_and_falls_back() {
        let registry = test_registry();
        let hk = hk416_config();
        let mut player = PlayerAttachments::default();

        // Defaults resolve to the split meshes.
        let resolved = resolve_attachment_meshes("hk416", &hk, &player, &registry);
        assert!(
            resolved["Optic"].mesh_path.contains("attachments/optic/hk416_optic.glb"),
            "default optic wrong: {}",
            resolved["Optic"].mesh_path
        );

        // Equipping the Vortex swaps the mesh.
        player.set("hk416".to_string(), "optic".to_string(), "vortex_xm157".to_string());
        let resolved = resolve_attachment_meshes("hk416", &hk, &player, &registry);
        assert!(
            resolved["Optic"].mesh_path.contains("vortex_xm157"),
            "vortex must win, got {}",
            resolved["Optic"].mesh_path
        );
        // Untouched slots resolve gun defaults (canonical mag mesh).
        assert!(
            resolved["Magazine"]
                .mesh_path
                .contains("attachments/magazine/mag_556_curved_30.glb"),
            "default mag wrong: {}",
            resolved["Magazine"].mesh_path
        );

        // Unknown ids fall back to the default.
        player.set("hk416".to_string(), "optic".to_string(), "nope_missing".to_string());
        let resolved = resolve_attachment_meshes("hk416", &hk, &player, &registry);
        assert!(resolved["Optic"].mesh_path.contains("hk416_optic"));

        // Slot mismatches fall back: a magazine never resolves into optics.
        player.set("hk416".to_string(), "optic".to_string(), "mag_762ak_30".to_string());
        let resolved = resolve_attachment_meshes("hk416", &hk, &player, &registry);
        assert!(
            resolved["Optic"].mesh_path.contains("hk416_optic"),
            "a magazine must never resolve into the optic slot"
        );
        // Fit mismatches fall back too: the AK mag fits no ar15 magwell.
        player.set("hk416".to_string(), "magazine".to_string(), "mag_762ak_30".to_string());
        let resolved = resolve_attachment_meshes("hk416", &hk, &player, &registry);
        assert!(
            resolved["Magazine"].mesh_path.contains("mag_556_curved_30"),
            "fit-blocked mag must fall back, got {}",
            resolved["Magazine"].mesh_path
        );

        // Clearing restores the default; names follow the same rules.
        player.clear("hk416", "optic");
        assert_eq!(
            effective_attachment_name("hk416", "optic", "Iron Sights", &player, &registry),
            "Iron Sights"
        );
        player.set("hk416".to_string(), "optic".to_string(), "vortex_xm157".to_string());
        assert_eq!(
            effective_attachment_name("hk416", "optic", "Iron Sights", &player, &registry),
            "Vortex XM157"
        );
    }

    fn test_ammo_registry() -> AmmoRegistry {
        let files = [
            ("ammo_556nato", include_str!("../../assets/weapons/attachments/ammo/ammo_556nato.json")),
            ("ammo_556mk262", include_str!("../../assets/weapons/attachments/ammo/ammo_556mk262.json")),
            ("ammo_762x39", include_str!("../../assets/weapons/attachments/ammo/ammo_762x39.json")),
            ("ammo_762x25", include_str!("../../assets/weapons/attachments/ammo/ammo_762x25.json")),
            ("ammo_762x51", include_str!("../../assets/weapons/attachments/ammo/ammo_762x51.json")),
            ("ammo_762x54r", include_str!("../../assets/weapons/attachments/ammo/ammo_762x54r.json")),
            ("ammo_9mm", include_str!("../../assets/weapons/attachments/ammo/ammo_9mm.json")),
            ("ammo_9x39", include_str!("../../assets/weapons/attachments/ammo/ammo_9x39.json")),
            ("ammo_57x28", include_str!("../../assets/weapons/attachments/ammo/ammo_57x28.json")),
            ("ammo_12ga", include_str!("../../assets/weapons/attachments/ammo/ammo_12ga.json")),
            ("ammo_12ga_birdshot", include_str!("../../assets/weapons/attachments/ammo/ammo_12ga_birdshot.json")),
            ("ammo_12ga_slug", include_str!("../../assets/weapons/attachments/ammo/ammo_12ga_slug.json")),
            ("ammo_12ga_flechette", include_str!("../../assets/weapons/attachments/ammo/ammo_12ga_flechette.json")),
            ("ammo_12ga_magnum", include_str!("../../assets/weapons/attachments/ammo/ammo_12ga_magnum.json")),
            ("ammo_45acp", include_str!("../../assets/weapons/attachments/ammo/ammo_45acp.json")),
            ("ammo_357mag", include_str!("../../assets/weapons/attachments/ammo/ammo_357mag.json")),
            ("ammo_455w", include_str!("../../assets/weapons/attachments/ammo/ammo_455w.json")),
            ("ammo_338lm", include_str!("../../assets/weapons/attachments/ammo/ammo_338lm.json")),
            ("ammo_50bmg", include_str!("../../assets/weapons/attachments/ammo/ammo_50bmg.json")),
            ("ammo_energy", include_str!("../../assets/weapons/attachments/ammo/ammo_energy.json")),
            ("ammo_545x39", include_str!("../../assets/weapons/attachments/ammo/ammo_545x39.json")),
            ("ammo_46x30", include_str!("../../assets/weapons/attachments/ammo/ammo_46x30.json")),
            ("ammo_3006", include_str!("../../assets/weapons/attachments/ammo/ammo_3006.json")),
            ("ammo_410", include_str!("../../assets/weapons/attachments/ammo/ammo_410.json")),
            ("ammo_410_birdshot", include_str!("../../assets/weapons/attachments/ammo/ammo_410_birdshot.json")),
            ("ammo_410_slug", include_str!("../../assets/weapons/attachments/ammo/ammo_410_slug.json")),
        ];
        let mut registry = AmmoRegistry::default();
        for (id, json) in files {
            let def: AmmoDef =
                serde_json::from_str(json).unwrap_or_else(|e| panic!("{id} failed to parse: {e}"));
            assert_eq!(def.id, id, "ammo id mismatch");
            assert!(def.damage > 0.0 && def.velocity > 0.0, "{id} needs ballistics");
            registry.by_id.insert(id.to_string(), def);
        }
        registry
    }

    #[test]
    fn ammo_catalog_gates_by_caliber() {
        let registry = test_ammo_registry();
        assert_eq!(registry.by_id.len(), 26, "all ammo seeds must load");

        let hk_opts = compatible_ammo("5.56x45mm", &registry);
        let ids: Vec<&str> = hk_opts.iter().map(|a| a.id.as_str()).collect();
        assert!(ids.contains(&"ammo_556nato"), "base 5.56 must list: {ids:?}");
        assert!(ids.contains(&"ammo_556mk262"), "Mk262 must list: {ids:?}");
        assert!(!ids.contains(&"ammo_9mm"), "9mm must not list on 5.56: {ids:?}");

        assert!(compatible_ammo("", &registry).is_empty(), "empty caliber offers none");
        let ak_opts = compatible_ammo("7.62x39mm", &registry);
        assert_eq!(ak_opts.len(), 1, "7.62x39 has one entry");
        assert_eq!(ak_opts[0].id, "ammo_762x39");

        // Shotgun shells: every 12ga gun gets the full 5-type lineup.
        let twelve = compatible_ammo("12ga", &registry);
        let twelve_ids: Vec<&str> = twelve.iter().map(|a| a.id.as_str()).collect();
        for want in [
            "ammo_12ga",
            "ammo_12ga_birdshot",
            "ammo_12ga_slug",
            "ammo_12ga_flechette",
            "ammo_12ga_magnum",
        ] {
            assert!(twelve_ids.contains(&want), "12ga missing {want}: {twelve_ids:?}");
        }
        // Slug collapses to a single tight projectile; birdshot fans wide.
        let slug = twelve.iter().find(|a| a.id == "ammo_12ga_slug").unwrap();
        assert_eq!(slug.pellet_count, Some(1));
        let bird = twelve.iter().find(|a| a.id == "ammo_12ga_birdshot").unwrap();
        assert!(bird.pellet_count.unwrap_or(0) > 8);
        assert!(bird.spread_cone.unwrap_or(0.0) > slug.spread_cone.unwrap_or(0.0));
    }

    #[test]
    fn sync_folds_catalog_ammo_into_configs() {
        let ammo_registry = test_ammo_registry();
        let hk = hk416_config();
        let mut registry = WeaponRegistry::default();
        registry.weapons.insert("hk416".to_string(), hk);
        let mut loadout = PlayerLoadout::default();
        loadout.primary = "hk416".to_string();
        let mut player = PlayerAttachments::default();

        // Default: the gun JSON's own ammo.
        sync_loadout_to_configs(&mut registry, &loadout, &player, &ammo_registry, &AttachmentRegistry::default());
        let ammo = registry.configs[&WeaponSlot::Primary]
            .attachments
            .ammo
            .as_ref()
            .unwrap();
        assert_eq!(ammo.name, "5.56 NATO");
        assert!((ammo.damage - 25.0).abs() < 1e-6);

        // Equipped Mk262 replaces ballistics wholesale.
        player.set("hk416".to_string(), "ammo".to_string(), "ammo_556mk262".to_string());
        sync_loadout_to_configs(&mut registry, &loadout, &player, &ammo_registry, &AttachmentRegistry::default());
        let ammo = registry.configs[&WeaponSlot::Primary]
            .attachments
            .ammo
            .as_ref()
            .unwrap();
        assert_eq!(ammo.name, "5.56 NATO Mk262");
        assert!((ammo.damage - 30.0).abs() < 1e-6);
        assert!((ammo.velocity - 940.0).abs() < 1e-6);

        // Wrong-caliber selections can never leak in (stale cross-gun state):
        // 9mm on a 5.56 gun leaves the gun default in place.
        player.set("hk416".to_string(), "ammo".to_string(), "ammo_9mm".to_string());
        sync_loadout_to_configs(&mut registry, &loadout, &player, &ammo_registry, &AttachmentRegistry::default());
        let ammo = registry.configs[&WeaponSlot::Primary]
            .attachments
            .ammo
            .as_ref()
            .unwrap();
        assert_eq!(
            ammo.name, "5.56 NATO",
            "mismatched caliber must not apply, got {}",
            ammo.name
        );
    }

    #[test]
    fn sync_folds_shotgun_spread_overrides() {
        let ammo_registry = test_ammo_registry();
        let mut shotgun = hk416_config();
        shotgun.meta.caliber = "12ga".to_string();
        shotgun.attributes.pellet_count = 8;
        shotgun.attributes.spread_cone = 4.0;
        let mut registry = WeaponRegistry::default();
        registry.weapons.insert("shotgun".to_string(), shotgun);
        let mut loadout = PlayerLoadout::default();
        loadout.primary = "shotgun".to_string();
        let mut player = PlayerAttachments::default();

        // Slug collapses to a single tight projectile.
        player.set(
            "shotgun".to_string(),
            "ammo".to_string(),
            "ammo_12ga_slug".to_string(),
        );
        sync_loadout_to_configs(
            &mut registry,
            &loadout,
            &player,
            &ammo_registry,
            &AttachmentRegistry::default(),
        );
        let cfg = &registry.configs[&WeaponSlot::Primary];
        assert_eq!(cfg.attributes.pellet_count, 1);
        assert!((cfg.attributes.spread_cone - 0.3).abs() < 1e-6);

        // Birdshot fans wide with no stale slug state.
        player.set(
            "shotgun".to_string(),
            "ammo".to_string(),
            "ammo_12ga_birdshot".to_string(),
        );
        sync_loadout_to_configs(
            &mut registry,
            &loadout,
            &player,
            &ammo_registry,
            &AttachmentRegistry::default(),
        );
        let cfg = &registry.configs[&WeaponSlot::Primary];
        assert_eq!(cfg.attributes.pellet_count, 16);
        assert!((cfg.attributes.spread_cone - 6.5).abs() < 1e-6);
    }

    #[test]
    fn sync_folds_modifiers_and_mag_capacity() {
        let ammo_registry = test_ammo_registry();
        let hk = hk416_config();
        let mut registry = WeaponRegistry::default();
        registry.weapons.insert("hk416".to_string(), hk);
        let mut loadout = PlayerLoadout::default();
        loadout.primary = "hk416".to_string();
        let mut player = PlayerAttachments::default();

        // Baseline: no selections, attributes untouched.
        let mut attachments = AttachmentRegistry::default();
        sync_loadout_to_configs(&mut registry, &loadout, &player, &ammo_registry, &attachments);
        let base_ads = registry.configs[&WeaponSlot::Primary].attributes.ads_speed;
        let base_cap = registry.configs[&WeaponSlot::Primary]
            .attachments
            .magazine
            .as_ref()
            .unwrap()
            .capacity;
        assert!((base_ads - 0.6).abs() < 1e-6);
        assert_eq!(base_cap, 30);

        // Hand-built drum: penalties + big capacity, fits ar15. Pool-gated
        // like everything else, so the test pool lists it explicitly.
        attachments.by_id.insert(
            "test_drum".to_string(),
            AttachmentDef {
                id: "test_drum".to_string(),
                name: "Test Drum".to_string(),
                slot: "magazine".to_string(),
                socket_type: "mag_well".to_string(),
                fit: FitRule {
                    families: vec!["ar15".to_string()],
                    ..Default::default()
                },
                interfaces: vec![],
                artist: "Test".to_string(),
                meta: AttachmentDefMeta::default(),
                provides_sockets: Default::default(),
                modifiers: [("mobility".to_string(), -0.15f32),
                    ("reload_speed".to_string(), 0.6f32)]
                .into_iter()
                .collect(),
                capacity: Some(100),
                carry_capacity: Some(200),
                special_effects: vec![],
            },
        );
        registry.weapons.get_mut("hk416").unwrap().pools.insert(
            "magazine".to_string(),
            vec!["test_drum".to_string(), ATTACHMENT_NONE.to_string()],
        );
        player.set("hk416".to_string(), "magazine".to_string(), "test_drum".to_string());
        sync_loadout_to_configs(&mut registry, &loadout, &player, &ammo_registry, &attachments);
        let cfg = &registry.configs[&WeaponSlot::Primary];
        let mag = cfg.attachments.magazine.as_ref().unwrap();
        assert_eq!(mag.capacity, 100, "drum capacity must apply");
        assert_eq!(mag.carry_capacity, 200);
        assert!(
            (cfg.attributes.mobility - 0.55).abs() < 1e-6,
            "hk416 mobility 0.7 - 0.15 drum penalty"
        );
        assert!(cfg.attributes.reload_speed > 2.5, "reload penalty must apply");

        // Unknown modifier keys are ignored, never crash the fold.
        attachments.by_id.get_mut("test_drum").unwrap().modifiers.insert(
            "nonsense_stat".to_string(),
            999.0,
        );
        sync_loadout_to_configs(&mut registry, &loadout, &player, &ammo_registry, &attachments);
        let cfg = &registry.configs[&WeaponSlot::Primary];
        assert_eq!(cfg.attachments.magazine.as_ref().unwrap().capacity, 100);

        // Explicit removal suppresses the mesh (empty path = spawn nothing).
        // Only slots whose pool lists __none__ (stock/muzzle/optic/grips).
        player.set(
            "hk416".to_string(),
            "stock".to_string(),
            ATTACHMENT_NONE.to_string(),
        );
        let resolved = resolve_attachment_meshes("hk416", &hk416_config(), &player, &attachments);
        assert_eq!(
            resolved.get("Stock").map(|r| r.mesh_path.as_str()),
            Some(""),
            "removed stock must resolve to an empty path"
        );
        // Clearing restores the default; names follow the same rules.
        player.clear("hk416", "stock");
        player.clear("hk416", "magazine");
        player.clear("hk416", "optic");
        assert_eq!(
            effective_attachment_name("hk416", "stock", "Standard Stock", &player, &attachments),
            "Standard Stock"
        );
        player.set(
            "hk416".to_string(),
            "stock".to_string(),
            ATTACHMENT_NONE.to_string(),
        );
        assert_eq!(
            effective_attachment_name("hk416", "stock", "Standard Stock", &player, &attachments),
            "None"
        );
    }

    #[test]
    fn pool_gating_rules() {
        let hk = hk416_config();
        // hk416's generated pools list its own parts + __none__ on removable.
        assert!(pool_allows(&hk, "magazine", "mag_556_curved_30"));
        assert!(!pool_allows(&hk, "magazine", "mag_762ak_30"));
        assert!(slot_allows_none(&hk, "stock"));
        assert!(slot_allows_none(&hk, "muzzle"));
        assert!(!slot_allows_none(&hk, "magazine"));
        assert!(!slot_allows_none(&hk, "barrel"));
        // Legacy keys read as the renamed `side` slot (savestate compat).
        assert_eq!(canonical_slot_key("other"), "side");
        assert_eq!(canonical_slot_key("sidebarrel"), "side");
        assert_eq!(canonical_slot_key("optic"), "optic");
        // No pool (un-split guns) = fit rules govern (open).
        let python: WeaponConfig = serde_json::from_str(include_str!(
            "../../assets/weapons/data/secondary/revolver/colt_python.json"
        ))
        .unwrap();
        assert!(pool_allows(&python, "magazine", "anything_at_all"));
    }

    #[test]
    fn modifier_effect_text_directions() {
        // Higher stability/ADS/mobility/accuracy is good.
        assert_eq!(
            modifier_effect_text("stability", 0.08),
            Some(("Stability +0.08".to_string(), true))
        );
        assert_eq!(
            modifier_effect_text("ads_speed", -0.05),
            Some(("ADS speed -0.05".to_string(), false))
        );
        // Lower recoil/reload/equip/fire times are good.
        assert_eq!(
            modifier_effect_text("vertical_recoil", -0.15),
            Some(("Vertical recoil -0.15".to_string(), true))
        );
        assert_eq!(
            modifier_effect_text("horizontal_recoil", 0.05),
            Some(("Horizontal recoil +0.05".to_string(), false))
        );
        assert_eq!(
            modifier_effect_text("reload_speed", 0.6),
            Some(("Reload speed +0.60".to_string(), false))
        );
        assert_eq!(modifier_effect_text("mobility", 0.0), None);
        assert_eq!(modifier_effect_text("nonsense", 1.0), None);
    }

    #[test]
    fn every_gun_with_ammo_declares_caliber() {
        let data_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/weapons/data");
        let mut checked = 0;
        let mut stack = vec![data_dir];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "json")
                    && !path.to_string_lossy().contains("xample")
                    && path.file_name().is_some_and(|n| n != "archetypes.json")
                {
                    let json = std::fs::read_to_string(&path).unwrap();
                    let cfg: WeaponConfig = serde_json::from_str(&json)
                        .unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()));
                    if cfg.attachments.ammo.is_some() {
                        assert!(
                            !cfg.meta.caliber.is_empty(),
                            "{} has ammo but no caliber",
                            path.display()
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 30, "should cover all gun JSONs, got {checked}");
    }

    #[test]
    fn every_socket_has_a_type() {
        // Regression: the batch surgery once wrote sockets without
        // socket_type, silently emptying every options list.
        let data_dir = std::path::PathBuf::from(env!("CARGO_MANIFEST_DIR"))
            .join("assets/weapons/data");
        let mut checked = 0;
        let mut stack = vec![data_dir];
        while let Some(dir) = stack.pop() {
            for entry in std::fs::read_dir(&dir).unwrap() {
                let path = entry.unwrap().path();
                if path.is_dir() {
                    stack.push(path);
                } else if path.extension().is_some_and(|e| e == "json")
                    && !path.to_string_lossy().contains("xample")
                    && path.file_name().is_some_and(|n| n != "archetypes.json")
                {
                    let json = std::fs::read_to_string(&path).unwrap();
                    let cfg: WeaponConfig = serde_json::from_str(&json)
                        .unwrap_or_else(|e| panic!("{} failed to parse: {e}", path.display()));
                    for (key, socket) in &cfg.sockets {
                        assert!(
                            !socket.socket_type.is_empty(),
                            "{} socket {key} has no type",
                            path.display()
                        );
                        checked += 1;
                    }
                }
            }
        }
        assert!(checked > 100, "should cover all sockets, got {checked}");
    }

    #[test]
    fn socket_type_falls_back_by_slot() {
        let mut config = hk416_config();
        // Optic socket with the type stripped falls back to optic_rail.
        if let Some(socket) = config.sockets.get_mut("optic") {
            socket.socket_type.clear();
        }
        assert_eq!(socket_type_for(&config, "optic"), Some("optic_rail"));
        assert_eq!(socket_type_for(&config, "magazine"), Some("mag_well"));
        assert_eq!(socket_type_for(&config, "nope"), None);
        let python: WeaponConfig = serde_json::from_str(include_str!(
            "../../assets/weapons/data/secondary/revolver/colt_python.json"
        ))
        .unwrap();
        assert_eq!(socket_type_for(&python, "optic"), None, "python has no sockets at all");
    }
}
