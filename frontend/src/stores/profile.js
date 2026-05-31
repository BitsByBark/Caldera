import { writable } from 'svelte/store';

export const profiles = writable([]);
export const activeProfile = writable(null);
