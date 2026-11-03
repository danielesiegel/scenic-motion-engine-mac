# bash completion for the `render` binary (scenic-motion-engine)
# Install: source this file, or copy to /usr/local/etc/bash_completion.d/render

_render() {
    local cur prev words cword
    _init_completion || return

    local flags="--audio --analysis --out -o --start --duration --fps --width --height --fonts --help -h"

    case "$prev" in
        --audio)
            _filedir '@(wav|mp3|flac|aac|m4a|ogg)'
            return 0
            ;;
        --analysis)
            _filedir json
            return 0
            ;;
        --out|-o)
            _filedir mp4
            return 0
            ;;
        --fonts)
            _filedir -d
            return 0
            ;;
        --start|--duration)
            # numeric seconds (float)
            COMPREPLY=()
            return 0
            ;;
        --fps)
            COMPREPLY=( $(compgen -W "24 30 48 60 90 120" -- "$cur") )
            return 0
            ;;
        --width)
            COMPREPLY=( $(compgen -W "640 1280 1920 2560 3840" -- "$cur") )
            return 0
            ;;
        --height)
            COMPREPLY=( $(compgen -W "360 720 1080 1440 2160" -- "$cur") )
            return 0
            ;;
    esac

    if [[ "$cur" == -* ]]; then
        COMPREPLY=( $(compgen -W "$flags" -- "$cur") )
        return 0
    fi

    COMPREPLY=()
    return 0
}

complete -F _render render
