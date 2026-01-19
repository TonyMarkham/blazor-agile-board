  namespace ProjectManagement.Core.Validation;                                                                                                     
                                                                                                                                                   
  using ProjectManagement.Core.Models;                                                                                                             
  using ProjectManagement.Core.Exceptions;                                                                                                         
                                                                                                                                                   
  public sealed class UpdateWorkItemRequestValidator : IValidator<UpdateWorkItemRequest>                                                           
  {                                                                                                                                                
      private const int MaxTitleLength = 200;                                                                                                      
      private const int MaxDescriptionLength = 10000;                                                                                              
                                                                                                                                                   
      public ValidationResult Validate(UpdateWorkItemRequest request)                                                                              
      {                                                                                                                                            
          var errors = new List<ValidationError>();                                                                                                
                                                                                                                                                   
          if (request.WorkItemId == Guid.Empty)                                                                                                    
              errors.Add(new("workItemId", "Work Item ID is required"));                                                                           
                                                                                                                                                   
          if (request.Title != null)                                                                                                               
          {                                                                                                                                        
              if (string.IsNullOrWhiteSpace(request.Title))                                                                                        
                  errors.Add(new("title", "Title cannot be empty"));                                                                               
              else if (request.Title.Length > MaxTitleLength)                                                                                      
                  errors.Add(new("title", $"Title must be {MaxTitleLength} characters or less"));                                                  
          }                                                                                                                                        
                                                                                                                                                   
          if (request.Description?.Length > MaxDescriptionLength)                                                                                  
              errors.Add(new("description", $"Description must be {MaxDescriptionLength} characters or less"));                                    
                                                                                                                                                   
          return errors.Count == 0 ? ValidationResult.Success() : ValidationResult.Failure(errors);                                                
      }                                                                                                                                            
  }  