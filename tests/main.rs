use std::{error::Error, ffi::OsStr, fs, path::Path};

use fabula::prelude::*;
use libtest_mimic::{Arguments, Trial};

pub enum TestPlanInstruction {
    ExpectCommand(Option<String>),
    ExpectDisabledOption(String),
    ExpectLine(Option<String>),
    ExpectOption(Option<String>),
    SelectOption(usize),
    Stop,
}

#[allow(dead_code)] // TODO: parse test plans
pub struct TestPlan {
    name: String,
    story: Story,
    instructions: Vec<TestPlanInstruction>,
}

impl From<TestPlan> for Trial {
    fn from(plan: TestPlan) -> Self {
        Trial::test(plan.name, move || {
            let _runner = StoryRunner::default();

            Ok(())
        })
    }
}

fn collect_tests_from(path: &Path, output: &mut Vec<Trial>) -> Result<(), Box<dyn Error>> {
    for entry in fs::read_dir(path)? {
        let info = entry?;
        let ty = info.file_type()?;
        let path = info.path();

        if ty.is_file() && path.extension() == Some(OsStr::new("testplan")) {
            let program_path = path.with_extension("yarn");
            if !program_path.exists() {
                continue;
            }

            let bare_path = path.with_extension("");
            let name = bare_path
                .file_name()
                .expect("files are ensured to have a name");
            let test = Trial::test(name.to_string_lossy(), move || Ok(()));

            output.push(test);
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
