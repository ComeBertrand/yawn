_yawn() {
    local cur prev
    cur="${COMP_WORDS[COMP_CWORD]}"
    prev="${COMP_WORDS[COMP_CWORD-1]}"
    case "$prev" in
        list)
            COMPREPLY=($(compgen -d -- "$cur"))
            ;;
        delete)
            COMPREPLY=($(compgen -W "$(yawn complete delete 2>/dev/null)" -- "$cur"))
            ;;
        open)
            COMPREPLY=($(compgen -W "$(yawn complete open 2>/dev/null)" -- "$cur"))
            ;;
        pick)
            COMPREPLY=($(compgen -d -- "$cur"))
            ;;
        *)
            COMPREPLY=($(compgen -W "list create delete open pick resolve" -- "$cur"))
            ;;
    esac
}
complete -F _yawn yawn
