#compdef yawn

_yawn() {
    local -a commands
    commands=(
        'list:Recursively discover git projects under a directory'
        'resolve:Map a pretty name back to an absolute path'
        'open:Open a terminal in the given directory'
        'create:Create a git worktree for the current project'
        'delete:Remove a worktree for the current project'
    )

    _arguments -C \
        '-h[Print help]' \
        '--help[Print help]' \
        '-V[Print version]' \
        '--version[Print version]' \
        '1:command:->cmd' \
        '*::arg:->args'

    case $state in
        cmd)
            _describe -t commands 'yawn commands' commands
            ;;
        args)
            case $words[1] in
                list)
                    _arguments \
                        '-p[Show human-readable names with annotations]' \
                        '--pretty[Show human-readable names with annotations]' \
                        '-h[Print help]' \
                        '--help[Print help]' \
                        '::path:_files -/'
                    ;;
                resolve)
                    _arguments \
                        '-P+[Directory to search]:path:_files -/' \
                        '--path=[Directory to search]:path:_files -/' \
                        '-h[Print help]' \
                        '--help[Print help]' \
                        ':name:'
                    ;;
                open)
                    _arguments \
                        '-h[Print help]' \
                        '--help[Print help]' \
                        ':path:{compadd $(yawn complete open 2>/dev/null)}'
                    ;;
                create)
                    _arguments \
                        '-s+[Base branch/ref]:source:' \
                        '--source=[Base branch/ref]:source:' \
                        '-o[Open a terminal in the worktree after creation]' \
                        '--open[Open a terminal in the worktree after creation]' \
                        '-h[Print help]' \
                        '--help[Print help]' \
                        ':name:'
                    ;;
                delete)
                    _arguments \
                        '-h[Print help]' \
                        '--help[Print help]' \
                        ':name:{compadd $(yawn complete delete 2>/dev/null)}'
                    ;;
            esac
            ;;
    esac
}

_yawn "$@"
