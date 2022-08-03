import { invoke } from "@tauri-apps/api/tauri";

export async function watch(path: string) {
	invoke("watch", { path });
}

export interface Media {
	tracks: string[];
	images: string[];
}

export async function list_media_files(path: string): Promise<Media> {
	return await invoke("list_media_files", { path });
}
