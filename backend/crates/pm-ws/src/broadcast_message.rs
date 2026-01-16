/// A broadcast message that will be sent to clients                                                                                                                             
#[derive(Debug, Clone)]
pub struct BroadcastMessage {
    /// Serialized protobuf message (ready to send on wire)                                                                                                                      
    pub payload: bytes::Bytes,
    /// Message type for metrics/logging                                                                                                                                         
    pub message_type: String,
}

impl BroadcastMessage {
    pub fn new(payload: bytes::Bytes, message_type: String) -> Self {
        Self {
            payload,
            message_type,
        }
    }
}
