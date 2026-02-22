# cogwise Implementation Plan

## What This Is

A `no_std` Rust library for tick-based game AI decision-making. Combines **behavior trees** (BT) for structured control flow with **utility AI** for data-driven scoring, giving game agents the best of both worlds: predictable high-level sequencing with intelligent, context-aware leaf decisions.

**Key design principles:**
- Trees are data (enums), not closures — serializable, inspectable, `Clone`-able
- User-defined `ActionId` / `ConditionId` enums — no trait objects, no dynamic dispatch in the hot path
- Integer ticks in the BT core — deterministic, rollback-safe
- `Float` trait only in the utility module — scoring needs continuous math
- `UtilitySelector` node bridges BT structure with utility scoring
- Blackboard uses `u32` keys — no string hashing at runtime
- Observer trait for debugging / visualization without runtime cost when unused

**Reference implementations:** See sibling repos `navex/` (steering behaviors — cogwise decides *what* to do, navex decides *how* to move), `tessera/` (spatial grids — cogwise queries spatial awareness through blackboard).

## Hard Rules

- `#![no_std]` with `extern crate alloc` — no std dependency in core library
- `rand_core` and `libm` are the ONLY dependencies
- All tests: `cargo test --target x86_64-pc-windows-msvc`
- WASM check: `cargo build --target wasm32-unknown-unknown --release`
- Deterministic: same tree + same blackboard state + same tick = same result
- No closures stored in tree nodes — trees must be `Clone + Debug + PartialEq`
- No string keys anywhere — blackboard uses `u32`, actions/conditions use user enums
- All physics/scoring math uses the `Float` trait — never raw `f32`/`f64` directly in generic code

---

## Phase 1: Core Enums

### Step 1: Status (status.rs)

The fundamental return type for every BT node tick.

```rust
/// Result of ticking a behavior tree node.
///
/// Every node in the tree returns one of these three values each tick.
/// The parent node uses the child's status to decide what to do next.
///
/// # Example
/// ```
/// use cogwise::Status;
///
/// let result = Status::Running;
/// assert!(!result.is_done());
/// assert!(Status::Success.is_done());
/// ```
#[derive(Copy, Clone, Debug, PartialEq, Eq, Hash)]
pub enum Status {
    /// The node is still executing and needs more ticks.
    Running,
    /// The node completed successfully.
    Success,
    /// The node failed.
    Failure,
}

impl Status {
    /// Returns `true` if the status is `Success` or `Failure` (i.e., not `Running`).
    pub fn is_done(self) -> bool {
        !matches!(self, Status::Running)
    }

    /// Returns `true` if the status is `Success`.
    pub fn is_success(self) -> bool {
        matches!(self, Status::Success)
    }

    /// Returns `true` if the status is `Failure`.
    pub fn is_failure(self) -> bool {
        matches!(self, Status::Failure)
    }

    /// Inverts Success ↔ Failure, leaves Running unchanged.
    pub fn invert(self) -> Self {
        match self {
            Status::Success => Status::Failure,
            Status::Failure => Status::Success,
            Status::Running => Status::Running,
        }
    }
}
```

### Step 2: ParallelPolicy (parallel.rs)

```rust
/// Determines when a Parallel node succeeds or fails.
///
/// A Parallel node ticks ALL children every frame. The policy determines
/// how many children must succeed/fail for the Parallel node itself to
/// report success or failure.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum ParallelPolicy {
    /// Succeeds when ALL children succeed. Fails when ANY child fails.
    RequireAll,
    /// Succeeds when ANY child succeeds. Fails when ALL children fail.
    RequireOne,
    /// Succeeds when N children succeed. Fails when too few can still reach N.
    RequireN(usize),
}
```

### Step 3: Decorator (decorator.rs)

```rust
/// Modifies the behavior of a single child node.
///
/// Decorators wrap a child node and transform its result or control
/// when/how often it is ticked. Each variant applies a specific
/// transformation.
///
/// # Example
/// ```
/// use cogwise::Decorator;
///
/// // Retry a child up to 3 times on failure
/// let retry = Decorator::Retry(3);
///
/// // Invert success/failure
/// let invert = Decorator::Inverter;
/// ```
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum Decorator {
    /// Inverts child result: Success → Failure, Failure → Success.
    /// Running passes through unchanged.
    Inverter,

    /// Repeats child N times. Succeeds after N completions.
    /// Fails immediately if child fails.
    Repeat(u32),

    /// Retries child on failure, up to N times.
    /// Succeeds immediately if child succeeds.
    Retry(u32),

    /// After child completes (success or failure), forces the node to
    /// return Failure for the next N ticks (cooldown period).
    /// Useful for rate-limiting actions like attacks.
    Cooldown(u32),

    /// Only ticks child if a blackboard key (u32) is set and truthy.
    /// Returns Failure if the guard condition is not met.
    Guard(u32),

    /// Keeps ticking child until it returns Success.
    /// Resets child on Failure and tries again. Never returns Failure.
    UntilSuccess,

    /// Keeps ticking child until it returns Failure.
    /// Resets child on Success and tries again. Never returns Success.
    UntilFail,

    /// Fails if child doesn't complete within N ticks.
    /// Passes through child's result if it completes in time.
    Timeout(u32),

    /// Always returns Success regardless of child's result.
    ForceSuccess,

    /// Always returns Failure regardless of child's result.
    ForceFailure,
}
```

### Step 4: BehaviorNode (node.rs)

```rust
/// A node in the behavior tree. Trees are composed by nesting these recursively.
///
/// Generic over:
/// - `A`: User-defined action ID enum (e.g., `enum EnemyAction { Attack, Patrol, Flee }`)
/// - `C`: User-defined condition ID enum (e.g., `enum EnemyCondition { IsPlayerVisible, IsLowHealth }`)
///
/// # Design
/// Nodes are pure data — no closures, no trait objects. This makes trees
/// `Clone`, `Debug`, serializable, and inspectable. The actual behavior
/// for actions/conditions is provided via handler traits at tick time.
///
/// # Example
/// ```
/// use cogwise::BehaviorNode;
///
/// // A sequence that checks if player is visible, then attacks
/// let tree: BehaviorNode<MyAction, MyCondition> = BehaviorNode::Sequence(vec![
///     BehaviorNode::Condition(MyCondition::IsPlayerVisible),
///     BehaviorNode::Action(MyAction::Attack),
/// ]);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum BehaviorNode<A, C> {
    /// Ticks children left-to-right. Returns Failure on first child failure.
    /// Returns Success only when ALL children succeed. Resumes from the
    /// last Running child on subsequent ticks.
    Sequence(Vec<BehaviorNode<A, C>>),

    /// Ticks children left-to-right. Returns Success on first child success.
    /// Returns Failure only when ALL children fail. Resumes from the
    /// last Running child on subsequent ticks.
    Selector(Vec<BehaviorNode<A, C>>),

    /// Ticks ALL children every frame. Uses ParallelPolicy to determine
    /// overall success/failure.
    Parallel {
        policy: ParallelPolicy,
        children: Vec<BehaviorNode<A, C>>,
    },

    /// Wraps a single child with a Decorator that modifies its behavior.
    Decorator {
        decorator: Decorator,
        child: Box<BehaviorNode<A, C>>,
    },

    /// Leaf node: executes a user-defined action via ActionHandler.
    Action(A),

    /// Leaf node: checks a user-defined condition via ConditionHandler.
    /// Returns Success if true, Failure if false. Never returns Running.
    Condition(C),

    /// Leaf node: waits for N ticks, then returns Success.
    Wait(u32),

    /// Evaluates all children using utility scoring, ticks only the best.
    /// Remembers the selected child while it returns Running.
    /// Each child must have an associated utility configuration.
    UtilitySelector {
        children: Vec<BehaviorNode<A, C>>,
        /// Utility config index per child (indexes into tree-level utility configs).
        utility_ids: Vec<u32>,
    },

    /// Selects a random child each time it starts (not re-randomized while Running).
    /// Requires RNG passed through context.
    RandomSelector(Vec<BehaviorNode<A, C>>),

    /// Selects a child based on weights. Higher weight = more likely.
    /// Requires RNG passed through context.
    WeightedSelector {
        children: Vec<BehaviorNode<A, C>>,
        weights: Vec<u32>,
    },
}
```

### Step 5: Handler Traits (leaf.rs)

```rust
/// Trait for executing user-defined actions.
///
/// Implement this trait on your game state or AI controller. The `A` type
/// parameter is your action enum — the handler matches on it to decide
/// what to do.
///
/// # Example
/// ```
/// use cogwise::{ActionHandler, Context, Status};
///
/// #[derive(Clone, Debug, PartialEq)]
/// enum EnemyAction { Attack, Patrol, Flee }
///
/// struct EnemyController { /* game state */ }
///
/// impl ActionHandler<EnemyAction> for EnemyController {
///     fn execute(&mut self, action: &EnemyAction, ctx: &mut Context) -> Status {
///         match action {
///             EnemyAction::Attack => {
///                 // Do attack logic
///                 Status::Success
///             }
///             EnemyAction::Patrol => Status::Running,
///             EnemyAction::Flee => Status::Running,
///         }
///     }
/// }
/// ```
pub trait ActionHandler<A> {
    /// Execute the given action. Return Running if the action needs more ticks.
    fn execute(&mut self, action: &A, ctx: &mut Context) -> Status;
}

/// Trait for evaluating user-defined conditions.
///
/// Conditions are instantaneous checks — they return `bool`, not `Status`.
/// The BT engine converts `true` → `Success`, `false` → `Failure`.
///
/// # Example
/// ```
/// use cogwise::{ConditionHandler, Context};
///
/// #[derive(Clone, Debug, PartialEq)]
/// enum EnemyCondition { IsPlayerVisible, IsLowHealth }
///
/// struct EnemyController { hp: i32 }
///
/// impl ConditionHandler<EnemyCondition> for EnemyController {
///     fn check(&self, condition: &EnemyCondition, ctx: &Context) -> bool {
///         match condition {
///             EnemyCondition::IsPlayerVisible => {
///                 // Check spatial query via blackboard
///                 ctx.blackboard().get_bool(0).unwrap_or(false)
///             }
///             EnemyCondition::IsLowHealth => self.hp < 30,
///         }
///     }
/// }
/// ```
pub trait ConditionHandler<C> {
    /// Check the condition. Returns true (Success) or false (Failure).
    fn check(&self, condition: &C, ctx: &Context) -> bool;
}
```

**Tests for Phase 1:**
- `status_invert` — Success ↔ Failure, Running unchanged
- `status_is_done` — Running = false, Success/Failure = true
- `decorator_clone` — all decorator variants are Clone + Debug
- `behavior_node_clone` — nested tree is Clone + PartialEq
- `parallel_policy_variants` — RequireAll, RequireOne, RequireN all constructible

---

## Phase 2: Blackboard + Context

### Step 6: BlackboardValue (blackboard.rs)

```rust
/// A typed value stored in the blackboard.
///
/// All values are fixed-size and `Copy` — no heap-allocated strings.
/// Float values use fixed-point (i32 with implicit 1/1000 scale) to
/// stay deterministic without requiring the Float trait in the BT core.
#[derive(Copy, Clone, Debug, PartialEq, Eq)]
pub enum BlackboardValue {
    /// 32-bit signed integer.
    Int(i32),
    /// Fixed-point float: value * 1000 (e.g., 1500 = 1.5).
    /// Use `BlackboardValue::from_f32(1.5)` to create.
    Fixed(i32),
    /// Boolean flag.
    Bool(bool),
    /// Entity reference (opaque ID).
    Entity(u32),
    /// 2D position or direction (integer components).
    Vec2(i32, i32),
}

impl BlackboardValue {
    /// Create a fixed-point value from a floating-point number.
    /// Multiplies by 1000 and truncates.
    pub fn from_f32(v: f32) -> Self {
        BlackboardValue::Fixed((v * 1000.0) as i32)
    }

    /// Extract as i32, or None if not Int.
    pub fn as_int(self) -> Option<i32> { /* ... */ }

    /// Extract as f32 (divides Fixed by 1000), or None if not Fixed.
    pub fn as_float(self) -> Option<f32> { /* ... */ }

    /// Extract as bool, or None if not Bool.
    pub fn as_bool(self) -> Option<bool> { /* ... */ }

    /// Extract as entity ID, or None if not Entity.
    pub fn as_entity(self) -> Option<u32> { /* ... */ }

    /// Extract as (i32, i32), or None if not Vec2.
    pub fn as_vec2(self) -> Option<(i32, i32)> { /* ... */ }

    /// Returns true if the value is "truthy":
    /// Int != 0, Fixed != 0, Bool == true, Entity != 0, Vec2 != (0,0).
    pub fn is_truthy(self) -> bool { /* ... */ }
}
```

### Step 7: Blackboard (blackboard.rs)

```rust
use alloc::collections::BTreeMap;

/// Key-value store for sharing data between behavior tree nodes.
///
/// Keys are `u32` — define constants in your game code:
/// ```
/// const BB_TARGET_POS: u32 = 0;
/// const BB_HEALTH: u32 = 1;
/// const BB_IS_ALERT: u32 = 2;
/// ```
///
/// The blackboard is shared across all nodes in a tree. Actions write
/// data, conditions and utility considerations read it.
#[derive(Clone, Debug, Default)]
pub struct Blackboard {
    entries: BTreeMap<u32, BlackboardValue>,
}

impl Blackboard {
    /// Create an empty blackboard.
    pub fn new() -> Self { /* ... */ }

    /// Get a value by key, or None if not present.
    pub fn get(&self, key: u32) -> Option<BlackboardValue> { /* ... */ }

    /// Get a value as i32, or None.
    pub fn get_int(&self, key: u32) -> Option<i32> { /* ... */ }

    /// Get a value as f32 (from Fixed), or None.
    pub fn get_float(&self, key: u32) -> Option<f32> { /* ... */ }

    /// Get a value as bool, or None.
    pub fn get_bool(&self, key: u32) -> Option<bool> { /* ... */ }

    /// Get a value as entity ID, or None.
    pub fn get_entity(&self, key: u32) -> Option<u32> { /* ... */ }

    /// Get a value as (i32, i32), or None.
    pub fn get_vec2(&self, key: u32) -> Option<(i32, i32)> { /* ... */ }

    /// Set a value. Overwrites any existing value at this key.
    pub fn set(&mut self, key: u32, value: BlackboardValue) { /* ... */ }

    /// Convenience setters.
    pub fn set_int(&mut self, key: u32, value: i32) { /* ... */ }
    pub fn set_float(&mut self, key: u32, value: f32) { /* ... */ }
    pub fn set_bool(&mut self, key: u32, value: bool) { /* ... */ }
    pub fn set_entity(&mut self, key: u32, value: u32) { /* ... */ }
    pub fn set_vec2(&mut self, key: u32, x: i32, y: i32) { /* ... */ }

    /// Returns true if the key exists.
    pub fn has(&self, key: u32) -> bool { /* ... */ }

    /// Remove a key. Returns the old value if it existed.
    pub fn remove(&mut self, key: u32) -> Option<BlackboardValue> { /* ... */ }

    /// Remove all entries.
    pub fn clear(&mut self) { /* ... */ }

    /// Number of entries.
    pub fn len(&self) -> usize { /* ... */ }

    /// True if empty.
    pub fn is_empty(&self) -> bool { /* ... */ }
}
```

### Step 8: Context (context.rs)

```rust
/// Execution context passed to every node during a tick.
///
/// Contains the current tick count, delta ticks since last frame,
/// a mutable reference to the blackboard, and an optional RNG for
/// random selection nodes.
pub struct Context<'a> {
    /// Monotonically increasing tick counter.
    tick: u64,
    /// Ticks elapsed since the last tick() call (usually 1).
    delta_ticks: u32,
    /// Shared blackboard for reading/writing AI state.
    blackboard: &'a mut Blackboard,
    /// Optional RNG for RandomSelector / WeightedSelector nodes.
    rng: Option<&'a mut dyn rand_core::RngCore>,
}

impl<'a> Context<'a> {
    /// Create a new context.
    pub fn new(
        tick: u64,
        delta_ticks: u32,
        blackboard: &'a mut Blackboard,
        rng: Option<&'a mut dyn rand_core::RngCore>,
    ) -> Self { /* ... */ }

    /// Current tick count.
    pub fn tick(&self) -> u64 { self.tick }

    /// Ticks since last frame.
    pub fn delta_ticks(&self) -> u32 { self.delta_ticks }

    /// Read-only blackboard access.
    pub fn blackboard(&self) -> &Blackboard { self.blackboard }

    /// Mutable blackboard access.
    pub fn blackboard_mut(&mut self) -> &mut Blackboard { self.blackboard }

    /// Get RNG reference (panics if no RNG was provided).
    pub fn rng(&mut self) -> &mut dyn rand_core::RngCore { /* ... */ }

    /// Check if RNG is available.
    pub fn has_rng(&self) -> bool { self.rng.is_some() }
}
```

**Tests for Phase 2:**
- `blackboard_set_get_int` — set and get back an Int value
- `blackboard_set_get_all_types` — every BlackboardValue variant round-trips
- `blackboard_overwrite` — setting same key overwrites
- `blackboard_remove` — remove returns old value, subsequent get returns None
- `blackboard_clear` — clear empties all entries
- `blackboard_is_truthy` — verify truthy/falsy for each variant
- `blackboard_from_f32` — Fixed(1500) from 1.5f32
- `context_tick_count` — tick counter accessible
- `context_blackboard_read_write` — blackboard mutable through context

---

## Phase 3: Node State + Tick Engine

This is the core of the library. The tick engine traverses the tree recursively, using per-node state to track which child is running.

### Step 9: NodeState (tick.rs)

```rust
/// Runtime state for a single node in the tree.
///
/// Each node in the tree gets a corresponding NodeState entry. This tracks
/// information that persists across ticks, like which child of a Sequence
/// is currently running, or how many ticks a Cooldown has remaining.
#[derive(Clone, Debug, Default)]
pub struct NodeState {
    /// For Sequence/Selector: index of the currently running child.
    /// Reset to 0 when the node completes.
    pub running_child: usize,

    /// For Wait: ticks remaining.
    /// For Cooldown decorator: cooldown ticks remaining.
    /// For Timeout decorator: ticks elapsed.
    pub tick_counter: u32,

    /// For Repeat/Retry: iterations completed so far.
    pub iteration_count: u32,

    /// For UtilitySelector: index of the currently selected child
    /// (persists while the selected child returns Running).
    pub selected_child: Option<usize>,

    /// For RandomSelector/WeightedSelector: selected child index
    /// (persists while Running, re-randomized on next activation).
    pub random_selection: Option<usize>,
}

impl NodeState {
    /// Reset all state to defaults (called when a node completes).
    pub fn reset(&mut self) {
        *self = Self::default();
    }
}
```

### Step 10: Node ID Assignment

Your job: assign a unique `usize` ID to each node in the tree via a depth-first pre-order traversal. The tree owns a `Vec<NodeState>` indexed by these IDs. This happens once when the tree is built.

```rust
/// Assigns sequential IDs to all nodes in the tree.
/// Returns the total number of nodes.
///
/// Algorithm (pre-order DFS):
/// ```text
/// assign_ids(node, next_id):
///     node.id = next_id
///     next_id += 1
///     for each child of node:
///         next_id = assign_ids(child, next_id)
///     return next_id
/// ```
fn assign_ids<A, C>(node: &BehaviorNode<A, C>) -> usize { /* ... */ }
```

### Step 11: The Tick Function

This is the most important function in the library. It recursively traverses the tree, executing each node according to its type.

**Algorithm pseudocode:**

```text
tick(node, node_id, states, ctx, action_handler, condition_handler, observer):
    observer.on_enter(node_id)
    state = &mut states[node_id]

    status = match node:
        Sequence(children):
            start_from = state.running_child
            for i in start_from..children.len():
                child_status = tick(children[i], child_id(i), ...)
                if child_status == Running:
                    state.running_child = i
                    return Running
                if child_status == Failure:
                    state.reset()
                    return Failure
            state.reset()
            return Success

        Selector(children):
            start_from = state.running_child
            for i in start_from..children.len():
                child_status = tick(children[i], child_id(i), ...)
                if child_status == Running:
                    state.running_child = i
                    return Running
                if child_status == Success:
                    state.reset()
                    return Success
            state.reset()
            return Failure

        Parallel { policy, children }:
            success_count = 0
            failure_count = 0
            for child in children:
                child_status = tick(child, ...)
                match child_status:
                    Success => success_count += 1
                    Failure => failure_count += 1
                    Running => ()
            match policy:
                RequireAll:
                    if failure_count > 0: Failure
                    elif success_count == children.len(): Success
                    else: Running
                RequireOne:
                    if success_count > 0: Success
                    elif failure_count == children.len(): Failure
                    else: Running
                RequireN(n):
                    if success_count >= n: Success
                    elif children.len() - failure_count < n: Failure
                    else: Running

        Decorator { decorator, child }:
            match decorator:
                Inverter:
                    tick(child).invert()

                Repeat(n):
                    child_status = tick(child, ...)
                    if child_status == Failure:
                        state.reset(); return Failure
                    if child_status == Success:
                        state.iteration_count += 1
                        if state.iteration_count >= n:
                            state.reset(); return Success
                        // Reset child state for next iteration
                        return Running
                    return Running  // child still running

                Retry(n):
                    child_status = tick(child, ...)
                    if child_status == Success:
                        state.reset(); return Success
                    if child_status == Failure:
                        state.iteration_count += 1
                        if state.iteration_count >= n:
                            state.reset(); return Failure
                        return Running
                    return Running

                Cooldown(ticks):
                    if state.tick_counter > 0:
                        state.tick_counter -= ctx.delta_ticks.min(state.tick_counter)
                        return Failure
                    child_status = tick(child, ...)
                    if child_status != Running:
                        state.tick_counter = ticks
                    return child_status

                Guard(key):
                    if !ctx.blackboard.get(key).map(|v| v.is_truthy()).unwrap_or(false):
                        return Failure
                    tick(child, ...)

                UntilSuccess:
                    child_status = tick(child, ...)
                    if child_status == Success:
                        state.reset(); return Success
                    if child_status == Failure:
                        // Reset child, try again next tick
                        return Running
                    return Running

                UntilFail:
                    child_status = tick(child, ...)
                    if child_status == Failure:
                        state.reset(); return Failure
                    if child_status == Success:
                        return Running
                    return Running

                Timeout(max_ticks):
                    state.tick_counter += ctx.delta_ticks
                    if state.tick_counter >= max_ticks:
                        state.reset(); return Failure
                    child_status = tick(child, ...)
                    if child_status != Running:
                        state.reset()
                    return child_status

                ForceSuccess:
                    child_status = tick(child, ...)
                    if child_status == Running: Running
                    else: Success

                ForceFailure:
                    child_status = tick(child, ...)
                    if child_status == Running: Running
                    else: Failure

        Action(action_id):
            action_handler.execute(action_id, ctx)

        Condition(condition_id):
            if condition_handler.check(condition_id, ctx): Success
            else: Failure

        Wait(ticks):
            state.tick_counter += ctx.delta_ticks
            if state.tick_counter >= ticks:
                state.reset(); return Success
            return Running

        UtilitySelector { children, utility_ids }:
            // If a child is currently running, keep ticking it
            if let Some(idx) = state.selected_child:
                child_status = tick(children[idx], ...)
                if child_status != Running:
                    state.selected_child = None
                    state.reset()
                return child_status
            // Otherwise, evaluate utility scores and pick best
            best_idx = evaluate_utility(utility_ids, ctx)
            state.selected_child = Some(best_idx)
            tick(children[best_idx], ...)

        RandomSelector(children):
            if let Some(idx) = state.random_selection:
                child_status = tick(children[idx], ...)
                if child_status != Running:
                    state.random_selection = None
                    state.reset()
                return child_status
            idx = ctx.rng().next_u32() % children.len()
            state.random_selection = Some(idx)
            tick(children[idx], ...)

        WeightedSelector { children, weights }:
            // Same as RandomSelector but using weighted random
            if let Some(idx) = state.random_selection:
                child_status = tick(children[idx], ...)
                if child_status != Running:
                    state.random_selection = None
                    state.reset()
                return child_status
            total_weight = weights.iter().sum()
            roll = ctx.rng().next_u32() % total_weight
            cumulative = 0
            for (i, w) in weights.iter().enumerate():
                cumulative += w
                if roll < cumulative:
                    state.random_selection = Some(i)
                    return tick(children[i], ...)

    observer.on_exit(node_id, status)
    return status
```

Your job: implement this as a recursive function. The key insight is that each node's children have predictable ID offsets based on the pre-order traversal, so you can compute child IDs without storing them.

```rust
/// Tick the behavior tree rooted at `node`.
///
/// This is the core recursive traversal function. Each node type has
/// specific tick semantics documented on the `BehaviorNode` enum.
///
/// # Type Parameters
/// - `A`: Action ID enum
/// - `C`: Condition ID enum
/// - `AH`: ActionHandler implementation
/// - `CH`: ConditionHandler implementation
/// - `O`: Observer implementation
pub fn tick_node<A, C, AH, CH, O>(
    node: &BehaviorNode<A, C>,
    node_id: usize,
    states: &mut [NodeState],
    ctx: &mut Context,
    action_handler: &mut AH,
    condition_handler: &CH,
    observer: &mut O,
) -> Status
where
    AH: ActionHandler<A>,
    CH: ConditionHandler<C>,
    O: Observer,
{
    // Implementation follows the pseudocode above
}
```

**Tests for Phase 3:**
- `tick_sequence_all_success` — sequence with 3 Success children returns Success
- `tick_sequence_first_failure` — sequence returns Failure on first child Failure
- `tick_sequence_resumes_running` — Running child is resumed on next tick
- `tick_selector_first_success` — selector returns Success on first child Success
- `tick_selector_all_failure` — selector returns Failure when all children fail
- `tick_parallel_require_all` — succeeds when all succeed, fails on first failure
- `tick_parallel_require_one` — succeeds on first success
- `tick_parallel_require_n` — succeeds when N children succeed
- `tick_decorator_inverter` — Success ↔ Failure
- `tick_decorator_repeat` — repeats child N times
- `tick_decorator_retry` — retries on failure up to N times
- `tick_decorator_cooldown` — blocks ticking for N ticks after completion
- `tick_decorator_guard` — checks blackboard key
- `tick_decorator_until_success` — keeps going until success
- `tick_decorator_timeout` — fails after N ticks
- `tick_decorator_force_success` — converts any result to Success
- `tick_wait` — returns Running for N ticks, then Success
- `tick_action` — calls ActionHandler
- `tick_condition_true` — returns Success when handler returns true
- `tick_condition_false` — returns Failure when handler returns false

---

## Phase 4: Tree Wrapper + Builder

### Step 12: BehaviorTree (tree.rs)

```rust
/// A complete behavior tree with associated runtime state.
///
/// Wraps a root `BehaviorNode`, a `Vec<NodeState>` for tracking runtime
/// state per node, and a `Blackboard` for shared data.
///
/// # Example
/// ```
/// use cogwise::{BehaviorTree, TreeBuilder, Status};
///
/// let tree = TreeBuilder::new()
///     .sequence()
///         .condition(IsPlayerVisible)
///         .action(Attack)
///     .end()
///     .build();
///
/// let mut bt = BehaviorTree::new(tree);
/// let status = bt.tick(&mut controller, &controller, &mut NoOpObserver);
/// ```
pub struct BehaviorTree<A, C> {
    /// The root node of the tree.
    root: BehaviorNode<A, C>,
    /// Per-node runtime state, indexed by node ID.
    states: Vec<NodeState>,
    /// Shared blackboard.
    blackboard: Blackboard,
    /// Total tick count.
    tick_count: u64,
}

impl<A, C> BehaviorTree<A, C> {
    /// Create a new tree from a root node. Allocates NodeState for each node.
    pub fn new(root: BehaviorNode<A, C>) -> Self { /* ... */ }

    /// Tick the tree once. Advances tick counter by `delta_ticks` (default 1).
    pub fn tick<AH, CH, O>(
        &mut self,
        action_handler: &mut AH,
        condition_handler: &CH,
        observer: &mut O,
    ) -> Status
    where
        AH: ActionHandler<A>,
        CH: ConditionHandler<C>,
        O: Observer,
    { /* ... */ }

    /// Tick with a specific delta and optional RNG.
    pub fn tick_with<AH, CH, O>(
        &mut self,
        delta_ticks: u32,
        rng: Option<&mut dyn rand_core::RngCore>,
        action_handler: &mut AH,
        condition_handler: &CH,
        observer: &mut O,
    ) -> Status
    where
        AH: ActionHandler<A>,
        CH: ConditionHandler<C>,
        O: Observer,
    { /* ... */ }

    /// Read-only access to the blackboard.
    pub fn blackboard(&self) -> &Blackboard { &self.blackboard }

    /// Mutable access to the blackboard.
    pub fn blackboard_mut(&mut self) -> &mut Blackboard { &mut self.blackboard }

    /// Reset all node states (but keep blackboard).
    pub fn reset(&mut self) { /* ... */ }

    /// Reset everything including blackboard.
    pub fn reset_all(&mut self) { /* ... */ }

    /// Current tick count.
    pub fn tick_count(&self) -> u64 { self.tick_count }

    /// Number of nodes in the tree.
    pub fn node_count(&self) -> usize { self.states.len() }
}
```

### Step 13: TreeBuilder (builder.rs)

```rust
/// Fluent API for constructing behavior trees.
///
/// Uses a stack-based approach: composite nodes (Sequence, Selector, etc.)
/// push onto the stack, `.end()` pops and adds to the parent.
///
/// # Example
/// ```
/// use cogwise::TreeBuilder;
///
/// // Guard → patrol between waypoints
/// let tree = TreeBuilder::new()
///     .selector()
///         // Combat branch
///         .sequence()
///             .condition(IsEnemyNear)
///             .selector()
///                 .sequence()
///                     .condition(HasAmmo)
///                     .action(Shoot)
///                 .end()
///                 .action(MeleeAttack)
///             .end()
///         .end()
///         // Patrol branch
///         .sequence()
///             .action(MoveToWaypoint)
///             .action(WaitAtWaypoint)
///         .end()
///     .end()
///     .build();
/// ```
pub struct TreeBuilder<A, C> {
    /// Stack of in-progress composite nodes.
    stack: Vec<BuilderFrame<A, C>>,
}

/// A frame on the builder stack representing a composite node being built.
struct BuilderFrame<A, C> {
    /// The type of composite node.
    node_type: CompositeType,
    /// Children accumulated so far.
    children: Vec<BehaviorNode<A, C>>,
    /// Additional data (weights for WeightedSelector, policy for Parallel, etc.).
    metadata: FrameMetadata,
}

impl<A, C> TreeBuilder<A, C> {
    pub fn new() -> Self { /* ... */ }

    /// Begin a Sequence node. Call .end() to close it.
    pub fn sequence(mut self) -> Self { /* ... */ }

    /// Begin a Selector node. Call .end() to close it.
    pub fn selector(mut self) -> Self { /* ... */ }

    /// Begin a Parallel node. Call .end() to close it.
    pub fn parallel(mut self, policy: ParallelPolicy) -> Self { /* ... */ }

    /// Begin a RandomSelector. Call .end() to close it.
    pub fn random_selector(mut self) -> Self { /* ... */ }

    /// Begin a WeightedSelector. Call .end() to close it.
    pub fn weighted_selector(mut self) -> Self { /* ... */ }

    /// Add an Action leaf to the current composite.
    pub fn action(mut self, action: A) -> Self { /* ... */ }

    /// Add a Condition leaf to the current composite.
    pub fn condition(mut self, condition: C) -> Self { /* ... */ }

    /// Add a Wait leaf.
    pub fn wait(mut self, ticks: u32) -> Self { /* ... */ }

    /// Wrap the next node with a Decorator.
    pub fn decorator(mut self, decorator: Decorator) -> Self { /* ... */ }

    /// Add a weight for WeightedSelector (must match child count).
    pub fn weight(mut self, w: u32) -> Self { /* ... */ }

    /// Close the current composite and add it to the parent.
    pub fn end(mut self) -> Self { /* ... */ }

    /// Finalize and return the root BehaviorNode.
    /// Panics if the stack is not properly balanced (unclosed composites).
    pub fn build(self) -> BehaviorNode<A, C> { /* ... */ }
}
```

**Tests for Phase 4:**
- `builder_simple_sequence` — builds Sequence([Action, Action])
- `builder_nested` — builds Selector([Sequence([...]), Action])
- `builder_with_decorator` — decorator wraps next node correctly
- `builder_weighted_selector` — weights associate with children
- `tree_tick_increments_counter` — tick count advances
- `tree_reset_clears_state` — reset zeroes all NodeState
- `tree_blackboard_access` — read/write through tree API

---

## Phase 5: Float + Response Curves (Utility AI Foundation)

### Step 14: Float Trait (float.rs)

Same pattern as sibling libraries (softy, navex). Your job: implement using `libm` for `no_std` math.

```rust
/// Trait abstracting floating-point operations for the utility AI module.
///
/// Implemented for `f32` and `f64`. Uses `libm` for no_std math functions.
/// Only used in the `utility` submodule — the BT core uses integers only.
pub trait Float:
    Copy + Clone + PartialEq + PartialOrd
    + core::ops::Add<Output = Self>
    + core::ops::Sub<Output = Self>
    + core::ops::Mul<Output = Self>
    + core::ops::Div<Output = Self>
    + core::ops::Neg<Output = Self>
    + Default + core::fmt::Debug
{
    fn zero() -> Self;
    fn one() -> Self;
    fn half() -> Self;
    fn two() -> Self;
    fn from_f32(v: f32) -> Self;
    fn to_f32(self) -> f32;
    fn sqrt(self) -> Self;
    fn exp(self) -> Self;
    fn ln(self) -> Self;
    fn abs(self) -> Self;
    fn min(self, other: Self) -> Self;
    fn max(self, other: Self) -> Self;
    fn powf(self, exp: Self) -> Self;

    fn clamp(self, min: Self, max: Self) -> Self {
        self.max(min).min(max)
    }

    fn lerp(self, other: Self, t: Self) -> Self {
        self + (other - self) * t
    }
}
```

### Step 15: Response Curves (utility/curve.rs)

Response curves map a normalized input [0, 1] to an output score [0, 1]. They are the building blocks of utility AI — each consideration uses a curve to express "how much do I care about this input at this level?"

```rust
/// A curve that maps an input value [0, 1] to an output score [0, 1].
///
/// Used by `Consideration` to transform blackboard values into utility scores.
///
/// # Curve Types
///
/// ```text
/// Linear:      y = slope * x + offset
/// Polynomial:  y = (x + offset) ^ exponent
/// Logistic:    y = 1 / (1 + e^(-steepness * (x - midpoint)))
/// Step:        y = if x >= threshold { 1 } else { 0 }
/// Inverse:     y = 1 / (x + offset)  (clamped to [0, 1])
/// Constant:    y = value (always returns the same score)
/// CustomPoints: y = piecewise linear interpolation between given points
/// ```
#[derive(Clone, Debug, PartialEq)]
pub enum ResponseCurve<F: Float> {
    /// Linear: `y = slope * x + offset`, clamped to [0, 1].
    Linear { slope: F, offset: F },

    /// Polynomial: `y = (x + offset) ^ exponent`, clamped to [0, 1].
    /// Use exponent > 1 for slow-start curves, < 1 for fast-start.
    Polynomial { exponent: F, offset: F },

    /// Logistic (S-curve): `y = 1 / (1 + e^(-steepness * (x - midpoint)))`.
    /// Good for threshold-like behavior with smooth transition.
    Logistic { midpoint: F, steepness: F },

    /// Step function: 0 below threshold, 1 at or above.
    Step { threshold: F },

    /// Inverse: `y = 1 / (x + offset)`, clamped to [0, 1].
    /// High score at low input, drops off.
    Inverse { offset: F },

    /// Always returns a fixed value.
    Constant(F),

    /// Piecewise linear interpolation between up to 8 points.
    /// Points must be sorted by x. Values outside range clamp to nearest.
    CustomPoints(Vec<(F, F)>),
}

impl<F: Float> ResponseCurve<F> {
    /// Evaluate the curve at input `x` (should be in [0, 1]).
    /// Output is clamped to [0, 1].
    pub fn evaluate(&self, x: F) -> F {
        let raw = match self {
            ResponseCurve::Linear { slope, offset } => {
                *slope * x + *offset
            }
            ResponseCurve::Polynomial { exponent, offset } => {
                (x + *offset).max(F::zero()).powf(*exponent)
            }
            ResponseCurve::Logistic { midpoint, steepness } => {
                let exp_val = (F::zero() - *steepness * (x - *midpoint)).exp();
                F::one() / (F::one() + exp_val)
            }
            ResponseCurve::Step { threshold } => {
                if x >= *threshold { F::one() } else { F::zero() }
            }
            ResponseCurve::Inverse { offset } => {
                F::one() / (x + *offset)
            }
            ResponseCurve::Constant(v) => *v,
            ResponseCurve::CustomPoints(points) => {
                // Piecewise linear interpolation
                // Binary search for surrounding points, lerp between them
                piecewise_lerp(points, x)
            }
        };
        raw.clamp(F::zero(), F::one())
    }
}
```

**Curve math reference:**

```text
Linear:
    y = slope * x + offset
    Common: slope=1, offset=0 (identity)
    slope=-1, offset=1 (inverted)

Polynomial:
    y = (x + offset) ^ exp
    exp=2: quadratic (slow start, fast finish)
    exp=0.5: square root (fast start, slow finish)
    exp=3: cubic (very slow start)

Logistic:
    y = 1 / (1 + e^(-k*(x-m)))
    m = midpoint (where curve crosses 0.5)
    k = steepness (higher = sharper transition)
    k=10, m=0.5: sharp S-curve centered at 0.5
    k=5, m=0.3: gentle S-curve, transitions early

Step:
    y = x >= threshold ? 1 : 0
    Simplest binary decision curve

Inverse:
    y = 1 / (x + offset)
    offset prevents division by zero
    Good for "urgency" — very high at low values

CustomPoints:
    Piecewise linear between sorted (x, y) pairs.
    For x between points[i] and points[i+1]:
        t = (x - points[i].x) / (points[i+1].x - points[i].x)
        y = lerp(points[i].y, points[i+1].y, t)
```

**Tests for Phase 5:**
- `float_f32_basics` — zero, one, from_f32 round-trip
- `float_f32_math` — sqrt(4)=2, exp(0)=1, ln(1)=0
- `curve_linear_identity` — slope=1, offset=0: evaluate(0.5) = 0.5
- `curve_linear_inverted` — slope=-1, offset=1: evaluate(0.0) = 1.0
- `curve_polynomial_quadratic` — exponent=2: evaluate(0.5) = 0.25
- `curve_logistic_midpoint` — evaluate(midpoint) ≈ 0.5
- `curve_step` — below=0, at/above=1
- `curve_clamp` — output always in [0, 1] even with extreme inputs
- `curve_custom_points` — interpolates correctly between defined points

---

## Phase 6: Utility AI

### Step 16: Consideration (utility/consideration.rs)

```rust
/// A single factor in a utility decision.
///
/// Reads a value from the blackboard, normalizes it to [0, 1], and maps
/// it through a response curve to produce a score.
///
/// # Example
/// ```
/// use cogwise::utility::{Consideration, ResponseCurve};
///
/// // "How much do I want to attack based on enemy distance?"
/// // Close = high score, far = low score (inverse curve)
/// let distance_factor = Consideration {
///     input_key: BB_ENEMY_DISTANCE,
///     curve: ResponseCurve::Inverse { offset: 0.1 },
///     weight: 1.0,
///     input_min: 0.0,    // closest possible
///     input_max: 100.0,   // farthest relevant distance
/// };
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Consideration<F: Float> {
    /// Blackboard key to read the input value from.
    pub input_key: u32,
    /// Response curve to map normalized input → score.
    pub curve: ResponseCurve<F>,
    /// Weight multiplier for this consideration (default 1.0).
    pub weight: F,
    /// Minimum input value (maps to curve input 0.0).
    pub input_min: F,
    /// Maximum input value (maps to curve input 1.0).
    pub input_max: F,
}

impl<F: Float> Consideration<F> {
    /// Evaluate this consideration against the blackboard.
    ///
    /// 1. Read the raw value from the blackboard (as Fixed → f32)
    /// 2. Normalize to [0, 1] using input_min/max
    /// 3. Map through the response curve
    /// 4. Multiply by weight
    ///
    /// Returns 0.0 if the blackboard key is missing.
    pub fn evaluate(&self, blackboard: &Blackboard) -> F { /* ... */ }
}
```

### Step 17: UtilityAction (utility/action.rs)

```rust
/// An action candidate with associated utility considerations.
///
/// The overall score is the **geometric mean** of all consideration scores,
/// multiplied by the action's base weight. Geometric mean ensures that a
/// single zero-score consideration vetoes the entire action.
///
/// # Scoring Algorithm
/// ```text
/// raw_scores = [c.evaluate(blackboard) for c in considerations]
/// product = raw_scores[0] * raw_scores[1] * ... * raw_scores[n-1]
/// geometric_mean = product ^ (1/n)
/// final_score = geometric_mean * weight
///
/// If momentum > 0 and this action was selected last tick:
///     final_score += momentum  (bonus for staying with current action)
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct UtilityAction<F: Float, A> {
    /// The action to execute if this scores highest.
    pub action_id: A,
    /// Factors that contribute to this action's score.
    pub considerations: Vec<Consideration<F>>,
    /// Base weight multiplier (default 1.0).
    pub weight: F,
    /// Bonus score for continuing this action from the previous tick.
    /// Prevents thrashing between actions with similar scores.
    pub momentum: F,
}

impl<F: Float, A> UtilityAction<F, A> {
    /// Calculate the utility score for this action.
    ///
    /// Uses geometric mean of consideration scores × weight.
    /// Adds momentum bonus if `is_current` is true.
    pub fn score(&self, blackboard: &Blackboard, is_current: bool) -> F {
        if self.considerations.is_empty() {
            return self.weight;
        }

        let n = self.considerations.len();
        let mut product = F::one();
        for c in &self.considerations {
            product = product * c.evaluate(blackboard);
        }

        // Geometric mean: product^(1/n)
        let inv_n = F::one() / F::from_f32(n as f32);
        let geo_mean = product.powf(inv_n);
        let mut score = geo_mean * self.weight;

        if is_current {
            score = score + self.momentum;
        }

        score
    }
}
```

### Step 18: Reasoner + SelectionMethod (utility/reasoner.rs)

```rust
/// Method for selecting among scored utility actions.
#[derive(Clone, Debug, PartialEq)]
pub enum SelectionMethod {
    /// Always pick the highest-scoring action.
    HighestScore,
    /// Pick randomly, weighted by score. Higher scores = more likely.
    /// Adds variety while still preferring high-scoring actions.
    WeightedRandom,
    /// Pick randomly from the top N scorers (equal probability among them).
    TopN(usize),
}

/// Evaluates a set of utility actions and selects the best one.
///
/// The Reasoner is the bridge between the utility AI system and the
/// behavior tree. A `UtilitySelector` node in the BT delegates to
/// the Reasoner to pick which child branch to execute.
///
/// # Example
/// ```
/// use cogwise::utility::{Reasoner, UtilityAction, SelectionMethod};
///
/// let reasoner: Reasoner<f32, EnemyAction> = Reasoner {
///     actions: vec![
///         attack_action,
///         flee_action,
///         patrol_action,
///     ],
///     selection_method: SelectionMethod::HighestScore,
/// };
///
/// let chosen_idx = reasoner.select(&blackboard, None, None);
/// ```
#[derive(Clone, Debug, PartialEq)]
pub struct Reasoner<F: Float, A> {
    /// Available actions with their utility configurations.
    pub actions: Vec<UtilityAction<F, A>>,
    /// How to select among scored actions.
    pub selection_method: SelectionMethod,
}

impl<F: Float, A> Reasoner<F, A> {
    /// Score all actions and select the best one.
    ///
    /// Returns the index of the selected action.
    ///
    /// # Arguments
    /// - `blackboard`: Current blackboard state
    /// - `current_action`: Index of the previously selected action (for momentum)
    /// - `rng`: Required for WeightedRandom and TopN selection methods
    pub fn select(
        &self,
        blackboard: &Blackboard,
        current_action: Option<usize>,
        rng: Option<&mut dyn rand_core::RngCore>,
    ) -> usize {
        // 1. Score all actions
        let scores: Vec<F> = self.actions.iter().enumerate().map(|(i, action)| {
            let is_current = current_action == Some(i);
            action.score(blackboard, is_current)
        }).collect();

        // 2. Select based on method
        match &self.selection_method {
            SelectionMethod::HighestScore => {
                // Return index of highest score
                scores.iter().enumerate()
                    .max_by(|a, b| a.1.partial_cmp(b.1).unwrap_or(core::cmp::Ordering::Equal))
                    .map(|(i, _)| i)
                    .unwrap_or(0)
            }
            SelectionMethod::WeightedRandom => {
                // Weighted random selection proportional to scores
                let rng = rng.expect("WeightedRandom requires RNG");
                let total: F = scores.iter().copied().fold(F::zero(), |a, b| a + b);
                if total <= F::zero() { return 0; }
                let roll = F::from_f32((rng.next_u32() as f32) / (u32::MAX as f32)) * total;
                let mut cumulative = F::zero();
                for (i, &score) in scores.iter().enumerate() {
                    cumulative = cumulative + score;
                    if roll < cumulative { return i; }
                }
                scores.len() - 1
            }
            SelectionMethod::TopN(n) => {
                // Pick randomly from top N scorers
                let rng = rng.expect("TopN requires RNG");
                let mut indices: Vec<usize> = (0..scores.len()).collect();
                indices.sort_by(|&a, &b|
                    scores[b].partial_cmp(&scores[a]).unwrap_or(core::cmp::Ordering::Equal)
                );
                let top_n = indices[..(*n).min(indices.len())].to_vec();
                top_n[rng.next_u32() as usize % top_n.len()]
            }
        }
    }

    /// Score all actions and return (index, score) pairs sorted by score descending.
    /// Useful for debugging.
    pub fn score_all(&self, blackboard: &Blackboard, current_action: Option<usize>) -> Vec<(usize, F)> { /* ... */ }
}
```

**Tests for Phase 6:**
- `consideration_reads_blackboard` — reads Fixed value, normalizes, applies curve
- `consideration_missing_key_returns_zero` — missing blackboard key → 0.0
- `consideration_normalizes_input` — input_min=0, input_max=100, raw=50 → normalized=0.5
- `utility_action_geometric_mean` — two considerations at 0.5 → geometric mean = 0.5
- `utility_action_zero_vetoes` — one consideration at 0.0 → total score = 0.0
- `utility_action_momentum` — current action gets momentum bonus
- `reasoner_highest_score` — picks the highest-scoring action
- `reasoner_weighted_random` — higher scores selected more often (statistical test)
- `reasoner_top_n` — only selects from top N

---

## Phase 7: Observer + Config + Error + Presets

### Step 19: Observer (observer.rs)

```rust
/// Trait for observing behavior tree execution.
///
/// Implement this to build debuggers, visualizers, or logging.
/// All methods have default no-op implementations.
pub trait Observer {
    /// Called when a node is about to be ticked.
    fn on_enter(&mut self, _node_id: usize) {}

    /// Called after a node has been ticked with its result.
    fn on_exit(&mut self, _node_id: usize, _status: Status) {}

    /// Called when a blackboard value is written.
    fn on_blackboard_write(&mut self, _key: u32, _value: BlackboardValue) {}

    /// Called when a utility evaluation completes.
    fn on_utility_score(&mut self, _action_index: usize, _score: f32) {}
}

/// A no-op observer that does nothing. Use as default.
pub struct NoOpObserver;
impl Observer for NoOpObserver {}

/// An observer that records all events for later inspection.
/// Useful for testing and debugging.
pub struct RecordingObserver {
    pub events: Vec<ObserverEvent>,
}

/// Events recorded by RecordingObserver.
#[derive(Clone, Debug)]
pub enum ObserverEvent {
    Enter(usize),
    Exit(usize, Status),
    BlackboardWrite(u32, BlackboardValue),
    UtilityScore(usize, f32),
}
```

### Step 20: Config (config.rs)

```rust
/// Configuration for the behavior tree engine.
#[derive(Clone, Debug)]
pub struct TreeConfig {
    /// Maximum tree depth for recursion (prevents stack overflow).
    /// Default: 64.
    pub max_depth: usize,

    /// Maximum nodes ticked per frame (prevents infinite loops).
    /// Default: 10_000.
    pub max_ticks_per_frame: usize,
}

impl Default for TreeConfig {
    fn default() -> Self {
        TreeConfig {
            max_depth: 64,
            max_ticks_per_frame: 10_000,
        }
    }
}
```

### Step 21: Error (error.rs)

```rust
/// Errors that can occur during tree construction or validation.
#[derive(Clone, Debug, PartialEq, Eq)]
pub enum TreeError {
    /// A composite node (Sequence, Selector, etc.) has no children.
    EmptyComposite,
    /// Tree exceeds maximum depth.
    MaxDepthExceeded(usize),
    /// WeightedSelector has mismatched children/weights count.
    WeightCountMismatch { children: usize, weights: usize },
    /// UtilitySelector has mismatched children/utility_ids count.
    UtilityIdCountMismatch { children: usize, ids: usize },
    /// Builder has unclosed composite nodes.
    UnbalancedBuilder(usize),
}
```

### Step 22: Presets (preset.rs)

```rust
/// Pre-built behavior tree templates for common AI patterns.
///
/// These return `BehaviorNode<u32, u32>` using numeric IDs.
/// Map your game's action/condition enums to these IDs.
///
/// # Action IDs (by convention)
/// - 0: Idle
/// - 1: Move to target
/// - 2: Attack
/// - 3: Flee
/// - 4: Patrol (move to next waypoint)
/// - 5: Wait at position
///
/// # Condition IDs (by convention)
/// - 0: Is enemy visible
/// - 1: Is enemy in range
/// - 2: Is health low
/// - 3: Is at waypoint
/// - 4: Has target

/// Simple patrol: move to waypoint → wait → next waypoint → repeat.
pub fn patrol() -> BehaviorNode<u32, u32> {
    BehaviorNode::Decorator {
        decorator: Decorator::Repeat(u32::MAX), // loop forever
        child: Box::new(BehaviorNode::Sequence(vec![
            BehaviorNode::Action(4),  // patrol (move to next waypoint)
            BehaviorNode::Wait(60),   // wait 60 ticks at waypoint
        ])),
    }
}

/// Combat melee: approach → attack when in range → retreat if hurt.
pub fn combat_melee() -> BehaviorNode<u32, u32> {
    BehaviorNode::Selector(vec![
        // Priority 1: flee if low health
        BehaviorNode::Sequence(vec![
            BehaviorNode::Condition(2),  // is_health_low
            BehaviorNode::Action(3),     // flee
        ]),
        // Priority 2: attack if in range
        BehaviorNode::Sequence(vec![
            BehaviorNode::Condition(1),  // is_enemy_in_range
            BehaviorNode::Action(2),     // attack
        ]),
        // Priority 3: approach if visible
        BehaviorNode::Sequence(vec![
            BehaviorNode::Condition(0),  // is_enemy_visible
            BehaviorNode::Action(1),     // move_to_target
        ]),
        // Fallback: idle
        BehaviorNode::Action(0),
    ])
}

/// Guard post: idle → alert on enemy → chase → return to post.
pub fn guard_post() -> BehaviorNode<u32, u32> {
    BehaviorNode::Selector(vec![
        // Chase if enemy visible and in range
        BehaviorNode::Sequence(vec![
            BehaviorNode::Condition(0),  // is_enemy_visible
            BehaviorNode::Condition(1),  // is_enemy_in_range
            BehaviorNode::Action(2),     // attack
        ]),
        // Approach if enemy visible
        BehaviorNode::Sequence(vec![
            BehaviorNode::Condition(0),  // is_enemy_visible
            BehaviorNode::Action(1),     // move_to_target
        ]),
        // Return to post if not at waypoint
        BehaviorNode::Sequence(vec![
            BehaviorNode::Decorator {
                decorator: Decorator::Inverter,
                child: Box::new(BehaviorNode::Condition(3)), // !is_at_waypoint
            },
            BehaviorNode::Action(4),  // patrol (return to post)
        ]),
        // Idle at post
        BehaviorNode::Action(0),
    ])
}
```

**Tests for Phase 7:**
- `observer_records_events` — RecordingObserver captures Enter/Exit sequence
- `observer_noop_compiles` — NoOpObserver works as default
- `tree_config_defaults` — default values are sensible
- `tree_error_variants` — all error variants constructible
- `preset_patrol_structure` — patrol tree has expected shape
- `preset_combat_priority` — combat tree checks flee before attack
- `preset_guard_returns` — guard tree returns to post when no enemy

---

## Phase 8: Integration + lib.rs + Tests

### Step 23: lib.rs Re-exports

```rust
#![no_std]
extern crate alloc;

pub mod status;
pub mod node;
pub mod decorator;
pub mod parallel;
pub mod leaf;
pub mod blackboard;
pub mod context;
pub mod tick;
pub mod tree;
pub mod builder;
pub mod utility;
pub mod observer;
pub mod config;
pub mod error;
pub mod preset;

// Re-exports for convenience
pub use status::Status;
pub use node::BehaviorNode;
pub use decorator::Decorator;
pub use parallel::ParallelPolicy;
pub use leaf::{ActionHandler, ConditionHandler};
pub use blackboard::{Blackboard, BlackboardValue};
pub use context::Context;
pub use tree::BehaviorTree;
pub use builder::TreeBuilder;
pub use observer::{Observer, NoOpObserver};
pub use config::TreeConfig;
pub use error::TreeError;
```

### Step 24: Full Test List

```text
# status.rs
- status_invert_success
- status_invert_failure
- status_invert_running
- status_is_done
- status_is_success
- status_is_failure

# blackboard.rs
- blackboard_set_get_int
- blackboard_set_get_fixed
- blackboard_set_get_bool
- blackboard_set_get_entity
- blackboard_set_get_vec2
- blackboard_overwrite
- blackboard_remove
- blackboard_clear
- blackboard_has
- blackboard_is_truthy_int
- blackboard_is_truthy_bool
- blackboard_from_f32

# tick.rs
- tick_sequence_all_success
- tick_sequence_first_failure
- tick_sequence_resumes_running
- tick_selector_first_success
- tick_selector_all_failure
- tick_selector_resumes_running
- tick_parallel_require_all_success
- tick_parallel_require_all_one_failure
- tick_parallel_require_one_success
- tick_parallel_require_n
- tick_decorator_inverter
- tick_decorator_repeat
- tick_decorator_retry
- tick_decorator_cooldown
- tick_decorator_guard_pass
- tick_decorator_guard_fail
- tick_decorator_until_success
- tick_decorator_until_fail
- tick_decorator_timeout
- tick_decorator_force_success
- tick_decorator_force_failure
- tick_wait_counts_ticks
- tick_action_delegates
- tick_condition_true
- tick_condition_false
- tick_random_selector_persists_running
- tick_weighted_selector_respects_weights

# builder.rs
- builder_simple_sequence
- builder_nested_composites
- builder_with_decorator
- builder_weighted_selector

# tree.rs
- tree_tick_increments_counter
- tree_reset_clears_state
- tree_reset_all_clears_blackboard

# utility/curve.rs
- curve_linear_identity
- curve_linear_inverted
- curve_polynomial_quadratic
- curve_polynomial_sqrt
- curve_logistic_midpoint
- curve_step_below
- curve_step_above
- curve_inverse
- curve_constant
- curve_custom_points
- curve_clamp_output

# utility/consideration.rs
- consideration_reads_blackboard
- consideration_missing_key
- consideration_normalizes_input

# utility/action.rs
- utility_action_geometric_mean
- utility_action_zero_vetoes
- utility_action_momentum_bonus
- utility_action_empty_considerations

# utility/reasoner.rs
- reasoner_highest_score
- reasoner_top_n
- reasoner_weighted_random_distribution

# preset.rs
- preset_patrol_loops
- preset_combat_flees_when_low
- preset_guard_returns_to_post

# integration
- integration_patrol_10_ticks
- integration_combat_scenario
- integration_utility_selector_picks_best
```

---

## Phase 9: WASM Demo

The demo should have 4 tabs, each showcasing a different feature.

#### Tab 1: BT Visualizer
- Display a behavior tree as a visual tree graph (nodes and edges)
- Color nodes by their last tick status: green=Success, red=Failure, yellow=Running, grey=not ticked
- Tick the tree on a timer (adjustable speed) or step manually
- Show the blackboard state in a side panel
- Use a preset tree (combat_melee) with simulated conditions

#### Tab 2: Utility Curves
- Display all ResponseCurve types as interactive graphs
- Click and drag control points to adjust curve parameters
- Show the output score in real-time as you move the mouse over the x-axis
- Side-by-side comparison of multiple curves

#### Tab 3: AI Sandbox
- Small grid world (20x20) rendered as tiles
- Place agents with different preset BTs (patrol, combat, guard)
- Click to place waypoints and enemies
- Watch agents make decisions in real-time
- Show each agent's current BT state and blackboard

#### Tab 4: Parallel Comparison
- Same scenario running with 4 different BT configurations
- Side-by-side grid worlds
- Demonstrates how different tree structures produce different behaviors
- E.g., aggressive vs defensive vs patrol vs random

**WASM bindings pattern** (in `demo-wasm/src/lib.rs`):
- Export a `Simulation` struct wrapping BehaviorTrees and grid world
- Export `tick()`, `render()`, `add_agent()`, `set_waypoint()` functions
- Use `web-sys` for canvas 2D rendering

**JS pattern** (in `demo-wasm/www/main.js`):
- ES6 module importing the wasm package
- `requestAnimationFrame` loop calling wasm `tick()` + JS `render()`
- Tab switching hides/shows relevant controls
- Canvas 2D rendering for grid world and tree visualization

---

## Algorithm References

### Behavior Tree Tick Semantics

```text
Sequence:
    Resume from last Running child (or start from first)
    For each child from resume point:
        status = tick(child)
        if Running: save index, return Running
        if Failure: reset, return Failure
    All succeeded: reset, return Success

Selector:
    Resume from last Running child (or start from first)
    For each child from resume point:
        status = tick(child)
        if Running: save index, return Running
        if Success: reset, return Success
    All failed: reset, return Failure

Parallel:
    Tick ALL children (do not short-circuit)
    Count successes and failures
    Apply policy:
        RequireAll: fail on any failure, succeed when all succeed
        RequireOne: succeed on any success, fail when all fail
        RequireN(n): succeed when n succeed, fail when impossible to reach n
```

### Utility AI Scoring

```text
For each UtilityAction:
    1. For each Consideration:
        a. Read blackboard[input_key] as float
        b. Normalize: normalized = (raw - input_min) / (input_max - input_min)
        c. Clamp normalized to [0, 1]
        d. Apply response curve: score = curve.evaluate(normalized)
        e. Multiply by weight: weighted = score * consideration.weight

    2. Compute geometric mean of all weighted scores:
        product = score_1 * score_2 * ... * score_n
        geo_mean = product ^ (1/n)

    3. Apply action weight:
        final = geo_mean * action.weight

    4. Add momentum if this was the previous action:
        final += momentum (if is_current)

Selection:
    HighestScore: argmax(scores)
    WeightedRandom: random proportional to scores
    TopN: uniform random among top N scores
```

### Geometric Mean vs Arithmetic Mean

```text
Why geometric mean?

Arithmetic mean: (0.9 + 0.0) / 2 = 0.45
    Problem: the zero doesn't kill the score enough.
    An agent might attack even with 0 ammo (ammo score = 0).

Geometric mean: (0.9 * 0.0) ^ 0.5 = 0.0
    A single zero vetoes the entire action.
    Any "must have" condition at 0 makes the action score 0.

This is the standard approach in utility AI literature (Dave Mark's
"Behavioral Mathematics for Game AI").
```

### Momentum (Hysteresis)

```text
Without momentum:
    Tick 1: Attack scores 0.51, Flee scores 0.49 → Attack
    Tick 2: Attack scores 0.49, Flee scores 0.51 → Flee
    Tick 3: Attack scores 0.51, Flee scores 0.49 → Attack
    Result: agent thrashes between actions every tick

With momentum = 0.1:
    Tick 1: Attack=0.51, Flee=0.49 → Attack (Attack is current)
    Tick 2: Attack=0.49+0.1=0.59, Flee=0.51 → Attack (momentum keeps it)
    Tick 3: Attack=0.49+0.1=0.59, Flee=0.51 → Attack (stable)
    Until Flee scores significantly higher (>0.59), agent stays with Attack
```

---

## Verification Checklist

```bash
# In cogwise/
cargo test --target x86_64-pc-windows-msvc
cargo build --target wasm32-unknown-unknown --release
cargo clippy --target x86_64-pc-windows-msvc

# WASM demo
cd demo-wasm
wasm-pack build --target web --release
# Serve demo-wasm/www/ with a local HTTP server and test in browser
```
