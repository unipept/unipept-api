//! No request parameter is a plain `bool`.
//!
//! `Flag` makes a boolean parameter strict, and the compiler rejects most ways around it: a field
//! using one of the shared `default_*` functions has to be a `Flag`, and `Flag` has no `Default`,
//! so a bare `#[serde(default)]` on one does not compile.
//!
//! It does not reject a new parameter that brings its own `fn default_x() -> bool`, or one with no
//! serde attribute at all. Both compile and both read `?flag=` as `true`. This reads the controller
//! sources and fails on either, so there is no list to keep in step with the code.

use std::{fs, path::Path};

/// Drops comments, so a `bool` named in prose is not read as a field and a field carrying a
/// trailing note is still read as one. `sort_by: String, // Can be "id", "name", or "rank"` is the
/// shape that matters: without cutting the note, the line does not end in its own type.
fn without_comments(source: &str) -> String {
    source
        .lines()
        .map(|line| line.split_once("//").map_or(line, |(code, _)| code))
        .filter(|line| !line.trim().is_empty())
        .collect::<Vec<_>>()
        .join("\n")
}

/// Every `struct` body whose derive names `Deserialize`, as `(name, body)`.
fn deserialized_structs(source: &str) -> Vec<(String, String)> {
    let mut found = Vec::new();
    for (index, _) in source.match_indices("struct ") {
        let before = &source[..index];
        let Some(derive) = before.rfind("#[derive(") else { continue };

        // the derive has to be the attribute block on this struct, not one further up the file
        if before[derive..].contains('}') || !before[derive..].contains("Deserialize") {
            continue;
        }

        let rest = &source[index..];
        let (Some(open), Some(close)) = (rest.find('{'), rest.find("\n}")) else { continue };
        if close < open {
            continue;
        }

        found.push((rest["struct ".len()..open].trim().to_string(), rest[open..close].to_string()));
    }
    found
}

#[test]
fn no_request_parameter_is_a_plain_bool() {
    let controllers = Path::new(env!("CARGO_MANIFEST_DIR")).join("src/controllers");
    let mut offenders = Vec::new();
    let mut structs = 0;

    let mut directories = vec![controllers.clone()];
    while let Some(directory) = directories.pop() {
        for entry in fs::read_dir(&directory).expect("the controllers directory is readable") {
            let path = entry.expect("a directory entry").path();
            if path.is_dir() {
                directories.push(path);
                continue;
            }
            if path.extension().is_none_or(|extension| extension != "rs") {
                continue;
            }

            let source = without_comments(&fs::read_to_string(&path).expect("a source file"));
            for (name, body) in deserialized_structs(&source) {
                structs += 1;
                for line in body.lines() {
                    let field = line.trim().trim_end_matches(',');
                    if field.ends_with(": bool") || field.ends_with(": Option<bool>") {
                        let file = path.strip_prefix(&controllers).unwrap_or(&path).display();
                        offenders.push(format!("{file}: {name}.{field}"));
                    }
                }
            }
        }
    }

    // Guards the scan itself: if the parsing ever stops matching, this fails rather than passing
    // over an empty set.
    assert!(structs > 10, "the scan found only {structs} deserialized structs, so it is not reading the sources");

    assert!(
        offenders.is_empty(),
        "these request parameters are a plain `bool`, so they read `?flag=` as `true`. Use `Flag`:\n  {}",
        offenders.join("\n  ")
    );
}
