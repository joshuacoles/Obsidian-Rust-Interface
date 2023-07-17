use walkdir::DirEntry;

pub fn is_hidden(entry: &DirEntry) -> bool {
    entry
        .file_name()
        .to_str()
        .map(|s| s.starts_with('.'))
        .unwrap_or(false)
}

pub fn is_markdown(entry: &DirEntry) -> bool {
    entry.path().extension().map(|s| s == "md").unwrap_or(false)
}
