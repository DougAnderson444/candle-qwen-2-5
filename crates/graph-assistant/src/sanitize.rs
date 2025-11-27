fn sanitize_id(id: &str) -> String {
    // DOT spec: IDs can be:
    // 1. Alphanumeric + underscore (no leading digit)
    // 2. Numeral [-]?(.[0-9]+ | [0-9]+(.[0-9]*)?)
    // 3. Quoted string "..."
    // 4. HTML string <...> (NOT SUPPORTED by dot_parser)

    if is_simple_id(id) {
        id.to_string()
    } else {
        // Quote it and escape special chars
        format!("\"{}\"", escape_quotes(id))
    }
}

fn is_simple_id(s: &str) -> bool {
    if s.is_empty() {
        return false;
    }

    let first = s.chars().next().unwrap();

    // Check if it's a valid simple ID (alphanumeric + underscore, no leading digit)
    first.is_alphabetic() || first == '_' && s.chars().all(|c| c.is_alphanumeric() || c == '_')
}

fn escape_quotes(s: &str) -> String {
    s.replace('\\', "\\\\")
        .replace('"', "\\\"")
        .replace('\n', "\\n")
        .replace('\r', "\\r")
}
