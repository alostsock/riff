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
	image_ids: string[];
}

export interface Image {
	id: string;
	path: string;
	thumb: string | null;
}

export interface Album {
	name: string;
	track_ids: string[];
	image_ids: string[];
}

export interface Artist {
	name: string;
	albums: Record<string, Album>;
	track_ids: string[];
}

export interface DirectoryContent {
	image_ids: string[];
	track_ids: string[];
}

export interface Media {
	root: string;
	tracks: Record<string, Track>;
	images: Record<string, Image>;
	artists: Record<string, Artist>;
	directories: Record<string, DirectoryContent>;
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
