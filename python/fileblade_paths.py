from __future__ import annotations

import os
import re
import shlex
from typing import Any
from urllib.parse import quote_from_bytes, unquote_to_bytes, urlsplit


def has_native_bytes(value: str) -> bool:
    return any(0xDC80 <= ord(char) <= 0xDCFF for char in value)


def path_text(value: str, base: str = "") -> str:
    if not has_native_bytes(value):
        return str(value)
    absolute = value if os.path.isabs(value) else os.path.join(base, value)
    if not os.path.isabs(absolute):
        raise ValueError("a native-byte path needs an absolute base")
    return "file://" + quote_from_bytes(os.fsencode(absolute), safe="/-._~")


def uri_path(value: str) -> str:
    if re.search(r"[\x00-\x20\x7f\\?#]", value):
        raise ValueError("file URI characters must be percent-encoded")
    parts = urlsplit(value)
    if not value[5:].startswith("//") or parts.netloc not in ("", "localhost"):
        raise ValueError("expected a local absolute file URI")
    if not parts.path.startswith("/"):
        raise ValueError("expected a local absolute file URI")
    if re.search(r"%(?![0-9a-fA-F]{2})", parts.path):
        raise ValueError("invalid file URI escape")
    return parts.path


def parse_path(value: str) -> str:
    if "\0" in value:
        raise ValueError("a path cannot contain NUL")
    if value[:5].lower() != "file:":
        return value
    raw = unquote_to_bytes(uri_path(value))
    if b"\0" in raw:
        raise ValueError("a path cannot contain NUL")
    return os.path.normpath(os.fsdecode(raw))


def display(value: str) -> str:
    parts = []
    for char in value:
        code = ord(char)
        if 0xDC80 <= code <= 0xDCFF:
            parts.append(f"\\x{code - 0xDC00:02X}")
        elif char in ("\n", "\r", "\t"):
            parts.append({"\n": "\\n", "\r": "\\r", "\t": "\\t"}[char])
        elif code < 32 or 127 <= code <= 159:
            parts.append(f"\\u{{{code:X}}}")
        else:
            parts.append("\\\\" if char == "\\" else char)
    return "".join(parts)


class NativePath(str):
    def __new__(cls, value: str, base: str = "") -> NativePath:
        instance = super().__new__(cls, value)
        instance.base = base
        return instance


def wire(value: Any) -> Any:
    if isinstance(value, NativePath):
        return path_text(value, value.base)
    if isinstance(value, dict):
        return {key: wire(item) for key, item in value.items()}
    if isinstance(value, (list, tuple)):
        return [wire(item) for item in value]
    if isinstance(value, str) and has_native_bytes(value):
        return display(value)
    return value


def shell_quote(value: str) -> str:
    if has_native_bytes(value):
        return "$'" + "".join(f"\\x{byte:02x}" for byte in os.fsencode(value)) + "'"
    return shlex.quote(value)
