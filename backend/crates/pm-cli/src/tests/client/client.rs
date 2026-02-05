use crate::Client;

#[test]
fn test_base_url_trailing_slash_trimmed() {
    let client = Client::new("http://localhost:8000/", None);
    assert_eq!(client.base_url, "http://localhost:8000");
}

#[test]
fn test_base_url_no_trailing_slash() {
    let client = Client::new("http://localhost:8000", None);
    assert_eq!(client.base_url, "http://localhost:8000");
}

#[test]
fn test_user_id_stored() {
    let client = Client::new("http://localhost:8000", Some("user-123"));
    assert_eq!(client.user_id, Some("user-123".to_string()));
}

#[test]
fn test_user_id_none() {
    let client = Client::new("http://localhost:8000", None);
    assert!(client.user_id.is_none());
}
