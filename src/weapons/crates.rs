//! Data-driven crate definitions and reward rolling.
//!
//! A crate is a JSON file under `assets/cosmetics/crates/`: display info,
//! cost, per-rarity drop weights, an optional thematic skin pool, and a
//! chance to drop a melee mesh unlock instead of a gun skin.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::BTreeMap;

use super::skins::{Inventory, Rarity, SkinId, SkinInstance, SkinRegistry};
use super::{WeaponRegistry, WeaponSlot};

pub const CRATE_DIR: &str = "assets/cosmetics/crates";

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct CrateDef {
    pub id: String,
    pub name: String,
    #[serde(default)]
    pub description: String,
    #[serde(default)]
    pub cost: u64,
    /// UI accent colour.
    #[serde(default = "default_accent")]
    pub accent: [f32; 3],
    /// Bigger weight = more likely. Missing tiers never drop.
    #[serde(default)]
    pub rarity_weights: BTreeMap<Rarity, u32>,
    /// Chance (0..1) that the roll is a melee mesh unlock instead of a skin.
    #[serde(default)]
    pub melee_chance: f32,
    /// Optional theme pool. Empty = any droppable skin.
    #[serde(default)]
    pub skin_pool: Vec<SkinId>,
}

fn default_accent() -> [f32; 3] {
    [0.35, 0.6, 0.9]
}

impl CrateDef {
    pub fn accent_color(&self) -> Color {
        Color::srgb(self.accent[0], self.accent[1], self.accent[2])
    }

    pub fn total_weight(&self) -> u32 {
        self.rarity_weights.values().sum()
    }

    /// Display-ready odds, ordered by rarity.
    pub fn odds(&self) -> Vec<(Rarity, f32)> {
        let total = self.total_weight().max(1) as f32;
        Rarity::ALL
            .iter()
            .filter_map(|rarity| {
                let weight = self.rarity_weights.get(rarity).copied().unwrap_or(0);
                (weight > 0).then_some((*rarity, weight as f32 / total * 100.0))
            })
            .collect()
    }

    /// Candidate skins for a rarity, honouring the crate's theme pool.
    fn skins_of_rarity(&self, rarity: Rarity, skins: &SkinRegistry) -> Vec<SkinId> {
        let base: Vec<SkinId> = if self.skin_pool.is_empty() {
            skins.droppable_of_rarity(rarity)
        } else {
            self.skin_pool
                .iter()
                .filter(|id| skins.rarity_of(id) == rarity && !id.is_default())
                .cloned()
                .collect()
        };
        base
    }
}

/// One crate opening's result.
#[derive(Debug, Clone, PartialEq)]
pub enum CrateReward {
    GunSkin {
        weapon_id: String,
        instance: SkinInstance,
    },
    MeleeUnlock {
        weapon_id: String,
    },
}

impl CrateReward {
    pub fn weapon_id(&self) -> &str {
        match self {
            CrateReward::GunSkin { weapon_id, .. } => weapon_id,
            CrateReward::MeleeUnlock { weapon_id } => weapon_id,
        }
    }
}

#[derive(Resource, Default)]
pub struct CrateRegistry {
    pub crates: Vec<CrateDef>,
}

impl CrateRegistry {
    pub fn load() -> Self {
        Self::load_from(CRATE_DIR)
    }

    pub fn load_from(dir: &str) -> Self {
        let mut registry = Self::default();
        let Ok(entries) = std::fs::read_dir(dir) else {
            debug!("no crate directory at {dir}; using built-in crates");
            return Self::builtin();
        };
        let mut defs = Vec::new();
        for entry in entries.flatten() {
            let path = entry.path();
            if path.extension().and_then(|e| e.to_str()) != Some("json") {
                continue;
            }
            let text = match std::fs::read_to_string(&path) {
                Ok(t) => t,
                Err(e) => {
                    warn!("crate {} unreadable: {e}", path.display());
                    continue;
                }
            };
            match serde_json::from_str::<CrateDef>(&text) {
                Ok(def) => defs.push(def),
                Err(e) => warn!("crate {} failed to parse: {e}", path.display()),
            }
        }
        defs.sort_by_key(|d| d.cost);
        registry.crates = defs;
        if registry.crates.is_empty() {
            registry = Self::builtin();
        }
        registry
    }

    /// Fallback used when the asset directory is missing (tests/headless).
    fn builtin() -> Self {
        let weights = |common: u32, uncommon: u32, rare: u32, epic: u32, legendary: u32, mythic: u32| {
            let mut map = BTreeMap::new();
            map.insert(Rarity::Common, common);
            map.insert(Rarity::Uncommon, uncommon);
            map.insert(Rarity::Rare, rare);
            map.insert(Rarity::Epic, epic);
            map.insert(Rarity::Legendary, legendary);
            map.insert(Rarity::Mythic, mythic);
            map
        };
        Self {
            crates: vec![
                CrateDef {
                    id: "standard".into(),
                    name: "Standard Crate".into(),
                    description: "Basic crate with standard drop rates.".into(),
                    cost: 50,
                    accent: [0.45, 0.5, 0.6],
                    rarity_weights: weights(55, 250, 130, 60, 9, 1),
                    melee_chance: 0.0,
                    skin_pool: Vec::new(),
                },
                CrateDef {
                    id: "tactical".into(),
                    name: "Tactical Crate".into(),
                    description: "Better odds for uncommon and rare skins.".into(),
                    cost: 100,
                    accent: [0.3, 0.7, 0.9],
                    rarity_weights: weights(325, 350, 200, 100, 20, 5),
                    melee_chance: 0.0,
                    skin_pool: Vec::new(),
                },
                CrateDef {
                    id: "elite".into(),
                    name: "Elite Crate".into(),
                    description: "Guaranteed rare or better.".into(),
                    cost: 250,
                    accent: [0.7, 0.3, 0.9],
                    rarity_weights: weights(0, 0, 500, 450, 40, 10),
                    melee_chance: 0.0,
                    skin_pool: Vec::new(),
                },
                CrateDef {
                    id: "legendary".into(),
                    name: "Legendary Crate".into(),
                    description: "Guaranteed epic or better. The best odds.".into(),
                    cost: 500,
                    accent: [0.9, 0.7, 0.2],
                    rarity_weights: weights(0, 0, 0, 850, 130, 20),
                    melee_chance: 0.0,
                    skin_pool: Vec::new(),
                },
            ],
        }
    }

    pub fn get(&self, id: &str) -> Option<&CrateDef> {
        self.crates.iter().find(|c| c.id == id)
    }

    /// Roll a reward: a melee mesh unlock, or a skin on a random gun the
    /// skin fits.
    pub fn roll_reward(
        &self,
        def: &CrateDef,
        skins: &SkinRegistry,
        weapons: &WeaponRegistry,
        inventory: &Inventory,
    ) -> CrateReward {
        use rand::Rng;
        let mut rng = rand::rng();

        if def.melee_chance > 0.0 && rng.random::<f32>() < def.melee_chance {
            let unlocked: Vec<String> = melee_weapon_ids(weapons)
                .into_iter()
                .filter(|id| {
                    !inventory.has_melee(id) && !super::melee::is_default_melee(id)
                })
                .collect();
            if !unlocked.is_empty() {
                let pick = rng.random_range(0..unlocked.len());
                return CrateReward::MeleeUnlock {
                    weapon_id: unlocked[pick].clone(),
                };
            }
        }

        // Pick a rarity by weight, then a skin of that rarity.
        let skin = roll_skin(def, skins, &mut rng);

        // Choose the gun this skin lands on.
        let fits = skins.def_or_default(&skin);
        let candidates: Vec<String> = if fits.is_universal() {
            skinnable_weapon_ids(weapons)
        } else {
            fits.weapons
                .iter()
                .filter(|id| weapons.weapons.contains_key(*id))
                .cloned()
                .collect()
        };
        let weapon_id = if candidates.is_empty() {
            "colt_m4a1".to_string()
        } else {
            candidates[rng.random_range(0..candidates.len())].clone()
        };

        CrateReward::GunSkin {
            weapon_id,
            instance: SkinInstance::roll(skin),
        }
    }
}

/// Roll a skin using the crate's rarity weights and theme pool.
pub fn roll_skin(def: &CrateDef, skins: &SkinRegistry, rng: &mut impl rand::Rng) -> SkinId {
    let mut pool: Vec<(Rarity, u32)> = def
        .rarity_weights
        .iter()
        .filter(|(_, w)| **w > 0)
        .map(|(r, w)| (*r, *w))
        .collect();
    if pool.is_empty() {
        pool = Rarity::ALL.iter().map(|r| (*r, r.drop_weight())).collect();
    }

    // Try the rolled rarity, walking up/down the ladder if the crate's pool
    // has no skins there so a roll never comes back empty.
    let total: u32 = pool.iter().map(|(_, w)| *w).sum();
    let mut roll = rng.random_range(0..total.max(1));
    let mut chosen = pool.first().map(|(r, _)| *r).unwrap_or(Rarity::Common);
    for (rarity, weight) in &pool {
        if roll < *weight {
            chosen = *rarity;
            break;
        }
        roll -= *weight;
    }

    let mut candidates = def.skins_of_rarity(chosen, skins);
    if candidates.is_empty() {
        for rarity in Rarity::ALL.iter().rev() {
            candidates = def.skins_of_rarity(*rarity, skins);
            if !candidates.is_empty() {
                break;
            }
        }
    }
    if candidates.is_empty() {
        return SkinId::default_id();
    }
    candidates[rng.random_range(0..candidates.len())].clone()
}

/// Gun ids that can receive skins from crates: primary and secondary only.
/// Melee is deliberately excluded for now — knives are not crate drops
/// (melee unlocks/skins are parked until the melee rework), so a roll can
/// never hand out a skin for a knife the player cannot use.
pub fn skinnable_weapon_ids(weapons: &WeaponRegistry) -> Vec<String> {
    let mut ids: Vec<String> = Vec::new();
    for slot in [WeaponSlot::Primary, WeaponSlot::Secondary] {
        if let Some(list) = weapons.by_slot.get(&slot) {
            ids.extend(list.iter().cloned());
        }
    }
    ids.sort();
    ids.dedup();
    ids
}

pub fn melee_weapon_ids(weapons: &WeaponRegistry) -> Vec<String> {
    let mut ids = weapons
        .by_slot
        .get(&WeaponSlot::Melee)
        .cloned()
        .unwrap_or_default();
    ids.sort();
    ids
}

/// Where owned items live after a roll: applies the reward, returning how
/// many copies of that skin the player already owned (0 = new).
pub fn grant_reward(reward: &CrateReward, inventory: &mut Inventory) -> Result<u32, String> {
    match reward {
        CrateReward::GunSkin {
            weapon_id,
            instance,
        } => {
            let owned = inventory.skin_count(weapon_id, &instance.skin);
            inventory.add_gun_skin(
                weapon_id,
                instance.skin.clone(),
                instance.wear,
                instance.seed,
                instance.tint,
            );
            Ok(owned)
        }
        CrateReward::MeleeUnlock { weapon_id } => {
            inventory.add_melee_unlock(weapon_id);
            Ok(0)
        }
    }
}

#[cfg(test)]
mod tests {
    use super::*;

    fn dummy_skins() -> SkinRegistry {
        // registry with a couple of skins in different tiers
        let mut reg = SkinRegistry::default();
        reg.skins.insert(
            SkinId::default_id(),
            super::super::SkinDef {
                id: SkinId::default_id(),
                name: "Default".into(),
                rarity: Rarity::Common,
                weapons: vec![],
                finish: Default::default(),
            },
        );
        for (id, rarity) in [("common_one", Rarity::Common), ("mythic_one", Rarity::Mythic)] {
            reg.skins.insert(
                SkinId::new(id),
                super::super::SkinDef {
                    id: SkinId::new(id),
                    name: id.into(),
                    rarity,
                    weapons: vec![],
                    finish: Default::default(),
                },
            );
        }
        reg
    }

    #[test]
    fn odds_sum_to_hundred() {
        let registry = CrateRegistry::builtin();
        let standard = registry.get("standard").unwrap();
        let sum: f32 = standard.odds().iter().map(|(_, p)| p).sum();
        assert!((sum - 100.0).abs() < 0.01, "odds sum {sum}");
        let elite = registry.get("elite").unwrap();
        assert!(elite.odds().iter().all(|(r, _)| *r >= Rarity::Rare));
    }

    #[test]
    fn roll_respects_theme_pool_and_never_empty() {
        let registry = CrateRegistry::builtin();
        let mut def = registry.get("standard").unwrap().clone();
        def.rarity_weights.clear();
        def.rarity_weights.insert(Rarity::Mythic, 1);
        def.skin_pool = vec![SkinId::new("common_one")];

        // No mythic skin in the pool -> falls back to a real skin.
        let mut pool_seen = false;
        for _ in 0..32 {
            let picked = roll_skin(&def, &dummy_skins(), &mut rand::rng());
            assert_ne!(picked, SkinId::default_id());
            if picked == SkinId::new("common_one") {
                pool_seen = true;
            }
        }
        assert!(pool_seen);
    }

    #[test]
    fn shipped_skin_and_crate_assets_are_consistent() {
        let skin_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/cosmetics/weapons/skins");
        let crate_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/cosmetics/crates");
        let skins = SkinRegistry::load_from(skin_dir);
        let crates = CrateRegistry::load_from(crate_dir);

        assert!(
            skins.skins.len() >= 24,
            "expected shipped skins, got {}",
            skins.skins.len()
        );
        assert!(crates.crates.len() >= 5, "expected shipped crates");

        for def in &crates.crates {
            assert!(!def.odds().is_empty(), "crate {} has no odds", def.id);
            for id in &def.skin_pool {
                assert!(
                    skins.get(id).is_some(),
                    "crate {} references unknown skin {id}",
                    def.id
                );
            }
        }

        // Every patterned skin's atlas must exist on disk.
        let assets = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/");
        for def in skins.skins.values() {
            if let Some(pattern) = &def.finish.pattern {
                let path = format!("{assets}{}", pattern.sheet);
                assert!(
                    std::path::Path::new(&path).exists(),
                    "skin {} atlas {} missing",
                    def.id,
                    pattern.sheet
                );
            }
        }
    }

    #[test]
    fn grant_reward_applies_skin_and_melee() {
        let mut inv = Inventory::default();
        let reward = CrateReward::GunSkin {
            weapon_id: "g17".into(),
            instance: SkinInstance {
                skin: SkinId::new("solid_red"),
                wear: 0.25,
                seed: 99,
                tint: None,
            },
        };
        assert_eq!(grant_reward(&reward, &mut inv).unwrap(), 0);
        assert_eq!(inv.skin_count("g17", &SkinId::new("solid_red")), 1);
        assert_eq!(grant_reward(&reward, &mut inv).unwrap(), 1);

        let melee = CrateReward::MeleeUnlock {
            weapon_id: "m9_bayonet".into(),
        };
        assert_eq!(grant_reward(&melee, &mut inv).unwrap(), 0);
        assert!(inv.has_melee("m9_bayonet"));
    }

    /// Knives are parked for the melee rework: crate skin rolls must never
    /// land on a melee weapon, so a drop can't hand out an unusable skin.
    #[test]
    fn skinnable_weapon_ids_exclude_melee() {
        let mut weapons = WeaponRegistry::default();
        weapons
            .by_slot
            .insert(WeaponSlot::Primary, vec!["hk416".into()]);
        weapons
            .by_slot
            .insert(WeaponSlot::Secondary, vec!["g17".into()]);
        weapons.by_slot.insert(
            WeaponSlot::Melee,
            vec!["km2000".into(), "axe".into(), "6x4_bayonet".into()],
        );
        let ids = skinnable_weapon_ids(&weapons);
        assert!(ids.contains(&"hk416".to_string()));
        assert!(ids.contains(&"g17".to_string()));
        assert!(
            !ids.iter().any(|id| id == "km2000" || id == "axe" || id == "6x4_bayonet"),
            "no knives: {ids:?}"
        );
    }

    /// The data side of the knife freeze: no shipped crate may roll a melee
    /// unlock either.
    #[test]
    fn shipped_crates_never_drop_melee() {
        let crate_dir = concat!(env!("CARGO_MANIFEST_DIR"), "/assets/cosmetics/crates");
        let crates = CrateRegistry::load_from(crate_dir);
        assert!(crates.crates.len() >= 5, "expected shipped crates");
        for def in &crates.crates {
            assert_eq!(
                def.melee_chance, 0.0,
                "crate {} still drops melee unlocks",
                def.id
            );
        }
    }
}
