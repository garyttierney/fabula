use thiserror::Error;

use crate::function::{CallContext, CallError, Library};
use crate::model::{Instruction, Node, NodeError, OpCode, Operand, Operands, Value, ValueError};
use crate::story::Story;
use crate::variables::VariableStore;

/// An event generated by stepping through multiple [Story] instructions that can
/// inform the user on how the narrative is unfolding.
#[derive(Debug, PartialEq, Eq)]
pub enum StoryEvent {
    Started,
    AddOption {
        enabled: bool,
        key: String,
        substitutions: Vec<String>,
        target: String,
    },
    ShowOptions,
    ShowLine {
        key: String,
        substitutions: Vec<String>,
    },
    Command(String),
    Complete,
}

/// A [`StoryCheckpoint`] represents an addressable point in the [Story]. It can be saved
/// and later resumed to reload
#[derive(Clone)]
pub struct StoryCheckpoint<'r> {
    /// The node the story checkpoint was created at, if any. If none, the [StoryRunner] will
    /// follow the starting node.
    node: &'r Node,

    /// Offset of the next instruction to execute within the node.
    node_instruction_offset: usize,

    /// The evaluation stack at the time of this checkpoint.
    stack: EvaluationStack,
}

impl<'r> StoryCheckpoint<'r> {
    const fn at(node: &'r Node, pc: usize, stack: EvaluationStack) -> Self {
        Self {
            node,
            node_instruction_offset: pc,
            stack,
        }
    }

    #[must_use]
    pub const fn new(node: &'r Node) -> StoryCheckpoint {
        Self {
            node,
            node_instruction_offset: 0,
            stack: EvaluationStack(vec![]),
        }
    }

    pub fn select_option(&mut self, name: String) {
        self.stack.push(name);
    }
}

/// The value stack.
#[derive(Clone)]
pub struct EvaluationStack(Vec<Value>);

impl EvaluationStack {
    /// # Errors
    ///
    /// Will return `Err` if there is no value at the top of the stack.
    pub fn pop_any(&mut self) -> Result<Value, ValueError> {
        self.0.pop().ok_or(ValueError::Missing)
    }

    /// # Errors
    ///
    /// See [`pop_any`]
    pub fn peek_any(&mut self) -> Result<Value, ValueError> {
        self.0.get(0).cloned().ok_or(ValueError::Missing)
    }

    /// # Errors
    ///
    /// See [`pop`]
    pub fn peek<T>(&mut self) -> Result<T, ValueError>
    where
        T: TryFrom<Value, Error = ValueError>,
    {
        self.0
            .get(0)
            .ok_or(ValueError::Missing)
            .and_then(|v| T::try_from(v.clone()))
    }

    /// # Errors
    ///
    /// Will return `Err` if there is no value at the top of the stack, or the value is of an
    /// incompatible type.
    pub fn pop<T>(&mut self) -> Result<T, ValueError>
    where
        T: TryFrom<Value, Error = ValueError>,
    {
        self.0
            .pop()
            .ok_or(ValueError::Missing)
            .and_then(|v| T::try_from(v))
    }

    pub fn push<T>(&mut self, value: T)
    where
        T: Into<Value>,
    {
        let v = value.into();
        self.0.push(v);
    }
}

#[derive(Error, Debug)]
#[error("encountered story error: {} at #{} in node '{}' - {:?}({:?})", .source, .pc, .node, .instruction.opcode(), .instruction.operands)]
pub struct StoryRunnerError {
    source: InstructionError,
    node: String,
    pc: usize,
    instruction: Instruction,
}

/// An error that occurred during evaluation of a [Story].
#[derive(Error, Debug)]
pub enum InstructionError {
    #[error(transparent)]
    BadNode(#[from] NodeError),

    /// An error has occurred while interpreting or decoding an instruction.
    #[error("error while decoding instructions, illegal opcode: {0}")]
    InvalidInstruction(i32),

    #[error("instruction is no longer supported")]
    UnsupportedInstruction(OpCode),

    #[error(transparent)]
    FunctionCall(#[from] CallError),

    #[error(transparent)]
    Evaluation(#[from] ValueError),
}

enum ControlFlow<'a> {
    Next,
    Jump(&'a Node, usize),
}

/// Driver for running and evaluating a [Story].
#[derive(Default)]
pub struct StoryRunner {
    library: Library,
}

impl StoryRunner {
    #[must_use]
    pub const fn new(library: Library) -> Self {
        Self { library }
    }

    fn execute<'s, V>(
        &'s self,
        story: &'s Story,
        node: &'s Node,
        opcode: OpCode,
        operands: &'s Vec<Operand>,
        stack: &mut EvaluationStack,
        variables: &mut V,
    ) -> Result<(ControlFlow, Option<StoryEvent>), InstructionError>
    where
        V: VariableStore,
    {
        return match opcode {
            OpCode::JumpTo => {
                let label_name = operands.at::<String>(0)?;
                let label_offset = node.resolve_label(&label_name)?;

                Ok((ControlFlow::Jump(node, label_offset), None))
            }
            OpCode::Jump => {
                let label_name = stack.pop::<String>()?;
                let label_offset = node.resolve_label(&label_name)?;

                Ok((ControlFlow::Jump(node, label_offset), None))
            }
            OpCode::RunLine => {
                let key = operands.at::<String>(0)?;

                let substitutions = if operands.len() > 1 {
                    let expression_count = operands.at::<f32>(1)? as usize;
                    let mut substitutions = Vec::with_capacity(expression_count);

                    for _ in 0..expression_count {
                        substitutions.push(stack.pop::<String>()?);
                    }

                    substitutions.reverse();
                    substitutions
                } else {
                    vec![]
                };

                Ok((
                    ControlFlow::Next,
                    Some(StoryEvent::ShowLine { key, substitutions }),
                ))
            }
            OpCode::RunCommand => {
                let mut command_text = operands.at::<String>(0)?;

                if operands.len() > 1 {
                    let expression_count = operands.at::<f32>(1)? as usize;

                    for index in (0..expression_count).rev() {
                        let substitution: String = stack.pop()?;
                        let search = format!("{{{}}}", index);

                        command_text = command_text.replace(&search, &substitution);
                    }
                }

                Ok((ControlFlow::Next, Some(StoryEvent::Command(command_text))))
            }
            OpCode::AddOption => {
                let key = operands.at::<String>(0)?;
                let target = operands.at::<String>(1)?;

                let substitutions = if operands.len() > 2 {
                    let expression_count = operands.at::<f32>(2)? as usize;
                    let mut substitutions = Vec::with_capacity(expression_count);

                    for _ in 0..expression_count {
                        substitutions.push(stack.pop::<String>()?);
                    }

                    substitutions.reverse();
                    substitutions
                } else {
                    vec![]
                };

                let enabled = if operands.len() > 3 && operands.at::<bool>(3)? {
                    stack.pop()?
                } else {
                    true
                };

                Ok((
                    ControlFlow::Next,
                    Some(StoryEvent::AddOption {
                        enabled,
                        key,
                        substitutions,
                        target,
                    }),
                ))
            }
            OpCode::ShowOptions => Ok((ControlFlow::Next, Some(StoryEvent::ShowOptions))),
            OpCode::PushString => {
                stack.push(operands.at::<String>(0)?);
                Ok((ControlFlow::Next, None))
            }
            OpCode::PushFloat => {
                stack.push(operands.at::<f32>(0)?);
                Ok((ControlFlow::Next, None))
            }
            OpCode::PushBool => {
                stack.push(operands.at::<bool>(0)?);
                Ok((ControlFlow::Next, None))
            }
            OpCode::PushNull => Err(InstructionError::UnsupportedInstruction(opcode)),
            OpCode::JumpIfFalse => {
                let condition = stack.peek::<bool>()?;
                let flow = if !condition {
                    let target_name = operands.at::<String>(0)?;
                    let target = node.resolve_label(&target_name)?;

                    ControlFlow::Jump(node, target)
                } else {
                    ControlFlow::Next
                };

                Ok((flow, None))
            }
            OpCode::Pop => {
                let _ = stack.pop_any()?;
                Ok((ControlFlow::Next, None))
            }
            OpCode::CallFunc => {
                let name = operands.at::<String>(0)?;
                let parameter_count = stack.pop::<f32>()? as usize;
                let mut parameters = Vec::with_capacity(parameter_count);

                for _ in 0..parameter_count {
                    parameters.push(stack.pop_any()?);
                }

                parameters.reverse();

                let cx = CallContext {
                    node,
                    story,
                    variables,
                };

                let return_value = self.library.call(name, cx, parameters)?;
                stack.push(return_value);

                Ok((ControlFlow::Next, None))
            }
            OpCode::PushVariable => {
                let var_name = operands.at::<String>(0)?;
                let var_value = variables
                    .get(&var_name)
                    .or_else(|| story.initial_value(&var_name));

                if let Some(value) = var_value {
                    stack.push(value.clone());
                    Ok((ControlFlow::Next, None))
                } else {
                    Err(InstructionError::Evaluation(ValueError::Missing))
                }
            }
            OpCode::StoreVariable => {
                let value = stack.peek_any()?;
                let var_name = operands.at::<String>(0)?;

                variables.set(&var_name, value);

                Ok((ControlFlow::Next, None))
            }
            OpCode::Stop => Ok((ControlFlow::Next, Some(StoryEvent::Complete))),
            OpCode::RunNode => {
                let node_name = stack.pop::<String>()?;
                let new_node = story.node(node_name).unwrap();

                Ok((ControlFlow::Jump(new_node, 0), None))
            }
        };
    }

    /// Advance the story forward from the given [checkpoint].
    ///
    /// # Errors
    ///
    /// Will return `Err` if could not be advanced due to an error decoding or evaluating
    /// instructions.
    pub fn step<'a, V: VariableStore>(
        &'a self,
        story: &'a Story,
        checkpoint: StoryCheckpoint<'a>,
        variables: &mut V,
    ) -> Result<(StoryCheckpoint, StoryEvent), StoryRunnerError> {
        let StoryCheckpoint {
            mut node,
            node_instruction_offset: mut pc,
            mut stack,
        } = checkpoint;

        loop {
            let instruction = &node.instructions[pc];
            let operands = &instruction.operands;
            let step = OpCode::from_i32(instruction.opcode)
                .ok_or(InstructionError::InvalidInstruction(instruction.opcode))
                .and_then(|opcode| {
                    self.execute(story, node, opcode, operands, &mut stack, variables)
                });

            let (flow, event) = match step {
                Ok((flow, event)) => (flow, event),
                Err(source) => {
                    return Err(StoryRunnerError {
                        source,
                        node: node.name.clone(),
                        pc,
                        instruction: instruction.clone(),
                    });
                }
            };

            (node, pc) = match flow {
                ControlFlow::Next => (node, pc + 1),
                ControlFlow::Jump(node, target) => (node, target),
            };

            if let Some(event) = event {
                return Ok((StoryCheckpoint::at(node, pc, stack), event));
            }
        }
    }
}
