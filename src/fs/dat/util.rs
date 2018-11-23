pub fn normalize_path(path: &str) -> String {
    let mut r = String::with_capacity(path.len());

    for c in path.chars() {
        build_normalized_path(&mut r, Some(c));
    }
    build_normalized_path(&mut r, None);

    r
}

pub fn build_normalized_path(path: &mut String, c: Option<char>) {
    if let Some(mut c) = c {
        c = if c == '/' {
            '\\'
        } else {
            c.to_ascii_lowercase()
        };

        path.push(c);
    }

    if path == ".\\" || c.is_none() && path == "." {
        path.truncate(0);
    } else if path.ends_with("\\.\\") {
        let l = path.len();
        path.remove(l - 1);
        path.remove(l - 2);
    }
}

#[cfg(test)]
mod tests {
    use super::normalize_path;

    #[test]
    fn normalizes_path_backslash() {
        assert_eq!(normalize_path("."), "");
        assert_eq!(normalize_path(".\\"), "");
        assert_eq!(normalize_path(".\\tSt"), "tst");
        assert_eq!(normalize_path(".\\."), "");
        assert_eq!(normalize_path(".\\.TsT"), ".tst");
        assert_eq!(normalize_path(".\\.tst\\."), ".tst\\.");
        assert_eq!(normalize_path(".\\.tst\\.\\tst2"), ".tst\\tst2");
    }

    #[test]
    fn normalizes_path_forward_slash() {
        assert_eq!(normalize_path("./"), "");
        assert_eq!(normalize_path("./tst"), "tst");
        assert_eq!(normalize_path("./."), "");
        assert_eq!(normalize_path("./.tst"), ".tst");
        assert_eq!(normalize_path("./.tst/."), ".tst\\.");
        assert_eq!(normalize_path("./.tst/./tst2"), ".tst\\tst2");
    }

    #[test]
    fn normalizes_path_mixed_slashes() {
        assert_eq!(normalize_path("./"), "");
        assert_eq!(normalize_path("./tst"), "tst");
        assert_eq!(normalize_path("./."), "");
        assert_eq!(normalize_path("./.tst"), ".tst");
        assert_eq!(normalize_path("./.tst\\."), ".tst\\.");
        assert_eq!(normalize_path("./.tst\\./tst2"), ".tst\\tst2");
    }
}
