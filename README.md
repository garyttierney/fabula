# fabula

Run interactive narratives authored by Yarn Spinner in Rust.

`fabula` is a lightweight implementation of the Yarn Spinner runtime.
It is capable of evaluating compiled `.yarn` files.

## Usage

```rust
fn load_and_run() -> Result<(), StoryRunnerError> {
    let story = StoryBuilder::default()
        .add_file(test_case!("sample-stories/sally.yarnc"))
        .build()?;
    
    let mut vars = HashMap::new();
    let runner = StoryRunner::new(story);
    
    loop {
        let (checkpoint, event) = runner.step(checkpoint, &mut vars);
        
        match event {
            StoryEvent::ShowLines { key, substitutions } => println!("{}", key)
        }
    }
}

```

## License

Licensed under either of

* Apache License, Version 2.0
([LICENSE-APACHE](LICENSE-APACHE) or http://www.apache.org/licenses/LICENSE-2.0)
* MIT license
([LICENSE-MIT](LICENSE-MIT) or http://opensource.org/licenses/MIT)

at your option.

## Contribution

Unless you explicitly state otherwise, any contribution intentionally submitted
for inclusion in the work by you shall be dual licensed as above, without any
additional terms or conditions.
