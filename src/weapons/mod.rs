use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum SkinRarity {
    Common,
    Uncommon,
    Rare,
    Epic,
    Legendary,
    Mythic,
}

impl SkinRarity {
    /// Drop chance weight (out of 1000)
    pub fn drop_weight(&self) -> u32 {
        match self {
            SkinRarity::Common => 500,     // 50%
            SkinRarity::Uncommon => 250,   // 25%
            SkinRarity::Rare => 150,       // 15%
            SkinRarity::Epic => 50,        // 5%
            SkinRarity::Legendary => 9,    // 0.9%
            SkinRarity::Mythic => 1,       // 0.1%
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            SkinRarity::Common => "Common",
            SkinRarity::Uncommon => "Uncommon",
            SkinRarity::Rare => "Rare",
            SkinRarity::Epic => "Epic",
            SkinRarity::Legendary => "Legendary",
            SkinRarity::Mythic => "Mythic",
        }
    }

    pub fn color(&self) -> Color {
        match self {
            SkinRarity::Common => Color::srgb(0.6, 0.6, 0.6),
            SkinRarity::Uncommon => Color::srgb(0.3, 0.7, 0.3),
            SkinRarity::Rare => Color::srgb(0.2, 0.4, 0.9),
            SkinRarity::Epic => Color::srgb(0.6, 0.2, 0.8),
            SkinRarity::Legendary => Color::srgb(0.9, 0.7, 0.1),
            SkinRarity::Mythic => Color::srgb(0.9, 0.15, 0.15),
        }
    }
}

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default, Serialize, Deserialize)]
pub enum WeaponSkin {
    #[default]
    Default,
    // Common
    SolidRed,
    SolidBlue,
    SolidGreen,
    SolidBlack,
    SolidWhite,
    // Uncommon
    CamoWoodland,
    CamoDesert,
    CarbonFiber,
    // Rare
    TigerStripe,
    SolidGold,
    ArcticWhite,
    // Epic
    NeonPink,
    DragonScale,
    // Legendary
    ChromeForge,
    VoidWalker,
    // Mythic
    Supernova,
}

impl WeaponSkin {
    pub fn all() -> &'static [WeaponSkin] {
        &[
            WeaponSkin::Default,
            WeaponSkin::SolidRed,
            WeaponSkin::SolidBlue,
            WeaponSkin::SolidGreen,
            WeaponSkin::SolidBlack,
            WeaponSkin::SolidWhite,
            WeaponSkin::CamoWoodland,
            WeaponSkin::CamoDesert,
            WeaponSkin::CarbonFiber,
            WeaponSkin::TigerStripe,
            WeaponSkin::SolidGold,
            WeaponSkin::ArcticWhite,
            WeaponSkin::NeonPink,
            WeaponSkin::DragonScale,
            WeaponSkin::ChromeForge,
            WeaponSkin::VoidWalker,
            WeaponSkin::Supernova,
        ]
    }

    /// All skins except Default that can drop from crates
    pub fn droppable() -> &'static [WeaponSkin] {
        &[
            WeaponSkin::SolidRed,
            WeaponSkin::SolidBlue,
            WeaponSkin::SolidGreen,
            WeaponSkin::SolidBlack,
            WeaponSkin::SolidWhite,
            WeaponSkin::CamoWoodland,
            WeaponSkin::CamoDesert,
            WeaponSkin::CarbonFiber,
            WeaponSkin::TigerStripe,
            WeaponSkin::SolidGold,
            WeaponSkin::ArcticWhite,
            WeaponSkin::NeonPink,
            WeaponSkin::DragonScale,
            WeaponSkin::ChromeForge,
            WeaponSkin::VoidWalker,
            WeaponSkin::Supernova,
        ]
    }

    pub fn rarity(&self) -> SkinRarity {
        match self {
            WeaponSkin::Default => SkinRarity::Common,
            WeaponSkin::SolidRed | WeaponSkin::SolidBlue | WeaponSkin::SolidGreen
                | WeaponSkin::SolidBlack | WeaponSkin::SolidWhite => SkinRarity::Common,
            WeaponSkin::CamoWoodland | WeaponSkin::CamoDesert
                | WeaponSkin::CarbonFiber => SkinRarity::Uncommon,
            WeaponSkin::TigerStripe | WeaponSkin::SolidGold
                | WeaponSkin::ArcticWhite => SkinRarity::Rare,
            WeaponSkin::NeonPink | WeaponSkin::DragonScale => SkinRarity::Epic,
            WeaponSkin::ChromeForge | WeaponSkin::VoidWalker => SkinRarity::Legendary,
            WeaponSkin::Supernova => SkinRarity::Mythic,
        }
    }

    pub fn display_name(&self) -> &str {
        match self {
            WeaponSkin::Default => "Default",
            WeaponSkin::SolidRed => "Red",
            WeaponSkin::SolidBlue => "Blue",
            WeaponSkin::SolidGreen => "Green",
            WeaponSkin::SolidGold => "Gold",
            WeaponSkin::SolidBlack => "Black",
            WeaponSkin::SolidWhite => "White",
            WeaponSkin::CamoWoodland => "Woodland",
            WeaponSkin::CamoDesert => "Desert",
            WeaponSkin::CarbonFiber => "Carbon",
            WeaponSkin::TigerStripe => "Tiger",
            WeaponSkin::ArcticWhite => "Arctic",
            WeaponSkin::NeonPink => "Neon Pink",
            WeaponSkin::DragonScale => "Dragon Scale",
            WeaponSkin::ChromeForge => "Chrome Forge",
            WeaponSkin::VoidWalker => "Void Walker",
            WeaponSkin::Supernova => "Supernova",
        }
    }

    pub fn swatch_color(&self) -> Color {
        match self {
            WeaponSkin::Default => Color::srgb(0.3, 0.3, 0.35),
            WeaponSkin::SolidRed => Color::srgb(0.7, 0.1, 0.1),
            WeaponSkin::SolidBlue => Color::srgb(0.1, 0.2, 0.7),
            WeaponSkin::SolidGreen => Color::srgb(0.1, 0.5, 0.15),
            WeaponSkin::SolidGold => Color::srgb(0.85, 0.7, 0.2),
            WeaponSkin::SolidBlack => Color::srgb(0.05, 0.05, 0.05),
            WeaponSkin::SolidWhite => Color::srgb(0.9, 0.9, 0.9),
            WeaponSkin::CamoWoodland => Color::srgb(0.2, 0.35, 0.15),
            WeaponSkin::CamoDesert => Color::srgb(0.6, 0.5, 0.3),
            WeaponSkin::CarbonFiber => Color::srgb(0.15, 0.15, 0.18),
            WeaponSkin::TigerStripe => Color::srgb(0.7, 0.45, 0.1),
            WeaponSkin::ArcticWhite => Color::srgb(0.85, 0.9, 0.95),
            WeaponSkin::NeonPink => Color::srgb(0.9, 0.1, 0.6),
            WeaponSkin::DragonScale => Color::srgb(0.15, 0.6, 0.3),
            WeaponSkin::ChromeForge => Color::srgb(0.75, 0.78, 0.82),
            WeaponSkin::VoidWalker => Color::srgb(0.1, 0.05, 0.2),
            WeaponSkin::Supernova => Color::srgb(0.95, 0.4, 0.1),
        }
    }

    pub fn to_material(&self) -> StandardMaterial {
        match self {
            WeaponSkin::Default => StandardMaterial {
                base_color: Color::srgb(0.25, 0.25, 0.3),
                metallic: 0.7,
                perceptual_roughness: 0.3,
                ..default()
            },
            WeaponSkin::SolidGold => StandardMaterial {
                base_color: Color::srgb(0.85, 0.7, 0.2),
                metallic: 0.9,
                perceptual_roughness: 0.15,
                ..default()
            },
            WeaponSkin::CarbonFiber => StandardMaterial {
                base_color: Color::srgb(0.12, 0.12, 0.15),
                metallic: 0.5,
                perceptual_roughness: 0.1,
                ..default()
            },
            WeaponSkin::SolidBlack => StandardMaterial {
                base_color: Color::srgb(0.05, 0.05, 0.05),
                metallic: 0.6,
                perceptual_roughness: 0.2,
                ..default()
            },
            WeaponSkin::ArcticWhite => StandardMaterial {
                base_color: Color::srgb(0.92, 0.95, 0.98),
                metallic: 0.4,
                perceptual_roughness: 0.15,
                ..default()
            },
            WeaponSkin::NeonPink => StandardMaterial {
                base_color: Color::srgb(0.9, 0.1, 0.6),
                metallic: 0.3,
                perceptual_roughness: 0.2,
                emissive: bevy::color::LinearRgba::new(0.4, 0.02, 0.25, 1.0),
                ..default()
            },
            WeaponSkin::DragonScale => StandardMaterial {
                base_color: Color::srgb(0.1, 0.5, 0.25),
                metallic: 0.7,
                perceptual_roughness: 0.25,
                ..default()
            },
            WeaponSkin::ChromeForge => StandardMaterial {
                base_color: Color::srgb(0.8, 0.82, 0.85),
                metallic: 0.95,
                perceptual_roughness: 0.05,
                ..default()
            },
            WeaponSkin::VoidWalker => StandardMaterial {
                base_color: Color::srgb(0.05, 0.02, 0.12),
                metallic: 0.8,
                perceptual_roughness: 0.1,
                emissive: bevy::color::LinearRgba::new(0.08, 0.0, 0.2, 1.0),
                ..default()
            },
            WeaponSkin::Supernova => StandardMaterial {
                base_color: Color::srgb(0.95, 0.35, 0.05),
                metallic: 0.6,
                perceptual_roughness: 0.1,
                emissive: bevy::color::LinearRgba::new(0.5, 0.15, 0.02, 1.0),
                ..default()
            },
            _ => StandardMaterial {
                base_color: self.swatch_color(),
                metallic: 0.3,
                perceptual_roughness: 0.5,
                ..default()
            },
        }
    }
}

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
    pub skin: WeaponSkin,
    pub applied: bool,
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
    pub meta: Option<AttachmentMeta>,
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
pub struct SidebarrelAttachment {
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
pub struct WeaponAttachments {
    pub optic: Option<OpticAttachment>,
    pub barrel: Option<BarrelAttachment>,
    pub underbarrel: Option<UnderbarrelAttachment>,
    pub sidebarrel: Option<SidebarrelAttachment>,
    pub magazine: Option<MagazineAttachment>,
    pub ammo: Option<AmmoAttachment>,
    pub stock: Option<StockAttachment>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WeaponConfig {
    pub info: WeaponInfo,
    pub meta: WeaponMeta,
    pub attributes: WeaponAttributes,
    #[serde(default)]
    pub attachments: WeaponAttachments,
}

pub type WeaponId = String;

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct PlayerLoadout {
    pub primary: WeaponId,
    pub secondary: WeaponId,
    pub melee: WeaponId,
    pub equipment: WeaponId,
    pub skins: HashMap<WeaponSlot, WeaponSkin>,
}

impl Default for PlayerLoadout {
    fn default() -> Self {
        Self {
            primary: "colt_m4a1".to_string(),
            secondary: "g17".to_string(),
            melee: "msbs_grot_bayonet".to_string(),
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

    pub fn get_skin(&self, slot: WeaponSlot) -> WeaponSkin {
        self.skins.get(&slot).copied().unwrap_or_default()
    }

    pub fn set_skin(&mut self, slot: WeaponSlot, skin: WeaponSkin) {
        self.skins.insert(slot, skin);
    }

    pub fn save(&self) {
        let path = "settings/savestate.json";
        
        // Load existing savestate or create new
        let mut savestate: serde_json::Value = match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({})),
            Err(_) => serde_json::json!({}),
        };
        
        // Update loadout section
        savestate["loadout"] = serde_json::to_value(self).unwrap();
        
        if let Ok(content) = serde_json::to_string_pretty(&savestate) {
            let _ = std::fs::write(path, content);
        }
    }

    pub fn load() -> Self {
        let path = "settings/savestate.json";
        match std::fs::read_to_string(path) {
            Ok(content) => {
                if let Ok(savestate) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(loadout) = savestate.get("loadout") {
                        return serde_json::from_value(loadout.clone()).unwrap_or_default();
                    }
                }
                Self::default()
            },
            Err(_) => Self::default(),
        }
    }
}

/// Persistent inventory of skins owned per weapon, with duplicate counts.
/// Stored as JSON at `settings/savestate.json`.
/// Keys: weapon_id -> skin_name -> count
#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct SkinInventory {
    pub owned: HashMap<String, HashMap<WeaponSkin, u32>>,
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
        let path = "settings/savestate.json";
        
        // Load existing savestate or create new
        let mut savestate: serde_json::Value = match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({})),
            Err(_) => serde_json::json!({}),
        };
        
        // Update credits section
        savestate["credits"] = serde_json::to_value(self).unwrap();
        
        if let Ok(content) = serde_json::to_string_pretty(&savestate) {
            let _ = std::fs::write(path, content);
        }
    }

    pub fn load() -> Self {
        let path = "settings/savestate.json";
        match std::fs::read_to_string(path) {
            Ok(content) => {
                if let Ok(savestate) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(credits) = savestate.get("credits") {
                        return serde_json::from_value(credits.clone()).unwrap_or_default();
                    }
                }
                Self::default()
            },
            Err(_) => Self::default(),
        }
    }

    /// Credit value for selling a skin of a given rarity
    pub fn sell_value(rarity: SkinRarity) -> u64 {
        match rarity {
            SkinRarity::Common => 10,
            SkinRarity::Uncommon => 25,
            SkinRarity::Rare => 50,
            SkinRarity::Epic => 100,
            SkinRarity::Legendary => 250,
            SkinRarity::Mythic => 1000,
        }
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

impl Default for SkinInventory {
    fn default() -> Self {
        Self {
            owned: HashMap::new(),
        }
    }
}

impl SkinInventory {
    pub fn add_skin(&mut self, weapon_id: &str, skin: WeaponSkin) {
        let entry = self.owned
            .entry(weapon_id.to_string())
            .or_default()
            .entry(skin)
            .or_insert(0);
        *entry += 1;
    }

    pub fn has_skin(&self, weapon_id: &str, skin: &WeaponSkin) -> bool {
        self.owned
            .get(weapon_id)
            .and_then(|skins| skins.get(skin))
            .map(|c| *c > 0)
            .unwrap_or(false)
    }

    pub fn owned_skins_for(&self, weapon_id: &str) -> Vec<WeaponSkin> {
        let mut result = vec![WeaponSkin::Default]; // Default always available
        if let Some(skins) = self.owned.get(weapon_id) {
            for (skin, count) in skins {
                if *count > 0 && *skin != WeaponSkin::Default {
                    result.push(*skin);
                }
            }
        }
        // Sort by rarity
        result.sort_by_key(|s| match s.rarity() {
            SkinRarity::Common => 0,
            SkinRarity::Uncommon => 1,
            SkinRarity::Rare => 2,
            SkinRarity::Epic => 3,
            SkinRarity::Legendary => 4,
            SkinRarity::Mythic => 5,
        });
        result
    }

    pub fn duplicate_count(&self, weapon_id: &str, skin: &WeaponSkin) -> u32 {
        self.owned
            .get(weapon_id)
            .and_then(|skins| skins.get(skin))
            .copied()
            .unwrap_or(0)
    }

    /// Sell one duplicate of a skin, returning true if successful.
    /// Only sells if count > 1 (keeps at least one copy).
    pub fn sell_duplicate(&mut self, weapon_id: &str, skin: &WeaponSkin) -> bool {
        if let Some(skins) = self.owned.get_mut(weapon_id) {
            if let Some(count) = skins.get_mut(skin) {
                if *count > 1 {
                    *count -= 1;
                    return true;
                }
            }
        }
        false
    }

    /// Sell a single copy of a skin, removing it entirely if count reaches 0.
    /// Returns true if successful.
    pub fn sell_skin(&mut self, weapon_id: &str, skin: &WeaponSkin) -> bool {
        if let Some(skins) = self.owned.get_mut(weapon_id) {
            if let Some(count) = skins.get_mut(skin) {
                if *count > 0 {
                    *count -= 1;
                    if *count == 0 {
                        skins.remove(skin);
                    }
                    return true;
                }
            }
        }
        false
    }

    /// Sell ALL duplicates (keeps 1 copy of each skin), returns total credits earned
    pub fn sell_all_duplicates(&mut self) -> Vec<(String, WeaponSkin, u32, u64)> {
        let mut sold = Vec::new();
        for (weapon_id, skins) in self.owned.iter_mut() {
            for (skin, count) in skins.iter_mut() {
                if *count > 1 {
                    let extras = *count - 1;
                    let value = PlayerCredits::sell_value(skin.rarity()) * extras as u64;
                    sold.push((weapon_id.clone(), *skin, extras, value));
                    *count = 1;
                }
            }
        }
        sold
    }

    /// Total number of duplicate skins across all weapons
    pub fn total_duplicates(&self) -> u32 {
        let mut total = 0u32;
        for skins in self.owned.values() {
            for count in skins.values() {
                if *count > 1 {
                    total += count - 1;
                }
            }
        }
        total
    }

    pub fn save(&self) {
        let path = "settings/savestate.json";
        
        // Load existing savestate or create new
        let mut savestate: serde_json::Value = match std::fs::read_to_string(path) {
            Ok(content) => serde_json::from_str(&content).unwrap_or_else(|_| serde_json::json!({})),
            Err(_) => serde_json::json!({}),
        };
        
        // Update inventory section
        savestate["inventory"] = serde_json::to_value(self).unwrap();
        
        if let Ok(content) = serde_json::to_string_pretty(&savestate) {
            let _ = std::fs::write(path, content);
        }
    }

    pub fn load() -> Self {
        let path = "settings/savestate.json";
        match std::fs::read_to_string(path) {
            Ok(content) => {
                if let Ok(savestate) = serde_json::from_str::<serde_json::Value>(&content) {
                    if let Some(inventory) = savestate.get("inventory") {
                        return serde_json::from_value(inventory.clone()).unwrap_or_default();
                    }
                }
                Self::default()
            },
            Err(_) => Self::default(),
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

pub struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponRegistry>();
        app.insert_resource(PlayerLoadout::load());
        app.insert_resource(SkinInventory::load());
        app.insert_resource(PlayerCredits::load());
        app.add_systems(PreStartup, load_all_weapon_configs);
        app.add_systems(Update, apply_weapon_skins);
    }
}

fn load_all_weapon_configs(
    mut registry: ResMut<WeaponRegistry>,
    loadout: Res<PlayerLoadout>,
) {
    let weapon_files: Vec<(&str, &str, WeaponSlot)> = vec![
        // Primary - Assault Rifles
        ("hk416", include_str!("../../assets/weapons/data/primary/assault/hk416.json"), WeaponSlot::Primary),
        ("ak-12", include_str!("../../assets/weapons/data/primary/assault/ak-12.json"), WeaponSlot::Primary),
        ("ak-47", include_str!("../../assets/weapons/data/primary/assault/ak-47.json"), WeaponSlot::Primary),
        ("fn_scar_h_std", include_str!("../../assets/weapons/data/primary/assault/fn_scar_h_std.json"), WeaponSlot::Primary),
        ("fn_scar_l_std", include_str!("../../assets/weapons/data/primary/assault/fn_scar_l_std.json"), WeaponSlot::Primary),
        ("sig_sg-553_r", include_str!("../../assets/weapons/data/primary/assault/sig_sg-553_r.json"), WeaponSlot::Primary),
        // Primary - Carbines
        ("aek-971", include_str!("../../assets/weapons/data/primary/carbine/aek-971.json"), WeaponSlot::Primary),
        ("colt_m4a1", include_str!("../../assets/weapons/data/primary/carbine/colt_m4a1.json"), WeaponSlot::Primary),
        ("colt_xm177_car-15", include_str!("../../assets/weapons/data/primary/carbine/colt_xm177_car-15.json"), WeaponSlot::Primary),
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
        // Secondary - Pistols
        ("g17", include_str!("../../assets/weapons/data/secondary/pistol/g17.json"), WeaponSlot::Secondary),
        ("colt_m1911_a1", include_str!("../../assets/weapons/data/secondary/pistol/colt_m1911_a1.json"), WeaponSlot::Secondary),
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

    sync_loadout_to_configs(&mut registry, &loadout);

    info!(
        "Loaded {} weapons ({} primary, {} secondary, {} melee, {} equipment)",
        registry.weapons.len(),
        registry.by_slot.get(&WeaponSlot::Primary).map(|v| v.len()).unwrap_or(0),
        registry.by_slot.get(&WeaponSlot::Secondary).map(|v| v.len()).unwrap_or(0),
        registry.by_slot.get(&WeaponSlot::Melee).map(|v| v.len()).unwrap_or(0),
        registry.by_slot.get(&WeaponSlot::Equipment).map(|v| v.len()).unwrap_or(0),
    );
}

pub fn sync_loadout_to_configs(registry: &mut WeaponRegistry, loadout: &PlayerLoadout) {
    for slot in [WeaponSlot::Primary, WeaponSlot::Secondary, WeaponSlot::Melee, WeaponSlot::Equipment] {
        let id = loadout.get_id_for_slot(slot);
        if let Some(config) = registry.weapons.get(id) {
            registry.configs.insert(slot, config.clone());
        }
    }
}

fn weapon_fallback_mesh(
    slot: WeaponSlot,
    skin: WeaponSkin,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> (Handle<Mesh>, Handle<StandardMaterial>) {
    let mat = if skin != WeaponSkin::Default {
        materials.add(skin.to_material())
    } else {
        match slot {
            WeaponSlot::Primary => materials.add(StandardMaterial {
                base_color: Color::srgb(0.25, 0.25, 0.3),
                metallic: 0.7,
                perceptual_roughness: 0.3,
                ..default()
            }),
            WeaponSlot::Secondary => materials.add(StandardMaterial {
                base_color: Color::srgb(0.2, 0.2, 0.25),
                metallic: 0.7,
                perceptual_roughness: 0.3,
                ..default()
            }),
            WeaponSlot::Melee => materials.add(StandardMaterial {
                base_color: Color::srgb(0.5, 0.5, 0.55),
                metallic: 0.9,
                perceptual_roughness: 0.1,
                ..default()
            }),
            WeaponSlot::Equipment => materials.add(StandardMaterial {
                base_color: Color::srgb(0.3, 0.35, 0.2),
                metallic: 0.3,
                perceptual_roughness: 0.7,
                ..default()
            }),
        }
    };
    let mesh = match slot {
        WeaponSlot::Primary => meshes.add(Cuboid::new(0.08, 0.12, 0.6)),
        WeaponSlot::Secondary => meshes.add(Cuboid::new(0.06, 0.12, 0.3)),
        WeaponSlot::Melee => meshes.add(Cuboid::new(0.04, 0.04, 0.4)),
        WeaponSlot::Equipment => meshes.add(Cuboid::new(0.15, 0.15, 0.15)),
    };
    (mesh, mat)
}

pub fn spawn_weapon_visual(
    commands: &mut Commands,
    slot: WeaponSlot,
    asset_server: &AssetServer,
    registry: &WeaponRegistry,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Entity {
    spawn_weapon_visual_skinned(commands, slot, WeaponSkin::Default, asset_server, registry, meshes, materials)
}

pub fn spawn_weapon_visual_skinned(
    commands: &mut Commands,
    slot: WeaponSlot,
    skin: WeaponSkin,
    asset_server: &AssetServer,
    registry: &WeaponRegistry,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Entity {
    if let Some(config) = registry.configs.get(&slot) {
        let transform = Transform::from_translation(Vec3::from(config.meta.position_offset))
            .with_rotation(Quat::from_euler(EulerRot::XYZ, config.meta.rotation_offset[0], config.meta.rotation_offset[1], config.meta.rotation_offset[2]))
            .with_scale(Vec3::splat(config.meta.scale));

        let model_file = config.meta.model_path.split('#').next().unwrap_or("");
        let model_exists = !model_file.is_empty()
            && std::path::Path::new(&format!("assets/{}", model_file)).exists();

        let entity = if model_exists {
            commands.spawn((
                SceneRoot(asset_server.load(&config.meta.model_path)),
                transform,
                BaseWeaponTransform(transform),
                WeaponRecoil::default(),
                WeaponSkinTag { skin, applied: false },
            )).id()
        } else {
            let (mesh, material) = weapon_fallback_mesh(slot, skin, meshes, materials);
            commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                transform,
                BaseWeaponTransform(transform),
                WeaponRecoil::default(),
                WeaponSkinTag { skin, applied: true },
            )).id()
        };

        // Spawn gray attachment placeholder blocks
        spawn_attachment_placeholders(commands, entity, config, meshes, materials);

        entity
    } else {
        let (mesh, material) = weapon_fallback_mesh(slot, skin, meshes, materials);
        let transform = Transform::from_xyz(0.5, -0.5, -1.0);

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            transform,
            BaseWeaponTransform(transform),
            WeaponRecoil::default(),
            WeaponSkinTag { skin, applied: true },
        )).id()
    }
}

/// Tag component for attachment placeholder blocks.
#[derive(Component)]
pub struct AttachmentPlaceholder {
    pub slot_name: String,
}

/// Spawn gray placeholder blocks at each attachment's meta position_offset.
/// Falls back to computed positions from other meta fields when position_offset is absent.
fn spawn_attachment_placeholders(
    commands: &mut Commands,
    weapon_entity: Entity,
    config: &WeaponConfig,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) {
    let gray_mat = materials.add(StandardMaterial {
        base_color: Color::srgba(0.4, 0.4, 0.45, 0.6),
        alpha_mode: AlphaMode::Blend,
        metallic: 0.3,
        perceptual_roughness: 0.6,
        ..default()
    });

    let attachments = &config.attachments;

    // Helper closure to spawn a block at a position
    let mut spawn_block = |commands: &mut Commands, name: &str, pos: [f32; 3], size: Vec3| {
        let child = commands.spawn((
            Mesh3d(meshes.add(Cuboid::new(size.x, size.y, size.z))),
            MeshMaterial3d(gray_mat.clone()),
            Transform::from_translation(Vec3::from(pos)),
            AttachmentPlaceholder { slot_name: name.to_string() },
        )).id();
        commands.entity(weapon_entity).add_child(child);
    };

    // Optic - fallback to aim_offset (raised slightly)
    if let Some(optic) = &attachments.optic {
        if let Some(meta) = &optic.meta {
            let pos = meta.position_offset.unwrap_or_else(|| {
                if let Some(aim) = meta.aim_offset {
                    [aim[0], aim[1] + 0.05, aim[2]]
                } else {
                    [0.0, 0.08, 0.1]
                }
            });
            spawn_block(commands, "Optic", pos, Vec3::new(0.15, 0.1, 0.15));
        }
    }

    // Barrel - fallback to muzzle_flash_offset (shifted back slightly)
    if let Some(barrel) = &attachments.barrel {
        if let Some(meta) = &barrel.meta {
            let pos = meta.position_offset.unwrap_or_else(|| {
                if let Some(mf) = meta.muzzle_flash_offset {
                    [mf[0] * 0.7, mf[1], mf[2]]
                } else {
                    [0.0, 0.0, -0.3]
                }
            });
            spawn_block(commands, "Barrel", pos, Vec3::new(0.08, 0.08, 0.3));
        }
    }

    // Underbarrel - fallback to below weapon center
    if let Some(ub) = &attachments.underbarrel {
        if let Some(meta) = &ub.meta {
            let pos = meta.position_offset.unwrap_or([0.0, -0.06, 0.1]);
            spawn_block(commands, "Underbarrel", pos, Vec3::new(0.1, 0.08, 0.2));
        }
    }

    // Sidebarrel - fallback to side of weapon
    if let Some(sb) = &attachments.sidebarrel {
        if let Some(meta) = &sb.meta {
            let pos = meta.position_offset.unwrap_or([0.06, 0.0, 0.0]);
            spawn_block(commands, "Sidebarrel", pos, Vec3::new(0.06, 0.06, 0.15));
        }
    }

    // Magazine - fallback to below weapon center
    if let Some(mag) = &attachments.magazine {
        let pos = if let Some(meta) = &mag.meta {
            meta.position_offset.unwrap_or([0.0, -0.1, 0.0])
        } else {
            [0.0, -0.1, 0.0]
        };
        spawn_block(commands, "Magazine", pos, Vec3::new(0.08, 0.15, 0.1));
    }

    // Stock - fallback to rear of weapon
    if let Some(stock) = &attachments.stock {
        if let Some(meta) = &stock.meta {
            let pos = meta.position_offset.unwrap_or([-0.3, 0.0, 0.0]);
            spawn_block(commands, "Stock", pos, Vec3::new(0.1, 0.1, 0.25));
        }
    }
}

pub fn spawn_weapon_visual_by_id(
    commands: &mut Commands,
    weapon_id: &str,
    skin: WeaponSkin,
    asset_server: &AssetServer,
    registry: &WeaponRegistry,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Option<Entity> {
    if let Some(config) = registry.weapons.get(weapon_id) {
        let transform = Transform::from_translation(Vec3::from(config.meta.position_offset))
            .with_rotation(Quat::from_euler(EulerRot::XYZ, config.meta.rotation_offset[0], config.meta.rotation_offset[1], config.meta.rotation_offset[2]))
            .with_scale(Vec3::splat(config.meta.scale));

        let model_file = config.meta.model_path.split('#').next().unwrap_or("");
        let model_exists = !model_file.is_empty()
            && std::path::Path::new(&format!("assets/{}", model_file)).exists();

        if model_exists {
            Some(commands.spawn((
                SceneRoot(asset_server.load(&config.meta.model_path)),
                transform,
                BaseWeaponTransform(transform),
                WeaponRecoil::default(),
            )).id())
        } else {
            let slot = slot_from_weapon_type(&config.meta.weapon_type);
            let (mesh, material) = weapon_fallback_mesh(slot, skin, meshes, materials);
            Some(commands.spawn((
                Mesh3d(mesh),
                MeshMaterial3d(material),
                transform,
                BaseWeaponTransform(transform),
                WeaponRecoil::default(),
            )).id())
        }
    } else {
        None
    }
}

pub fn slot_from_weapon_type(weapon_type: &str) -> WeaponSlot {
    match weapon_type {
        "Melee" | "2H Blade" | "Blade" | "Bayonet" => WeaponSlot::Melee,
        "Grenade" | "Equipment" => WeaponSlot::Equipment,
        "Pistol" | "Revolver" | "Machine Pistol" | "Secondary" => WeaponSlot::Secondary,
        _ => WeaponSlot::Primary,
    }
}

/// System that applies skin materials to all mesh descendants of a weapon entity.
/// Runs every frame but only processes entities where `applied` is false.
fn apply_weapon_skins(
    mut skin_query: Query<(Entity, &mut WeaponSkinTag)>,
    children_query: Query<&Children>,
    mut material_query: Query<&mut MeshMaterial3d<StandardMaterial>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, mut skin_tag) in skin_query.iter_mut() {
        if skin_tag.applied || skin_tag.skin == WeaponSkin::Default {
            skin_tag.applied = true;
            continue;
        }

        // Check if we have any mesh descendants yet (scene may still be loading)
        let mut found_any = false;
        let skin_mat = materials.add(skin_tag.skin.to_material());

        // Recursively find all mesh children
        let mut stack: Vec<Entity> = vec![entity];
        while let Some(current) = stack.pop() {
            if let Ok(mat_handle) = material_query.get_mut(current) {
                found_any = true;
                // Override the material
                let mut mat = mat_handle;
                *mat = MeshMaterial3d(skin_mat.clone());
            }
            if let Ok(children) = children_query.get(current) {
                for child in children.iter() {
                    stack.push(child);
                }
            }
        }

        if found_any {
            skin_tag.applied = true;
        }
    }
}
