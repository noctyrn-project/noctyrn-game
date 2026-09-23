//! Melee archetypes.
//!
//! Every melee mesh is a collectible variant of one of four stat
//! archetypes: one/two-handed blade and one/two-handed blunt. The
//! archetype owns the stats; the per-weapon JSON only points at one and
//! supplies the mesh. Adding a melee weapon is adding a mesh.

use bevy::prelude::*;
use serde::{Deserialize, Serialize};
use std::collections::HashMap;

use super::WeaponRegistry;

/// Meshes every player owns; all other melee meshes are crate drops.
pub const DEFAULT_MELEE: [&str; 2] = ["km2000", "axe"];

pub fn is_default_melee(weapon_id: &str) -> bool {
    DEFAULT_MELEE.contains(&weapon_id)
}

#[derive(Debug, Clone, Serialize, Deserialize)]
pub struct MeleeArchetype {
    pub name: String,
    pub attack_speed: f32,
    pub equip_speed: f32,
    pub stab_damage: f32,
    pub slash_damage: f32,
    pub back_stab_multiplier: f32,
    pub reach: f32,
    pub mobility: f32,
    /// Wider (lower) dot-product cone for swings.
    pub slash_cone: f32,
    /// Narrower (higher) cone for thrusts.
    pub stab_cone: f32,
}

#[derive(Resource, Default)]
pub struct MeleeArchetypes {
    pub map: HashMap<String, MeleeArchetype>,
}

impl MeleeArchetypes {
    pub fn load() -> Self {
        let text = include_str!("../../assets/weapons/data/melee/archetypes.json");
        match serde_json::from_str::<HashMap<String, MeleeArchetype>>(text) {
            Ok(map) => Self { map },
            Err(e) => {
                warn!("failed to parse melee archetypes: {e}");
                Self::default()
            }
        }
    }

    pub fn get(&self, id: &str) -> Option<&MeleeArchetype> {
        self.map.get(id)
    }
}

/// Overwrite every melee config's attributes from its archetype. Archetypes
/// are authoritative: per-weapon JSON attributes are ignored for melee.
pub fn apply_archetypes(registry: &mut WeaponRegistry, archetypes: &MeleeArchetypes) {
    let mut applied = 0;
    for (id, config) in registry.weapons.iter_mut() {
        if config.meta.archetype.is_empty() {
            continue;
        }
        let Some(arch) = archetypes.get(&config.meta.archetype) else {
            warn!("weapon '{id}' references unknown melee archetype '{}'", config.meta.archetype);
            continue;
        };
        config.attributes.attack_speed = arch.attack_speed;
        config.attributes.equip_speed = arch.equip_speed;
        config.attributes.stab_damage = arch.stab_damage;
        config.attributes.slash_damage = arch.slash_damage;
        config.attributes.back_stab_multiplier = arch.back_stab_multiplier;
        config.attributes.reach = arch.reach;
        config.attributes.mobility = arch.mobility;
        config.attributes.slash_cone = arch.slash_cone;
        config.attributes.stab_cone = arch.stab_cone;
        applied += 1;
    }
    info!(
        "Applied melee archetypes to {applied} weapons ({} archetypes)",
        archetypes.map.len()
    );
}

/// Archetype ids that have no mesh yet, in ladder order (for future models).
pub fn archetype_ids() -> [&'static str; 4] {
    ["1h_blade", "2h_blade", "1h_blunt", "2h_blunt"]
}

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn archetypes_override_melee_config_stats() {
        let json = include_str!("../../assets/weapons/data/melee/km2000.json");
        let config: super::super::WeaponConfig = serde_json::from_str(json).unwrap();
        assert_eq!(config.meta.archetype, "1h_blade");

        let mut registry = WeaponRegistry::default();
        registry.weapons.insert("km2000".to_string(), config);
        apply_archetypes(&mut registry, &MeleeArchetypes::load());

        let stats = &registry.weapons.get("km2000").unwrap().attributes;
        let arch = MeleeArchetypes::load();
        let arch = arch.get("1h_blade").unwrap();
        assert!((stats.stab_damage - arch.stab_damage).abs() < 1e-6);
        assert!((stats.reach - arch.reach).abs() < 1e-6);
        assert!((stats.back_stab_multiplier - arch.back_stab_multiplier).abs() < 1e-6);
        assert!((stats.slash_cone - arch.slash_cone).abs() < 1e-6);
        assert!(stats.slash_cone < stats.stab_cone);
    }

    #[test]
    fn default_melee_helpers() {
        assert!(is_default_melee("km2000"));
        assert!(is_default_melee("axe"));
        assert!(!is_default_melee("m9_bayonet"));
    }

    #[test]
    fn archetypes_parse_and_have_sane_stats() {
        let archetypes = MeleeArchetypes::load();
        for id in archetype_ids() {
            let arch = archetypes.get(id).unwrap_or_else(|| panic!("missing {id}"));
            assert!(arch.attack_speed > 0.0, "{id} attack speed");
            assert!(arch.reach > 0.5, "{id} reach");
            assert!(arch.stab_damage > 0.0 && arch.slash_damage > 0.0, "{id} damage");
            assert!(arch.back_stab_multiplier >= 1.0, "{id} backstab");
            assert!(arch.slash_cone < arch.stab_cone, "{id} cone ordering");
        }
        // Two-handed blades swing harder and reach further than one-handed.
        let one = archetypes.get("1h_blade").unwrap();
        let two = archetypes.get("2h_blade").unwrap();
        assert!(two.slash_damage > one.slash_damage);
        assert!(two.reach > one.reach);
    }
}
