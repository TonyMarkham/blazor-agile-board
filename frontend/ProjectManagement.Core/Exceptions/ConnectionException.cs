  namespace ProjectManagement.Core.Exceptions;                                                                                                     
                                                                                                                                                   
  /// <summary>                                                                                                                                    
  /// Connection-related failures (WebSocket connection issues).                                                                                   
  /// </summary>                                                                                                                                   
  public sealed class ConnectionException : ProjectManagementException                                                                             
  {                                                                                                                                                
      public override string ErrorCode => "CONNECTION_FAILED";                                                                                     
      public override string UserMessage => "Unable to connect to server. Please check your connection.";                                          
                                                                                                                                                   
      public ConnectionState LastKnownState { get; init; }                                                                                         
      public TimeSpan? RetryAfter { get; init; }                                                                                                   
                                                                                                                                                   
      public ConnectionException(string message) : base(message) { }                                                                               
      public ConnectionException(string message, Exception inner) : base(message, inner) { }                                                       
  }                                                                                                                                                
                                                                                                                                                   
  /// <summary>                                                                                                                                    
  /// Connection state enum for tracking WebSocket status.                                                                                         
  /// </summary>                                                                                                                                   
  public enum ConnectionState                                                                                                                      
  {                                                                                                                                                
      Disconnected,                                                                                                                                
      Connecting,                                                                                                                                  
      Connected,                                                                                                                                   
      Reconnecting,                                                                                                                                
      Closed                                                                                                                                       
  } 