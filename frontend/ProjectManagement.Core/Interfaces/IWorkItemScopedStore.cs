namespace ProjectManagement.Core.Interfaces;                                                                                                     
                                                                                                                                                   
/// <summary>                                                                                                                                    
/// Store for work-item-scoped entities.                                                                                                         
/// </summary>                                                                                                                                   
public interface IWorkItemScopedStore<T> : IEntityStore<T>                                                                                       
    where T : IEntity, ISoftDeletable, IWorkItemScoped                                                                                           
{                                                                                                                                                
    /// <summary>Get all entities for a specific work item.</summary>                                                                            
    IReadOnlyList<T> GetByWorkItem(Guid workItemId);                                                                                             
}