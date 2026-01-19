namespace ProjectManagement.Core.Models;                                                                                                         
                                                                                                                                                   
public sealed record CreateSprintRequest                                                                                                         
{                                                                                                                                                
    public required Guid ProjectId { get; init; }                                                                                                
    public required string Name { get; init; }                                                                                                   
    public string? Goal { get; init; }                                                                                                           
    public required DateTime StartDate { get; init; }                                                                                            
    public required DateTime EndDate { get; init; }                                                                                              
}