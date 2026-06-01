import { invoke } from '@tauri-apps/api/core';

export const getSteamGames = (steamPath = null) => invoke('get_steam_games', { steamPath });
export const addManualGame = (name, installPath) => invoke('add_manual_game', { name, installPath });
export const setWorkingDirectory = (path = null) => invoke('set_working_directory', { path });
export const getSettingsSchema = () => invoke('get_settings_schema');
export const getSettingsValues = () => invoke('get_settings_values');
export const saveSettingsValues = (values) => invoke('save_settings_values', { values });
export const getGameArtwork = (appId, steamPath = null) => invoke('get_game_artwork', { appId, steamPath });
export const getGameConfig = (gameId) => invoke('get_game_config', { gameId });
export const saveGameConfig = (gameId, config) => invoke('save_game_config', { gameId, config });
