_yawn() {
    local cur prev
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    case "$prev" in
        list)
            COMPREPLY=($(compgen -d -- "$cur"))
            ;;
        delete)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=($(compgen -W "--branch --force --help" -- "$cur"))
            else
                COMPREPLY=($(compgen -W "$(yawn complete delete 2>/dev/null)" -- "$cur"))
            fi
            ;;
        open)
            COMPREPLY=($(compgen -W "$(yawn complete open 2>/dev/null)" -- "$cur"))
            ;;
        pick)
            COMPREPLY=($(compgen -d -- "$cur"))
            ;;
        create)
            if [[ "$cur" == -* ]]; then
                COMPREPLY=($(compgen -W "--source --open --init --help" -- "$cur"))
            fi
            ;;
        *)
            COMPREPLY=($(compgen -W "list create delete open pick resolve init" -- "$cur"))
            ;;
    esac
}
complete -F _yawn yawn
