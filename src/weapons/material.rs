//! Weapon skin material: a `StandardMaterial` extended with per-copy wear,
//! pattern seeds, animated emissive/UV scroll, and procedural galaxy
//! nebulae. Built once per equipped skin instance and shared by every mesh
//! of that weapon (gun body, attachments, viewmodel, preview, melee).
//!
//! # Safety switches
//!
//! The advanced (custom-shader) path can be disabled at runtime, because a
//! custom PBR fragment shader is the kind of thing that can hang a buggy
//! GPU driver. Defaults are conservative:
//!
//! * `NOCTYRN_SKIN_MAT=on|advanced|1` — use the custom material. Anything
//!   else (the default) uses plain `StandardMaterial` finishes.
//! * `NOCTYRN_SKIN_FEATURES=all|none|wear|galaxy|pattern|anim` — comma-free
//!   single feature or `all`/`none`. Turns individual shader features into
//!   `#ifdef` shader defs so one build can bisect a driver hang.
//! * `NOCTYRN_SKIN_WARMUP=0` — skip the startup warmup draw.

use bevy::pbr::{ExtendedMaterial, MaterialExtension, MaterialExtensionKey, MaterialExtensionPipeline};
use bevy::prelude::*;
use bevy::render::render_resource::AsBindGroup;
use bevy::render::render_resource::SpecializedMeshPipelineError;
use bevy::shader::ShaderRef;
use std::sync::OnceLock;

pub type WeaponMaterial = ExtendedMaterial<StandardMaterial, WeaponSkinExt>;

pub const WEAPON_SKIN_SHADER: &str = "shaders/weapon_skin.wgsl";

/// Which material path weapon skins use.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum SkinMaterialMode {
    /// Plain `StandardMaterial` finishes: base colour, texture, normal map,
    /// steady emissive. No wear, galaxy, pattern stamping or animation.
    Plain,
    /// Custom shader material with the full finish feature set.
    Advanced,
}

/// Resolved once from `NOCTYRN_SKIN_MAT`. Defaults to `Advanced` (wear,
/// galaxy, pattern stamping, animation); set `NOCTYRN_SKIN_MAT=off` (or
/// `plain`) to fall back to plain `StandardMaterial` finishes if a driver
/// has trouble with the custom shader.
pub fn skin_material_mode() -> SkinMaterialMode {
    static MODE: OnceLock<SkinMaterialMode> = OnceLock::new();
    *MODE.get_or_init(|| {
        let plain = std::env::var("NOCTYRN_SKIN_MAT")
            .map(|v| matches!(v.as_str(), "off" | "plain" | "0" | "false"))
            .unwrap_or(false);
        if plain {
            SkinMaterialMode::Plain
        } else {
            SkinMaterialMode::Advanced
        }
    })
}

/// Individual shader features, resolved once from `NOCTYRN_SKIN_FEATURES`.
#[derive(Debug, Clone, Copy)]
pub struct SkinFeatures {
    pub wear: bool,
    pub galaxy: bool,
    pub pattern: bool,
    pub gradient: bool,
    pub anim: bool,
}

impl SkinFeatures {
    pub const ALL: SkinFeatures = SkinFeatures {
        wear: true,
        galaxy: true,
        pattern: true,
        gradient: true,
        anim: true,
    };

    pub const NONE: SkinFeatures = SkinFeatures {
        wear: false,
        galaxy: false,
        pattern: false,
        gradient: false,
        anim: false,
    };
}

/// Resolved once from `NOCTYRN_SKIN_FEATURES` (`all`, `none`, or a
/// comma-separated subset of `wear`, `galaxy`, `pattern`, `anim`).
/// Defaults to all features.
pub fn skin_features() -> SkinFeatures {
    static FEATURES: OnceLock<SkinFeatures> = OnceLock::new();
    *FEATURES.get_or_init(|| {
        let value = std::env::var("NOCTYRN_SKIN_FEATURES").unwrap_or_else(|_| "all".to_string());
        let value = value.trim().to_ascii_lowercase();
        match value.as_str() {
            "all" | "" => return SkinFeatures::ALL,
            "none" => return SkinFeatures::NONE,
            _ => {}
        }
        let mut features = SkinFeatures::NONE;
        for part in value.split(',') {
            match part.trim() {
                "wear" => features.wear = true,
                "galaxy" => features.galaxy = true,
                "pattern" => features.pattern = true,
                "gradient" => features.gradient = true,
                "anim" => features.anim = true,
                other => warn!("unknown NOCTYRN_SKIN_FEATURES entry '{other}'"),
            }
        }
        features
    })
}

/// `NOCTYRN_SKIN_WARMUP=0` disables the startup warmup draw.
pub fn skin_warmup_enabled() -> bool {
    static ENABLED: OnceLock<bool> = OnceLock::new();
    *ENABLED.get_or_init(|| {
        std::env::var("NOCTYRN_SKIN_WARMUP")
            .map(|v| !matches!(v.as_str(), "0" | "off" | "false"))
            .unwrap_or(true)
    })
}

/// Extension data. Field order must match the `WeaponSkinExt` struct in
/// `weapon_skin.wgsl` (all bindings are packed into one uniform).
#[derive(Asset, TypePath, AsBindGroup, Debug, Clone)]
pub struct WeaponSkinExt {
    /// x = wear (0..1), y = time seconds, z = pattern seed (low bits),
    /// w = galaxy enabled (0/1)
    #[uniform(100)]
    pub params: Vec4,
    /// x = emissive pulse hz (0 = steady), y = emissive strength,
    /// z/w = uv scroll per second
    #[uniform(100)]
    pub anim: Vec4,
    #[uniform(100)]
    pub palette_a: Vec4,
    #[uniform(100)]
    pub palette_b: Vec4,
    #[uniform(100)]
    pub palette_c: Vec4,
    /// x = nebula density, y = scroll speed, z = star amount,
    /// w = layer parallax
    #[uniform(100)]
    pub galaxy: Vec4,
    /// x = star density (cells/unit), y = twinkle amount, z = core/plane
    /// glow, w = star layers (1..4)
    #[uniform(100)]
    pub galaxy_b: Vec4,
    /// x = atlas cols, y = atlas rows, z = pattern strength, w = enabled
    #[uniform(100)]
    pub pattern: Vec4,
    /// Gradient tint (rgb) plus axis flag in `w` (0 = x, 1 = y).
    /// Enabled by `extra.x`.
    #[uniform(100)]
    pub gradient: Vec4,
    /// x = gradient enabled, y = pattern uses its fixed offset (else the
    /// per-copy seed picks the stamp), z/w unused.
    #[uniform(100)]
    pub extra: Vec4,
    #[texture(101)]
    #[sampler(102)]
    pub pattern_atlas: Option<Handle<Image>>,
}

impl Default for WeaponSkinExt {
    fn default() -> Self {
        Self {
            params: Vec4::new(0.0, 0.0, 0.0, 0.0),
            anim: Vec4::ZERO,
            palette_a: Vec4::new(0.3, 0.1, 0.6, 1.0),
            palette_b: Vec4::new(0.1, 0.2, 0.7, 1.0),
            palette_c: Vec4::new(0.9, 0.4, 0.8, 1.0),
            galaxy: Vec4::ZERO,
            galaxy_b: Vec4::ZERO,
            pattern: Vec4::ZERO,
            gradient: Vec4::ZERO,
            extra: Vec4::ZERO,
            pattern_atlas: None,
        }
    }
}

impl MaterialExtension for WeaponSkinExt {
    fn fragment_shader() -> ShaderRef {
        WEAPON_SKIN_SHADER.into()
    }

    /// The same shader handles the deferred path (it branches on
    /// `PREPASS_PIPELINE`), matching Bevy's official extended-material
    /// example.
    fn deferred_fragment_shader() -> ShaderRef {
        WEAPON_SKIN_SHADER.into()
    }

    fn specialize(
        _pipeline: &MaterialExtensionPipeline,
        descriptor: &mut bevy::render::render_resource::RenderPipelineDescriptor,
        _layout: &bevy::mesh::MeshVertexBufferLayoutRef,
        _key: MaterialExtensionKey<Self>,
    ) -> Result<(), SpecializedMeshPipelineError> {
        let features = skin_features();
        if let Some(fragment) = descriptor.fragment.as_mut() {
            if features.wear {
                fragment.shader_defs.push("SKIN_WEAR".into());
            }
            if features.galaxy {
                fragment.shader_defs.push("SKIN_GALAXY".into());
            }
            if features.pattern {
                fragment.shader_defs.push("SKIN_PATTERN".into());
            }
            if features.gradient {
                fragment.shader_defs.push("SKIN_GRADIENT".into());
            }
            if features.anim {
                fragment.shader_defs.push("SKIN_ANIM".into());
            }
        }
        Ok(())
    }
}
