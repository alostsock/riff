import { runInAction, makeAutoObservable } from "mobx";

import { list_media_files, Media } from "./tauri-commands";

class MediaState {
	directory: string | null = null;
	isLoading = false;

	media: Media | null = null;

	constructor() {
		makeAutoObservable(this);
	}

	async setDirectory(path: string) {
		this.directory = path;
		this.isLoading = true;
		const media = await list_media_files(path);
		runInAction(() => {
			this.media = media;
			this.isLoading = false;
		});
	}
}

export default new MediaState();
