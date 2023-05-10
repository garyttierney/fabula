use thiserror::Error;

#[allow(clippy::all, clippy::pedantic, clippy::nursery, clippy::cargo)]
pub mod proto {
    include!(concat!(env!("OUT_DIR"), "/yarn.rs"));
}

pub use proto::{instruction::*, operand::*, *};

#[derive(Error, Debug)]
pub enum ValueError {
    #[error("unexpected type")]
    UnexpectedType,

    #[error("no value given")]
    Missing,
}

macro_rules! value_conversion {
    ($name: path, $ty: ty) => {
        impl TryFrom<Value> for $ty {
            type Error = ValueError;

            fn try_from(value: Value) -> Result<$ty, Self::Error> {
                match value {
                    $name(value) => Ok(value),
                    _ => Err(ValueError::UnexpectedType),
                }
            }
        }

        impl From<$ty> for Value {
            fn from(value: $ty) -> Self {
                $name(value)
            }
        }
    };
}

value_conversion!(Value::StringValue, String);
value_conversion!(Value::BoolValue, bool);
value_conversion!(Value::FloatValue, f32);

pub trait Operands {
    fn at<T>(&self, index: usize) -> Result<T, ValueError>
    where
        T: TryFrom<Value, Error = ValueError>;
}

impl Operands for Vec<Operand> {
    fn at<T>(&self, index: usize) -> Result<T, ValueError>
    where
        T: TryFrom<Value, Error = ValueError>,
    {
        self.get(index)
            .and_then(|operand| operand.value.clone()) // TODO: avoid clone
            .ok_or(ValueError::Missing)
            .and_then(std::convert::TryInto::try_into)
    }
}

#[derive(Error, Debug)]
pub enum NodeError {
    #[error("the label named by '{0}' could not be resolved")]
    InvalidLabel(String),
}

impl Node {
    pub fn resolve_label(&self, name: &str) -> Result<usize, NodeError> {
        self.labels
            .get(name)
            .map(|pc| *pc as usize)
            .ok_or_else(|| NodeError::InvalidLabel(name.to_string()))
    }
}
