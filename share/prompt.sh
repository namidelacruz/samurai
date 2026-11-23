# samurai: written by Samurai (Settings › Look › Terminal). Sourced by ~/.bashrc and ~/.zshrc.
# Line one: the folder, the git branch and how many files changed, how long the last command
# took (when it took two seconds or more) and its error code (only when it failed).
# Line two: › and your command. Turn it off in Settings and these lines are taken out again.

__samurai_git() {
    command -v git >/dev/null 2>&1 || return 0
    __sb=$(git symbolic-ref --short HEAD 2>/dev/null || git rev-parse --short HEAD 2>/dev/null) || return 0
    __sn=$(git status --porcelain 2>/dev/null | wc -l | tr -d ' ')
    if [ "${__sn:-0}" -gt 0 ]; then printf '%s %s' "$__sb" "~$__sn"; else printf '%s' "$__sb"; fi
}

__samurai_took() {
    __st=$1
    if [ "$__st" -ge 3600 ]; then printf '%dh%dm' $((__st / 3600)) $((__st % 3600 / 60))
    elif [ "$__st" -ge 60 ]; then printf '%dm%ds' $((__st / 60)) $((__st % 60))
    elif [ "$__st" -ge 2 ]; then printf '%ds' "$__st"
    fi
}

if [ -n "${BASH_VERSION:-}" ]; then
    __samurai_start=
    __samurai_debug() { [ -z "$__samurai_start" ] && __samurai_start=$SECONDS; }
    trap '__samurai_debug' DEBUG
    __samurai_ps1() {
        local code=$? took="" g tail=""
        [ -n "$__samurai_start" ] && took=$(__samurai_took $((SECONDS - __samurai_start)))
        __samurai_start=
        g=$(__samurai_git)
        [ -n "$took" ] && tail="$tail  \[\e[2m\]$took\[\e[0m\]"
        [ "$code" -ne 0 ] && tail="$tail  \[\e[31m\]✕ $code\[\e[0m\]"
        local branch=""
        if [ -n "$g" ]; then
            branch="  \[\e[34m\]${g%% *}\[\e[0m\]"
            [ "$g" != "${g%% *}" ] && branch="$branch \[\e[33m\]${g#* }\[\e[0m\]"
        fi
        PS1="\n\[\e[1m\]\w\[\e[0m\]$branch$tail  \[\e[2m\]\A\[\e[0m\]\n\[\e[1;34m\]›\[\e[0m\] "
    }
    case ";${PROMPT_COMMAND:-};" in
        *"__samurai_ps1"*) ;;
        *) PROMPT_COMMAND="__samurai_ps1${PROMPT_COMMAND:+;$PROMPT_COMMAND}" ;;
    esac
elif [ -n "${ZSH_VERSION:-}" ]; then
    zmodload zsh/datetime 2>/dev/null
    autoload -Uz add-zsh-hook
    setopt PROMPT_SUBST
    __samurai_preexec() { __samurai_start=$EPOCHSECONDS; }
    __samurai_precmd() {
        local code=$? took="" g
        [ -n "$__samurai_start" ] && took=$(__samurai_took $((EPOCHSECONDS - __samurai_start)))
        __samurai_start=
        g=$(__samurai_git)
        local branch=""
        if [ -n "$g" ]; then
            branch="  %F{blue}${g%% *}%f"
            [ "$g" != "${g%% *}" ] && branch="$branch %F{yellow}${g#* }%f"
        fi
        PROMPT=$'\n'"%B%~%b$branch"$'\n'"%B%F{blue}›%f%b "
        RPROMPT=""
        [ -n "$took" ] && RPROMPT="%F{8}$took%f "
        [ "$code" -ne 0 ] && RPROMPT="$RPROMPT%F{red}✕ $code%f "
        RPROMPT="$RPROMPT%F{8}%T%f"
    }
    add-zsh-hook preexec __samurai_preexec
    add-zsh-hook precmd __samurai_precmd
fi
