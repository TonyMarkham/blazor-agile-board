  namespace ProjectManagement.Core.Models;                                                                                                         
                                                                                                                                                   
  using ProjectManagement.Core.Interfaces;                                                                                                         
                                                                                                                                                   
  /// <summary>                                                                                                                                    
  /// Polymorphic work item representing Project, Epic, Story, or Task.                                                                            
  /// Immutable record for thread safety.                                                                                                          
  /// </summary>                                                                                                                                   
  public sealed record WorkItem :                                                                                                                  
      IAuditable,                                                                                                                                  
      IProjectScoped,                                                                                                                              
      IVersioned,                                                                                                                                  
      IPositioned,                                                                                                                                 
      IHierarchical<WorkItem>,                                                                                                                     
      ISprintAssignable,                                                                                                                           
      IUserAssignable,                                                                                                                             
      IStatusTracked                                                                                                                               
  {                                                                                                                                                
      public Guid Id { get; init; }                                                                                                                
      public WorkItemType ItemType { get; init; }                                                                                                  
      public Guid? ParentId { get; init; }                                                                                                         
      public Guid ProjectId { get; init; }                                                                                                         
      public int Position { get; init; }                                                                                                           
      public string Title { get; init; } = string.Empty;                                                                                           
      public string? Description { get; init; }                                                                                                    
      public string Status { get; init; } = "backlog";                                                                                             
      public string Priority { get; init; } = "medium";                                                                                            
      public Guid? AssigneeId { get; init; }                                                                                                       
      public int? StoryPoints { get; init; }                                                                                                       
      public Guid? SprintId { get; init; }                                                                                                         
      public int Version { get; init; }                                                                                                            
      public DateTime CreatedAt { get; init; }                                                                                                     
      public DateTime UpdatedAt { get; init; }                                                                                                     
      public Guid CreatedBy { get; init; }                                                                                                         
      public Guid UpdatedBy { get; init; }                                                                                                         
      public DateTime? DeletedAt { get; init; }                                                                                                    
  } 