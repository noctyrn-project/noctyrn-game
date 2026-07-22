pub mod main_menu;
pub mod loadout;
pub mod crate_menu;
pub mod cosmetics;
pub mod gamemode;
pub mod login;
pub mod profile;
pub mod friends;
pub mod party;
pub mod invite;

use bevy::prelude::*;

use crate::player::GameState;

// Re-exports used by other modules
pub use main_menu::CancelSearchButton;

#[derive(Debug, Clone, Copy, PartialEq, Eq, Hash, Default)]
pub enum GameMode {
    #[default]
    FreeForAll,
    TeamDeathmatch,
    KillConfirmed,
    CaptureTheFlag,
    Assassins,
    KingOfTheHill,
    Hardpoint,
    CapturePoint,
    TestingGrounds,
    Juggernaut,
    HighExplosives,
    OneInTheChamber,
    GunGame,
    Infected,
}

impl GameMode {
    pub fn display_name(&self) -> &'static str {
        match self {
            GameMode::FreeForAll => "FREE FOR ALL",
            GameMode::TeamDeathmatch => "TEAM DEATHMATCH",
            GameMode::KillConfirmed => "KILL CONFIRMED",
            GameMode::CaptureTheFlag => "CAPTURE THE FLAG",
            GameMode::Assassins => "ASSASSINS",
            GameMode::KingOfTheHill => "KING OF THE HILL",
            GameMode::Hardpoint => "HARDPOINT",
            GameMode::CapturePoint => "CAPTURE POINT",
            GameMode::TestingGrounds => "TESTING GROUNDS",
            GameMode::Juggernaut => "JUGGERNAUT",
            GameMode::HighExplosives => "HIGH EXPLOSIVES",
            GameMode::OneInTheChamber => "ONE IN THE CHAMBER",
            GameMode::GunGame => "GUN GAME",
            GameMode::Infected => "INFECTED",
        }
    }

    pub fn short_name(&self) -> &'static str {
        match self {
            GameMode::FreeForAll => "FFA",
            GameMode::TeamDeathmatch => "TDM",
            GameMode::KillConfirmed => "KC",
            GameMode::CaptureTheFlag => "CTF",
            GameMode::Assassins => "ASN",
            GameMode::KingOfTheHill => "KOTH",
            GameMode::Hardpoint => "HP",
            GameMode::CapturePoint => "CP",
            GameMode::TestingGrounds => "TG",
            GameMode::Juggernaut => "JGR",
            GameMode::HighExplosives => "HE",
            GameMode::OneInTheChamber => "OITC",
            GameMode::GunGame => "GG",
            GameMode::Infected => "INF",
        }
    }

    pub fn description(&self) -> &'static str {
        match self {
            GameMode::FreeForAll => "Every player for themselves. Get the most kills to win.",
            GameMode::TeamDeathmatch => "Two teams fight. First to 150 kills wins.",
            GameMode::KillConfirmed => "Team mode. Collect dog tags from fallen enemies to score.",
            GameMode::CaptureTheFlag => "Steal the enemy flag and return it to your base to score.",
            GameMode::Assassins => "Each player has a specific target. Hunt them down.",
            GameMode::KingOfTheHill => "Hold the zone for 5 seconds to capture. Control it to win.",
            GameMode::Hardpoint => "Three smaller zones. Hold them to earn points.",
            GameMode::CapturePoint => "Three moving zones. Capture them instantly to score.",
            GameMode::TestingGrounds => "Practice with all weapons. Spawn targets, test movement.",
            GameMode::Juggernaut => "A random player becomes the Juggernaut with 1000 HP and a minigun. Kill them to become the new Juggernaut.",
            GameMode::HighExplosives => "Explosive weapons only. RPGs, grenade launchers, and more.",
            GameMode::OneInTheChamber => "One bullet. One kill grants another bullet. Knife as backup.",
            GameMode::GunGame => "Cycle through every weapon. First to get a kill with each wins.",
            GameMode::Infected => "One player starts infected. Kill survivors to spread the infection.",
        }
    }

    pub fn player_count(&self) -> &'static str {
        match self {
            GameMode::TestingGrounds => "1 Player",
            _ => "Up to 50 Players",
        }
    }

    pub fn accent_color(&self) -> Color {
        match self {
            GameMode::FreeForAll => Color::srgb(0.9, 0.3, 0.3),
            GameMode::TeamDeathmatch => Color::srgb(0.3, 0.5, 0.9),
            GameMode::KillConfirmed => Color::srgb(0.9, 0.7, 0.2),
            GameMode::CaptureTheFlag => Color::srgb(0.3, 0.8, 0.5),
            GameMode::Assassins => Color::srgb(0.7, 0.2, 0.8),
            GameMode::KingOfTheHill => Color::srgb(0.9, 0.5, 0.1),
            GameMode::Hardpoint => Color::srgb(0.2, 0.7, 0.8),
            GameMode::CapturePoint => Color::srgb(0.5, 0.8, 0.3),
            GameMode::TestingGrounds => Color::srgb(0.4, 0.7, 0.9),
            GameMode::Juggernaut => Color::srgb(0.95, 0.2, 0.1),
            GameMode::HighExplosives => Color::srgb(1.0, 0.6, 0.0),
            GameMode::OneInTheChamber => Color::srgb(0.6, 0.6, 0.6),
            GameMode::GunGame => Color::srgb(0.2, 0.9, 0.4),
            GameMode::Infected => Color::srgb(0.4, 0.8, 0.1),
        }
    }

    pub fn competitive_modes() -> &'static [GameMode] {
        &[
            GameMode::FreeForAll,
            GameMode::TeamDeathmatch,
            GameMode::KillConfirmed,
            GameMode::CaptureTheFlag,
            GameMode::Assassins,
            GameMode::KingOfTheHill,
            GameMode::Hardpoint,
            GameMode::CapturePoint,
        ]
    }

    pub fn ltm_modes() -> &'static [GameMode] {
        &[
            GameMode::Juggernaut,
            GameMode::HighExplosives,
            GameMode::OneInTheChamber,
            GameMode::GunGame,
            GameMode::Infected,
        ]
    }

    pub fn is_team_mode(&self) -> bool {
        matches!(
            self,
            GameMode::TeamDeathmatch
                | GameMode::KillConfirmed
                | GameMode::CaptureTheFlag
                | GameMode::KingOfTheHill
                | GameMode::Hardpoint
                | GameMode::CapturePoint
                | GameMode::Infected
        )
    }

    pub fn is_ltm(&self) -> bool {
        matches!(
            self,
            GameMode::Juggernaut
                | GameMode::HighExplosives
                | GameMode::OneInTheChamber
                | GameMode::GunGame
                | GameMode::Infected
        )
    }
}

#[derive(Resource)]
pub struct SelectedGameMode {
    pub mode: GameMode,
}

impl Default for SelectedGameMode {
    fn default() -> Self {
        Self { mode: GameMode::FreeForAll }
    }
}

pub fn to_shared_gamemode(mode: GameMode) -> noctyrn_shared::GameMode {
    match mode {
        GameMode::FreeForAll => noctyrn_shared::GameMode::FreeForAll,
        GameMode::TeamDeathmatch => noctyrn_shared::GameMode::TeamDeathmatch,
        GameMode::KillConfirmed => noctyrn_shared::GameMode::KillConfirmed,
        GameMode::CaptureTheFlag => noctyrn_shared::GameMode::CaptureTheFlag,
        GameMode::Assassins => noctyrn_shared::GameMode::Assassins,
        GameMode::KingOfTheHill => noctyrn_shared::GameMode::KingOfTheHill,
        GameMode::Hardpoint => noctyrn_shared::GameMode::Hardpoint,
        GameMode::CapturePoint => noctyrn_shared::GameMode::CapturePoint,
        GameMode::TestingGrounds => noctyrn_shared::GameMode::TestingGrounds,
        GameMode::Juggernaut => noctyrn_shared::GameMode::Juggernaut,
        GameMode::HighExplosives => noctyrn_shared::GameMode::HighExplosives,
        GameMode::OneInTheChamber => noctyrn_shared::GameMode::OneInTheChamber,
        GameMode::GunGame => noctyrn_shared::GameMode::GunGame,
        GameMode::Infected => noctyrn_shared::GameMode::Infected,
    }
}

#[derive(Component)]
pub struct MenuCamera;

pub fn ensure_menu_camera(
    mut commands: Commands,
    existing: Query<Entity, With<MenuCamera>>,
) {
    if existing.is_empty() {
        commands.spawn((Camera2d, MenuCamera));
    }
}

pub fn despawn_menu_camera(mut commands: Commands, query: Query<Entity, With<MenuCamera>>) {
    for entity in query.iter() {
        commands.entity(entity).despawn();
    }
}

pub struct MenuPlugin;

impl Plugin for MenuPlugin {
    fn build(&self, app: &mut App) {
        app.init_resource::<loadout::LoadoutUiState>();
        app.init_resource::<loadout::LoadoutDragState>();
        app.init_resource::<crate_menu::CrateState>();
        app.init_resource::<crate_menu::CrateWeaponPickerState>();
        app.init_resource::<SelectedGameMode>();
        app.init_resource::<cosmetics::SellConfirmState>();
        app.init_resource::<login::LoginUiState>();
        app.init_resource::<profile::ProfileOverlayState>();
        app.init_resource::<friends::FriendsUiState>();
        app.init_resource::<main_menu::MatchmakingTimer>();
        app.init_resource::<invite::InviteTimer>();
        app.init_resource::<gamemode::ActiveGameModeTab>();

        app.add_systems(OnEnter(GameState::MainMenu), (
            main_menu::setup_main_menu_scene,
            main_menu::spawn_main_menu,
            login::try_auto_login,
        ));
        app.add_systems(OnExit(GameState::MainMenu), (
            main_menu::despawn_main_menu,
            main_menu::cleanup_main_menu_scene,
            main_menu::despawn_escape_menu,
            main_menu::despawn_server_notification,
            party::despawn_party_indicator,
            friends::despawn_friends_panel,
            invite::despawn_invite_banner,
            login::force_despawn_login_overlay,
            profile::force_despawn_profile_overlay,
        ));
        app.add_systems(Update, main_menu::main_menu_interaction.run_if(in_state(GameState::MainMenu)));
        app.add_systems(Update, main_menu::main_menu_hover.run_if(in_state(GameState::MainMenu)));
        app.add_systems(Update, main_menu::main_menu_profile_handler.run_if(in_state(GameState::MainMenu)));
        app.add_systems(Update, (
            main_menu::rotate_main_menu_pill,
            main_menu::matchmaking_notifier_update,
            main_menu::main_menu_matchmaking_handler,
            main_menu::game_mode_selector_visibility,
            main_menu::server_connection_notification,
        ).run_if(in_state(GameState::MainMenu)));

        // Profile overlay (within MainMenu)
        app.add_systems(Update, (
            profile::spawn_profile_overlay_system,
            profile::despawn_profile_overlay,
        ).run_if(in_state(GameState::MainMenu)));
        app.add_systems(Update, (
            profile::profile_interaction,
            profile::profile_update_data,
            profile::request_profile_data,
        ).run_if(in_state(GameState::MainMenu)));

        // Login overlay (within MainMenu)
        app.add_systems(Update, (
            login::spawn_login_overlay_system,
            login::despawn_login_overlay,
        ).run_if(in_state(GameState::MainMenu)));
        app.add_systems(Update, (
            login::login_interaction,
            login::login_text_input,
            login::update_login_display,
            login::login_handle_network_events,
        ).run_if(in_state(GameState::MainMenu)));

        app.add_systems(OnEnter(GameState::LoadoutSelect), (
            loadout::setup_loadout_scene,
            loadout::spawn_loadout_menu,
        ));
        app.add_systems(OnExit(GameState::LoadoutSelect), (
            loadout::despawn_loadout_menu,
            loadout::cleanup_loadout_scene,
        ));
        app.add_systems(Update, (
            loadout::loadout_interaction,
            loadout::update_loadout_ui,
            loadout::update_loadout_tabs,
            loadout::handle_loadout_drag,
            loadout::update_loadout_preview_model,
        ).run_if(in_state(GameState::LoadoutSelect)));

        app.add_systems(OnEnter(GameState::CrateOpening), (
            ensure_menu_camera, crate_menu::spawn_crate_menu,
        ));
        app.add_systems(OnExit(GameState::CrateOpening), crate_menu::despawn_crate_menu);
        app.add_systems(Update, (
            crate_menu::crate_interaction,
            crate_menu::update_crate_animation,
            crate_menu::crate_weapon_picker_interaction,
            crate_menu::crate_skip_interaction,
        ).run_if(in_state(GameState::CrateOpening)));

        app.add_systems(OnEnter(GameState::GameModeSelect), (
            ensure_menu_camera, gamemode::spawn_gamemode_menu,
        ));
        app.add_systems(OnExit(GameState::GameModeSelect), gamemode::despawn_gamemode_menu);
        app.add_systems(Update, (
            gamemode::gamemode_interaction,
            gamemode::gamemode_hover,
        ).run_if(in_state(GameState::GameModeSelect)));

        app.add_systems(OnEnter(GameState::Cosmetics), (
            ensure_menu_camera, cosmetics::spawn_cosmetics_menu,
        ));
        app.add_systems(OnExit(GameState::Cosmetics), cosmetics::despawn_cosmetics_menu);
        app.add_systems(Update, (
            cosmetics::cosmetics_interaction,
            cosmetics::cosmetics_hover,
            cosmetics::sell_confirm_interaction,
        ).run_if(in_state(GameState::Cosmetics)));

        app.add_systems(OnEnter(GameState::Playing), despawn_menu_camera);

        // Party indicator overlay (spawn/despawn based on party_state)
        app.add_systems(Update, party::spawn_party_indicator.run_if(in_state(GameState::MainMenu)));
        app.add_systems(Update, party::despawn_party_indicator.run_if(in_state(GameState::MainMenu)));

        // Friends panel overlay (toggle, spawn, actions)
        app.add_systems(Update, (
            friends::toggle_friends_panel,
            friends::close_friends_panel,
            friends::friends_click_outside,
            friends::spawn_friends_panel,
            friends::despawn_friends_panel,
            friends::friends_search_input,
            friends::friends_search_focus_handler,
            friends::friends_add_button_handler,
            friends::friends_party_invite_handler,
            friends::friends_confirm_remove_handler,
            friends::friends_cancel_remove_handler,
            friends::friends_remove_handler,
            friends::friends_accept_request_handler,
            friends::friends_decline_request_handler,
            friends::friends_tab_interaction,
            friends::friends_handle_network_events,
            friends::friends_go_to_profile_handler,
        ).run_if(in_state(GameState::MainMenu)));

        // Invite banner (visible in any menu state)
        app.add_systems(Update, (
            invite::spawn_invite_banner,
            invite::despawn_invite_banner,
            invite::invite_accept_handler,
            invite::invite_decline_handler,
            invite::invite_timer_tick,
        ));

        // Escape menu
        app.add_systems(Update, (
            main_menu::escape_menu_interaction,
        ).run_if(in_state(GameState::MainMenu)));

        // Party kick handler (visible from MainMenu)
        app.add_systems(Update, party::party_kick_handler.run_if(in_state(GameState::MainMenu)));
    }
}
