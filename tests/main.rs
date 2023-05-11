use std::{
    collections::HashMap,
    error::Error,
    ffi::OsStr,
    fmt::Display,
    fs::{self, File},
    io::{BufRead, BufReader},
    path::Path,
};

use fabula::{prelude::*, story};
use libtest_mimic::{Arguments, Trial};

#[derive(Debug)]
pub enum TestPlanInstruction {
    ExpectCommand(Option<String>),
    ExpectDisabledOption(String),
    ExpectLine(Option<String>),
    ExpectOption(Option<String>),
    SelectOption(usize),
    Stop,
}

#[derive(Debug)]
pub struct TestPlan {
    name: String,
    story: Story,
    instructions: Vec<TestPlanInstruction>,
}

#[derive(Debug)]
pub enum TestPlanParseError {
    UnknownInstruction(String),
    IllegalFormat(String),
    MissingValue,
}

impl Error for TestPlanParseError {}
impl Display for TestPlanParseError {
    fn fmt(&self, f: &mut std::fmt::Formatter<'_>) -> std::fmt::Result {
        write!(f, "test plan parse error: {:?}", self)
    }
}

impl From<TestPlan> for Trial {
    fn from(plan: TestPlan) -> Self {
        Trial::test(plan.name, move || {
            let runner = StoryRunner::default();
            let events = plan.instructions.into_iter();
            let mut vars = HashMap::new();
            let mut checkpoint = plan
                .story
                .checkpoint_at("Start")
                .expect("unable to find start node");

            let mut option_targets = vec![];
            for expected_event in events {
                let event: StoryEvent;
                (checkpoint, event) = runner.step(&plan.story, checkpoint, &mut vars)?;

                match expected_event {
                    TestPlanInstruction::ExpectOption(_) => {
                        match event {
                            StoryEvent::AddOption {
                                target, enabled, ..
                            } => {
                                eprintln!("{target} {enabled}");
                                if enabled {
                                    option_targets.push(target);
                                }
                            }
                            _ => panic!("expected new option, found {event:?}"),
                        };
                    }
                    TestPlanInstruction::ExpectCommand(command) => {
                        assert_eq!(
                            StoryEvent::Command(command.expect("no command string given")),
                            event
                        );
                    }
                    TestPlanInstruction::SelectOption(option) => {
                        assert_eq!(StoryEvent::ShowOptions, event);
                        checkpoint.select_option(option_targets.remove(option - 1));
                        option_targets.clear();
                    }
                    _ => {}
                }
            }
            Ok(())
        })
    }
}

impl TestPlan {
    pub fn load(path: &Path) -> Result<Self, Box<dyn Error>> {
        let file = File::open(path)?;
        let reader = BufReader::new(file);
        let instructions: Vec<TestPlanInstruction> = reader
            .lines()
            .filter_map(|line| {
                let text = line.ok()?;
                if text.is_empty() || text.starts_with('#') {
                    None
                } else {
                    Some(text)
                }
            })
            .map(|text| {
                let (ty, value_text) = text
                    .split_once(':')
                    .ok_or(TestPlanParseError::IllegalFormat(text.clone()))?;

                let value = if value_text.is_empty() {
                    None
                } else {
                    Some(value_text.trim_start().to_string())
                };

                Ok(match ty {
                    "line" => TestPlanInstruction::ExpectLine(value),
                    "option" => TestPlanInstruction::ExpectOption(value),
                    "select" => TestPlanInstruction::SelectOption(
                        value
                            .expect("select instruction must have an option provided")
                            .parse::<usize>()
                            .map_err(|_| TestPlanParseError::MissingValue)?,
                    ),
                    "command" => TestPlanInstruction::ExpectCommand(value),
                    "stop" => TestPlanInstruction::Stop,
                    _ => return Err(TestPlanParseError::UnknownInstruction(ty.to_string())),
                })
            })
            .collect::<Result<Vec<TestPlanInstruction>, _>>()?;

        let name = path
            .file_stem()
            .expect("file must have a stem component")
            .to_string_lossy()
            .to_string();
        let story_path = path.with_extension("yarnc");
        let story = story::Builder::default().add_file(story_path).build()?;

        Ok(TestPlan {
            name,
            story,
            instructions,
        })
    }
}

fn collect_tests_from(path: &Path, output: &mut Vec<Trial>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(path)? {
        let info = entry?;
        let ty = info.file_type()?;
        let path = info.path();

        if ty.is_file() && path.extension() == Some(OsStr::new("testplan")) {
            let program_path = path.with_extension("yarnc");
            if !program_path.exists() {
                eprintln!("Skipping {}, no compiled yarn file", path.display());
                continue;
            }

            let test_plan = TestPlan::load(&path)?;
            output.push(test_plan.into());
        } else if ty.is_dir() {
            collect_tests_from(Path::new(&path), output)?;
        }
    }

    Ok(())
}

pub fn main() -> Result<(), Box<dyn Error>> {
    let args = Arguments::from_args();
    let mut tests = vec![];
    let test_plan_root = concat!(
        env!("CARGO_MANIFEST_DIR"),
        "/third-party/yarn-spinner/Tests/TestCases"
    );

    collect_tests_from(Path::new(test_plan_root), &mut tests)?;

    libtest_mimic::run(&args, tests).exit();
}
