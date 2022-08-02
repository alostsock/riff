import { invoke } from "@tauri-apps/api/tauri";

export function list_files() {
	invoke("list_files");
}
