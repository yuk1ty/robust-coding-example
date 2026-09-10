# AGENTS.md

- Use Always-Valid Domain Model
  - Make invalid states unrepresentable. A domain object must never exist in an invalid state, so no validation of the object itself is needed after construction.
  - Keep fields private and expose them only through getters. Never provide public setters or allow direct field mutation.
  - Validate invariants at construction time. Constructors must return a `Result` (or equivalent) and reject invalid input, so a successfully constructed value is guaranteed to be valid.
  - Use newtypes and value objects instead of raw primitives, so that invalid or mixed-up values are caught at the type level.
  - Enforce business rules inside state-transition methods. Transitions consume the current value and return a new valid instance instead of mutating in place.
  - Push input validation to the boundary of the application (e.g. handlers or adapters); once data crosses into the domain layer, it is already valid.
  - The only acceptable bypass of validation is reconstitution from trusted persisted data. Do not use such a path for anything else.
- Follow functional programming style.
  - Prefer to make data immutable as much as possible, except for optimising the performance. Immutability can sometimes cause performance degradation (e.g. needs more memory usage), so it's best not to use it in every situation.
  - Specify three components: Actions, Calculation, Data (This principle is written in the book "Grokking Simplicity"). Specifically, carefully isolate Actions.
    - Actions: Depend on how many times or when it is run. Also called functions with side-effects, side-effecting functions, impure functions. Examples: Send an email, read from a database, including I/O operations.
    - Calculations: Computations from input to output. Also called pure functions, mathematical functions. Examples: Find the maximum number, check if an email address is valid.
    - Data: Facts about events. Examples: The email address a user gave us, the dollar amount read from a bank’s API.
