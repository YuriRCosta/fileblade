use super::*;

pub(super) fn parse_search_query(
    query: &str,
    options: SearchOptions,
) -> Result<SearchSpec, String> {
    let text = query.trim().chars().collect::<Vec<_>>();
    let mut spec = SearchSpec {
        options,
        ..SearchSpec::default()
    };
    let mut index = 0_usize;
    while index < text.len() {
        while index < text.len() && text[index].is_whitespace() {
            index += 1;
        }
        if index >= text.len() {
            break;
        }
        let negate = index + 1 < text.len()
            && matches!(text[index], '-' | '!')
            && !text[index + 1].is_whitespace();
        if negate {
            index += 1;
        }
        let (key, next) = read_field(&text, index);
        index = next;
        let (value, exact, next) = read_value(&text, index);
        index = next;
        if !value.is_empty() {
            append_search_value(&mut spec, &key, value, exact, negate);
        }
    }
    for term in &mut spec.terms {
        term.case_sensitive = options.case_sensitive || term.exact;
        if options.regex {
            term.pattern = Some(
                RegexBuilder::new(&term.text)
                    .case_insensitive(!term.case_sensitive)
                    .size_limit(1 << 20)
                    .build()
                    .map_err(|error| format!("invalid regex {:?}: {error}", term.text))?,
            );
        }
    }
    Ok(spec)
}

pub(super) fn read_field(text: &[char], index: usize) -> (String, usize) {
    let mut cursor = index;
    while cursor < text.len() {
        if text[cursor] == ':' {
            if cursor == index {
                return (String::new(), index);
            }
            let candidate = text[index..cursor]
                .iter()
                .collect::<String>()
                .to_lowercase();
            if matches!(
                candidate.as_str(),
                "type"
                    | "format"
                    | "ext"
                    | "extension"
                    | "mime"
                    | "in"
                    | "name"
                    | "path"
                    | "content"
                    | "grep"
                    | "c"
                    | "scope"
            ) {
                return (candidate, cursor + 1);
            }
            return (String::new(), index);
        }
        if text[cursor].is_whitespace() || text[cursor] == '"' {
            break;
        }
        cursor += 1;
    }
    (String::new(), index)
}

pub(super) fn read_value(text: &[char], index: usize) -> (String, bool, usize) {
    if index < text.len() && text[index] == '"' {
        let closing = text[index + 1..]
            .iter()
            .position(|character| *character == '"')
            .map(|offset| index + 1 + offset);
        let end = closing.unwrap_or(text.len());
        return (
            text[index + 1..end].iter().collect(),
            true,
            closing.map(|value| value + 1).unwrap_or(text.len()),
        );
    }
    let mut end = index;
    while end < text.len() && !text[end].is_whitespace() {
        end += 1;
    }
    (text[index..end].iter().collect(), false, end)
}

pub(super) fn split_term_syntax(value: String, exact: bool, regex: bool) -> (String, TermKind) {
    if exact || regex {
        let kind = if exact {
            TermKind::Substring
        } else {
            TermKind::Fuzzy
        };
        return (value, kind);
    }
    let (mut text, mut kind) = match value.as_bytes() {
        [b'\'', ..] => (&value[1..], TermKind::Substring),
        [b'^', ..] => (&value[1..], TermKind::Prefix),
        _ => (value.as_str(), TermKind::Fuzzy),
    };
    let literal_dollar = text.ends_with("\\$");
    if let Some(rest) = text
        .strip_suffix('$')
        .filter(|rest| !literal_dollar && !rest.is_empty())
    {
        text = rest;
        kind = match kind {
            TermKind::Fuzzy => TermKind::Suffix,
            _ => TermKind::Exact,
        };
    }
    (text.replace("\\$", "$"), kind)
}

pub(super) fn append_search_value(
    spec: &mut SearchSpec,
    key: &str,
    value: String,
    exact: bool,
    negate: bool,
) {
    let normalized_key = match key {
        "ext" | "extension" => "format",
        "grep" | "c" => "content",
        value => value,
    };
    if normalized_key == "content" {
        spec.content = Some(value);
        return;
    }
    if normalized_key == "scope" {
        spec.scope = match value.to_lowercase().as_str() {
            "home" | "~" => Scope::Home,
            "everywhere" | "all" | "/" => Scope::Everywhere,
            _ => Scope::Here,
        };
        return;
    }
    if !matches!(normalized_key, "type" | "format" | "mime" | "in") {
        let (text, kind) = split_term_syntax(value, exact, spec.options.regex);
        if text.is_empty() {
            return;
        }
        spec.terms.push(SearchTerm {
            text,
            exact,
            negate,
            field: if key == "name" {
                "name".to_string()
            } else {
                String::new()
            },
            case_sensitive: exact,
            pattern: None,
            kind,
        });
        return;
    }
    for raw_part in value.split(',') {
        let value = normalize_filter(normalized_key, raw_part.trim());
        if !value.is_empty() {
            spec.filters.push(SearchFilter {
                key: normalized_key.to_string(),
                value,
                negate,
            });
        }
    }
}

pub(super) fn normalize_filter(key: &str, value: &str) -> String {
    match key {
        "type" => match value.to_lowercase().as_str() {
            "folder" | "folders" | "dir" | "dirs" | "directory" | "directories" => {
                "dir".to_string()
            }
            "file" | "files" => "file".to_string(),
            "link" | "links" | "symlink" | "symlinks" => "link".to_string(),
            "repo" | "repos" | "git" | "repository" => "repo".to_string(),
            "image" | "images" | "img" | "picture" => "image".to_string(),
            "text" | "txt" => "text".to_string(),
            "video" | "videos" => "video".to_string(),
            "audio" | "sound" | "music" => "audio".to_string(),
            _ => String::new(),
        },
        "format" => value.to_lowercase().trim_start_matches('.').to_string(),
        "mime" => value.to_lowercase(),
        "in" => value.trim_matches('/').to_string(),
        _ => value.to_string(),
    }
}

impl SearchSpec {
    pub(super) fn empty(&self) -> bool {
        self.terms.is_empty() && self.filters.is_empty() && self.content.is_none()
    }

    pub(super) fn globs(&self) -> Vec<String> {
        let mut globs = Vec::new();
        for (key, negate) in [
            ("format", false),
            ("format", true),
            ("in", false),
            ("in", true),
        ] {
            for value in self.values(key, negate) {
                let glob = if key == "format" {
                    format!("*.{value}")
                } else {
                    format!("{value}/**")
                };
                globs.push(if negate { format!("!{glob}") } else { glob });
            }
        }
        globs
    }

    pub(super) fn values<'a>(
        &'a self,
        key: &'a str,
        negate: bool,
    ) -> impl Iterator<Item = &'a str> {
        self.filters
            .iter()
            .filter(move |filter| filter.key == key && filter.negate == negate)
            .map(|filter| filter.value.as_str())
    }

    pub(super) fn matcher_pattern(&self) -> String {
        self.terms
            .iter()
            .filter(|term| term.matcher_handled())
            .map(SearchTerm::matcher_atom)
            .collect::<Vec<_>>()
            .join(" ")
    }

    pub(super) fn verified_terms(&self) -> impl Iterator<Item = &SearchTerm> {
        self.terms.iter().filter(|term| !term.matcher_handled())
    }

    pub(super) fn accepts_entry(&self, relative: &str, flags: &IndexFlags) -> bool {
        let name = relative.rsplit('/').next().unwrap_or(relative);
        if !self
            .verified_terms()
            .all(|term| term.matches(name, relative))
        {
            return false;
        }
        let wanted = self.values("type", false).collect::<Vec<_>>();
        let excluded = self.values("type", true).collect::<Vec<_>>();
        let plain_kind = if flags.is_dir { "dir" } else { "file" };
        if excluded.contains(&plain_kind) || (flags.is_symlink && excluded.contains(&"link")) {
            return false;
        }
        let structural = ["dir", "file", "link"];
        let wanted_structural = wanted
            .iter()
            .filter(|kind| structural.contains(kind))
            .collect::<Vec<_>>();
        if !wanted_structural.is_empty()
            && wanted.iter().all(|kind| structural.contains(kind))
            && !wanted_structural
                .iter()
                .any(|kind| **kind == plain_kind || (**kind == "link" && flags.is_symlink))
        {
            return false;
        }
        let extension = extension_of(name);
        if !allowed_value(
            &extension,
            self.values("format", false),
            self.values("format", true),
        ) {
            return false;
        }
        allowed_starts_with(
            &relative.to_lowercase(),
            self.values("in", false),
            self.values("in", true),
            true,
        )
    }

    pub(super) fn describe(&self) -> String {
        self.terms
            .iter()
            .filter(|term| {
                term.negate || term.exact || !term.field.is_empty() || term.kind != TermKind::Fuzzy
            })
            .map(SearchTerm::describe)
            .chain(self.filters.iter().map(SearchFilter::describe))
            .chain(
                self.content
                    .iter()
                    .map(|needle| format!("content:{needle:?}")),
            )
            .chain(
                (self.scope != Scope::Here)
                    .then(|| format!("scope:{:?}", self.scope).to_lowercase()),
            )
            .collect::<Vec<_>>()
            .join("  ")
    }
}

impl SearchTerm {
    pub(super) fn matcher_handled(&self) -> bool {
        self.field.is_empty() && !self.exact && self.pattern.is_none()
    }

    pub(super) fn matcher_atom(&self) -> String {
        let mut atom = String::new();
        if self.negate {
            atom.push('!');
        }
        match self.kind {
            TermKind::Prefix | TermKind::Exact => atom.push('^'),
            TermKind::Substring => atom.push('\''),
            TermKind::Fuzzy | TermKind::Suffix => {}
        }
        atom.push_str(&index::escape_atom(&self.text));
        if matches!(self.kind, TermKind::Suffix | TermKind::Exact) {
            atom.push('$');
        }
        atom
    }

    pub(super) fn matches(&self, name: &str, relative: &str) -> bool {
        let haystack = if self.field == "name" { name } else { relative };
        let found = if let Some(pattern) = &self.pattern {
            pattern.is_match(haystack)
        } else {
            let (source, needle) = if self.case_sensitive {
                (haystack.to_string(), self.text.clone())
            } else {
                (haystack.to_lowercase(), self.text.to_lowercase())
            };
            match self.kind {
                TermKind::Fuzzy | TermKind::Substring => source.contains(&needle),
                TermKind::Prefix => source.starts_with(&needle),
                TermKind::Suffix => source.ends_with(&needle),
                TermKind::Exact => source == needle,
            }
        };
        if self.negate { !found } else { found }
    }

    pub(super) fn occurrences(&self, haystack: &str) -> Vec<(usize, usize)> {
        if let Some(pattern) = &self.pattern {
            return pattern
                .find_iter(haystack)
                .filter(|found| found.end() > found.start())
                .map(|found| {
                    let start = haystack[..found.start()].chars().count();
                    let end = start + haystack[found.start()..found.end()].chars().count();
                    (start, end)
                })
                .collect();
        }
        let (source, needle) = if self.case_sensitive {
            (haystack.to_string(), self.text.clone())
        } else {
            (haystack.to_lowercase(), self.text.to_lowercase())
        };
        if needle.is_empty() {
            return Vec::new();
        }
        let needle_chars = needle.chars().count();
        let source_chars = source.chars().count();
        match self.kind {
            TermKind::Prefix | TermKind::Exact => {
                if source.starts_with(&needle) {
                    vec![(0, needle_chars)]
                } else {
                    Vec::new()
                }
            }
            TermKind::Suffix => {
                if source.ends_with(&needle) {
                    vec![(source_chars - needle_chars, source_chars)]
                } else {
                    Vec::new()
                }
            }
            TermKind::Fuzzy | TermKind::Substring => source
                .match_indices(&needle)
                .map(|(byte, _)| {
                    let start = source[..byte].chars().count();
                    (start, start + needle_chars)
                })
                .collect(),
        }
    }

    pub(super) fn describe(&self) -> String {
        let mut text = String::new();
        if self.negate {
            text.push('-');
        }
        if !self.field.is_empty() {
            text.push_str(&self.field);
            text.push(':');
        }
        if self.exact {
            text.push('"');
            text.push_str(&self.text);
            text.push('"');
            return text;
        }
        match self.kind {
            TermKind::Prefix | TermKind::Exact => text.push('^'),
            TermKind::Substring => text.push('\''),
            TermKind::Fuzzy | TermKind::Suffix => {}
        }
        text.push_str(&self.text);
        if matches!(self.kind, TermKind::Suffix | TermKind::Exact) {
            text.push('$');
        }
        text
    }
}

impl SearchFilter {
    pub(super) fn describe(&self) -> String {
        format!(
            "{}{}:{}",
            if self.negate { "-" } else { "" },
            self.key,
            self.value
        )
    }
}

pub(super) fn extension_of(name: &str) -> String {
    name.rfind('.')
        .filter(|index| *index > 0 && *index < name.len() - 1)
        .map(|index| name[index + 1..].to_lowercase())
        .unwrap_or_default()
}
