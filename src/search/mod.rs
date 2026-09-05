use crate::AppResult;
use crate::common::{
    display_path, error_payload, expanded_path, parse_path, path_error, path_text,
};
use crate::filesystem::entry_for_path_with_git;
use crate::git::{
    GitRepository, cached_git_repositories_for_markers_bounded, decorate_git_entry,
    deleted_git_entry, indexed_git_counts_for_path, indexed_git_status_for_path,
    nearest_git_marker, repository_status_index_cancellable,
};
use crate::index::{self, Hit, IndexEntry, IndexFlags};
use nucleo::Matcher;
use nucleo::pattern::Pattern;
use regex::{Regex, RegexBuilder};
use serde_json::{Value, json};
use std::collections::{HashMap, HashSet};
use std::io;
use std::path::{Path, PathBuf};
use std::sync::atomic::{AtomicBool, Ordering};
use std::thread;
use std::time::{Duration, Instant};

mod parse;
mod rows;
mod run;
use parse::*;
use rows::*;
pub use run::*;
pub const SEARCH_CANDIDATE_CAP: usize = 2000;
pub const SEARCH_RESPONSE_BYTES: usize = 4 * 1024 * 1024;
pub const SEARCH_DEADLINE: Duration = Duration::from_secs(4);
pub const SEARCH_BACKEND: &str = "nucleo";
const SEARCH_CANDIDATE_MULTIPLIER: usize = 4;
const SEARCH_QUERY_BYTES: usize = 64 * 1024;
const SEARCH_REPOSITORY_CAP: usize = 64;
const SEARCH_PROGRESS_INTERVAL: Duration = Duration::from_millis(300);
const SEARCH_TICK_MS: u64 = 20;
const SEARCH_IDLE_SLEEP: Duration = Duration::from_millis(12);

#[derive(Clone, Copy, Debug, Default)]
pub struct SearchOptions {
    pub case_sensitive: bool,
    pub regex: bool,
}

#[derive(Clone, Copy, Debug, Eq, PartialEq)]
enum TermKind {
    Fuzzy,
    Substring,
    Prefix,
    Suffix,
    Exact,
}

#[derive(Clone, Debug)]
struct SearchTerm {
    text: String,
    exact: bool,
    negate: bool,
    field: String,
    case_sensitive: bool,
    pattern: Option<Regex>,
    kind: TermKind,
}

#[derive(Clone, Debug)]
struct SearchFilter {
    key: String,
    value: String,
    negate: bool,
}

#[derive(Clone, Debug, Default)]
struct SearchSpec {
    terms: Vec<SearchTerm>,
    filters: Vec<SearchFilter>,
    options: SearchOptions,
    content: Option<String>,
    scope: Scope,
}

#[derive(Clone, Copy, Debug, Default, Eq, PartialEq)]
enum Scope {
    #[default]
    Here,
    Home,
    Everywhere,
}

#[derive(Clone, Copy, Debug)]
pub struct SearchRequest<'a> {
    pub root: &'a str,
    pub query: &'a str,
    pub show_hidden: bool,
    pub limit: usize,
    pub repository_roots: &'a [String],
    pub options: SearchOptions,
    pub fresh: bool,
    pub list: Option<&'a str>,
    pub tree: bool,
    pub git_enabled: bool,
}

pub fn query_document(query: &str) -> Value {
    match parse_search_query(query, SearchOptions::default()) {
        Ok(spec) => json!({
            "ok": true,
            "terms": spec.terms.iter().map(|term| json!({
                "text": term.text,
                "negate": term.negate,
                "exact": term.exact,
                "field": term.field,
                "kind": match term.kind {
                    TermKind::Fuzzy => "fuzzy",
                    TermKind::Substring => "substring",
                    TermKind::Prefix => "prefix",
                    TermKind::Suffix => "suffix",
                    TermKind::Exact => "exact",
                },
            })).collect::<Vec<_>>(),
            "filters": spec.filters.iter().map(|filter| json!({
                "key": filter.key,
                "value": filter.value,
                "negate": filter.negate,
            })).collect::<Vec<_>>(),
            "content": spec.content,
        }),
        Err(error) => json!({"ok": false, "error": error}),
    }
}
