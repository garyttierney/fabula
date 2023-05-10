use super::CallContext;
use crate::model::Value;

pub fn visit_count_var_name(node_name: &str) -> String {
    format!("$Yarn.Internal.Visiting.{node_name}")
}

/// Get the number of times the node named by [name] has been visited.
pub fn visited_count(context: CallContext, name: String) -> f32 {
    let var_name = visit_count_var_name(&name);

    if let Some(Value::FloatValue(visit_count)) = context.variables.get(&var_name) {
        *visit_count
    } else {
        0.0
    }
}

/// Check if the node identified by [name] has been visited before.
pub fn visited(context: CallContext, name: String) -> bool {
    let visit_count = visited_count(context, name);
    visit_count > 0.0
}
