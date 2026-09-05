.pragma library

var names = {
  "agents.md": "󰍔",
  "changelog.md": "󰍔",
  "contributing.md": "󰍔",
  "todo.md": "󰍔",
  "readme.md": "󰍔",
  "license": "",
  "dockerfile": "󰡨",
  "makefile": "󱁤",
  "cmakelists.txt": "󱁤",
  "justfile": "󰖷",
  "gemfile": "󰴭",
  "rakefile": "󰴭",
  "cargo.toml": "",
  "cargo.lock": "",
  "pyproject.toml": "",
  "requirements.txt": "󱘎",
  "go.mod": "󰫺",
  "go.sum": "󰟓",
  "package.json": "󰘦",
  "package-lock.json": "󰘦",
  "pnpm-lock.yaml": "",
  "yarn.lock": "",
  "tsconfig.json": "󰘦",
  ".gitignore": "󰊢",
  ".gitmodules": "󰒓",
  ".editorconfig": "",
  ".prettierrc": "󰘦",
  ".eslintrc": "󰘦"
}

var extensions = {
  "lua": "",
  "py": "",
  "pyi": "",
  "js": "",
  "mjs": "",
  "cjs": "",
  "ts": "",
  "jsx": "",
  "tsx": "",
  "json": "",
  "jsonc": "",
  "yaml": "",
  "yml": "",
  "sh": "",
  "bash": "",
  "zsh": "",
  "fish": "",
  "md": "󰍔",
  "markdown": "󰍔",
  "css": "",
  "scss": "󰌜",
  "html": "",
  "htm": "",
  "go": "",
  "rs": "",
  "rb": "",
  "php": "",
  "java": "",
  "cs": "󰌛",
  "sql": "",
  "graphql": "",
  "gql": "",
  "xml": "󰗀",
  "toml": "",
  "ini": "󰯂",
  "conf": "󰒓",
  "pdf": "",
  "svg": "󰜡",
  "jpg": "󰈥",
  "jpeg": "󰈥",
  "png": "",
  "gif": "󰵸",
  "webp": "󰈟",
  "mp4": "󰈫",
  "mkv": "󰈫",
  "mov": "󰈫",
  "mp3": "󰈣",
  "wav": "󰈣",
  "flac": "󰈣",
  "zip": "󰗄",
  "gz": "󰗄",
  "xz": "󰗄",
  "tar": "󰗄",
  "qml": "",
  "cpp": "",
  "cc": "",
  "cxx": "",
  "c": "",
  "h": "󰫵",
  "hpp": "󰫵",
  "vue": "",
  "svelte": "",
  "csv": "",
  "txt": "󰦪"
}

function fileIcon(name, isSymlink) {
  if (isSymlink) return ""
  var lower = String(name || "").toLowerCase()
  if (names[lower]) return names[lower]
  var dot = lower.lastIndexOf(".")
  var extension = dot >= 0 ? lower.slice(dot + 1) : ""
  return extensions[extension] || "󰈔"
}

function entryIcon(name, isDir, isSymlink, expanded, isGitRepo) {
  if (isSymlink) return fileIcon(name, true)
  if (isDir) return isGitRepo ? "󰊢" : folderIcon(expanded)
  return fileIcon(name, false)
}

function folderIcon(expanded) {
  return expanded ? "" : ""
}

function expanderIcon(expanded) {
  return expanded ? "" : ""
}

function safeThemeIconName(value) {
  var icon = String(value || "")
  return /^[A-Za-z0-9][A-Za-z0-9._-]{0,255}$/.test(icon) ? icon : ""
}
