use bevy::prelude::*;
use serde::Deserialize;
use std::collections::HashMap;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash)]
pub enum WeaponSlot {
    Primary,
    Secondary,
    Melee,
    Equipment,
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
    pub weight: f32,
    pub length: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WeaponMeta {
    pub model_path: String,
    pub icon_path: String,
    pub position_offset: [f32; 3],
    pub rotation_offset: [f32; 3],
    pub scale: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WeaponAttributes {
    pub fire_rate: f32,
    pub fire_modes: Vec<String>,
    pub recoil_control: f32,
    pub ergonomics: f32,
    pub accuracy: f32,
    pub vertical_recoil: f32,
    pub horizontal_recoil: f32,
    pub reload_speed: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AttachmentMeta {
    #[serde(default)]
    pub model_path: String,
    #[serde(default)]
    pub aim_offset: Option<[f32; 3]>,
    #[serde(default)]
    pub muzzle_flash_offset: Option<[f32; 3]>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpticStats {
    pub zoom_level: f32,
    pub ergonomics_penalty: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct OpticAttachment {
    pub name: String,
    pub meta: Option<AttachmentMeta>,
    pub stats: OpticStats,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BarrelStats {
    pub length: f32,
    pub velocity_mult: f32,
    pub recoil_mult: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct BarrelAttachment {
    pub name: String,
    pub meta: Option<AttachmentMeta>,
    pub stats: BarrelStats,
}

#[derive(Debug, Deserialize, Clone)]
pub struct MagazineAttachment {
    pub name: String,
    pub capacity: u32,
    pub carry_capacity: u32,
    pub reload_speed_mult: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct AmmoAttachment {
    pub name: String,
    pub damage: f32,
    pub penetration: f32,
    pub velocity: f32,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WeaponAttachments {
    pub optic: Option<OpticAttachment>,
    pub barrel: Option<BarrelAttachment>,
    pub magazine: Option<MagazineAttachment>,
    pub ammo: Option<AmmoAttachment>,
}

#[derive(Debug, Deserialize, Clone)]
pub struct WeaponConfig {
    pub info: WeaponInfo,
    pub meta: WeaponMeta,
    pub attributes: WeaponAttributes,
    pub attachments: WeaponAttachments,
}

#[derive(Resource, Default)]
pub struct WeaponRegistry {
    pub configs: HashMap<WeaponSlot, WeaponConfig>,
}

pub struct WeaponsPlugin;

impl Plugin for WeaponsPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<WeaponRegistry>();
        app.add_systems(Startup, load_weapon_configs);
    }
}

fn load_weapon_configs(mut registry: ResMut<WeaponRegistry>) {
    let primary_json = include_str!("../../assets/weapons/data/primary/hk416.json");
    let secondary_json = include_str!("../../assets/weapons/data/secondary/g17.json");
    let melee_json = include_str!("../../assets/weapons/data/melee/msbs_grot_bayonet.json");
    let equipment_json = include_str!("../../assets/weapons/data/equipment/rgd-5.json");
    
    if let Ok(config) = serde_json::from_str::<WeaponConfig>(primary_json) {
        registry.configs.insert(WeaponSlot::Primary, config);
    }
    
    if let Ok(config) = serde_json::from_str::<WeaponConfig>(secondary_json) {
        registry.configs.insert(WeaponSlot::Secondary, config);
    }

    if let Ok(config) = serde_json::from_str::<WeaponConfig>(melee_json) {
        registry.configs.insert(WeaponSlot::Melee, config);
    }

    if let Ok(config) = serde_json::from_str::<WeaponConfig>(equipment_json) {
        registry.configs.insert(WeaponSlot::Equipment, config);
    }
}

pub fn spawn_weapon_visual(
    commands: &mut Commands,
    slot: WeaponSlot,
    asset_server: &AssetServer,
    registry: &WeaponRegistry,
    meshes: &mut Assets<Mesh>,
    materials: &mut Assets<StandardMaterial>,
) -> Entity {
    if let Some(config) = registry.configs.get(&slot) {
        let transform = Transform::from_translation(Vec3::from(config.meta.position_offset))
            .with_rotation(Quat::from_euler(EulerRot::XYZ, config.meta.rotation_offset[0], config.meta.rotation_offset[1], config.meta.rotation_offset[2]))
            .with_scale(Vec3::splat(config.meta.scale));

        commands.spawn((
            SceneRoot(asset_server.load(&config.meta.model_path)),
            transform,
            BaseWeaponTransform(transform),
            WeaponRecoil::default(),
        )).id()
    } else {
        // Fallback for missing configs (Melee/Equipment)
        let (mesh, material) = match slot {
            WeaponSlot::Melee => (
                meshes.add(Cuboid::new(0.1, 0.5, 0.1)),
                materials.add(Color::srgb(0.2, 0.2, 0.8)),
            ),
            WeaponSlot::Equipment => (
                meshes.add(Cuboid::new(0.3, 0.3, 0.3)),
                materials.add(Color::srgb(0.8, 0.8, 0.2)),
            ),
            _ => (
                meshes.add(Cuboid::new(0.2, 0.2, 0.2)),
                materials.add(Color::srgb(1.0, 0.0, 1.0)),
            ),
        };
        
        let transform = Transform::from_xyz(0.5, -0.5, -1.0);

        commands.spawn((
            Mesh3d(mesh),
            MeshMaterial3d(material),
            transform,
            BaseWeaponTransform(transform),
            WeaponRecoil::default(),
        )).id()
    }
}
