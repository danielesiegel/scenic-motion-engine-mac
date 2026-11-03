# fish completion for the `render` binary (scenic-motion-engine)
# Install: copy to ~/.config/fish/completions/render.fish

complete -c render -f

complete -c render -l audio       -r -F -d 'Input audio track (wav/mp3/flac/aac)'
complete -c render -l analysis    -r -F -d 'JSON analysis file produced by the analyzer'
complete -c render -l out    -s o -r -F -d 'Output MP4 path (required)'
complete -c render -l start       -r    -d 'Start offset in seconds (default 0)'
complete -c render -l duration    -r    -d 'Render length in seconds (default 10)'
complete -c render -l fps         -r    -d 'Frames per second (default 60)' -a '24 30 48 60 90 120'
complete -c render -l width       -r    -d 'Frame width in pixels (default 1920)' -a '640 1280 1920 2560 3840'
complete -c render -l height      -r    -d 'Frame height in pixels (default 1080)' -a '360 720 1080 1440 2160'
complete -c render -l fonts       -r -F -d 'Font directory (default assets/fonts)'
complete -c render -l help   -s h       -d 'Show help and exit'
