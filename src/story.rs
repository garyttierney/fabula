use std::collections::HashMap;
use std::fs::read;
use std::path::PathBuf;

use prost::{DecodeError, Message};
use thiserror::Error;

use crate::model::{Node, Program, Value};
use crate::runner::StoryCheckpoint;

#[derive(Debug)]
pub struct Story {
    program: Program,
}

impl Story {
    pub fn initial_value<S>(&self, name: S) -> Option<&Value>
    where
        S: AsRef<str>,
    {
        self.program
            .initial_values
            .get(name.as_ref())
            .map(|operand| operand.value.as_ref().expect("operand must have a value"))
    }

    pub fn node<S>(&self, name: S) -> Option<&Node>
    where
        S: AsRef<str>,
    {
        self.program.nodes.get(name.as_ref())
    }

    pub fn checkpoint_at<S>(&self, name: S) -> Option<StoryCheckpoint>
    where
        S: AsRef<str>,
    {
        self.node(name).map(StoryCheckpoint::new)
    }
}

pub enum Source {
    ProgramFile(PathBuf),
    Program(Program),
}

#[derive(Copy, Clone, Debug)]
pub enum AmbiguityReason {
    InitialValueName,
    NodeName,
}

#[derive(Error, Debug)]
pub enum BuilderError {
    #[error("ambiguity found in loaded program: {1}")]
    Ambiguity(AmbiguityReason, String),

    #[error("i/o error occurred when loading program")]
    Io(#[from] std::io::Error),

    #[error("failed to decode program file")]
    Protocol(#[from] DecodeError),
}

#[derive(Default)]
pub struct Builder {
    sources: Vec<Source>,
}

impl Builder {
    #[must_use]
    pub fn add_file<P: Into<PathBuf>>(mut self, path: P) -> Self {
        self.sources.push(Source::ProgramFile(path.into()));
        self
    }

    #[must_use]
    pub fn add_program(mut self, program: Program) -> Self {
        self.sources.push(Source::Program(program));
        self
    }

    /// Create a [`Story`] from the Yarn [`Program`]s added to this builder.
    ///
    /// # Errors
    ///
    /// Returns `Err` if a program could not be loaded or combining all
    /// available programs would result in conflicts/ambiguities.
    pub fn build(self) -> Result<Story, BuilderError> {
        fn merge<V>(
            dest: &mut HashMap<String, V>,
            source: HashMap<String, V>,
            err_source: AmbiguityReason,
        ) -> Result<(), BuilderError> {
            source.into_iter().try_for_each(|(key, node)| {
                if dest.contains_key(&key) {
                    return Err(BuilderError::Ambiguity(err_source, key));
                }

                dest.insert(key, node);
                Ok(())
            })
        }

        let mut root = Program::default();
        for source in self.sources {
            let Program {
                name: _,
                nodes,
                initial_values,
            } = match source {
                Source::ProgramFile(path) => {
                    read(&path).and_then(|data| Ok(Program::decode(&data[..])?))?
                }
                Source::Program(program) => program,
            };

            merge(&mut root.nodes, nodes, AmbiguityReason::NodeName)?;
            merge(
                &mut root.initial_values,
                initial_values,
                AmbiguityReason::InitialValueName,
            )?;
        }

        Ok(Story { program: root })
    }
}
