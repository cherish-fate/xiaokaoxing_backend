pub fn is_valid_email(email: &str) -> bool {
    if email.is_empty() || !email.contains('@') {
        return false;
    }
    let parts: Vec<&str> = email.split('@').collect();
    parts.len() == 2
        && !parts[0].is_empty()
        && parts[1].contains('.')
        && !parts[1].starts_with('.')
        && !parts[1].ends_with('.')
}
