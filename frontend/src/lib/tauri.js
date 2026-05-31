import { invoke } from '@tauri-apps/api/core';

export const getSteamGames = () => invoke('get_steam_games');
export const getGameArtwork = (appId) => invoke('get_game_artwork', { appId });
export const getGameConfig = (gameId) => invoke('get_game_config', { gameId });
export const saveGameConfig = (gameId, config) => invoke('save_game_config', { gameId, config });
