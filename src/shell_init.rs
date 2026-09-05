pub const BASH: &str = r#"fileblade-file-widget() {
  local prefix=${READLINE_LINE:0:READLINE_POINT}
  local word=${prefix##* }
  local selected
  selected=$(fileblade pick --mode open --multiple --root "$PWD" --query "$word" --shell-quote) || return
  READLINE_LINE="${prefix:0:${#prefix}-${#word}}${selected}${READLINE_LINE:READLINE_POINT}"
  READLINE_POINT=$((${#prefix} - ${#word} + ${#selected}))
}

__fileblade_cd__() {
  local dir
  dir=$(fileblade pick --mode folder --root "$PWD") || return
  printf 'builtin cd -- %q' "$dir"
}

fb() {
  builtin cd -- "$(fileblade root)"
}

bind -m emacs-standard '"\C-\e(": redraw-current-line'
bind -m vi-command '"\C-z": emacs-editing-mode'
bind -m vi-insert '"\C-z": emacs-editing-mode'
bind -m emacs-standard '"\C-z": vi-editing-mode'
bind -m emacs-standard -x "\"${FILEBLADE_FILE_KEY:-\\C-t}\": fileblade-file-widget"
bind -m vi-command -x "\"${FILEBLADE_FILE_KEY:-\\C-t}\": fileblade-file-widget"
bind -m vi-insert -x "\"${FILEBLADE_FILE_KEY:-\\C-t}\": fileblade-file-widget"
bind -m emacs-standard "\"${FILEBLADE_CD_KEY:-\\ec}\": \" \\C-b\\C-k \\C-u\`__fileblade_cd__\`\\e\\C-e\\C-\\e(\\C-m\\C-y\\C-h\\e \\C-y\\ey\\C-x\\C-x\\C-d\\C-y\\ey\\C-_\""
bind -m vi-command "\"${FILEBLADE_CD_KEY:-\\ec}\": \"\\C-z${FILEBLADE_CD_KEY:-\\ec}\\C-z\""
bind -m vi-insert "\"${FILEBLADE_CD_KEY:-\\ec}\": \"\\C-z${FILEBLADE_CD_KEY:-\\ec}\\C-z\""
"#;

pub const ZSH: &str = r#"fileblade-file-widget() {
  local word=${LBUFFER##* }
  local selected
  selected=$(fileblade pick --mode open --multiple --root "$PWD" --query "$word" --shell-quote) || { zle redisplay; return 0 }
  LBUFFER="${LBUFFER[1,$((${#LBUFFER} - ${#word}))]}${selected}"
  zle reset-prompt
}

fileblade-cd-widget() {
  setopt localoptions pipefail no_aliases 2> /dev/null
  local dir
  dir=$(fileblade pick --mode folder --root "$PWD") || { zle redisplay; return 0 }
  zle push-line
  BUFFER="builtin cd -- ${(q)dir}"
  zle accept-line
  local ret=$?
  unset dir
  zle reset-prompt
  return $ret
}

fb() {
  builtin cd -- "$(fileblade root)"
}

zle -N fileblade-file-widget
zle -N fileblade-cd-widget
for keymap in emacs vicmd viins; do
  bindkey -M "$keymap" "${FILEBLADE_FILE_KEY:-^T}" fileblade-file-widget
  bindkey -M "$keymap" "${FILEBLADE_CD_KEY:-\ec}" fileblade-cd-widget
done
"#;

pub fn script(zsh: bool) -> &'static str {
    if zsh { ZSH } else { BASH }
}

pub fn shell_quote(path: &str) -> String {
    let safe = !path.is_empty()
        && path
            .bytes()
            .all(|byte| byte.is_ascii_alphanumeric() || b"_./-+:@%,=".contains(&byte));
    if safe {
        path.to_string()
    } else {
        format!("'{}'", path.replace('\'', "'\\''"))
    }
}
