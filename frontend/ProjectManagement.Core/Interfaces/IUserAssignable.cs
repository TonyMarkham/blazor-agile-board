namespace ProjectManagement.Core.Interfaces;                                                                                                     
                                                                                                                                                   
/// <summary>                                                                                                                                    
/// Entity that can be assigned to a user.                                                                                                       
/// </summary>                                                                                                                                   
public interface IUserAssignable                                                                                                                 
{                                                                                                                                                
    /// <summary>The user this entity is assigned to.</summary>                                                                                  
    Guid? AssigneeId { get; }                                                                                                                    
}