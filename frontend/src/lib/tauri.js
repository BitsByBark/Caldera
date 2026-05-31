import { invoke } from '@tauri-apps/api/core';

export const getSteamGames = (steamPath = null) => invoke('get_steam_games', { steamPath });
export const getGameArtwork = (appId, steamPath = null) => invoke('get_game_artwork', { appId, steamPath });
export const getGameConfig = (gameId) => invoke('get_game_config', { gameId });
export const saveGameConfig = (gameId, config) => invoke('save_game_config', { gameId, config });
