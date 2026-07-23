use bevy::prelude::*;
use serde::{Serialize, Deserialize};
use crate::player::GameState;
use crate::player::shooting::Projectile;
use bevy::ecs::relationship::Relationship;
use crate::ui_config::UiConfig;
use crate::menu::{GameMode, SelectedGameMode};
use crate::weapons::PlayerCredits;
use crate::player::{MainCamera, PhysicalTranslation, PreviousPhysicalTranslation, Velocity};
use crate::gamemodes::team_deathmatch::TeamSpawnArea;
use rand::Rng;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Match State & Scoring
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

/// Tracks the current match state: score, timer, objectives.
#[derive(Resource, Debug)]
pub struct MatchState {
    pub mode: GameMode,
    pub player_score: i32,
    pub enemy_score: i32,
    pub kills: u32,
    pub deaths: u32,
    pub assists: u32,
    pub match_timer: Timer,
    pub score_limit: i32,
    pub round: u32,
    pub match_over: bool,
    pub objective_progress: f32,   // 0.0–1.0 for zone-based modes
    pub objective_held: bool,      // Whether player holds objective
    pub dog_tags_collected: u32,   // Kill Confirmed
    pub flags_captured: u32,       // CTF
    pub hardpoint_time: f32,       // Time spent on hardpoint
    pub xp_earned: u64,            // XP earned this match (points)
    pub capture_timers: Vec<f32>,  // Per-zone capture progress (KOTH/HP)
    pub capture_owners: Vec<i8>,   // -1 = enemy, 0 = neutral, 1 = player
    pub cp_positions: Vec<Vec3>,   // For CapturePoint moving zones
    pub cp_move_timer: Timer,      // Timer for CP zone relocation
}

impl MatchState {
    pub fn new(mode: GameMode) -> Self {
        let (score_limit, duration) = match mode {
            GameMode::FreeForAll => (30, 600.0),
            GameMode::TeamDeathmatch => (150, 600.0),
            GameMode::KillConfirmed => (100, 600.0),
            GameMode::CaptureTheFlag => (3, 600.0),
            GameMode::Assassins => (20, 480.0),
            GameMode::KingOfTheHill => (250, 600.0),
            GameMode::Hardpoint => (250, 600.0),
            GameMode::CapturePoint => (200, 600.0),
            GameMode::TestingGrounds => (0, 0.0),  // No limit
            GameMode::Juggernaut => (25, 600.0),
            GameMode::HighExplosives => (30, 480.0),
            GameMode::OneInTheChamber => (20, 480.0),
            GameMode::GunGame => (0, 600.0), // first to cycle all weapons
            GameMode::Infected => (0, 300.0), // survive 5 min
        };

        let num_zones = match mode {
            GameMode::KingOfTheHill => 1,
            GameMode::Hardpoint => 3,
            GameMode::CapturePoint => 3,
            _ => 0,
        };

        Self {
            mode,
            player_score: 0,
            enemy_score: 0,
            kills: 0,
            deaths: 0,
            assists: 0,
            match_timer: if duration > 0.0 {
                Timer::from_seconds(duration, TimerMode::Once)
            } else {
                Timer::from_seconds(86400.0, TimerMode::Once) // ~24 hours for sandbox modes
            },
            score_limit,
            round: 1,
            match_over: false,
            objective_progress: 0.0,
            objective_held: false,
            dog_tags_collected: 0,
            flags_captured: 0,
            hardpoint_time: 0.0,
            xp_earned: 0,
            capture_timers: vec![0.0; num_zones],
            capture_owners: vec![0; num_zones],
            cp_positions: Vec::new(),
            cp_move_timer: Timer::from_seconds(30.0, TimerMode::Repeating),
        }
    }

    /// Award XP (points) for an action. XP never affects win conditions.
    pub fn award_xp(&mut self, amount: u64) {
        self.xp_earned += amount;
    }

    pub fn add_kill(&mut self) {
        self.kills += 1;
        // Award XP for the kill
        self.award_xp(100);
        // Update win-condition scoring
        match self.mode {
            GameMode::FreeForAll | GameMode::Assassins => {
                self.player_score += 1;
            }
            GameMode::TeamDeathmatch => {
                self.player_score += 1;
            }
            GameMode::Juggernaut | GameMode::HighExplosives
            | GameMode::OneInTheChamber | GameMode::GunGame => {
                self.player_score += 1;
            }
            GameMode::KillConfirmed => {
                // Score is from dog tags, not kills directly
            }
            _ => {
                // Objective modes: kills don't directly score
            }
        }
    }

    pub fn add_death(&mut self) {
        self.deaths += 1;
        match self.mode {
            GameMode::TeamDeathmatch | GameMode::KillConfirmed => {
                self.enemy_score += 1;
            }
            _ => {}
        }
    }

    pub fn collect_dog_tag(&mut self) {
        self.dog_tags_collected += 1;
        self.player_score += 1;
        self.award_xp(150); // bonus XP for confirming a kill
    }

    pub fn capture_flag(&mut self) {
        self.flags_captured += 1;
        self.player_score += 1;
        self.award_xp(500); // flag capture is a big XP reward
    }

    pub fn is_over(&self) -> bool {
        if self.match_over { return true; }
        if self.score_limit > 0 && (self.player_score >= self.score_limit || self.enemy_score >= self.score_limit) {
            return true;
        }
        if self.match_timer.is_finished() {
            return true;
        }
        false
    }

    pub fn time_remaining(&self) -> f32 {
        (self.match_timer.duration().as_secs_f32() - self.match_timer.elapsed_secs()).max(0.0)
    }

    pub fn format_time_remaining(&self) -> String {
        let secs = self.time_remaining();
        if secs >= f32::MAX / 2.0 { return "∞".to_string(); }
        let m = (secs / 60.0) as u32;
        let s = (secs % 60.0) as u32;
        format!("{:02}:{:02}", m, s)
    }

    /// Did the player's team win?
    pub fn player_won(&self) -> bool {
        self.player_score >= self.enemy_score
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Scoreboard UI (Tab-held)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
pub struct ScoreboardUi;

#[derive(Component)]
pub struct MatchHudUi;

#[derive(Component)]
pub struct MatchHudTimer;

#[derive(Component)]
pub struct MatchHudScore;

#[derive(Component)]
pub struct MatchOverScreen;

#[derive(Component)]
pub struct MatchOverDismiss;

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Objective Zone (for KOTH, Hardpoint, CP)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

#[derive(Component)]
pub struct ObjectiveZone {
    pub radius: f32,
    pub capture_rate: f32,
}

#[derive(Component)]
pub struct DogTag {
    pub timer: Timer,
}

#[derive(Component)]
pub struct FlagEntity {
    pub team: u8,  // 0 = player's, 1 = enemy's
    pub held: bool,
    pub home: Vec3,
}

#[derive(Resource, Clone, Debug, Serialize, Deserialize)]
pub struct PlayerProgression {
    pub xp: u64,
    pub level: u32,
}

impl Default for PlayerProgression {
    fn default() -> Self {
        Self { xp: 0, level: 1 }
    }
}

impl PlayerProgression {
    pub fn save(&self) {
        crate::storage::save_section("savestate.json", "progression", self);
    }

    pub fn load() -> Self {
        crate::storage::load_section("savestate.json", "progression")
    }

    pub fn add_xp(&mut self, amount: u64) {
        self.xp += amount;
        let old_level = self.level;
        self.level = self.calculate_level();
        if self.level > old_level {
            println!("Level Up! You are now level {}", self.level);
        }
        self.save();
    }

    pub fn calculate_level(&self) -> u32 {
        // Simple formula: level = sqrt(xp / 100) + 1
        ((self.xp as f64 / 100.0).sqrt() as u32) + 1
    }
    
    pub fn xp_for_next_level(&self) -> u64 {
        let next_level = self.level + 1;
        ((next_level - 1) as u64).pow(2) * 100
    }
}

/// Marker for the "YOU HAVE THE FLAG" notification text.
#[derive(Component)]
pub struct FlagNotificationUi;

/// Marker for the CTF flag trail particles.
#[derive(Component)]
pub struct FlagTrailParticle {
    pub timer: Timer,
}

/// Resource tracking if the player currently holds a flag.
#[derive(Resource, Default)]
pub struct PlayerHasFlag(pub bool);

/// The player's team assignment (0 = red, 1 = blue).
#[derive(Resource, Default)]
pub struct PlayerTeam(pub u8);

/// Marker added to player on spawn; consumed by assign_team_spawn system.
#[derive(Component, Default)]
pub struct NeedsTeamSpawn;

pub struct GameplayPlugin;

impl Plugin for GameplayPlugin {
    fn build(&self, app: &mut App) {
        app.insert_resource(PlayerProgression::load());
        app.init_resource::<PlayerHasFlag>();
        app.init_resource::<PlayerTeam>();
        app.add_message::<DeathEvent>();
        app.add_systems(OnEnter(GameState::Playing), (init_match_state, spawn_enemies, spawn_player_ui, spawn_match_hud, spawn_objectives));
        app.add_systems(OnExit(GameState::Playing), despawn_gameplay_entities);
        app.add_systems(Update, (
            update_health_bars,
            update_player_health_ui,
            turret_fire,
            handle_death,
            assign_team_spawn,
        ).run_if(in_state(GameState::Playing)));
        app.add_systems(Update, (
            check_player_death,
            spectate_camera,
            respawn_player,
            billboard_system,
            handle_regeneration,
            update_death_screen,
            death_screen_respawn_button,
        ).run_if(in_state(GameState::Playing)));
        app.add_systems(Update, (
            update_match_timer,
            update_match_hud,
            toggle_scoreboard,
            update_objective_zones,
            update_dog_tags,
            update_ctf_flags,
            update_flag_trail,
            check_match_over,
            handle_match_over_dismiss,
        ).run_if(in_state(GameState::Playing)));
    }
}

#[derive(Resource)]
pub struct KillerInfo(pub String);

fn despawn_gameplay_entities(
    mut commands: Commands,
    query: Query<Entity, Or<(With<Enemy>, With<DeathScreen>, With<Turret>, With<TurretProjectile>, With<SpectatorTarget>, With<Billboard>)>>,
    health_ui_query: Query<Entity, Or<(With<PlayerHealthUi>, With<PlayerHealthBar>)>>,
    match_ui_query: Query<Entity, Or<(With<MatchHudUi>, With<ScoreboardUi>, With<MatchOverScreen>)>>,
    objective_query: Query<Entity, Or<(With<ObjectiveZone>, With<DogTag>, With<FlagEntity>)>>,
    flag_ui_query: Query<Entity, Or<(With<FlagNotificationUi>, With<FlagTrailParticle>)>>,
    mut player_has_flag: ResMut<PlayerHasFlag>,
) {
    for entity in query.iter() {
        if let Ok(mut cmds) = commands.get_entity(entity) {
            cmds.despawn();
        }
    }
    for entity in health_ui_query.iter() {
        if let Ok(mut cmds) = commands.get_entity(entity) {
            cmds.despawn();
        }
    }
    for entity in match_ui_query.iter() {
        if let Ok(mut cmds) = commands.get_entity(entity) {
            cmds.despawn();
        }
    }
    for entity in objective_query.iter() {
        if let Ok(mut cmds) = commands.get_entity(entity) {
            cmds.despawn();
        }
    }
    for entity in flag_ui_query.iter() {
        if let Ok(mut cmds) = commands.get_entity(entity) {
            cmds.despawn();
        }
    }
    player_has_flag.0 = false;
    commands.remove_resource::<RespawnTimer>();
    commands.remove_resource::<KillerInfo>();
    commands.remove_resource::<MatchState>();
}

#[derive(Resource)]
pub struct RespawnTimer(pub Timer);

#[derive(Component, Default)]
pub struct SpectatorTarget;

#[derive(Component)]
pub struct Billboard;

#[derive(Component)]
pub struct Regenerating {
    pub timer: Timer, // Delay before regen starts
    pub current_rate: f32,
    pub base_rate: f32,
    pub max_rate: f32,
    pub ramp_up_speed: f32,
}

impl Default for Regenerating {
    fn default() -> Self {
        Self {
            timer: Timer::from_seconds(5.0, TimerMode::Once), // 5 seconds delay
            current_rate: 1.0,
            base_rate: 1.0, // 1 HP/sec start
            max_rate: 20.0, // 20 HP/sec max
            ramp_up_speed: 5.0, // +5 HP/sec per second
        }
    }
}

fn handle_regeneration(
    time: Res<Time>,
    mut query: Query<(&mut Health, &mut Regenerating)>,
) {
    for (mut health, mut regen) in query.iter_mut() {
        if health.current < health.max {
            regen.timer.tick(time.delta());
            if regen.timer.is_finished() {
                // Ramp up rate
                regen.current_rate = (regen.current_rate + regen.ramp_up_speed * time.delta_secs()).min(regen.max_rate);
                // Apply regen
                health.current = (health.current + regen.current_rate * time.delta_secs()).min(health.max);
            }
        } else {
            // Reset if full (or handled by damage reset)
             // We don't reset timer here because if we are full, we are "safe". 
             // But if we take damage, we want delay.
             // The timer is "time since last damage".
             // If we are full, we haven't taken damage recently (or we healed up).
             // Actually, if we are full, we don't need to tick.
        }
    }
}

fn billboard_system(
    mut query: Query<&mut Transform, With<Billboard>>,
    camera_query: Query<&Transform, (With<MainCamera>, Without<Billboard>)>,
) {
    let camera_transform = if let Some(t) = camera_query.iter().next() { t } else { return };
    
    for mut transform in query.iter_mut() {
        transform.rotation = camera_transform.rotation;
    }
}

#[derive(Component)]
pub struct PlayerHealthUi;

#[derive(Component)]
pub struct PlayerHealthBar;

#[derive(Component)]
pub struct DeathScreen;

#[derive(Component)]
pub struct DeathScreenKillerText;

#[derive(Component)]
pub struct DeathScreenTimerText;

#[derive(Component)]
pub struct DeathScreenRespawnButton;

fn spawn_player_ui(mut commands: Commands, ui_config: Res<UiConfig>) {
    let config = &ui_config.health_bar;
    // Health Bar Container
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            left: Val::Px(config.position[0]),
            bottom: Val::Px(config.position[1]),
            width: Val::Px(config.size[0]),
            height: Val::Px(config.size[1]),
            border: UiRect::all(Val::Px(2.0)),
            ..default()
        },
        BackgroundColor(Color::BLACK),
        BorderColor::all(Color::WHITE),
        PlayerHealthUi,
    )).with_children(|parent| {
        // Health Bar Fill
        parent.spawn((
            Node {
                width: Val::Percent(100.0),
                height: Val::Percent(100.0),
                ..default()
            },
            BackgroundColor(Color::srgba(config.color[0], config.color[1], config.color[2], config.color[3])),
            PlayerHealthBar,
        ));
        
        // Health Text Overlay
        parent.spawn((
            Text::new("100 / 100"),
            TextFont { font_size: FontSize::Px(20.0),
                ..default()
            },
            TextColor(Color::srgba(config.text_color[0], config.text_color[1], config.text_color[2], config.text_color[3])),
            Node {
                position_type: PositionType::Absolute,
                left: Val::Px(10.0),
                top: Val::Px(5.0), // Approximate centering
                ..default()
            },
            PlayerHealthUi,
        ));
    });

    // Death Screen (full overlay with info)
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            display: Display::None,
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            justify_content: JustifyContent::Center,
            row_gap: Val::Px(20.0),
            ..default()
        },
        BackgroundColor(Color::srgba(0.1, 0.0, 0.0, 0.7)),
        GlobalZIndex(150),
        DeathScreen,
    )).with_children(|parent| {
        // "YOU WERE KILLED BY" label
        parent.spawn((
            Text::new("YOU WERE KILLED BY"),
            TextFont { font_size: FontSize::Px(28.0), ..default() },
            TextColor(Color::srgba(0.8, 0.2, 0.2, 1.0)),
        ));
        // Killer name (dynamic)
        parent.spawn((
            Text::new("Unknown"),
            TextFont { font_size: FontSize::Px(48.0), ..default() },
            TextColor(Color::srgba(1.0, 0.3, 0.3, 1.0)),
            DeathScreenKillerText,
        ));
        // Spacer
        parent.spawn(Node { height: Val::Px(30.0), ..default() });
        // Respawn timer text
        parent.spawn((
            Text::new("Respawning in 5.0s"),
            TextFont { font_size: FontSize::Px(24.0), ..default() },
            TextColor(Color::srgba(0.9, 0.9, 0.9, 1.0)),
            DeathScreenTimerText,
        ));
        // Respawn button
        parent.spawn((
            Node {
                width: Val::Px(200.0),
                height: Val::Px(50.0),
                justify_content: JustifyContent::Center,
                align_items: AlignItems::Center,
                margin: UiRect::top(Val::Px(20.0)),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.3, 0.6, 0.3, 0.9)),
            BorderColor::all(Color::WHITE),
            Button,
            DeathScreenRespawnButton,
        )).with_children(|btn| {
            btn.spawn((
                Text::new("RESPAWN"),
                TextFont { font_size: FontSize::Px(22.0), ..default() },
                TextColor(Color::WHITE),
            ));
        });
    });
}

fn update_player_health_ui(
    mut text_query: Query<&mut Text, With<PlayerHealthUi>>,
    mut bar_query: Query<&mut Node, With<PlayerHealthBar>>,
    mut death_screen_query: Query<&mut Node, (With<DeathScreen>, Without<PlayerHealthBar>)>,
    player_query: Query<&Health, With<PlayerBody>>,
) {
    let mut text = if let Ok(t) = text_query.single_mut() { t } else { return };
    let mut bar = if let Ok(b) = bar_query.single_mut() { b } else { return };
    let mut death_screen = if let Ok(d) = death_screen_query.single_mut() { d } else { return };
    
    if let Ok(health) = player_query.single() {
        text.0 = format!("{:.0} / {:.0}", health.current, health.max);
        bar.width = Val::Percent((health.current / health.max * 100.0).clamp(0.0, 100.0));
        
        if health.current <= 0.0 {
            death_screen.display = Display::Flex;
        } else {
            death_screen.display = Display::None;
        }
    }
}

#[derive(Component)]
pub struct Health {
    pub current: f32,
    pub max: f32,
}

#[derive(Component)]
#[require(SpectatorTarget)]
pub struct Enemy;

#[derive(Component)]
#[require(
    NeedsTeamSpawn,
    Regenerating,
    Health { current: 100.0, max: 100.0 },
)]
pub struct PlayerBody; // Tag for player to take damage

#[derive(Component)]
pub struct HealthBar {
    pub target: Entity,
    pub offset: Vec3,
}

#[derive(Component)]
pub struct Turret {
    pub fire_timer: Timer,
}

#[derive(Component)]
pub struct HealthBarForeground;

fn spawn_enemies(
    mut commands: Commands,
    asset_server: Res<AssetServer>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
    selected_mode: Res<SelectedGameMode>,
) {
    // Only spawn test enemies/turret for testing grounds
    if selected_mode.mode != GameMode::TestingGrounds {
        return;
    }
    let healths = [1.0, 50.0, 100.0, 500.0];
    let start_x = -5.0;
    let spacing = 3.0;

    // Create Health Bar Materials
    let bg_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.2, 0.0, 0.0), // Dark Red
        unlit: true,
        ..default()
    });
    let fg_material = materials.add(StandardMaterial {
        base_color: Color::srgb(0.0, 1.0, 0.0), // Green
        unlit: true,
        ..default()
    });
    let bar_mesh = meshes.add(Rectangle::new(1.0, 0.15));

    for (i, &hp) in healths.iter().enumerate() {
        let pos = Vec3::new(start_x + i as f32 * spacing, 0.0, -10.0);
        
        let enemy = commands.spawn((
            WorldAssetRoot(asset_server.load("characters/default.glb#Scene0")),
            Transform::from_translation(pos).with_scale(Vec3::splat(1.0)),
            Visibility::default(),
            Enemy,
            Health { current: hp, max: hp },
        )).id();

        commands.entity(enemy).with_children(|parent| {
            parent.spawn((
                Transform::from_translation(Vec3::new(0.0, 2.2, 0.0)),
                HealthBar { target: enemy, offset: Vec3::new(0.0, 2.2, 0.0) },
                Billboard,
                Visibility::Inherited,
            )).with_children(|hb_parent| {
                // Background
                hb_parent.spawn((
                    Mesh3d(bar_mesh.clone()),
                    MeshMaterial3d(bg_material.clone()),
                    Transform::from_translation(Vec3::new(0.0, 0.0, -0.01)), // Slightly behind
                ));
                // Foreground
                hb_parent.spawn((
                    Mesh3d(bar_mesh.clone()),
                    MeshMaterial3d(fg_material.clone()),
                    Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                    HealthBarForeground,
                ));
            });
        });
    }

    // Spawn Turret
    let turret = commands.spawn((
        Mesh3d(meshes.add(Cuboid::new(1.0, 1.0, 1.0))),
        MeshMaterial3d(materials.add(Color::srgb(0.8, 0.1, 0.1))),
        Transform::from_xyz(7.0, 0.5, -10.0).looking_at(Vec3::new(7.0, 0.5, 0.0), Vec3::Y),
        Visibility::default(),
        Turret {
            fire_timer: Timer::from_seconds(2.0, TimerMode::Repeating),
        },
        Health { current: 200.0, max: 200.0 },
        Enemy,
    )).id();

    commands.entity(turret).with_children(|parent| {
        parent.spawn((
            Transform::from_translation(Vec3::new(0.0, 1.5, 0.0)),
            HealthBar { target: turret, offset: Vec3::new(0.0, 1.5, 0.0) },
            Billboard,
            Visibility::Inherited,
        )).with_children(|hb_parent| {
            // Background
            hb_parent.spawn((
                Mesh3d(bar_mesh.clone()),
                MeshMaterial3d(bg_material.clone()),
                Transform::from_translation(Vec3::new(0.0, 0.0, -0.01)),
            ));
            // Foreground
            hb_parent.spawn((
                Mesh3d(bar_mesh.clone()),
                MeshMaterial3d(fg_material.clone()),
                Transform::from_translation(Vec3::new(0.0, 0.0, 0.0)),
                HealthBarForeground,
            ));
        });
    });
}

fn update_health_bars(
    mut query: Query<(&mut Transform, &ChildOf), With<HealthBarForeground>>,
    health_bar_query: Query<&ChildOf, With<HealthBar>>,
    health_query: Query<&Health>,
) {
    for (mut transform, parent) in query.iter_mut() {
        // parent is the HealthBar container
        if let Ok(grandparent) = health_bar_query.get(parent.get()) {
            // grandparent is the Enemy/Turret
            if let Ok(health) = health_query.get(grandparent.get()) {
                let percent = (health.current / health.max).clamp(0.0, 1.0);
                transform.scale.x = percent;
                // Anchor to left: Move x by (1.0 - percent) * width / 2.0 * -1.0 ?
                // Default quad is centered.
                // If scale is 0.5, it shrinks to center.
                // To anchor left, we need to shift it left by (1.0 - percent) * 0.5
                transform.translation.x = -0.5 * (1.0 - percent);
            }
        }
    }
}

fn turret_fire(
    mut commands: Commands,
    time: Res<Time>,
    mut query: Query<(&mut Turret, &Transform)>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (mut turret, transform) in query.iter_mut() {
        turret.fire_timer.tick(time.delta());
        if turret.fire_timer.just_finished() {
            let forward = transform.forward();
            commands.spawn((
                Mesh3d(meshes.add(Sphere::new(0.2))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgb(1.0, 0.0, 0.0),
                    emissive: LinearRgba::RED * 5.0,
                    ..default()
                })),
                Transform::from_translation(transform.translation + forward * 1.0),
                Projectile {
                    velocity: forward * 20.0,
                    timer: Timer::from_seconds(5.0, TimerMode::Once),
                    damage: 25.0,
                    from_player: false,
                    source_name: "Turret".to_string(),
                },
                TurretProjectile, // Tag to distinguish if needed, or just use Projectile
            ));
        }
    }
}

#[derive(Component)]
pub struct TurretProjectile;

fn handle_death(
    mut commands: Commands,
    query: Query<(Entity, &Health, &GlobalTransform, Option<&Enemy>, Option<&Turret>), Without<PlayerBody>>,
    mut death_events: MessageWriter<DeathEvent>,
    mut progression: ResMut<PlayerProgression>,
    mut match_state: Option<ResMut<MatchState>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    for (entity, health, global_tf, enemy, turret) in query.iter() {
        if health.current <= 0.0 {
            let name = if turret.is_some() { "Turret" } else if enemy.is_some() { "Target Dummy" } else { "Unknown" };
            death_events.write(DeathEvent {
                message: format!("Player killed {}", name),
            });
            
            // Grant XP for kill
            let xp_reward = if turret.is_some() { 50 } else { 25 };
            progression.add_xp(xp_reward);
            
            // Update match scoring
            if let Some(ref mut ms) = match_state {
                ms.add_kill();

                // In Kill Confirmed mode, spawn a dog tag at the death location
                if ms.mode == GameMode::KillConfirmed {
                    let pos = global_tf.translation();
                    commands.spawn((
                        Mesh3d(meshes.add(Cuboid::new(0.3, 0.3, 0.05))),
                        MeshMaterial3d(materials.add(StandardMaterial {
                            base_color: Color::srgb(0.9, 0.7, 0.1),
                            emissive: bevy::color::LinearRgba::new(2.0, 1.5, 0.2, 1.0),
                            ..default()
                        })),
                        Transform::from_translation(pos + Vec3::Y * 0.3),
                        DogTag {
                            timer: Timer::from_seconds(15.0, TimerMode::Once),
                        },
                    ));
                }
            }
            
            commands.entity(entity).despawn();
        }
    }
}

fn check_player_death(
    mut commands: Commands,
    mut player_query: Query<(Entity, &Health), With<PlayerBody>>,
    timer: Option<Res<RespawnTimer>>,
    mut match_state: Option<ResMut<MatchState>>,
) {
    if timer.is_some() { return; }

    if let Some((_entity, health)) = player_query.iter_mut().next() {
        if health.current <= 0.0 {
            if let Some(ref mut ms) = match_state {
                ms.add_death();
            }
            commands.insert_resource(RespawnTimer(Timer::from_seconds(5.0, TimerMode::Once)));
        }
    }
}

fn spectate_camera(
    mut camera_query: Query<&mut Transform, With<MainCamera>>,
    targets: Query<&GlobalTransform, With<SpectatorTarget>>,
    time: Res<Time>,
    mut timer: Option<ResMut<RespawnTimer>>,
) {
    if let Some(timer) = timer.as_mut() {
        timer.0.tick(time.delta());
        
        if let Some(mut cam_transform) = camera_query.iter_mut().next() {
            // Find a target to spectate (just pick first for now)
            if let Some(target) = targets.iter().next() {
                let target_pos = target.translation();
                let target_look = target_pos + Vec3::Y * 1.0;
                let cam_pos = target_pos + Vec3::new(0.0, 5.0, 5.0);
                
                cam_transform.translation = cam_transform.translation.lerp(cam_pos, time.delta_secs() * 2.0);
                cam_transform.look_at(target_look, Vec3::Y);
            }
        }
    }
}

fn respawn_player(
    mut commands: Commands,
    mut query: Query<(&mut Health, &mut Transform, &mut PhysicalTranslation, &mut PreviousPhysicalTranslation, &mut Velocity), With<PlayerBody>>,
    timer: Option<Res<RespawnTimer>>,
    player_team: Option<Res<PlayerTeam>>,
    spawn_areas: Query<&TeamSpawnArea>,
    selected_mode: Res<SelectedGameMode>,
) {
    if let Some(timer) = timer {
        if timer.0.is_finished() {
            if let Some((mut health, mut transform, mut phys, mut prev_phys, mut velocity)) = query.iter_mut().next() {
                health.current = health.max;
                velocity.0 = Vec3::ZERO;
                let spawn_pos = team_spawn_pos(&player_team, &spawn_areas, selected_mode.mode);
                transform.translation = spawn_pos;
                phys.0 = spawn_pos;
                prev_phys.0 = spawn_pos;
                commands.remove_resource::<RespawnTimer>();
                commands.remove_resource::<KillerInfo>();
            }
        }
    }
}

fn update_death_screen(
    timer: Option<Res<RespawnTimer>>,
    killer_info: Option<Res<KillerInfo>>,
    mut killer_text_query: Query<&mut Text, (With<DeathScreenKillerText>, Without<DeathScreenTimerText>)>,
    mut timer_text_query: Query<&mut Text, (With<DeathScreenTimerText>, Without<DeathScreenKillerText>)>,
) {
    if let Some(timer) = timer {
        let remaining = timer.0.duration().as_secs_f32() - timer.0.elapsed_secs();
        if let Ok(mut timer_text) = timer_text_query.single_mut() {
            timer_text.0 = format!("Respawning in {:.1}s", remaining.max(0.0));
        }
        if let Ok(mut killer_text) = killer_text_query.single_mut() {
            let name = killer_info.as_ref().map(|k| k.0.as_str()).unwrap_or("Unknown");
            killer_text.0 = name.to_string();
        }
    }
}

fn death_screen_respawn_button(
    mut commands: Commands,
    interaction_query: Query<&Interaction, (Changed<Interaction>, With<DeathScreenRespawnButton>)>,
    mut player_query: Query<(&mut Health, &mut Transform, &mut PhysicalTranslation, &mut PreviousPhysicalTranslation, &mut Velocity), With<PlayerBody>>,
    timer: Option<Res<RespawnTimer>>,
    player_team: Option<Res<PlayerTeam>>,
    spawn_areas: Query<&TeamSpawnArea>,
    selected_mode: Res<SelectedGameMode>,
) {
    if timer.is_none() { return; }
    for interaction in interaction_query.iter() {
        if *interaction == Interaction::Pressed {
            if let Some((mut health, mut transform, mut phys, mut prev_phys, mut velocity)) = player_query.iter_mut().next() {
                health.current = health.max;
                velocity.0 = Vec3::ZERO;
                let spawn_pos = team_spawn_pos(&player_team, &spawn_areas, selected_mode.mode);
                transform.translation = spawn_pos;
                phys.0 = spawn_pos;
                prev_phys.0 = spawn_pos;
                commands.remove_resource::<RespawnTimer>();
                commands.remove_resource::<KillerInfo>();
            }
        }
    }
}

/// Compute a spawn position within the player's team spawn area, falling back
/// to the origin for non-team modes or when no area is available.
fn team_spawn_pos(
    player_team: &Option<Res<PlayerTeam>>,
    spawn_areas: &Query<&TeamSpawnArea>,
    mode: GameMode,
) -> Vec3 {
    if mode.is_team_mode() {
        if let Some(team) = player_team {
            for area in spawn_areas.iter() {
                if area.team == team.0 {
                    let mut rng = rand::rng();
                    let offset = Vec3::new(
                        rng.random_range(-area.radius..area.radius),
                        0.0,
                        rng.random_range(-area.radius..area.radius),
                    );
                    return area.center + offset;
                }
            }
        }
    }
    Vec3::new(0.0, 1.0, 0.0)
}

/// One-shot system: move the player to their team spawn area on the first
/// frame after spawning.
fn assign_team_spawn(
    mut commands: Commands,
    player_team: Option<Res<PlayerTeam>>,
    spawn_areas: Query<&TeamSpawnArea>,
    mut player_query: Query<(Entity, &mut PhysicalTranslation, &mut PreviousPhysicalTranslation), With<NeedsTeamSpawn>>,
    selected_mode: Res<SelectedGameMode>,
) {
    if player_query.is_empty() { return; }

    let spawn_pos = team_spawn_pos(&player_team, &spawn_areas, selected_mode.mode);

    for (entity, mut phys, mut prev_phys) in player_query.iter_mut() {
        phys.0 = spawn_pos;
        prev_phys.0 = spawn_pos;
        commands.entity(entity).remove::<NeedsTeamSpawn>();
    }
}

#[derive(Message)]
pub struct DeathEvent {
    pub message: String,
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Match Initialization & Game Mode Systems
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn init_match_state(
    mut commands: Commands,
    selected_mode: Res<SelectedGameMode>,
) {
    commands.insert_resource(MatchState::new(selected_mode.mode));
    // Assign the player to team 0 (red) for team modes
    commands.insert_resource(PlayerTeam(0));
}

fn spawn_match_hud(
    mut commands: Commands,
    selected_mode: Res<SelectedGameMode>,
) {
    let mode = selected_mode.mode;
    let accent = mode.accent_color();

    // Top-center match HUD bar
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            top: Val::Px(0.0),
            left: Val::Percent(50.0),
            width: Val::Px(420.0),
            height: Val::Px(48.0),
            margin: UiRect::left(Val::Px(-210.0)),
            flex_direction: FlexDirection::Row,
            justify_content: JustifyContent::SpaceBetween,
            align_items: AlignItems::Center,
            padding: UiRect::horizontal(Val::Px(16.0)),
            ..default()
        },
        BackgroundColor(Color::srgba(0.02, 0.02, 0.05, 0.85)),
        GlobalZIndex(50),
        MatchHudUi,
    )).with_children(|hud| {
        // Player score
        hud.spawn((
            Text::new("0"),
            TextFont { font_size: FontSize::Px(22.0), ..default() },
            TextColor(Color::srgb(0.3, 0.7, 1.0)),
            MatchHudScore,
        ));

        // Center: mode name + timer
        hud.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            ..default()
        }).with_children(|center| {
            center.spawn((
                Text::new(mode.short_name()),
                TextFont { font_size: FontSize::Px(10.0), ..default() },
                TextColor(accent),
            ));
            center.spawn((
                Text::new("10:00"),
                TextFont { font_size: FontSize::Px(18.0), ..default() },
                TextColor(Color::WHITE),
                MatchHudTimer,
            ));
        });

        // Enemy score
        hud.spawn((
            Text::new("0"),
            TextFont { font_size: FontSize::Px(22.0), ..default() },
            TextColor(Color::srgb(1.0, 0.3, 0.3)),
        ));
    });
}

fn spawn_objectives(
    _commands: Commands,
    _selected_mode: Res<SelectedGameMode>,
    _meshes: ResMut<Assets<Mesh>>,
    _materials: ResMut<Assets<StandardMaterial>>,
) {
    // Mode-specific objectives (flags, zones, etc.) are now spawned by
    // gamemodes::spawn_mode_entities in world/mod.rs::spawn_game_map.
}

fn update_match_timer(
    time: Res<Time>,
    mut match_state: Option<ResMut<MatchState>>,
) {
    if let Some(ref mut ms) = match_state {
        if !ms.match_over {
            ms.match_timer.tick(time.delta());
        }
    }
}

fn update_match_hud(
    match_state: Option<Res<MatchState>>,
    mut timer_query: Query<&mut Text, (With<MatchHudTimer>, Without<MatchHudScore>)>,
    mut score_query: Query<&mut Text, (With<MatchHudScore>, Without<MatchHudTimer>)>,
) {
    let Some(ms) = match_state else { return };
    
    for mut text in timer_query.iter_mut() {
        text.0 = ms.format_time_remaining();
    }
    for mut text in score_query.iter_mut() {
        text.0 = format!("{}", ms.player_score);
    }
}

fn update_objective_zones(
    time: Res<Time>,
    mut match_state: Option<ResMut<MatchState>>,
    zone_query: Query<(Entity, &Transform, &ObjectiveZone)>,
    player_query: Query<&Transform, (With<PlayerBody>, Without<ObjectiveZone>)>,
    _zone_mat_query: Query<&mut MeshMaterial3d<StandardMaterial>, With<ObjectiveZone>>,
    _materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(ref mut ms) = match_state else { return };
    if ms.match_over { return; }
    
    let is_objective_mode = matches!(ms.mode, GameMode::KingOfTheHill | GameMode::Hardpoint | GameMode::CapturePoint);
    if !is_objective_mode { return; }
    
    let player_pos = if let Ok(pt) = player_query.single() {
        pt.translation
    } else { return };
    
    let dt = time.delta_secs();
    
    // CapturePoint: move zones periodically
    if ms.mode == GameMode::CapturePoint {
        ms.cp_move_timer.tick(time.delta());
        if ms.cp_move_timer.just_finished() {
            // Relocate CP zones (positions are tracked in cp_positions)
            let mut rng = rand::rng();
            for pos in ms.cp_positions.iter_mut() {
                pos.x = rng.random_range(-40.0..40.0);
                pos.z = rng.random_range(-40.0..40.0);
            }
        }
    }

    for (i, (_entity, zone_tf, zone)) in zone_query.iter().enumerate() {
        if i >= ms.capture_timers.len() { break; }
        
        let zone_pos = zone_tf.translation;
        let dist = Vec2::new(player_pos.x - zone_pos.x, player_pos.z - zone_pos.z).length();
        let on_zone = dist < zone.radius;

        match ms.mode {
            GameMode::KingOfTheHill => {
                // 5 seconds of standing on zone to capture
                if on_zone {
                    ms.capture_timers[i] = (ms.capture_timers[i] + dt).min(5.0);
                    if ms.capture_timers[i] >= 5.0 && ms.capture_owners[i] != 1 {
                        ms.capture_owners[i] = 1; // Player captured
                    }
                } else {
                    // Uncapture after 5 seconds off point
                    if ms.capture_owners[i] == 1 {
                        ms.capture_timers[i] = (ms.capture_timers[i] - dt).max(0.0);
                        if ms.capture_timers[i] <= 0.0 {
                            ms.capture_owners[i] = 0; // Uncaptured
                        }
                    }
                }
                // Score while captured
                if ms.capture_owners[i] == 1 {
                    ms.hardpoint_time += dt;
                    if ms.hardpoint_time.fract() < dt {
                        ms.player_score += 1;
                        ms.award_xp(10);
                    }
                }
            }
            GameMode::Hardpoint => {
                // 3 smaller zones, same 5s capture mechanic
                if on_zone {
                    ms.capture_timers[i] = (ms.capture_timers[i] + dt).min(5.0);
                    if ms.capture_timers[i] >= 5.0 && ms.capture_owners[i] != 1 {
                        ms.capture_owners[i] = 1;
                        ms.award_xp(50);
                    }
                } else if ms.capture_owners[i] == 1 {
                    ms.capture_timers[i] = (ms.capture_timers[i] - dt).max(0.0);
                    if ms.capture_timers[i] <= 0.0 {
                        ms.capture_owners[i] = 0;
                    }
                }
                if ms.capture_owners[i] == 1 {
                    ms.hardpoint_time += dt;
                    if ms.hardpoint_time.fract() < dt {
                        ms.player_score += 1;
                        ms.award_xp(10);
                    }
                }
            }
            GameMode::CapturePoint => {
                // Instant capture
                if on_zone && ms.capture_owners[i] != 1 {
                    ms.capture_owners[i] = 1;
                    ms.player_score += 1;
                    ms.award_xp(75);
                }
            }
            _ => {}
        }

        ms.objective_held = on_zone;
    }
}

fn update_dog_tags(
    mut commands: Commands,
    time: Res<Time>,
    mut tag_query: Query<(Entity, &mut DogTag, &Transform)>,
    player_query: Query<&Transform, (With<PlayerBody>, Without<DogTag>)>,
    mut match_state: Option<ResMut<MatchState>>,
) {
    let player_pos = if let Ok(pt) = player_query.single() {
        pt.translation
    } else { return };

    for (entity, mut tag, tag_tf) in tag_query.iter_mut() {
        tag.timer.tick(time.delta());
        // Expire after timeout
        if tag.timer.is_finished() {
            commands.entity(entity).despawn();
            continue;
        }
        // Pick up if player is close
        let dist = player_pos.distance(tag_tf.translation);
        if dist < 2.0 {
            if let Some(ref mut ms) = match_state {
                ms.collect_dog_tag();
            }
            commands.entity(entity).despawn();
        }
    }
}

fn check_match_over(
    mut commands: Commands,
    match_state: Option<Res<MatchState>>,
    existing: Query<Entity, With<MatchOverScreen>>,
    mut progression: ResMut<PlayerProgression>,
    mut credits: ResMut<PlayerCredits>,
) {
    let Some(ms) = match_state else { return };
    if !ms.is_over() { return; }
    if !existing.is_empty() { return; } // Already showing
    
    let won = ms.player_won();
    let title = if won { "VICTORY" } else { "DEFEAT" };
    let title_color = if won { Color::srgb(0.2, 0.8, 0.3) } else { Color::srgb(0.9, 0.2, 0.2) };
    
    // Award XP from match
    if ms.xp_earned > 0 {
        progression.add_xp(ms.xp_earned);
    }
    
    // Award credits for winning
    if won && ms.mode != GameMode::TestingGrounds {
        let credit_reward = if ms.mode.is_team_mode() { 50 } else { 150 };
        credits.balance += credit_reward;
        credits.save();
    }
    
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.7)),
        GlobalZIndex(180),
        MatchOverScreen,
    )).with_children(|root| {
        root.spawn(Node {
            flex_direction: FlexDirection::Column,
            align_items: AlignItems::Center,
            row_gap: Val::Px(16.0),
            padding: UiRect::all(Val::Px(40.0)),
            ..default()
        }).with_children(|card| {
            card.spawn((
                Text::new(title),
                TextFont { font_size: FontSize::Px(52.0), ..default() },
                TextColor(title_color),
            ));
            card.spawn((
                Text::new(format!("Mode: {}", ms.mode.display_name())),
                TextFont { font_size: FontSize::Px(16.0), ..default() },
                TextColor(Color::srgba(0.7, 0.7, 0.8, 0.9)),
            ));
            card.spawn((
                Text::new(format!("Score: {} - {}", ms.player_score, ms.enemy_score)),
                TextFont { font_size: FontSize::Px(24.0), ..default() },
                TextColor(Color::WHITE),
            ));
            card.spawn((
                Text::new(format!("K/D: {} / {}   Assists: {}", ms.kills, ms.deaths, ms.assists)),
                TextFont { font_size: FontSize::Px(16.0), ..default() },
                TextColor(Color::srgba(0.6, 0.6, 0.7, 0.9)),
            ));

            // Return to menu button
            card.spawn((
                Button,
                Node {
                    width: Val::Px(200.0),
                    height: Val::Px(46.0),
                    justify_content: JustifyContent::Center,
                    align_items: AlignItems::Center,
                    margin: UiRect::top(Val::Px(20.0)),
                    ..default()
                },
                BackgroundColor(Color::srgb(0.2, 0.2, 0.3)),
                MatchOverDismiss,
            )).with_children(|btn| {
                btn.spawn((
                    Text::new("RETURN TO MENU"),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(Color::WHITE),
                ));
            });
        });
    });
}

fn handle_match_over_dismiss(
    mut next_state: ResMut<NextState<GameState>>,
    query: Query<&Interaction, (Changed<Interaction>, With<MatchOverDismiss>)>,
    keyboard: Res<ButtonInput<KeyCode>>,
    match_over_query: Query<Entity, With<MatchOverScreen>>,
) {
    if match_over_query.is_empty() { return; }
    
    for interaction in query.iter() {
        if *interaction == Interaction::Pressed {
            next_state.set(GameState::MainMenu);
            return;
        }
    }
    if keyboard.just_pressed(KeyCode::Escape) || keyboard.just_pressed(KeyCode::Enter) {
        next_state.set(GameState::MainMenu);
    }
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// Scoreboard (Tab-held overlay)
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn toggle_scoreboard(
    mut commands: Commands,
    keyboard: Res<ButtonInput<KeyCode>>,
    keybinds: Res<crate::player::Keybinds>,
    scoreboard_query: Query<Entity, With<ScoreboardUi>>,
    match_state: Option<Res<MatchState>>,
    progression: Res<PlayerProgression>,
    scoreboard_data: Option<Res<crate::net::ScoreboardData>>,
    conn_state: Res<crate::net::ConnectionState>,
) {
    // Show while Scoreboard key is held, hide when released
    if keyboard.just_pressed(keybinds.scoreboard) {
        if scoreboard_query.is_empty() {
            spawn_scoreboard(&mut commands, match_state.as_deref(), &progression, scoreboard_data.as_deref(), &conn_state);
        }
    }
    if keyboard.just_released(keybinds.scoreboard) {
        for entity in scoreboard_query.iter() {
            commands.entity(entity).despawn();
        }
    }
}

fn spawn_scoreboard(
    commands: &mut Commands,
    match_state: Option<&MatchState>,
    _progression: &PlayerProgression,
    scoreboard_data: Option<&crate::net::ScoreboardData>,
    conn_state: &crate::net::ConnectionState,
) {
    let (mode_name, time_str, _is_team) = if let Some(ms) = match_state {
        (
            ms.mode.display_name(),
            ms.format_time_remaining(),
            ms.mode.is_team_mode(),
        )
    } else {
        ("Unknown", "-".to_string(), false)
    };
    
    let local_username = conn_state.username().unwrap_or("You").to_string();
    
    commands.spawn((
        Node {
            position_type: PositionType::Absolute,
            width: Val::Percent(100.0),
            height: Val::Percent(100.0),
            justify_content: JustifyContent::Center,
            align_items: AlignItems::Center,
            ..default()
        },
        BackgroundColor(Color::srgba(0.0, 0.0, 0.0, 0.6)),
        GlobalZIndex(160),
        ScoreboardUi,
    )).with_children(|root| {
        root.spawn((
            Node {
                width: Val::Px(600.0),
                flex_direction: FlexDirection::Column,
                padding: UiRect::all(Val::Px(24.0)),
                row_gap: Val::Px(8.0),
                border: UiRect::all(Val::Px(2.0)),
                ..default()
            },
            BackgroundColor(Color::srgba(0.04, 0.04, 0.08, 0.95)),
            BorderColor::from(Color::srgba(0.3, 0.3, 0.4, 0.5)),
        )).with_children(|card| {
            // Header
            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                align_items: AlignItems::Center,
                ..default()
            }).with_children(|header| {
                header.spawn((
                    Text::new("SCOREBOARD"),
                    TextFont { font_size: FontSize::Px(22.0), ..default() },
                    TextColor(Color::WHITE),
                ));
                header.spawn((
                    Text::new(mode_name),
                    TextFont { font_size: FontSize::Px(14.0), ..default() },
                    TextColor(match_state.map(|ms| ms.mode.accent_color()).unwrap_or(Color::WHITE)),
                ));
            });

            // Divider
            card.spawn((
                Node { width: Val::Percent(100.0), height: Val::Px(1.0), ..default() },
                BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.4)),
            ));

            // Column headers
            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                padding: UiRect::horizontal(Val::Px(8.0)),
                ..default()
            }).with_children(|cols| {
                for (label, w) in [("PLAYER", 200.0), ("SCORE", 80.0), ("KILLS", 80.0), ("DEATHS", 80.0), ("K/D", 80.0)] {
                    cols.spawn((
                        Text::new(label),
                        TextFont { font_size: FontSize::Px(11.0), ..default() },
                        TextColor(Color::srgba(0.5, 0.5, 0.6, 0.8)),
                        Node { width: Val::Px(w), ..default() },
                    ));
                }
            });

            // Build sorted player list from ScoreboardData
            let mut player_rows: Vec<(String, i32, u32, u32, f32)> = Vec::new();
            if let Some(sd) = scoreboard_data {
                let mut ids: Vec<uuid::Uuid> = sd.names.keys().cloned().collect();
                ids.sort_by(|a, b| sd.scores.get(b).unwrap_or(&0).cmp(sd.scores.get(a).unwrap_or(&0)));
                for id in ids {
                    let name = sd.get_or_name(&id);
                    let score = *sd.scores.get(&id).unwrap_or(&0);
                    let kills = *sd.kills.get(&id).unwrap_or(&0);
                    let deaths = *sd.deaths.get(&id).unwrap_or(&0);
                    let kd = if deaths > 0 { kills as f32 / deaths as f32 } else { kills as f32 };
                    player_rows.push((name, score, kills, deaths, kd));
                }
            }
            // Fallback: show local player if no scoreboard data
            if player_rows.is_empty() {
                let kills = match_state.map(|ms| ms.kills).unwrap_or(0);
                let deaths = match_state.map(|ms| ms.deaths).unwrap_or(0);
                let score = match_state.map(|ms| ms.player_score).unwrap_or(0);
                let kd = if deaths > 0 { kills as f32 / deaths as f32 } else { kills as f32 };
                player_rows.push((format!("{} (You)", local_username), score, kills, deaths, kd));
            }

            for (i, (name, score, kills, deaths, kd)) in player_rows.iter().enumerate() {
                let is_local = name.contains(&local_username) || name == &local_username;
                let bg = if is_local { Color::srgba(0.08, 0.12, 0.25, 0.6) } else if i % 2 == 0 { Color::srgba(0.06, 0.06, 0.1, 0.4) } else { Color::NONE };
                card.spawn((
                    Node {
                        flex_direction: FlexDirection::Row,
                        justify_content: JustifyContent::SpaceBetween,
                        padding: UiRect::vertical(Val::Px(4.0)),
                        ..default()
                    },
                    BackgroundColor(bg),
                )).with_children(|row| {
                    let name_display = if is_local { format!("{name} (You)") } else { name.clone() };
                    let row_data = [(name_display, 200.0, if is_local { Color::srgb(0.4, 0.7, 1.0) } else { Color::WHITE }),
                                   (format!("{score}"), 80.0, Color::WHITE),
                                   (format!("{kills}"), 80.0, Color::WHITE),
                                   (format!("{deaths}"), 80.0, Color::WHITE),
                                   (format!("{kd:.2}"), 80.0, Color::WHITE)];
                    for (val, w, color) in row_data {
                        row.spawn((
                            Text::new(val),
                            TextFont { font_size: FontSize::Px(13.0), ..default() },
                            TextColor(color),
                            Node { width: Val::Px(w), ..default() },
                        ));
                    }
                });
            }

            // Divider
            card.spawn((
                Node { width: Val::Percent(100.0), height: Val::Px(1.0), ..default() },
                BackgroundColor(Color::srgba(0.3, 0.3, 0.4, 0.4)),
            ));

            // Footer with time
            card.spawn(Node {
                flex_direction: FlexDirection::Row,
                justify_content: JustifyContent::SpaceBetween,
                ..default()
            }).with_children(|footer| {
                footer.spawn((
                    Text::new(format!("Time Remaining: {}", time_str)),
                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                    TextColor(Color::srgba(0.6, 0.6, 0.7, 0.8)),
                ));
                footer.spawn((
                    Text::new(format!("Players: {}", player_rows.len())),
                    TextFont { font_size: FontSize::Px(12.0), ..default() },
                    TextColor(Color::srgba(0.6, 0.6, 0.7, 0.8)),
                ));
            });
        });
    });
}

// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━
// CTF Flag Systems
// ━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━━

fn update_ctf_flags(
    mut commands: Commands,
    mut flag_query: Query<(Entity, &mut FlagEntity, &mut Transform), Without<PlayerBody>>,
    player_query: Query<&Transform, (With<PlayerBody>, Without<FlagEntity>)>,
    mut match_state: Option<ResMut<MatchState>>,
    mut player_has_flag: ResMut<PlayerHasFlag>,
    notification_query: Query<Entity, With<FlagNotificationUi>>,
    mut meshes: ResMut<Assets<Mesh>>,
    mut materials: ResMut<Assets<StandardMaterial>>,
) {
    let Some(ref mut ms) = match_state else { return };
    if ms.mode != GameMode::CaptureTheFlag { return; }
    
    let player_pos = if let Ok(pt) = player_query.single() {
        pt.translation
    } else { return };
    
    // Find own flag home position (team 0) for the capture check
    let own_flag_home = flag_query.iter()
        .find(|(_, f, _)| f.team == 0)
        .map(|(_, f, _)| f.home)
        .unwrap_or(Vec3::ZERO);
    
    for (_entity, mut flag, mut flag_tf) in flag_query.iter_mut() {
        if flag.team != 1 { continue; } // Only handle enemy flag
        
        let pickup_dist = 2.5;
        let capture_dist = 3.0;
        
        if !flag.held {
            // Check if player is close enough to pick up
            let dist = player_pos.distance(flag_tf.translation);
            if dist < pickup_dist {
                flag.held = true;
                player_has_flag.0 = true;
                
                // Spawn "YOU HAVE THE FLAG" notification
                if notification_query.is_empty() {
                    commands.spawn((
                        Node {
                            position_type: PositionType::Absolute,
                            top: Val::Percent(20.0),
                            left: Val::Percent(50.0),
                            margin: UiRect::left(Val::Px(-150.0)),
                            width: Val::Px(300.0),
                            height: Val::Px(50.0),
                            justify_content: JustifyContent::Center,
                            align_items: AlignItems::Center,
                            ..default()
                        },
                        BackgroundColor(Color::srgba(0.1, 0.4, 0.15, 0.85)),
                        GlobalZIndex(100),
                        FlagNotificationUi,
                    )).with_children(|parent| {
                        parent.spawn((
                            Text::new("[FLAG] YOU HAVE THE FLAG"),
                            TextFont { font_size: FontSize::Px(22.0), ..default() },
                            TextColor(Color::srgb(0.3, 1.0, 0.4)),
                        ));
                    });
                }
            }
        } else {
            // Flag follows the player
            flag_tf.translation = player_pos + Vec3::new(0.0, 2.5, -0.5);
            
            // Spawn trail particles behind the player
            commands.spawn((
                Mesh3d(meshes.add(Sphere::new(0.15))),
                MeshMaterial3d(materials.add(StandardMaterial {
                    base_color: Color::srgba(1.0, 0.3, 0.3, 0.6),
                    emissive: bevy::color::LinearRgba::new(3.0, 0.5, 0.5, 1.0),
                    alpha_mode: AlphaMode::Blend,
                    ..default()
                })),
                Transform::from_translation(player_pos + Vec3::Y * 1.0),
                FlagTrailParticle {
                    timer: Timer::from_seconds(1.5, TimerMode::Once),
                },
            ));
            
            // Check if player reached own base (own flag location)
            let dist_to_base = Vec2::new(
                player_pos.x - own_flag_home.x,
                player_pos.z - own_flag_home.z,
            ).length();
            if dist_to_base < capture_dist {
                // Flag captured!
                ms.capture_flag();
                flag.held = false;
                flag_tf.translation = flag.home;
                player_has_flag.0 = false;
                
                // Remove "YOU HAVE THE FLAG" notification
                for notif in notification_query.iter() {
                    commands.entity(notif).despawn();
                }
                
                // Spawn "FLAG CAPTURED!" notification briefly
                commands.spawn((
                    Node {
                        position_type: PositionType::Absolute,
                        top: Val::Percent(20.0),
                        left: Val::Percent(50.0),
                        margin: UiRect::left(Val::Px(-150.0)),
                        width: Val::Px(300.0),
                        height: Val::Px(50.0),
                        justify_content: JustifyContent::Center,
                        align_items: AlignItems::Center,
                        ..default()
                    },
                    BackgroundColor(Color::srgba(0.9, 0.7, 0.1, 0.9)),
                    GlobalZIndex(100),
                    FlagNotificationUi,
                )).with_children(|parent| {
                    parent.spawn((
                        Text::new("[FLAG CAPTURED!]"),
                        TextFont { font_size: FontSize::Px(26.0), ..default() },
                        TextColor(Color::WHITE),
                    ));
                });
                break;
            }
        }
    }
}

fn update_flag_trail(
    mut commands: Commands,
    time: Res<Time>,
    mut trail_query: Query<(Entity, &mut FlagTrailParticle, &mut Transform)>,
) {
    for (entity, mut particle, mut tf) in trail_query.iter_mut() {
        particle.timer.tick(time.delta());
        
        // Fade out by shrinking and rising
        let remaining = particle.timer.fraction_remaining();
        tf.scale = Vec3::splat(remaining.max(0.05));
        tf.translation.y += time.delta_secs() * 0.5;
        
        if particle.timer.is_finished() {
            commands.entity(entity).despawn();
        }
    }
}
