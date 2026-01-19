  namespace ProjectManagement.Core.Interfaces;                                                                                                     
                                                                                                                                                   
  /// <summary>                                                                                                                                    
  /// Generic store interface for entities with common operations.                                                                                 
  /// </summary>                                                                                                                                   
  public interface IEntityStore<T> : IDisposable where T : IEntity, ISoftDeletable                                                                 
  {                                                                                                                                                
      /// <summary>Fired when store contents change.</summary>                                                                                     
      event Action? OnChanged;                                                                                                                     
                                                                                                                                                   
      /// <summary>Get entity by ID (returns null if not found or deleted).</summary>                                                              
      T? GetById(Guid id);                                                                                                                         
                                                                                                                                                   
      /// <summary>Get all non-deleted entities.</summary>                                                                                         
      IReadOnlyList<T> GetAll();                                                                                                                   
                                                                                                                                                   
      /// <summary>Check if entity exists and is not deleted.</summary>                                                                            
      bool Exists(Guid id);                                                                                                                        
  }  