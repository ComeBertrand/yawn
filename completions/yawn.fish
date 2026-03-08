# Disable file completions by default
complete -c yawn -f

# Subcommands
complete -c yawn -n "__fish_use_subcommand" -a list -d 'Recursively discover git projects under a directory'
complete -c yawn -n "__fish_use_subcommand" -a resolve -d 'Map a pretty name back to an absolute path'
complete -c yawn -n "__fish_use_subcommand" -a pick -d 'Interactively pick a project and open a terminal in it'
complete -c yawn -n "__fish_use_subcommand" -a open -d 'Open a terminal in the given directory'
complete -c yawn -n "__fish_use_subcommand" -a create -d 'Create a git worktree for the current project'
complete -c yawn -n "__fish_use_subcommand" -a delete -d 'Remove a worktree for the current project'
complete -c yawn -n "__fish_use_subcommand" -a init -d 'Initialize the current directory'

# Global flags
complete -c yawn -s h -l help -d 'Print help'
complete -c yawn -s V -l version -d 'Print version'

# list
complete -c yawn -n "__fish_seen_subcommand_from list" -l json -d 'Output as JSON array'
complete -c yawn -n "__fish_seen_subcommand_from list" -l raw -d 'Output absolute paths, one per line'
complete -c yawn -n "__fish_seen_subcommand_from list" -l porcelain -d 'Force stable flat output for scripting'
complete -c yawn -n "__fish_seen_subcommand_from list" -F

# resolve
complete -c yawn -n "__fish_seen_subcommand_from resolve" -s P -l path -d 'Directory to search' -r -F

# pick
complete -c yawn -n "__fish_seen_subcommand_from pick" -s F -l finder -d 'Finder command' -r
complete -c yawn -n "__fish_seen_subcommand_from pick" -F

# open
complete -c yawn -n "__fish_seen_subcommand_from open" -s c -l command -d 'Command to open the terminal' -r
complete -c yawn -n "__fish_seen_subcommand_from open" -a "(yawn complete open 2>/dev/null)"

# create
complete -c yawn -n "__fish_seen_subcommand_from create" -s s -l source -d 'Base branch/ref' -r
complete -c yawn -n "__fish_seen_subcommand_from create" -s o -l open -d 'Open a terminal after creation'
complete -c yawn -n "__fish_seen_subcommand_from create" -s i -l init -d 'Run init after creating the worktree'

# delete
complete -c yawn -n "__fish_seen_subcommand_from delete" -s b -l branch -d 'Also delete the local branch'
complete -c yawn -n "__fish_seen_subcommand_from delete" -s f -l force -d 'Force removal even if worktree has uncommitted changes'
complete -c yawn -n "__fish_seen_subcommand_from delete" -a "(yawn complete delete 2>/dev/null)"
