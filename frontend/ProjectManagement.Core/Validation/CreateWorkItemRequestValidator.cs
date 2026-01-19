  namespace ProjectManagement.Core.Validation;                                                                                                     
                                                                                                                                                   
  using ProjectManagement.Core.Models;                                                                                                             
  using ProjectManagement.Core.Exceptions;                                                                                                         
                                                                                                                                                   
  public sealed class CreateWorkItemRequestValidator : IValidator<CreateWorkItemRequest>                                                           
  {                                                                                                                                                
      private const int MaxTitleLength = 200;                                                                                                      
      private const int MaxDescriptionLength = 10000;                                                                                              
                                                                                                                                                   
      public ValidationResult Validate(CreateWorkItemRequest request)                                                                              
      {                                                                                                                                            
          var errors = new List<ValidationError>();                                                                                                
                                                                                                                                                   
          if (string.IsNullOrWhiteSpace(request.Title))                                                                                            
              errors.Add(new("title", "Title is required"));                                                                                       
          else if (request.Title.Length > MaxTitleLength)                                                                                          
              errors.Add(new("title", $"Title must be {MaxTitleLength} characters or less"));                                                      
                                                                                                                                                   
          if (request.ProjectId == Guid.Empty)                                                                                                     
              errors.Add(new("projectId", "Project ID is required"));                                                                              
                                                                                                                                                   
          if (request.Description?.Length > MaxDescriptionLength)                                                                                  
              errors.Add(new("description", $"Description must be {MaxDescriptionLength} characters or less"));                                    
                                                                                                                                                   
          if (request.ItemType == WorkItemType.Project && request.ParentId.HasValue)                                                               
              errors.Add(new("parentId", "Projects cannot have a parent"));                                                                        
                                                                                                                                                   
          return errors.Count == 0 ? ValidationResult.Success() : ValidationResult.Failure(errors);                                                
      }                                                                                                                                            
  }  