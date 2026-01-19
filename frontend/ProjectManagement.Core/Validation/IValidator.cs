namespace ProjectManagement.Core.Validation;                                                                                                     
                                                                                                                                                   
public interface IValidator<T>                                                                                                                   
{                                                                                                                                                
    ValidationResult Validate(T instance);                                                                                                       
}