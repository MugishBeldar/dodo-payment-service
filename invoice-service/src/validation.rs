use validator::{Validate, ValidationError, ValidationErrors, ValidationErrorsKind};

use crate::errors::AppError;

pub fn validate_request<T: Validate>(value: &T) -> Result<(), AppError> {
    value
        .validate()
        .map_err(|errors| AppError::BadRequest(format_validation_errors(&errors)))
}

pub fn validate_non_blank(value: &str) -> Result<(), ValidationError> {
    if value.trim().is_empty() {
        return Err(ValidationError::new("required"));
    }

    Ok(())
}

pub fn validate_http_url(value: &str) -> Result<(), ValidationError> {
    let trimmed = value.trim();
    if !(trimmed.starts_with("http://") || trimmed.starts_with("https://")) {
        return Err(ValidationError::new("http_url"));
    }

    Ok(())
}

pub fn validate_invoice_state(value: &str) -> Result<(), ValidationError> {
    match value.trim() {
        "draft" | "open" | "paid" | "void" | "uncollectible" => Ok(()),
        _ => Err(ValidationError::new("invoice_state")),
    }
}

pub fn validate_uuid_string(value: &str) -> Result<(), ValidationError> {
    uuid::Uuid::parse_str(value.trim())
        .map(|_| ())
        .map_err(|_| ValidationError::new("uuid"))
}

fn format_validation_errors(errors: &ValidationErrors) -> String {
    let mut messages = Vec::new();
    collect_validation_errors(errors, String::new(), &mut messages);
    messages.join(", ")
}

fn collect_validation_errors(errors: &ValidationErrors, path: String, messages: &mut Vec<String>) {
    for (field, kind) in errors.errors() {
        let field_path = if path.is_empty() {
            field.to_string()
        } else {
            format!("{path}.{field}")
        };

        match kind {
            ValidationErrorsKind::Field(field_errors) => {
                for error in field_errors {
                    messages.push(format_field_error(&field_path, error));
                }
            }
            ValidationErrorsKind::Struct(nested) => {
                collect_validation_errors(nested, field_path, messages);
            }
            ValidationErrorsKind::List(items) => {
                for (index, nested) in items {
                    collect_validation_errors(nested, format!("{}[{}]", field_path, index), messages);
                }
            }
        }
    }
}

fn format_field_error(field_path: &str, error: &ValidationError) -> String {
    match error.code.as_ref() {
        "required" => format!("{field_path} is required"),
        "email" => format!("{field_path} must be a valid email address"),
        "http_url" => format!("{field_path} must start with http:// or https://"),
        "invoice_state" => format!(
            "{field_path} must be one of: draft, open, paid, void, uncollectible"
        ),
        "uuid" => format!("{field_path} must be a valid UUID"),
        "length" => format!("{field_path} has an invalid length"),
        "range" => format!("{field_path} is out of range"),
        _ => format!("{field_path} is invalid"),
    }
}