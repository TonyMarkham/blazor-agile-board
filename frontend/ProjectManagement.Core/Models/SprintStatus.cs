namespace ProjectManagement.Core.Models;

public enum SprintStatus                                                                                                                         
{                                                                                                                                                
  Planned = 1,                                                                                                                                 
  Active = 2,                                                                                                                                  
  Completed = 3,                                                                                                                               
  Cancelled = 4                                                                                                                                
}

public static class SprintStatusExtensions
{
    public static string ToDisplayString(this SprintStatus status) => status switch
    {
        SprintStatus.Planned => "Planned",
        SprintStatus.Active => "Active",
        SprintStatus.Completed => "Completed",
        SprintStatus.Cancelled => "Cancelled",
        _ => throw new ArgumentOutOfRangeException(nameof(status))
    };
}