# treenav zsh integration
# Add this to your .zshrc: source /path/to/treenav.zsh

# Function to navigate with treenav
tn() {
    local dir
    dir=$(treenav "$@")
    if [[ -n "$dir" && -d "$dir" ]]; then
        cd "$dir"
    fi
}
