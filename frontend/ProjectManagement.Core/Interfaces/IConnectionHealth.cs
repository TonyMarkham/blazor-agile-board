  namespace ProjectManagement.Core.Interfaces;                                                                                                     
                                                                                                                                                   
  public interface IConnectionHealth                                                                                                               
  {                                                                                                                                                
      ConnectionQuality Quality { get; }                                                                                                           
      TimeSpan? Latency { get; }                                                                                                                   
      DateTime? LastMessageReceived { get; }                                                                                                       
      DateTime? LastMessageSent { get; }                                                                                                           
      int PendingRequestCount { get; }                                                                                                             
      int ReconnectAttempts { get; }                                                                                                               
  }                                                                                                                                                
                                                                                                                                                   
  public enum ConnectionQuality                                                                                                                    
  {                                                                                                                                                
      Unknown,                                                                                                                                     
      Excellent,  // <100ms latency                                                                                                                
      Good,       // 100-300ms                                                                                                                     
      Fair,       // 300-1000ms                                                                                                                    
      Poor,       // >1000ms or packet loss                                                                                                        
      Disconnected                                                                                                                                 
  }  