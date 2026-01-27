use pm_proto::FieldChange;

/// Generic builder for tracking field changes
pub struct FieldChangeBuilder {
    changes: Vec<FieldChange>,
}

impl Default for FieldChangeBuilder {
    fn default() -> Self {
        Self::new()
    }
}

impl FieldChangeBuilder {
    pub fn new() -> Self {
        Self {
            changes: Vec::new(),
        }
    }

    /// Track a required field change
    pub fn track<T: ToString + ?Sized>(&mut self, field_name: &str, old_value: &T, new_value: &T) {
        let old_str = old_value.to_string();
        let new_str = new_value.to_string();
        if old_str != new_str {
            self.changes.push(FieldChange {
                field_name: field_name.to_string(),
                old_value: Some(old_str),
                new_value: Some(new_str),
            });
        }
    }

    /// Track an optional field change
    pub fn track_option<T: ToString>(
        &mut self,
        field_name: &str,
        old_value: &Option<T>,
        new_value: &Option<T>,
    ) {
        let old_str = old_value.as_ref().map(|v| v.to_string());
        let new_str = new_value.as_ref().map(|v| v.to_string());
        if old_str != new_str {
            self.changes.push(FieldChange {
                field_name: field_name.to_string(),
                old_value: old_str,
                new_value: new_str,
            });
        }
    }

    /// Build the final list of changes
    pub fn build(self) -> Vec<FieldChange> {
        self.changes
    }
}
