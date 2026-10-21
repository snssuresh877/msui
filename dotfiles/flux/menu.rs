
"      Open Line in Nvim" => "text/all", "alacritty -e nvim +%l %p", "no_command_dialog"
# --- Core Operations ---
"󰋼      Add to Quick List" => "directory", "builtin::add_to_quick_list"
"󰓩      Open in New Tab" => "directory", "builtin::open_new_tab"
"󰪶      Send to Quick List" => "all", "builtin::quick_list_transfer"
"󰱝      Other Application..." => "file", "builtin::open_with_dialog"
"󰉋      Inspect Directory" => "directory", "builtin::inspect_dir"
"󰆏      Copy" => "all", "builtin::copy"
"󰆐      Cut" => "all", "builtin::cut"
"󰏊      Paste" => "all", "builtin::paste"
"󰑕      Rename" => "all", "builtin::rename"
"󰓹      Edit Tags" => "file", "builtin::tagfile"
"󰐕      Install Font" => "font/all", "builtin::install_fonts"
"󰩹      Delete" => "all", "builtin::delete"
"󰦬      Restore File" => "trash", "gio trash --restore %p"
"󰆴      Shred File (Permanent)" => "all", "python3 $HOME/.local/share/flux/scripts/flux_shredder.py %p", "no_command_dialog"

# --- Navigation & System ---
"      Open Terminal" => "directory", "alacritty --working-directory=%p", "no_command_dialog"
"󰋜      Toggle Pin" => "directory", "builtin::toggle_pin"
"󰨞      Open in VSCode" => "text/all, application/all", "code %p", "no_command_dialog"
"󰋽      File Properties" => "file", "flux-fm --file-properties %p", "no_command_dialog"
"󰋊      Folder Info" => "directory", "baobab %p", "no_command_dialog"
"󰉋      New Folder" => "directory", "builtin::new_folder"
"󰈔      New File" => "directory", "builtin::new_file"

# --- Icon Customization ---
"󰸉      Icons > Select Folder Icon" => "directory", "builtin::select_folder_icon"
"󰸉      Icons > Set File Icon" => "all", "builtin::set_custom_icon"
"󰸉      Icons > Reset File Icon" => "all", "builtin::reset_custom_icon"
"󰸉      Icons > Set Extension Icon" => "file", "builtin::set_extension_icon"
"󰸉      Icons > Reset Extension Icon" => "file", "builtin::reset_extension_icon"

# --- Archives & Compression ---
"󰛖      Compress > To ZIP" => "all", "/usr/bin/python $HOME/.local/share/flux/scripts/flux_simple_compressor.py %p"
"󰛖      Compress > To 7Z" => "all", "7z a -m0=lzma2 -mx=9 archive.7z %f"
"󰛖      Compress > To TAR.GZ" => "all", "/usr/bin/python $HOME/.local/share/flux/scripts/flux_simple_compressor.py %p"
"󰛖      Extract Here!" => "application/zip, application/x-7z-compressed, application/x-rar, application/x-tar", "7z x %p -o%p_extracted"

# --- Media Edit ---
"󰽰      Media Edit > Join Videos" => "video/all", "python3 $HOME/.local/share/flux/scripts/join_videos.py %p"
"󰽰      Media Edit > Cut Video" => "video/all", "python $HOME/.local/share/flux/scripts/video_cutter.py %p", "no_command_dialog"
"󰽰      Media Edit > Mix Audio" => "audio/", "python3 $HOME/.local/share/flux/scripts/mix_audio.py %p"
"󰽰      Media Edit > Merge Video + Audio" => "video/all+audio/all", "python3 $HOME/.local/share/flux/scripts/join_mp4_mp3.py %p"

# --- Media Convert ---
"󰽰      Media Convert > To MP4" => "video/all", "ffmpeg -i %p -codec copy %p.mp4"
"󰽰      Media Convert > To MKV" => "video/all", "ffmpeg -i %p -codec copy %p.mkv"
"󰽰      Media Convert > To WebM" => "video/all", "ffmpeg -i %p -codec copy %p.webm"
"󰽰      Media Convert > To MOV" => "video/all", "ffmpeg -i %p -codec copy %p.mov"
"󰝚      Media Convert > Audio to MP3" => "audio/x-opus+ogg, audio/vnd.wave, audio/ogg", "ffmpeg -i %p -vn -ab 192k -ar 44100 -y %p.mp3"

# --- Media Optimization & Extraction ---
"󰠝      Media Extract > MP3 from Video" => "video/all", "ffmpeg -i %p -vn -acodec libmp3lame -q:a 2 %p.mp3"
"󰕧      Media Optimize > Reduce Video Size" => "video/all", "ffmpeg -i %p -vcodec libx265 -crf 28 -tag:v hvc1 -preset faster %p_reduced.mp4"

# --- Images & Wallpaper ---
"󰸉      Image Wallpaper > Set (swww)" => "image/all", "swww img %p"
"󰸉      Image Wallpaper > Set (wbg)" => "image/all", "cp %p ~/Images/fav.jpg && (killall wbg 2>/dev/null; (wbg -s $HOME/Images/fav.jpg </dev/null >/dev/null 2>&1 &))", "no_command_dialog"
"󰸉      Image Convert > To PNG" => "image/webp", "magick %p %p.png"
"󰸉      Image Convert > To AVIF" => "image/all", "avifenc --jobs all -q 65 %p %p.avif"
"󰸉      Image Convert > To JPG" => "image/all", "magick %p -quality 75 -strip %p-output.jpg"
"󰏦      Convert to PDF" => "image/all, application/pdf, application/msword, application/vnd.openxmlformats-officedocument.wordprocessingml.document", "python3 $HOME/.local/share/flux/scripts/pdf_converter.py %p"

# --- Flux Backgrounds ---
"󰸉      Flux Background > Window Background"       => "image/all", "builtin::set_bg_window", "Window background updated!"
"󰸉      Flux Background > Left Sidebar Background" => "image/all", "builtin::set_bg_sidebar_left", "Sidebar background updated!"
"󰸉      Flux Background > Right Sidebar Background"=> "image/all", "builtin::set_bg_sidebar_right", "Right panel background updated!"
"󰸉      Flux Background > Reset All Backgrounds" => "all", "builtin::clear_backgrounds", "Backgrounds reset to default"

# --- Tools ---
"󰯦      Tools > Git Gui" => "directory", "git gui", "no_command_dialog"
"󰯦      Tools > Download Video (1080p)" => "directory", "cd %p && yt-dlp -f 'bv[height<=1080]+ba/b[height<=1080]' $(wl-paste)"
"󰯦      Tools > Copy Path" => "all", "echo -n %p | wl-copy"
"󰯦      Tools > Copy Name" => "all", "basename %p | tr -d '\n' | wl-copy"
"󰯦      Tools > Advanced Archive Manager" => "all", "/usr/bin/python $HOME/.local/share/flux/scripts/flux_compressor.py %p", "no_command_dialog"
