import { writable } from 'svelte/store'

export const settings = writable({
    theme: 'rebellion',
    ui_scale: 100,
    text_scale: 100,
    accent_color: '#E8B84B',
    parallel_downloads: 4,
    metadata_cache: 'auto',
    steam_path: '',
    working_directory: '',
    sort_order: 'date',
    default_game: 'none',
})
