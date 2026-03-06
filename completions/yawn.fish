# Disable file completions by default
complete -c yawn -f

# Subcommands
complete -c yawn -n "__fish_use_subcommand" -a list -d 'Recursively discover git projects under a directory'
complete -c yawn -n "__fish_use_subcommand" -a resolve -d 'Map a pretty name back to an absolute path'
complete -c yawn -n "__fish_use_subcommand" -a open -d 'Open a terminal in the given directory'
complete -c yawn -n "__fish_use_subcommand" -a create -d 'Create a git worktree for the current project'
complete -c yawn -n "__fish_use_subcommand" -a delete -d 'Remove a worktree for the current project'

# Global flags
complete -c yawn -s h -l help -d 'Print help'
complete -c yawn -s V -l version -d 'Print version'

# list
complete -c yawn -n "__fish_seen_subcommand_from list" -s p -l pretty -d 'Show human-readable names with annotations'
complete -c yawn -n "__fish_seen_subcommand_from list" -F

# resolve
complete -c yawn -n "__fish_seen_subcommand_from resolve" -s P -l path -d 'Directory to search' -r -F

# open — dynamic completions
complete -c yawn -n "__fish_seen_subcommand_from open" -a "(yawn complete open 2>/dev/null)"

# create
complete -c yawn -n "__fish_seen_subcommand_from create" -s s -l source -d 'Base branch/ref' -r
complete -c yawn -n "__fish_seen_subcommand_from create" -s o -l open -d 'Open a terminal after creation'

# delete — dynamic completions
complete -c yawn -n "__fish_seen_subcommand_from delete" -a "(yawn complete delete 2>/dev/null)"
