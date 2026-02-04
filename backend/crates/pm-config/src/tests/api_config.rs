use crate::{ApiConfig, DEFAULT_LLM_USER_ID, DEFAULT_LLM_USER_NAME};

#[test]
fn test_default_api_config() {
    let config = ApiConfig::default();
    assert!(config.enabled);
    assert_eq!(config.llm_user_id, DEFAULT_LLM_USER_ID);
    assert_eq!(config.llm_user_name, DEFAULT_LLM_USER_NAME);
}

#[test]
fn test_llm_user_uuid_valid() {
    let config = ApiConfig::default();
    let uuid = config.llm_user_uuid();
    assert_eq!(uuid.to_string(), DEFAULT_LLM_USER_ID);
}

#[test]
fn test_llm_user_uuid_invalid_falls_back() {
    let config = ApiConfig {
        llm_user_id: "not-a-uuid".to_string(),
        ..Default::default()
    };
    let uuid = config.llm_user_uuid();
    assert_eq!(uuid.to_string(), DEFAULT_LLM_USER_ID);
}
