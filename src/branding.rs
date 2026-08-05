use bevy::asset::LoadState;
use bevy::image::{TextureAtlas, TextureAtlasLayout};
use bevy::prelude::*;

use crate::maps;
use crate::player::GameState;
use crate::world::SelectedMapId;

/// Branded splash + loading screens.
///
/// Both screens show the Noctyrn logo animation baked from
/// `noctyrn-extras/assets/ui/animation.svg` (see `noctyrn-extras/bake`):
/// hold the first frame 0.5s, play the 30-frame animation over 1.0s, then
/// hold the final frame 0.5s.
pub struct BrandingPlugin;

/// Animation timeline constants.
pub const FIRST_HOLD: f32 = 0.5;
pub const ANIM_PLAY: f32 = 1.0;
pub const LAST_HOLD: f32 = 0.5;
pub const ANIM_TOTAL: f32 = FIRST_HOLD + ANIM_PLAY + LAST_HOLD;

/// Sprite sheet layout (matches the output of the bake tool).
pub const SHEET_PATH: &str = "ui/noctyrn_anim_sheet.png";
pub const SHEET_FRAMES: u32 = 30;
pub const SHEET_COLS: u32 = 6;
pub const SHEET_ROWS: u32 = 5;
pub const SHEET_CELL: UVec2 = UVec2::new(1024, 171);

/// Brand background color (#141018).
pub const BRAND_BG: Color = crate::theme::BG_BASE;

/// Marker on the fullscreen root of a branded screen.
#[derive(Component)]
pub struct BrandedScreen;

/// Timeline for the logo animation on a branded screen.
#[derive(Component)]
pub struct BrandTimeline {
    pub elapsed: f32,
}

impl Default for BrandTimeline {
    fn default() -> Self {
        Self { elapsed: 0.0 }
    }
}

/// Marker on the animated logo node.
#[derive(Component)]
pub struct BrandLogo;

/// The map GLB being pre-loaded by the loading screen, if any.
/// `None` means the map is procedural and has nothing to wait on.
#[derive(Resource, Default)]
pub struct PendingMapLoad {
    pub scene: Option<Handle<WorldAsset>>,
}

/// The lobby scene pre-loaded by the splash screen so the main menu
/// background appears instantly.
#[derive(Resource, Default)]
pub struct PendingBootLoad {
    pub lobby: Option<Handle<WorldAsset>>,
}

/// Where the loading screen is headed when it finishes.
#[derive(Debug, Clone, Copy, PartialEq, Eq)]
pub enum LoadingTarget {
    /// Joining a match — wait for the map assets, then enter gameplay.
    IntoMatch,
    /// Leaving a match — nothing to load, just play the animation.
    BackToMenu,
}

#[derive(Resource)]
pub struct PendingLoadingTarget(pub LoadingTarget);

impl Default for PendingLoadingTarget {
    fn default() -> Self {
        Self(LoadingTarget::IntoMatch)
    }
}

impl Plugin for BrandingPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<PendingMapLoad>()
            .init_resource::<PendingBootLoad>()
            .init_resource::<PendingLoadingTarget>()
            .add_systems(OnEnter(GameState::Splash), spawn_splash)
            .add_systems(OnExit(GameState::Splash), despawn_branded_screens)
            .add_systems(OnEnter(GameState::Loading), spawn_loading)
            .add_systems(OnExit(GameState::Loading), despawn_branded_screens)
            .add_systems(
                Update,
                (
                    advance_brand_timeline,
                    splash_finish.run_if(in_state(GameState::Splash)),
                    loading_finish.run_if(in_state(GameState::Loading)),
                ),
            );
    }
}

/// Frame index for a given elapsed time on the animation timeline.
fn frame_at(t: f32) -> usize {
    if t < FIRST_HOLD {
        0
    } else if t < FIRST_HOLD + ANIM_PLAY {
        let p = (t - FIRST_HOLD) / ANIM_PLAY;
        ((p * SHEET_FRAMES as f32).floor() as usize).min(SHEET_FRAMES as usize - 1)
    } else {
        SHEET_FRAMES as usize - 1
    }
}

/// The splash screen: fullscreen branding + logo animation. Warms up and
/// waits for the lobby scene so the main menu background appears instantly.
fn spawn_splash(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_assets: ResMut<Assets<TextureAtlasLayout>>,
    mut boot: ResMut<PendingBootLoad>,
) {
    spawn_branded_screen(&mut commands, &asset_server, &mut atlas_assets);
    boot.lobby = Some(asset_server.load::<WorldAsset>("maps/lobby.glb#Scene0"));
}

/// The loading screen: same branding, plus a pre-load of the match map's
/// GLB so the transition into `Playing` is instant. When leaving a match
/// (`LoadingTarget::BackToMenu`) there is nothing to pre-load.
fn spawn_loading(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut atlas_assets: ResMut<Assets<TextureAtlasLayout>>,
    selected_map: Res<SelectedMapId>,
    mut pending: ResMut<PendingMapLoad>,
    target: Res<PendingLoadingTarget>,
) {
    spawn_branded_screen(&mut commands, &asset_server, &mut atlas_assets);

    // GLB maps load asynchronously; procedural maps have nothing to load.
    pending.scene = match target.0 {
        LoadingTarget::BackToMenu => None,
        LoadingTarget::IntoMatch => match selected_map.0.as_str() {
            "dust_storm" | "city" => {
                Some(asset_server.load::<WorldAsset>(&maps::config::load(&selected_map.0).glb))
            }
            _ => None,
        },
    };
}

fn spawn_branded_screen(
    commands: &mut Commands,
    asset_server: &AssetServer,
    atlas_assets: &mut Assets<TextureAtlasLayout>,
) -> Entity {
    let sheet: Handle<Image> = asset_server.load(SHEET_PATH);
    let layout = atlas_assets.add(TextureAtlasLayout::from_grid(
        SHEET_CELL,
        SHEET_COLS,
        SHEET_ROWS,
        None,
        None,
    ));

    let mut root = commands.spawn((
        Node {
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            ..default()
        },
        BackgroundColor(BRAND_BG),
        BrandedScreen,
    ));
    root.with_child((
        ImageNode::from_atlas_image(
            sheet,
            TextureAtlas {
                layout,
                index: 0,
            },
        ),
        Node {
            width: Val::Percent(90.0),
            aspect_ratio: Some(SHEET_CELL.x as f32 / SHEET_CELL.y as f32),
            ..default()
        },
        BrandTimeline::default(),
        BrandLogo,
    ));
    // Branded screens always bring their own camera — at boot and when
    // leaving a match there is no other camera alive.
    root.with_child((
        Camera2d::default(),
        Camera {
            order: 1,
            clear_color: ClearColorConfig::Custom(BRAND_BG),
            ..default()
        },
    ));
    root.id()
}

fn despawn_branded_screens(
    mut commands: Commands,
    screens: Query<Entity, With<BrandedScreen>>,
) {
    for entity in &screens {
        commands.entity(entity).despawn();
    }
}

fn advance_brand_timeline(
    time: Res<Time>,
    mut logos: Query<(&mut BrandTimeline, &mut ImageNode), With<BrandLogo>>,
) {
    for (mut timeline, mut image) in &mut logos {
        timeline.elapsed += time.delta_secs();
        if let Some(atlas) = image.texture_atlas.as_mut() {
            atlas.index = frame_at(timeline.elapsed);
        }
    }
}

/// Splash → MainMenu once the animation has fully played AND the lobby
/// scene is ready, so the menu background appears instantly.
fn splash_finish(
    mut next_state: ResMut<NextState<GameState>>,
    asset_server: Res<AssetServer>,
    boot: Res<PendingBootLoad>,
    timelines: Query<&BrandTimeline>,
) {
    let animation_done = timelines.iter().all(|t| t.elapsed >= ANIM_TOTAL);
    if !animation_done {
        return;
    }
    let lobby_ready = match &boot.lobby {
        None => true,
        Some(handle) => match asset_server.get_load_states(handle.id()) {
            Some((LoadState::Loaded, _, _)) | Some((LoadState::Failed(_), _, _)) => true,
            _ => false,
        },
    };
    if lobby_ready {
        next_state.set(GameState::MainMenu);
    }
}

/// Loading → Playing (into a match) or → MainMenu (leaving a match) once the
/// animation has fully played AND the map assets are ready, if applicable.
fn loading_finish(
    mut next_state: ResMut<NextState<GameState>>,
    asset_server: Res<AssetServer>,
    pending: Res<PendingMapLoad>,
    target: Res<PendingLoadingTarget>,
    timelines: Query<&BrandTimeline>,
) {
    let animation_done = timelines.iter().all(|t| t.elapsed >= ANIM_TOTAL);
    if !animation_done {
        return;
    }
    let map_ready = match &pending.scene {
        None => true,
        Some(handle) => match asset_server.get_load_states(handle.id()) {
            Some((LoadState::Loaded, _, _)) | Some((LoadState::Failed(_), _, _)) => true,
            _ => false,
        },
    };
    if map_ready {
        let destination = match target.0 {
            LoadingTarget::IntoMatch => GameState::Playing,
            LoadingTarget::BackToMenu => GameState::MainMenu,
        };
        next_state.set(destination);
    }
}
