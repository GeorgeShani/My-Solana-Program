// =============================================================================
//  counter - a tiny Solana program written with Rust + Anchor
//
//  Goal: show how a Solana program is structured AND show the basics of Rust
//  (constants, structs, enums, impl blocks, traits, generics, match, if/else,
//  loops, Option/Result, closures, iterators, ownership, checked math, tests)
//  in one readable file.
//
//  Read it top to bottom. Every "RUST BASIC" / "SOLANA QUIRK" note is
//  explained in more depth in the README.
// =============================================================================

use anchor_lang::prelude::*;
use std::fmt; // brings the `fmt` module into scope (used by `impl Display`)

// The program's on-chain address. `anchor keys sync` rewrites this for you.
declare_id!("4Se6gs3Hf6jeDfcNpheFvLLcBnvtoh2qHVx4yxy8gRhh");

// ----- RUST BASIC 1: constants ------------------------------------------------
// `const` values are known at compile time and must have an explicit type.
pub const MAX_COUNT: u64 = 1_000_000; // `_` is just a readability separator
pub const MAX_BATCH: u8 = 20; // upper bound for loops (see SOLANA QUIRK 2)
pub const MAX_LABEL_BYTES: usize = 32; // keep in sync with #[max_len(32)] below
pub const MAX_COLLATZ_STEPS: u32 = 200; // bound for the `loop` in collatz_steps
pub const HISTORY_LEN: usize = 5; // size of the fixed array stored on-chain

// =============================================================================
//  THE PROGRAM (the instructions users can call)
// =============================================================================
#[program]
pub mod counter {
    use super::*;

    /// Creates the counter account and stores the starting value.
    pub fn initialize(ctx: Context<Initialize>, start: u64) -> Result<()> {
        // ----- RUST BASIC 2: Result and early return --------------------------
        // `require!` is Anchor's shortcut for "if !condition { return Err(..) }".
        require!(start <= MAX_COUNT, CounterError::TooLarge);

        // ----- SOLANA QUIRK 3: sysvars (Clock, Rent) --------------------------
        // Programs cannot call `SystemTime::now()`: every validator must get
        // the exact same answer. Instead the network exposes shared state
        // ("sysvars") such as the Clock (time + slot) and Rent (storage cost).
        let clock = Clock::get()?;
        let account_bytes = 8 + Counter::INIT_SPACE;
        let rent_needed = Rent::get()?.minimum_balance(account_bytes);
        msg!(
            "slot={} time={} | {} bytes need {} lamports to be rent-exempt",
            clock.slot,
            clock.unix_timestamp,
            account_bytes,
            rent_needed
        );

        // `&mut` = a mutable borrow. We may change the account data because
        // Anchor gave us exclusive access to it.
        let counter = &mut ctx.accounts.counter;
        counter.authority = ctx.accounts.authority.key();
        counter.count = start;
        counter.status = Status::Active;
        counter.operations = 0;
        counter.last_updated = clock.unix_timestamp;
        counter.history = [0; HISTORY_LEN]; // array literal: 5 copies of 0
        counter.label = String::new();

        msg!("Counter created with start value {}", start);
        Ok(())
    }

    /// Adds `amount` to the counter (conditionals + checked math).
    pub fn increment(ctx: Context<Update>, amount: u64) -> Result<()> {
        let counter = &mut ctx.accounts.counter;

        // ----- RUST BASIC 3: if / else if / else ------------------------------
        if !counter.is_active() {
            return err!(CounterError::Paused);
        } else if amount == 0 {
            return err!(CounterError::ZeroAmount);
        }

        // ----- SOLANA QUIRK 1: integer overflow -------------------------------
        // `checked_add` returns `Option<u64>`: `Some(sum)` or `None` on
        // overflow. `.ok_or(..)` turns `None` into an error and `?` returns
        // early if there is one. Never use plain `+` on user input.
        let new_count = counter
            .count
            .checked_add(amount)
            .ok_or(CounterError::Overflow)?;
        require!(new_count <= MAX_COUNT, CounterError::TooLarge);

        counter.record(new_count)?;
        msg!("Incremented by {} -> {}", amount, counter.count);
        Ok(())
    }

    /// Subtracts `amount`; refuses to go below zero.
    pub fn decrement(ctx: Context<Update>, amount: u64) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        require!(counter.is_active(), CounterError::Paused);

        // `match` on an `Option`: the compiler forces us to handle BOTH cases.
        match counter.count.checked_sub(amount) {
            Some(new_count) => {
                counter.record(new_count)?;
                msg!("Decremented by {} -> {}", amount, new_count);
                Ok(())
            }
            None => err!(CounterError::Underflow),
        }
    }

    /// Adds `step` to the counter up to `times` times using a `for` loop.
    pub fn batch_increment(ctx: Context<Update>, times: u8, step: u64) -> Result<()> {
        let counter = &mut ctx.accounts.counter;
        require!(counter.is_active(), CounterError::Paused);

        // ----- SOLANA QUIRK 2: compute budget ---------------------------------
        // Every transaction has a limited "compute unit" budget (200k by
        // default). Unbounded loops could burn all of it, so we cap them.
        require!(times <= MAX_BATCH, CounterError::BatchTooBig);

        // ----- RUST BASIC 4: `for` loops, `break`, `continue` -----------------
        // `0..times` is a Range: 0, 1, 2, ... times-1 (end is exclusive).
        let mut value = counter.count; // a local copy: cheap to modify in the loop
        for i in 0..times {
            if i % 2 == 1 {
                continue; // skip odd iterations
            }
            match value.checked_add(step) {
                Some(v) if v <= MAX_COUNT => value = v, // match guard
                _ => {
                    msg!("Hit the ceiling at iteration {}, stopping early", i);
                    break; // leave the loop entirely
                }
            }
        }

        // Write back once: one history entry + one event for the whole batch.
        counter.record(value)?;
        msg!("Batch finished, counter = {}", counter.count);
        Ok(())
    }

    /// Flips between Active and Paused (enum + `match`).
    pub fn toggle_pause(ctx: Context<Update>) -> Result<()> {
        let counter = &mut ctx.accounts.counter;

        // ----- RUST BASIC 5: enums + exhaustive match -------------------------
        counter.status = match counter.status {
            Status::Active => Status::Paused,
            Status::Paused => Status::Active,
        };

        // `{}` uses our `impl fmt::Display for Status`, `{:?}` would use Debug.
        msg!("Status is now {}", counter.status);
        Ok(())
    }

    /// Sets the counter back to zero.
    pub fn reset(ctx: Context<Update>) -> Result<()> {
        ctx.accounts.counter.record(0)?;
        msg!("Counter reset");
        Ok(())
    }

    /// Stores a text label. Shows Strings, ownership and shadowing.
    pub fn set_label(ctx: Context<Update>, label: String) -> Result<()> {
        // ----- RUST BASIC 9: ownership ----------------------------------------
        // `label` is an owned `String`; it was *moved* into this function.
        // `normalize` only *borrows* it (`&str`), so we still own it afterwards.
        let cleaned = normalize(&label);

        // ----- RUST BASIC 10: shadowing ---------------------------------------
        // A new `let` with the same name creates a NEW variable that hides the
        // old one. Handy for "transform then keep using the same name".
        let label = cleaned; // `label` is now the trimmed version

        // ----- SOLANA QUIRK 4: variable-size data has a fixed budget ----------
        // The account was created with room for at most 32 bytes of text, and
        // its size is fixed. `.len()` counts BYTES, not characters ("é" = 2).
        require!(label.len() <= MAX_LABEL_BYTES, CounterError::LabelTooLong);
        require!(!label.is_empty(), CounterError::EmptyLabel);

        // Moving `label` into the account: from here on `label` is unusable.
        ctx.accounts.counter.label = label;
        msg!("Label set to \"{}\"", ctx.accounts.counter.label);
        Ok(())
    }

    /// Hands control of the counter to another wallet.
    pub fn transfer_authority(ctx: Context<Update>, new_authority: Pubkey) -> Result<()> {
        require_keys_neq!(
            new_authority,
            ctx.accounts.counter.authority,
            CounterError::SameAuthority
        );
        ctx.accounts.counter.authority = new_authority;
        msg!("Authority is now {}", new_authority);
        Ok(())
    }

    /// Deletes the account and refunds its rent to the authority.
    pub fn close_counter(_ctx: Context<CloseCounter>) -> Result<()> {
        // ----- SOLANA QUIRK 5: closing accounts -------------------------------
        // Nothing to write: the `close = authority` constraint on the account
        // moves all lamports to the authority and wipes the data afterwards.
        msg!("Counter closed, rent refunded");
        Ok(())
    }

    /// Read-only tour of more Rust features. It only writes to the log.
    pub fn inspect(ctx: Context<Inspect>) -> Result<()> {
        let counter = &ctx.accounts.counter; // shared (read-only) borrow
        let count = counter.count;

        // `match` with ranges and a `_` catch-all arm.
        let size = match count {
            0 => "zero",
            1..=9 => "single digit",
            10..=999 => "small",
            1_000..=99_999 => "medium",
            _ => "large",
        };

        // `if` used as an expression (both branches return a &str).
        let parity = if count % 2 == 0 { "even" } else { "odd" };

        // Call a normal helper function (defined at the bottom of the file).
        let digits = count_digits(count);

        // ----- RUST BASIC 6: arrays, iterators and closures -------------------
        let milestones = [10u64, 100, 1_000, 10_000];
        let reached = milestones
            .iter() // iterate by reference
            .filter(|&&m| count >= m) // closure: keep milestones already reached
            .count();
        let next = milestones.iter().find(|&&m| m > count); // Option<&u64>

        // ----- RUST BASIC 7: tuples and `if let` ------------------------------
        let (ops, status) = (counter.operations, counter.status);
        if let Some(n) = next {
            msg!("Next milestone: {}", n);
        } else {
            msg!("All milestones reached!");
        }

        // ----- RUST BASIC 11: generics, slices, `sum`, `matches!` -------------
        // `largest` works for any type that can be compared and copied.
        let peak = largest(&counter.history).unwrap_or(0);
        let total: u64 = counter.history.iter().sum();
        let average = total / HISTORY_LEN as u64; // `as` = numeric cast
        let has_history = counter.history.iter().any(|&h| h != 0);
        let paused = matches!(status, Status::Paused); // pattern -> bool

        // ----- RUST BASIC 12: `loop` that returns a value ---------------------
        let collatz = match collatz_steps(count) {
            Some(steps) => format!("{} steps to reach 1", steps),
            None => String::from("undefined / too long"),
        };

        msg!(
            "count={} ({}, {}, {} digits), milestones reached={}, ops={}, status={}",
            count,
            size,
            parity,
            digits,
            reached,
            ops,
            status
        );
        msg!(
            "history peak={} avg={} has_history={} paused={} label=\"{}\" collatz: {}",
            peak,
            average,
            has_history,
            paused,
            counter.label,
            collatz
        );

        // Look at the transaction log: the runtime itself prints
        // "Program ... consumed X of Y compute units" for every call.
        Ok(())
    }
}

// =============================================================================
//  HELPER FUNCTIONS (plain Rust, not instructions)
// =============================================================================

/// Counts decimal digits with a `while` loop.
/// `mut` is required to change a variable: Rust variables are immutable by default.
fn count_digits(mut n: u64) -> u32 {
    let mut digits = 1;
    while n >= 10 {
        n /= 10;
        digits += 1;
    }
    digits // no `;` on the last line = this is the return value
}

/// Borrows a `&str` and returns a new owned `String` (trimmed, single spaces).
fn normalize(raw: &str) -> String {
    raw.split_whitespace().collect::<Vec<&str>>().join(" ")
}

/// Returns the biggest item of a slice, or `None` if it is empty.
/// `<T: PartialOrd + Copy>` = "any type T that can be compared and copied".
fn largest<T: PartialOrd + Copy>(items: &[T]) -> Option<T> {
    let mut iter = items.iter();
    let mut best = *iter.next()?; // `?` also works on Option: None returns None
    for &item in iter {
        if item > best {
            best = item;
        }
    }
    Some(best)
}

/// Collatz sequence length. `loop` runs forever until `break`, and `break`
/// can hand back a value: this whole `loop { .. }` is the function's result.
fn collatz_steps(start: u64) -> Option<u32> {
    if start == 0 {
        return None;
    }
    let mut n = start;
    let mut steps = 0u32;
    loop {
        if n == 1 {
            break Some(steps);
        }
        if steps >= MAX_COLLATZ_STEPS {
            break None; // bounded, see SOLANA QUIRK 2
        }
        n = if n % 2 == 0 {
            n / 2
        } else {
            match n.checked_mul(3).and_then(|x| x.checked_add(1)) {
                Some(next) => next,
                None => break None,
            }
        };
        steps += 1;
    }
}

// =============================================================================
//  ACCOUNTS: what each instruction needs, and how it is validated
// =============================================================================

#[derive(Accounts)]
pub struct Initialize<'info> {
    // `init` = create a brand-new account owned by this program.
    // `space` = 8 bytes (Anchor's type tag) + the size of our struct.
    #[account(init, payer = authority, space = 8 + Counter::INIT_SPACE)]
    pub counter: Account<'info, Counter>,

    // Pays the rent and becomes the owner; must sign the transaction.
    #[account(mut)]
    pub authority: Signer<'info>,

    pub system_program: Program<'info, System>,
}

#[derive(Accounts)]
pub struct Update<'info> {
    // `has_one = authority` checks counter.authority == authority.key().
    #[account(mut, has_one = authority @ CounterError::Unauthorized)]
    pub counter: Account<'info, Counter>,

    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct CloseCounter<'info> {
    // `close = authority` refunds the lamports to `authority` after the call.
    #[account(mut, has_one = authority @ CounterError::Unauthorized, close = authority)]
    pub counter: Account<'info, Counter>,

    #[account(mut)]
    pub authority: Signer<'info>,
}

#[derive(Accounts)]
pub struct Inspect<'info> {
    pub counter: Account<'info, Counter>,
}

// =============================================================================
//  DATA: what is stored on-chain
// =============================================================================

// ----- RUST BASIC 8: structs -------------------------------------------------
#[account]
#[derive(InitSpace)]
pub struct Counter {
    pub authority: Pubkey,          // 32 bytes - who may change this counter
    pub count: u64,                 // 8 bytes
    pub status: Status,             // 1 byte (enum discriminant)
    pub operations: u32,            // 4 bytes - successful updates so far
    pub last_updated: i64,          // 8 bytes - unix timestamp from the Clock
    pub history: [u64; HISTORY_LEN], // 40 bytes - previous values, newest first
    #[max_len(32)]
    pub label: String,              // 4 (length prefix) + up to 32 bytes
}

// ----- RUST BASIC 13: impl blocks (methods) ----------------------------------
// `&self` = read-only method, `&mut self` = method that changes the struct.
impl Counter {
    fn is_active(&self) -> bool {
        self.status == Status::Active
    }

    /// Stores a new value, pushes the old one into `history`, stamps the time
    /// and emits an event. Every state-changing instruction goes through here.
    fn record(&mut self, new_count: u64) -> Result<()> {
        let clock = Clock::get()?;

        self.history.rotate_right(1); // shift everything one slot to the right
        self.history[0] = self.count; // the previous value becomes newest entry

        let old_count = self.count;
        self.count = new_count;
        self.operations = self
            .operations
            .checked_add(1)
            .ok_or(CounterError::Overflow)?;
        self.last_updated = clock.unix_timestamp;

        // ----- SOLANA QUIRK 6: events ------------------------------------------
        // `emit!` writes structured data into the transaction logs so that
        // off-chain apps (indexers, UIs) can listen without reading accounts.
        emit!(CounterChanged {
            authority: self.authority,
            old_count,
            new_count,
            slot: clock.slot,
        });
        Ok(())
    }
}

// `derive` auto-generates trait implementations (Clone, PartialEq, Debug...).
#[derive(AnchorSerialize, AnchorDeserialize, Clone, Copy, PartialEq, Eq, Debug, InitSpace)]
pub enum Status {
    Active,
    Paused,
}

// ----- RUST BASIC 14: traits -------------------------------------------------
// A trait is a shared behaviour. `Display` is the standard-library trait behind
// the `{}` placeholder; implementing it teaches Rust how to print `Status`.
impl fmt::Display for Status {
    fn fmt(&self, f: &mut fmt::Formatter) -> fmt::Result {
        let text = match self {
            Status::Active => "Active",
            Status::Paused => "Paused",
        };
        write!(f, "{}", text)
    }
}

#[event]
pub struct CounterChanged {
    pub authority: Pubkey,
    pub old_count: u64,
    pub new_count: u64,
    pub slot: u64,
}

// =============================================================================
//  ERRORS
// =============================================================================
#[error_code]
pub enum CounterError {
    #[msg("Only the counter's authority can do this")]
    Unauthorized,
    #[msg("The counter is paused")]
    Paused,
    #[msg("Amount must be greater than zero")]
    ZeroAmount,
    #[msg("Arithmetic overflow")]
    Overflow,
    #[msg("Cannot go below zero")]
    Underflow,
    #[msg("Value exceeds the maximum allowed count")]
    TooLarge,
    #[msg("Batch size exceeds the maximum")]
    BatchTooBig,
    #[msg("Label is longer than 32 bytes")]
    LabelTooLong,
    #[msg("Label cannot be empty")]
    EmptyLabel,
    #[msg("New authority must differ from the current one")]
    SameAuthority,
}

// =============================================================================
//  UNIT TESTS (RUST BASIC 15)
// =============================================================================
// `#[cfg(test)]` code is only compiled by `cargo test`, never into the program.
// Pure functions like these can be tested without any blockchain at all:
//     cargo test --lib
#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn counts_digits() {
        assert_eq!(count_digits(0), 1);
        assert_eq!(count_digits(9), 1);
        assert_eq!(count_digits(10), 2);
        assert_eq!(count_digits(999_999), 6);
    }

    #[test]
    fn normalizes_text() {
        assert_eq!(normalize("  hello    solana  "), "hello solana");
    }

    #[test]
    fn finds_largest() {
        assert_eq!(largest(&[3u64, 9, 4]), Some(9));
        assert_eq!(largest::<u64>(&[]), None);
    }

    #[test]
    fn collatz_lengths() {
        assert_eq!(collatz_steps(0), None);
        assert_eq!(collatz_steps(1), Some(0));
        assert_eq!(collatz_steps(27), Some(111));
    }

    #[test]
    fn status_displays() {
        assert_eq!(Status::Active.to_string(), "Active");
        assert_eq!(format!("{:?}", Status::Paused), "Paused");
    }
}
