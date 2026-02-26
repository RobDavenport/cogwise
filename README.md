# cogwise

`cogwise` is a `no_std` Rust library for tick-based game AI decision making.
It combines behavior trees for control flow and utility AI for scoring.

## Highlights

- `no_std` core library (`alloc` only)
- deterministic tick semantics
- data-only trees (`Clone + Debug + PartialEq`)
- integer blackboard keys (`u32`)
- utility AI with pluggable response curves
- optional observer hooks for debugging

## Quick Start

```rust
use cogwise::{BehaviorNode, BehaviorTree, NoOpObserver, Status};
use cogwise::{ActionHandler, ConditionHandler, Context};

#[derive(Clone, Debug, PartialEq)]
enum Action {
    Attack,
}

#[derive(Clone, Debug, PartialEq)]
enum Condition {
    Always,
}

struct Controller;

impl ActionHandler<Action> for Controller {
    fn execute(&mut self, action: &Action, _ctx: &mut Context) -> Status {
        match action {
            Action::Attack => Status::Success,
        }
    }
}

impl ConditionHandler<Condition> for Controller {
    fn check(&self, condition: &Condition, _ctx: &Context) -> bool {
        match condition {
            Condition::Always => true,
        }
    }
}

let tree = BehaviorNode::Sequence(vec![
    BehaviorNode::Condition(Condition::Always),
    BehaviorNode::Action(Action::Attack),
]);

let mut bt = BehaviorTree::new(tree);
let mut controller = Controller;
let mut observer = NoOpObserver;
let result = bt.tick(&mut controller, &controller, &mut observer);
assert_eq!(result, Status::Success);
```

## Verification

```bash
cargo test --target x86_64-pc-windows-msvc
cargo build --target wasm32-unknown-unknown --release
```

## Demo (GitHub Pages)

A WASM demo is provided in `demo-wasm/` and can be deployed via GitHub Pages
using `.github/workflows/pages.yml`.

Live demo: https://robdavenport.github.io/cogwise/

Local demo build:

```bash
cargo build --manifest-path demo-wasm/Cargo.toml --target wasm32-unknown-unknown --release
cd demo-wasm
wasm-pack build --target web --release
```
