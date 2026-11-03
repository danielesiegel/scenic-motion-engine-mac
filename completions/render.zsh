#compdef render
# zsh completion for the `render` binary (scenic-motion-engine)
# Install: place in a directory on $fpath (e.g. /usr/local/share/zsh/site-functions/_render)
# and ensure `autoload -Uz compinit && compinit` is run.

_render() {
    local -a args
    args=(
        '--audio[input audio track (wav/mp3/flac/aac)]:audio file:_files -g "*.(wav|mp3|flac|aac|m4a|ogg)"'
        '--analysis[JSON analysis file produced by the analyzer]:analysis json:_files -g "*.json"'
        '(--out -o)'{--out,-o}'[output MP4 path (required)]:output mp4:_files -g "*.mp4"'
        '--start[start offset in seconds (default 0)]:seconds:'
        '--duration[render length in seconds (default 10)]:seconds:'
        '--fps[frames per second (default 60)]:fps:(24 30 48 60 90 120)'
        '--width[frame width in pixels (default 1920)]:width:(640 1280 1920 2560 3840)'
        '--height[frame height in pixels (default 1080)]:height:(360 720 1080 1440 2160)'
        '--fonts[font directory (default assets/fonts)]:fonts dir:_files -/'
        '(-h --help)'{-h,--help}'[show help and exit]'
    )
    _arguments -s -S $args
}

_render "$@"
