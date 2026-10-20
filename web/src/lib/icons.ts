// SPDX-License-Identifier: AGPL-3.0-or-later
// Copyright (c) 2026 Theo-lbg

// Icons: Material Symbols (Apache-2.0), imported by name so that only the
// icons listed here end up in the bundle.
const files = import.meta.glob(
	'/node_modules/@material-symbols/svg-400/outlined/{add,search,tune,help,settings,home,folder,folder-fill,folder_open,folder_shared,group,schedule,star,star-fill,delete,close,check,cloud,cloud_upload,upload_file,drive_folder_upload,create_new_folder,download,share,link,edit,drive_file_move,content_copy,more_vert,arrow_upward,arrow_downward,arrow_drop_down,view_list,grid_view,info,chevron_right,keyboard_arrow_down,keyboard_arrow_up,description,image,movie,audio_file,picture_as_pdf,folder_zip,code,text_snippet,table_chart,slideshow,draft,restore_from_trash,delete_forever,person_add,lock,public,logout,manage_accounts,storage,dns,check_circle,error,refresh,arrow_back,zoom_in,zoom_out,fullscreen,open_in_new,keyboard_arrow_left,keyboard_arrow_right,menu,filter_list,visibility,person,warning,cancel,check_box,check_box_outline_blank,indeterminate_check_box,cloud_done,sort,content_cut,done_all,hourglass_empty,undo,contrast,light_mode,dark_mode,apps,photo_library,photo_camera,play_arrow,pause,comment,history,send}.svg',
	{ query: '?raw', import: 'default', eager: true }
) as Record<string, string>;

/** name → inner SVG markup (viewBox is always "0 -960 960 960"). */
export const ICONS: Record<string, string> = {};
for (const [path, svg] of Object.entries(files)) {
	const name = path.split('/').pop()!.replace('.svg', '');
	const inner = svg.replace(/^[\s\S]*?<svg[^>]*>/, '').replace(/<\/svg>\s*$/, '');
	ICONS[name] = inner;
}

export type IconName = string;
