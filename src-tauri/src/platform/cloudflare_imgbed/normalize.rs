pub fn normalize_directory(value: &str) -> String {
    value
        .trim()
        .trim_matches('/')
        .split('/')
        .filter(|part| !part.is_empty() && *part != ".")
        .collect::<Vec<_>>()
        .join("/")
}

pub fn join_directory(directory: &str, name: &str) -> String {
    let directory = normalize_directory(directory);
    let name = name.trim().trim_matches('/');
    if directory.is_empty() {
        name.to_string()
    } else {
        format!("{directory}/{name}")
    }
}
