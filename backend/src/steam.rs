use crate::{ArtworkPaths, SteamGame};

pub fn get_steam_games_stub() -> Vec<SteamGame> {
    // Full implementation plan:
    // - Linux: scan ~/.steam/steam/steamapps/ for .acf app manifest files.
    // - Windows: read Steam install path from registry, then scan steamapps.
    // - Parse AppID/name/install path from manifests and return Vec<SteamGame>.
    vec![
        SteamGame {
            app_id: "1".into(),
            name: "Skyrim Special Edition".into(),
            install_path: "".into(),
        },
        SteamGame {
            app_id: "2".into(),
            name: "Fallout 4".into(),
            install_path: "".into(),
        },
        SteamGame {
            app_id: "3".into(),
            name: "Cyberpunk 2077".into(),
            install_path: "".into(),
        },
        SteamGame {
            app_id: "4".into(),
            name: "Elden Ring".into(),
            install_path: "".into(),
        },
        SteamGame {
            app_id: "5".into(),
            name: "Baldur's Gate 3".into(),
            install_path: "".into(),
        },
        SteamGame {
            app_id: "6".into(),
            name: "Stardew Valley".into(),
            install_path: "".into(),
        },
        SteamGame {
            app_id: "7".into(),
            name: "The Witcher 3".into(),
            install_path: "".into(),
        },
        SteamGame {
            app_id: "8".into(),
            name: "RimWorld".into(),
            install_path: "".into(),
        },
        SteamGame {
            app_id: "9".into(),
            name: "Mount & Blade II".into(),
            install_path: "".into(),
        },
        SteamGame {
            app_id: "10".into(),
            name: "Kenshi".into(),
            install_path: "".into(),
        },
    ]
}

pub fn get_game_artwork_stub(app_id: String) -> ArtworkPaths {
    ArtworkPaths {
        banner: format!(
            "~/.steam/steam/appcache/librarycache/{}_library_600x900.jpg",
            app_id
        ),
        hero: format!(
            "~/.steam/steam/appcache/librarycache/{}_library_hero.jpg",
            app_id
        ),
    }
}
