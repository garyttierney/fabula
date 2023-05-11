use std::{collections::HashMap, marker::PhantomData};
use std::any::type_name;

use thiserror::Error;

use crate::{
    model::{Node, Value, ValueError},
    story::Story,
    variables::VariableStore,
};

mod builtins;

pub struct Library {
    functions: HashMap<String, Box<dyn UntypedFunction>>,
}

impl Library {
    #[must_use]
    pub fn new() -> Self {
        Self {
            functions: HashMap::default(),
        }
    }

    pub fn builtins() -> Self {
        let mut library = Self::new();
        library.register("visited", builtins::visited);
        library.register("visited_count", builtins::visited_count);
        library.register("floor", |_ctx: CallContext, a: f32| a.floor());
        library.register("ceil", |_ctx: CallContext, a: f32| a.ceil());
        library.register("Bool.EqualTo", |_ctx: CallContext, a: bool, b: bool| a == b);
        library.register("Bool.Not", |_ctx: CallContext, a: bool| !a);
        library.register("Bool.And", |_ctx: CallContext, a: bool, b: bool| a && b);
        library.register("Number.Add", |_ctx: CallContext, a: f32, b: f32| a + b);
        library
    }

    /// Call a function with the given [`name`] and [`args`].
    ///
    /// # Errors
    /// - [`CallError::UnknownFunction`] if there is no function with the given name.
    /// - [`CallError::InvalidArguments`] if the arguments passed do not match the call signature.
    pub fn call(
        &self,
        name: String,
        context: CallContext,
        args: Vec<Value>,
    ) -> Result<Value, CallError> {
        self.functions
            .get(&name)
            .map_or(Err(CallError::UnknownFunction(name)), |function| {
                function.call(context, args)
            })
    }

    pub fn register<Marker, F, S: Into<String>>(&mut self, name: S, function: F)
    where
        F: Function<Marker> + 'static,
        Marker: 'static,
    {
        let handle = FunctionHandle {
            function,
            marker: PhantomData::default(),
        };

        self.functions.insert(name.into(), Box::new(handle));
    }
}

impl Default for Library {
    fn default() -> Self {
        Self::builtins()
    }
}

#[derive(Error, Debug)]
pub enum CallError {
    #[error("no function found named '{0}'")]
    UnknownFunction(String),

    #[error("invalid argument type, expected {0}, found {1:?}")]
    InvalidArguments(&'static str, Value),

    #[error("invalid argument count, expected {0}, found {1}")]
    InvalidArgumentCount(usize, usize)
}

/// A function that can be registered with and called by scripts running in the Yarn runtime.
pub trait Function<Ty> {
    type Return: Into<Value>;

    fn call(&self, context: CallContext, args: Vec<Value>) -> Result<Value, CallError>;
}

pub trait UntypedFunction {
    fn call(&self, context: CallContext, args: Vec<Value>) -> Result<Value, CallError>;
}

pub struct FunctionHandle<S, F>
where
    F: Function<S>,
{
    function: F,
    marker: PhantomData<S>,
}

impl<S, F> UntypedFunction for FunctionHandle<S, F>
where
    F: Function<S>,
{
    fn call(&self, context: CallContext, args: Vec<Value>) -> Result<Value, CallError> {
        self.function.call(context, args)
    }
}

pub struct CallContext<'r> {
    pub node: &'r Node,
    pub story: &'r Story,
    pub variables: &'r mut dyn VariableStore,
}

macro_rules! param_count {
    () => {0};
    ($head: tt $($tail:tt)*) => { 1 + param_count!($($tail)*) }; 
}

// https://github.com/yarn-slinger/yarn-slinger/blob/6b74f8d3b9d5caace05240ba1bf737dff2035b1f/crates/core/src/yarn_fn/function_wrapping.rs#L21
macro_rules! impl_function {
    ($($param: ident),*) => {
        #[allow(non_snake_case)]
        impl<F, R, $($param,)*> Function<fn(CallContext, $($param,)*) -> R> for F
        where
            F: Fn(CallContext, $($param,)*) -> R,
            $($param: TryFrom<Value, Error = ValueError>,)*
            R: Into<Value>,
        {
            type Return = R;

            #[allow(non_snake_case)]
            fn call(&self, context: CallContext, args: Vec<Value>) -> Result<Value, CallError> {
                    let [$($param,)*] = &args[..] else {
                       return Err(CallError::InvalidArgumentCount(param_count!($($param)*), args.len()));
                    };

                    let input = (
                        $($param
                            .clone()
                            .try_into()
                            .or_else(|_| Err(CallError::InvalidArguments(type_name::<$param>(), $param.clone())))?,
                        )*
                    );
                    let ($($param,)*) = input;
                    Ok(self(context, $($param,)*).into())
            }
        }
    };
}

impl_function!(P1);
impl_function!(P1, P2);
