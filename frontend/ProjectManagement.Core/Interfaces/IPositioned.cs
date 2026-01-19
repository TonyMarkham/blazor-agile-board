namespace ProjectManagement.Core.Interfaces;                                                                                                     
                                                                                                                                                   
/// <summary>                                                                                                                                    
/// Entity that has a position for ordering (drag-and-drop support).                                                                             
/// </summary>                                                                                                                                   
public interface IPositioned                                                                                                                     
{                                                                                                                                                
    /// <summary>Position for ordering within parent/container.</summary>                                                                        
    int Position { get; }                                                                                                                        
}