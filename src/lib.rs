#![warn(clippy::all, clippy::missing_errors_doc, clippy::missing_safety_doc)]
#![deny(clippy::panic)]

pub mod function;
pub mod model;
pub mod runner;
pub mod state;
pub mod story;
pub mod variables;

pub mod prelude {
    pub use super::{runner::*, story::*};
}

#[cfg(test)]
mod tests {
    use std::collections::HashMap;

    use super::prelude::*;
    use crate::function::Library;

    type TestResult = Result<(), Box<dyn std::error::Error>>;

    macro_rules! test_case {
        ($fname:expr) => {
            concat!(env!("CARGO_MANIFEST_DIR"), "/test-data/", $fname)
        };
    }

    #[test]
    pub fn loads_sally_sample() -> TestResult {
        let story = Builder::default()
            .add_file(test_case!("sample-stories/sally.yarnc"))
            .build()?;

        let runner = StoryRunner::new(Library::default());

        let mut vars = HashMap::new();
        let mut previous = story
            .checkpoint_at("Sally")
            .expect("unable to find start node");

        loop {
            let event: StoryEvent;
            (previous, event) = runner.step(&story, previous, &mut vars)?;

            match event {
                StoryEvent::Complete => break,
                _ => eprintln!("{event:#?}"),
            }
        }

        Ok(())
    }
}
