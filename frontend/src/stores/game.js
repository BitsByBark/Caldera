import { writable } from 'svelte/store';
import { getSteamGames } from '../lib/tauri';

export const gameList = writable([]);
export const currentGame = writable(null);

export async function loadGameList() {
  const games = await getSteamGames();
  gameList.set(games);
}
