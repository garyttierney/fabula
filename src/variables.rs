use std::collections::HashMap;
use std::hash::BuildHasher;

use crate::model::operand::Value;

pub trait VariableStore {
    /// Get a reference to the current value of the variable named by [name].
    fn get(&self, name: &str) -> Option<&Value>;

    /// Set the current value of the variable named by [name] and return the old value, if any.
    fn set(&mut self, name: &str, value: Value) -> Option<Value>;
}

impl<S: BuildHasher> VariableStore for HashMap<String, Value, S> {
    fn get(&self, name: &str) -> Option<&Value> {
        self.get(name)
    }

    fn set(&mut self, name: &str, value: Value) -> Option<Value> {
        self.insert(name.to_string(), value)
    }
}
