import { invoke } from "@tauri-apps/api/tauri";

export async function watch(path: string) {
	invoke("watch", { path });
}

export interface Track {
	id: string;
	path: string;
	tag_format: string;

	title: string | null;
	artist: string | null;
	album: string | null;
	album_artist: string | null;
	disc: number | null;
	track: number | null;
	duration: number | null;
	image: number | null;
}

export interface Image {
	id: string;
	path: string;
	thumb: string | null;
}

export interface Media {
	tracks: Record<string, Track>;
	images: Record<string, Image>;
}

export async function list_media_files(directory: string): Promise<Media> {
	const start = performance.now();
	const media: Media = await invoke("list_media_files", { directory });
	console.log(
		[
			`fetched in ${performance.now() - start} ms:`,
			`\t${Object.keys(media.tracks).length} tracks`,
			`\t${Object.keys(media.images).length} images`,
		].join("\n")
	);
	return media;
}
