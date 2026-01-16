//! Standard error codes for WebSocket responses.                                                      

/// Input validation failed                                                                            
pub const VALIDATION_ERROR: &str = "VALIDATION_ERROR";

/// Resource not found                                                                                 
pub const NOT_FOUND: &str = "NOT_FOUND";

/// User lacks required permission                                                                     
pub const UNAUTHORIZED: &str = "UNAUTHORIZED";

/// Optimistic lock conflict - resource was modified                                                   
pub const CONFLICT: &str = "CONFLICT";

/// Delete blocked due to dependencies                                                                 
pub const DELETE_BLOCKED: &str = "DELETE_BLOCKED";

/// Internal server error                                                                              
pub const INTERNAL_ERROR: &str = "INTERNAL_ERROR";

/// Invalid message format                                                                             
pub const INVALID_MESSAGE: &str = "INVALID_MESSAGE";

/// Rate limit exceeded                                                                                
pub const RATE_LIMITED: &str = "RATE_LIMITED";
