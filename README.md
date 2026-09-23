# My-Solana-Program: a Counter on Solana (Rust + Anchor)

A small but complete Solana program, written in **Rust** with the **Anchor**
framework, that I wrote, built, deployed to **Devnet** and executed myself:

```
   WRITE  ─────►  BUILD  ─────►  DEPLOY  ─────►  EXECUTE
  lib.rs        anchor build   anchor deploy    devnet client
 (Rust code)   (.so + IDL)    (upload to chain) (send transactions)
```

It does two jobs at once:

1. **Shows how a Solana program lives and runs on a blockchain** (accounts,
   instructions, transactions, fees, rent, signers...).
2. **Shows the basics of the Rust language** (variables, `if`, loops, `match`,
   structs, enums, `impl`, traits, generics, `Option`/`Result`, ownership,
   closures, iterators, tests...) inside a real program, in one file:
   [`programs/counter/src/lib.rs`](programs/counter/src/lib.rs).

The README is written for someone who is **new to blockchain**. If a word looks
scary, it is explained in the [Glossary](#18-glossary).

## Table of contents

1. [The 5-minute mental model](#1-the-5-minute-mental-model)
2. [Blockchain concepts from zero](#2-blockchain-concepts-from-zero)
3. [How Solana is different](#3-how-solana-is-different-pow-pos-poh)
4. [The Solana account model](#4-the-solana-account-model)
5. [Programs, instructions and transactions](#5-programs-instructions-and-transactions)
6. [Fees, compute units and rent](#6-fees-compute-units-and-rent)
7. [What Anchor adds](#7-what-anchor-adds-and-why-we-use-it)
8. [The toolchain and how the parts work together](#8-the-toolchain-and-how-the-parts-work-together)
9. [Project layout: every file explained](#9-project-layout-every-file-explained)
10. [Walkthrough of `lib.rs`](#10-walkthrough-of-librs)
11. [Rust basics cheat sheet (from this code)](#11-rust-basics-cheat-sheet-from-this-code)
12. [Solana quirks cheat sheet](#12-solana-quirks-cheat-sheet)
13. [Setup (Windows + WSL)](#13-setup-windows--wsl)
14. [Write → Build → Deploy → Execute, step by step](#14-write--build--deploy--execute-step-by-step)
15. [My Devnet deployment record](#15-my-devnet-deployment-record)
16. [Preview: PDAs and ATAs (next topic)](#16-preview-pdas-and-atas-next-topic)
17. [Troubleshooting](#17-troubleshooting)
18. [Glossary](#18-glossary)
19. [Further reading](#19-further-reading)

---

## 1. The 5-minute mental model

Think of Solana as **one giant shared computer** that thousands of independent
machines (validators) run together and always agree about.

| Everyday idea | Solana equivalent |
|---|---|
| A shared spreadsheet everyone can read, nobody can secretly edit | The **ledger** (blockchain) |
| Each row of the spreadsheet | An **account** (holds SOL and/or data) |
| A cell you can only change by following rules | Data owned by a **program** |
| The rules themselves (code) | A **program** (a special executable account) |
| Filling in a form to ask for a change | A **transaction** containing **instructions** |
| Your signature on the form | A cryptographic **signature** by your **keypair** |
| A processing fee | **Transaction fee** (paid in SOL) |
| A deposit that keeps your row from being deleted | **Rent** (a refundable, rent-exempt balance) |

In this repo:

- The **program** is `counter`: rules for a counter (add, subtract, pause,
  reset, label, close...).
- Each user creates a **counter account** that stores the number and some extra
  fields. Only the person who created it (its `authority`) can change it.
- To change the counter you send a **transaction** with an **instruction** such
  as `increment(5)`. Validators run the program's Rust code, and if nothing
  fails, the new state is saved on-chain.

---

## 2. Blockchain concepts from zero

### 2.1 What is a blockchain?

A blockchain is a **database that is copied on many computers** and updated by
rules that everybody follows, so no single company or person controls it.
Records are grouped into **blocks**; each block contains a fingerprint (hash) of
the previous one, forming a **chain**. Changing an old record would change its
fingerprint and break every later block, so history is practically tamper-proof.

### 2.2 Nodes and validators

A **node** is a computer running the blockchain software. On Solana, nodes that
take part in agreeing on new blocks are called **validators**. They stake SOL
as collateral; behaving honestly earns rewards.

### 2.3 Keys, wallets and addresses

- A **keypair** = a **private key** (secret) + a **public key** (shareable).
- The **public key** is your **address**, e.g. `5PgQWusb...vipP4`
  (base58 text, 32 bytes underneath). On Solana it is called a `Pubkey`.
- The **private key** lets you **sign**. A signature proves "the owner of this
  address approved this", without revealing the private key.
- A **wallet** is software (or a file such as `~/.config/solana/id.json`) that
  stores keypairs and signs for you. **Never share or commit a private key.**

### 2.4 Tokens, SOL and lamports

**SOL** is Solana's native coin. It pays fees and rent. The smallest unit is the
**lamport**: `1 SOL = 1,000,000,000 lamports`. On **Devnet**, SOL is free and
worthless (it is for testing).

### 2.5 Transactions

A **transaction** is a signed message that says "run these instructions". It is
**atomic**: either every instruction succeeds or none of their changes are kept.

### 2.6 Clusters (networks)

| Cluster | Purpose | SOL value |
|---|---|---|
| `localnet` | A validator running on your own PC | free |
| **`devnet`** | Public playground for developers (**we use this**) | free (faucet) |
| `testnet` | Testing validator/network upgrades | free |
| `mainnet-beta` | The real network | real money |

### 2.7 Slots, epochs and commitment

- **Slot**: a time window (~400 ms) in which one validator may produce a block.
- **Epoch**: a group of 432,000 slots (~2 days); staking and leader schedules
  update per epoch.
- **Commitment levels** describe how sure you are a transaction is final:
  `processed` (seen by one node) < `confirmed` (voted on by a supermajority) <
  `finalized` (cannot be rolled back). We use `confirmed`.

### 2.8 Block explorers

A website that lets you look up any address or transaction, e.g.
[Solana Explorer](https://explorer.solana.com/?cluster=devnet) or
Solscan (select "Devnet"). You will use it to see your deployed program and your
transactions.

---

## 3. How Solana is different: PoW, PoS, PoH

A blockchain needs a way for strangers to agree on "what is the next block?"
This is **consensus**.

| Mechanism | Idea | Used by |
|---|---|---|
| **Proof of Work (PoW)** | Miners race to solve a hard puzzle (hashing). Winner adds the block. Security comes from burning electricity. Slow and energy-hungry. | Bitcoin |
| **Proof of Stake (PoS)** | Validators lock up ("stake") coins. They are chosen to propose/vote on blocks in proportion to their stake, and lose stake if they cheat. | Ethereum, **Solana** |
| **Proof of History (PoH)** | A **cryptographic clock**: a chain of SHA-256 hashes where each output feeds the next. Producing it takes real time, but *checking* it is fast, so it proves "this event happened before that one" without asking everyone. | **Solana only** |

Solana's consensus = **PoS (Tower BFT voting)** + **PoH as a clock** that lets
validators agree on ordering and time without lots of messaging. Together with
parallel transaction execution ("Sealevel"), this is why Solana can be fast and
cheap. (PoH is *not* a consensus by itself; it is what makes voting efficient.)

---

## 4. The Solana account model

On Solana **everything is an account**: your wallet, a token balance, a program,
the data your program stores. Every account has the same shape:

```
┌──────────────────────── Account ─────────────────────────┐
│ address     (Pubkey)  where to find it                   │
│ lamports    (u64)     its SOL balance                    │
│ data        (bytes)   arbitrary content (up to 10 MiB)   │
│ owner       (Pubkey)  the PROGRAM allowed to change data │
│ executable  (bool)    true = this account is a program   │
│ rent_epoch  (u64)     legacy rent bookkeeping            │
└──────────────────────────────────────────────────────────┘
```

Key rules:

- **Only the owner program may change an account's `data`** or subtract its
  lamports. Anyone may *add* lamports to any account.
- **Programs are stateless.** A program's code cannot store variables between
  calls. State lives in **separate data accounts** which are passed *into* each
  call. (This is the biggest difference from Ethereum-style contracts.)
- A **wallet** (like yours) is an account owned by the **System Program**.
- Our `Counter` data account is **owned by our `counter` program**, so only our
  code can modify it.

```
   Your wallet account                     Counter account (data)
   owner: System Program                   owner: counter program  ◄─ only our code writes here
   lamports: 2.5 SOL                       lamports: ~0.00135 SOL (rent-exempt)
                                           data: [discriminator | authority | count | status ...]
                                                     ▲
   counter program account (executable) ─────────────┘  reads/writes it when called
```

**Discriminator:** Anchor prefixes every account's data with 8 bytes, the first
8 bytes of `sha256("account:Counter")`. It stops you passing the wrong kind of
account to an instruction. That is the `8 +` in `space = 8 + Counter::INIT_SPACE`.

---

## 5. Programs, instructions and transactions

```
Transaction  (signed by one or more keypairs, max ~1232 bytes)
 ├─ Instruction 1: program_id + [account list] + instruction data
 ├─ Instruction 2: ...
 └─ Signatures
```

- **Program**: deployed code (compiled Rust → sBPF bytecode) that runs inside
  the validators' VM.
- **Instruction**: "call program X, give it these accounts, with this data".
  For us: `increment`, `decrement`, `batch_increment`, ...
- **Instruction data** is bytes: 8-byte *instruction discriminator*
  (`sha256("global:increment")[..8]`) followed by the encoded arguments.
- **Accounts list**: every account an instruction will touch must be listed
  up front, marked **writable** or read-only and **signer** or not. This lets
  the runtime run non-overlapping transactions **in parallel**.
- **Signers**: accounts that signed the transaction. `Signer<'info>` in Anchor
  guarantees the key really signed.
- **CPI (Cross-Program Invocation)**: a program calling another program (for
  example calling the System Program to create an account). Anchor's `init`
  does this for us.

### What happens when you call `increment(5)`

```
 1. Client builds an instruction: program=counter, accounts=[counter, authority],
    data=[disc("increment"), 5]
 2. Client wraps it in a transaction, sets a recent blockhash, SIGNS with your key
 3. Client sends it to a Devnet RPC node (HTTPS)
 4. The node forwards it to the current leader validator
 5. The runtime loads the accounts, then runs our .so program in the VM
 6. Anchor code: checks discriminator, owner, signer, `has_one` -> runs our function
 7. Our Rust: checks status, does checked_add, updates fields, emits an event, logs
 8. Success -> account changes are committed; fee is charged; other validators vote
 9. Client sees "confirmed"; anyone can now read the new count
    (If ANY step returns Err, all changes are discarded; only the fee is paid)
```

---

## 6. Fees, compute units and rent

| Cost | What it is | Roughly |
|---|---|---|
| **Base fee** | Per signature on the transaction | 5,000 lamports (0.000005 SOL) |
| **Priority fee** | Optional tip to be processed sooner | optional |
| **Compute units (CU)** | Metering of CPU work. Default budget 200,000 CU per instruction (max 1.4M per transaction). Exceed it and the transaction fails. | free, but limited |
| **Rent-exempt deposit** | Lamports an account must hold to exist forever. Depends on its size. Refundable when the account is closed. | `(128 + bytes) × rate` lamports; the rate is set by the network (5,080 lamports/byte on Devnet when I measured it; it has changed over time). Ask the network: `solana rent <bytes>` |
| **Program deploy cost** | Rent for the program's bytes (+ a temporary buffer). Refundable by closing the program. | a few SOL on Devnet |

Our counter account is `8 + 129 = 137` bytes →
`(128 + 137) × 5,080 = 1,346,200` lamports ≈ **0.00135 SOL** (confirm with
`solana rent 137`). The `initialize`
instruction prints this number to the log using the `Rent` sysvar.

Other hard limits worth knowing: a transaction is at most 1,232 bytes; an
account at most 10 MiB; the program stack frame is 4 KB and heap 32 KB; call
depth (CPI) at most 4.

---

## 7. What Anchor adds (and why we use it)

Raw Solana programs must manually deserialize accounts, check owners/signers,
and encode data, and forgetting a check is a classic source of hacks. **Anchor**
is a framework that generates that boilerplate from macros:

| Anchor piece | What it does for us |
|---|---|
| `declare_id!("...")` | Fixes the program's address in the code |
| `#[program]` | Marks the module whose `pub fn`s are instructions; generates the router that reads the 8-byte instruction discriminator and calls the right function |
| `#[derive(Accounts)]` | Describes and **validates** the accounts an instruction needs |
| `#[account(...)]` constraints | `init`, `mut`, `has_one`, `close`, `space`, `payer`: checks + actions declared instead of hand-written |
| `Signer<'info>` | Fails unless that account signed |
| `Account<'info, T>` | Checks owner = our program + discriminator, then deserializes `T` |
| `Program<'info, System>` | Checks the account really is the System Program |
| `#[account] struct` | Turns a struct into an on-chain account type |
| `#[derive(InitSpace)]` | Computes a struct's size so we do not count bytes by hand |
| `#[error_code]` | Custom errors with numbers (6000+) and messages |
| `#[event]` + `emit!` | Structured log data for off-chain apps |
| **IDL** (`target/idl/counter.json`) | A JSON "manual" of the program (instructions, accounts, types, errors) that clients use to build transactions |

---

## 8. The toolchain and how the parts work together

| Tool | What it is | Role here |
|---|---|---|
| **Rust** (`rustc`, `cargo`) | The language, compiler and package manager | Compiles the code; `rust-toolchain.toml` pins version 1.89.0 |
| **Solana CLI** (`solana`, `solana-keygen`) | Command line for the network | Wallet, choose cluster, airdrop, inspect accounts, deploy |
| **Platform tools** (`cargo-build-sbf`) | Solana's own Rust compiler build targeting **sBPF** | Turns your Rust into the `.so` that runs on-chain |
| **Anchor CLI** (`anchor`) | Framework CLI | `anchor build` / `deploy` / `keys` wrap the tools above; generates the IDL |
| **`anchor-lang`** (crate) | The Rust library with the macros | Used inside `lib.rs` |
| **WSL** | Linux inside Windows | Where all commands run (Solana tooling targets Linux/macOS) |
| **Devnet RPC** | `https://api.devnet.solana.com` | The door through which the CLI/client reach the network |

How they connect:

```
            ┌────────────────────────────  YOUR PC (WSL)  ───────────────────────────┐
            │                                                                         │
 lib.rs ──► │ anchor build ─► cargo-build-sbf ─► target/deploy/counter.so  (program)  │
 (source)   │                    └─► anchor-lang macros ─► target/idl/counter.json    │
            │                                                     │                   │
            │ solana-keygen ─► ~/.config/solana/id.json (wallet)  │ (describes API)   │
            │                                                     ▼                   │
            │ anchor deploy ──────────────┐                 client (Rust example)     │
            └─────────────────────────────┼─────────────────────┬───────────────────┘
                                          │  JSON-RPC over HTTPS│
                                          ▼                     ▼
                            ┌──────────── Solana Devnet ────────────────┐
                            │ counter program account (executable)      │
                            │ Counter data accounts (one per user)      │
                            │ System Program, Clock/Rent sysvars        │
                            └───────────────────────────────────────────┘
```

The program **address** is decided by `target/deploy/counter-keypair.json`.
`declare_id!` in `lib.rs` and `[programs.*]` in `Anchor.toml` must contain that
same public key (`anchor keys sync` / `anchor keys list` manage this).

---

## 9. Project layout: every file explained

```
My-Solana-Program/
├── Anchor.toml                  Anchor settings (cluster, wallet, program IDs, scripts)
├── Cargo.toml                   Rust "workspace" root + release build settings
├── Cargo.lock                   Exact dependency versions (commit it)
├── rust-toolchain.toml          Pins the Rust version for reproducible builds
├── .gitignore / .prettierignore Files git / prettier should skip
├── programs/
│   └── counter/
│       ├── Cargo.toml           This program's crate: name, features, dependencies
│       ├── src/lib.rs           ★ THE PROGRAM (all the Rust lives here)
│       ├── tests/               Rust integration tests that run the compiled program in-process
│       └── examples/            Devnet client: sends real transactions to the deployed program
└── target/                      BUILD OUTPUT (git-ignored, recreated by `anchor build`)
    ├── deploy/counter.so        the compiled program (what is uploaded to the chain)
    ├── deploy/counter-keypair.json   the program's address keypair (keep it!)
    ├── idl/counter.json         the IDL (machine-readable API description)
    └── types/counter.ts         TypeScript types generated from the IDL
```

**Why `target/` exists.** It is Cargo's output folder. Compiling produces
files that are not source code, and deploying needs them: the `.so` bytecode is
literally what you upload, and `counter-keypair.json` fixes the program's
address. It is regenerated on every build, so it is **git-ignored**, except that
you must **back up `counter-keypair.json`** if you want to keep the same
program address (or the ability to upgrade that deployment).

**Why there is no `app/` folder.** `anchor init` normally creates an empty
`app/` directory as a placeholder for a web frontend. This project has no
frontend (the Rust client in `examples/` plays that role), so it was deleted.
You only need `app/` if you build a website or mobile UI that talks to the
program.

**Why `Cargo.toml` sets `overflow-checks = true`.** By default Rust *removes*
integer-overflow checks in release builds (numbers silently wrap around), and
Solana programs are always release builds. The scaffold turns them back on so
plain `+` panics instead of wrapping. We still use `checked_add` everywhere
important because it lets us return a *clear custom error* instead of a panic.

---

## 10. Walkthrough of `lib.rs`

The file reads top to bottom in this order:

| Section | Contents |
|---|---|
| Imports and `declare_id!` | Pull in Anchor; pin the program address |
| Constants | `MAX_COUNT`, `MAX_BATCH`, `MAX_LABEL_BYTES`, `MAX_COLLATZ_STEPS`, `HISTORY_LEN` |
| `#[program] mod counter` | The 10 instructions |
| Helper functions | `count_digits`, `normalize`, `largest`, `collatz_steps` |
| `#[derive(Accounts)]` structs | `Initialize`, `Update`, `CloseCounter`, `Inspect` |
| Data types | `Counter` struct (+ `impl`), `Status` enum (+ `Display`), `CounterChanged` event |
| Errors | `CounterError` enum |
| Unit tests | `#[cfg(test)] mod tests` |

### 10.1 The data stored on-chain

```rust
pub struct Counter {
    pub authority: Pubkey,            // 32 bytes  who may change it
    pub count: u64,                   //  8 bytes  the number
    pub status: Status,               //  1 byte   Active | Paused
    pub operations: u32,              //  4 bytes  successful updates so far
    pub last_updated: i64,            //  8 bytes  unix time from the Clock
    pub history: [u64; 5],            // 40 bytes  previous values, newest first
    pub label: String,                //  4 + up to 32 bytes
}                                     // = 129 bytes (+ 8 discriminator = 137)
```

### 10.2 The instructions

| Instruction | Arguments | Who can call | What it demonstrates |
|---|---|---|---|
| `initialize` | `start: u64` | anyone (becomes authority) | Creating an account (`init`), sysvars, `require!` |
| `increment` | `amount: u64` | authority | `if / else if`, `checked_add`, `?`, error returns |
| `decrement` | `amount: u64` | authority | `match` on `Option`, underflow protection |
| `batch_increment` | `times: u8, step: u64` | authority | `for`, `continue`, `break`, match guards, bounded loops |
| `toggle_pause` | none | authority | enum + exhaustive `match`, `Display` |
| `reset` | none | authority | Reusing the shared `record` method |
| `set_label` | `label: String` | authority | Ownership, borrowing, shadowing, byte limits |
| `transfer_authority` | `new_authority: Pubkey` | authority | `require_keys_neq!`, changing ownership of control |
| `inspect` | none | anyone | Read-only tour: ranges, closures, iterators, generics, `loop` |
| `close_counter` | none | authority | `close` constraint: delete account, refund rent |

Every state-changing instruction funnels through `Counter::record`, which
rotates the `history` array, updates `operations` and `last_updated`, and emits
a `CounterChanged` event.

### 10.3 Access control

`#[account(mut, has_one = authority @ CounterError::Unauthorized)]` means:
"the `authority` account passed in must equal the `authority` stored inside the
counter", and `Signer<'info>` means "and it must have signed". Together: **only
the creator can modify their counter.** Anyone else gets error `Unauthorized`.
This is the single most important pattern in Solana security: *always verify
who is allowed to do what*, because the caller chooses every account they pass.

---

## 11. Rust basics cheat sheet (from this code)

Each item below is marked `RUST BASIC n` in `lib.rs`.

| # | Concept | Example in the code | Explanation |
|---|---|---|---|
| 1 | **Constants** | `pub const MAX_COUNT: u64 = 1_000_000;` | Compile-time values with explicit types. `_` is a readability separator. |
| 2 | **`Result` and `?`** | `-> Result<()>`, `Ok(())`, `...?` | Functions that can fail return `Result`. `?` returns the error early, otherwise unwraps the value. There are no exceptions in Rust. |
| 3 | **`if / else if / else`** | in `increment` | Conditions must be `bool` (no "truthy" numbers). `if` is also an *expression* that produces a value: `let parity = if n % 2 == 0 { "even" } else { "odd" };` |
| 4 | **Loops** | `for i in 0..times`, `while n >= 10`, `loop { break value }` | `0..5` is 0,1,2,3,4 (end excluded); `0..=5` includes 5. `continue` skips, `break` exits. `loop` runs until `break`, and `break x` returns `x`. |
| 5 | **Enums and `match`** | `enum Status { Active, Paused }` | An enum is "one of several variants". `match` must cover **every** variant, so you cannot forget a case. |
| 6 | **Arrays, iterators, closures** | `milestones.iter().filter(\|&&m\| count >= m).count()` | Arrays have a fixed size `[T; N]`. Iterators chain operations lazily. `\|x\| ...` is a closure (anonymous function). |
| 7 | **Tuples, `if let`** | `let (ops, status) = (...)`, `if let Some(n) = next` | Group values; `if let` handles one pattern without a full `match`. |
| 8 | **Structs** | `pub struct Counter { ... }` | Named bundles of fields. `#[derive(...)]` auto-generates trait impls. |
| 9 | **Ownership and borrowing** | `normalize(&label)` | Each value has one owner. `&x` lends read-only, `&mut x` lends for changing; the compiler enforces this, preventing whole classes of bugs. Assigning a `String` to another variable **moves** it. |
| 10 | **Shadowing** | `let label = cleaned;` | A new `let` with the same name hides the old variable (can even change type). Different from `mut`, which changes the same variable. |
| 11 | **Generics** | `fn largest<T: PartialOrd + Copy>(items: &[T]) -> Option<T>` | One function for many types, with *trait bounds* saying what `T` must support. |
| 12 | **Loop values** | `break Some(steps)` in `collatz_steps` | See #4. |
| 13 | **`impl` blocks** | `impl Counter { fn is_active(&self) ... }` | Attach methods to a type. `&self` reads, `&mut self` modifies. |
| 14 | **Traits** | `impl fmt::Display for Status` | A trait is a shared capability (like an interface). Implementing `Display` makes `{}` work for your type. |
| 15 | **Unit tests** | `#[cfg(test)] mod tests` | Run with `cargo test --lib`; not included in the on-chain program. |

Other Rust features you will notice:

- **Immutable by default**: `let x = 5;` cannot change; write `let mut x = 5;`.
- **No `null`**: absence is `Option<T>` = `Some(value)` or `None`; the compiler
  makes you handle both.
- **Last expression = return value** when there is no trailing `;`.
- **Macros end in `!`**: `msg!`, `require!`, `err!`, `emit!`, `format!`, `matches!`.
- **Numeric types are explicit**: `u8`, `u32`, `u64`, `i64` (unsigned/signed,
  bit width). `as` converts between them; mixing types needs a cast.
- **Strings**: `String` (owned, growable) vs `&str` (borrowed view).
  `.len()` is **bytes**, not characters.
- **Attributes** `#[...]` add behaviour to the item below them.
- **Lifetimes** `'info` say "these references live as long as the transaction's
  accounts do". You mostly just copy them in Anchor code.

---

## 12. Solana quirks cheat sheet

Each item is marked `SOLANA QUIRK n` in `lib.rs`.

| # | Quirk | Why it matters |
|---|---|---|
| 1 | **Use checked arithmetic** (`checked_add`, `checked_sub`) | Integer overflow silently wraps in normal Rust release builds. An attacker who can overflow a balance can mint themselves money. Always handle `None`. |
| 2 | **Compute budget limits loops** | Each transaction gets limited compute units. Never loop over unbounded/user-controlled sizes; cap it (`MAX_BATCH`) and keep logging small. |
| 3 | **No wall clock, use sysvars** | Validators must all compute identical results, so `SystemTime::now()` is not allowed. Use `Clock::get()` (slot, unix timestamp) and `Rent::get()`. No randomness either without an oracle. |
| 4 | **Account size is fixed at creation** | A `String` needs a declared max (`#[max_len(32)]`). Space is paid for up front as rent. `.len()` counts bytes. |
| 5 | **Closing accounts** | To delete data you `close` the account so its rent lamports return to you. Merely zeroing fields leaves it alive. |
| 6 | **Events** | `emit!` writes to the transaction log so off-chain apps can index activity without polling accounts. |
| 7 | **Caller supplies accounts, so validate all of them** | `has_one`, `Signer`, owner + discriminator checks (Anchor does most automatically). |
| 8 | **Programs are stateless** | State lives in accounts that you pass in. |
| 9 | **All-or-nothing transactions** | An error in any instruction reverts everything (you still pay the fee). |
| 10 | **Upgradeable by default** | A deployed program can be replaced by whoever holds the *upgrade authority* (your wallet). Real projects later revoke or multisig it. |
| 11 | **Logs are cheap but not free** | `msg!` consumes compute units; heavy formatting (`format!`) costs more. |
| 12 | **Fixed 8-byte discriminators** | Instruction and account types are identified by hash prefixes, so names matter (renaming an instruction changes its discriminator). |

---

## 13. Setup (Windows + WSL)

Everything runs inside **WSL** (Windows Subsystem for Linux). Tools used:
Rust 1.89 (pinned by `rust-toolchain.toml`; the machine also has `rustc 1.98`),
Anchor CLI 1.1.2, Solana CLI 3.1.10.

```bash
rustc --version      # Rust compiler
anchor --version     # Anchor CLI
solana --version     # Solana CLI
```

Open a WSL shell in the project (the Windows folder is visible under `/mnt/c`):

```bash
cd "/mnt/c/Users/HP/Desktop/Coding by George S/Default/My-Solana-Program"
```

> **Performance tip:** builds are much faster on the Linux filesystem
> (`~/...`) than on `/mnt/c/...`. Cargo writes tens of thousands of small files,
> which is very slow across the Windows/WSL boundary. In my case one dependency
> compile took over 14 minutes on `/mnt/c` and about 2 minutes on the WSL disk.
> Clone or copy the repo into your WSL home (e.g. `~/build/counter`) and build
> there; copy only the source back to Windows.

**First `anchor build` downloads ~500 MB** of Solana "platform tools" (the
sBPF-capable Rust compiler) into `~/.cache/solana/`. That is normal and only
happens once.

### Create a Devnet wallet

```bash
solana config set --url devnet             # talk to Devnet
solana-keygen new --no-bip39-passphrase    # creates ~/.config/solana/id.json (skip if it exists)
solana address                             # show your public address
solana airdrop 2                           # get free Devnet SOL
solana balance
```

The CLI faucet is rate-limited. If it fails, use the web faucet
<https://faucet.solana.com> (choose **Devnet**, paste your address).

---

## 14. Write → Build → Deploy → Execute, step by step

### Step 1: WRITE

Edit [`programs/counter/src/lib.rs`](programs/counter/src/lib.rs).

### Step 2: BUILD

```bash
anchor build
```

What happens: `cargo-build-sbf` compiles the crate with the Solana toolchain
into `target/deploy/counter.so`; Anchor's macros also emit the IDL
(`target/idl/counter.json`) and TypeScript types. The first build creates
`target/deploy/counter-keypair.json`, which decides your program address.

Run the pure-Rust unit tests (no blockchain needed) and the in-process
integration tests:

```bash
cargo test --lib                 # unit tests in lib.rs
cargo test                       # + integration tests (needs `anchor build` first)
```

Make sure the program ID in the code matches the keypair:

```bash
anchor keys list                 # shows the ID from target/deploy/counter-keypair.json
anchor keys sync                 # rewrites declare_id! and Anchor.toml if they differ
```

### Step 3: DEPLOY

```bash
solana config set --url devnet
solana program deploy target/deploy/counter.so --program-id target/deploy/counter-keypair.json
# `anchor deploy` is the Anchor wrapper around the same operation (Anchor.toml
# already points at devnet). I deployed with the raw Solana CLI command above.
```

What happens: the `.so` is uploaded in chunks into a temporary *buffer
account*, then moved into a **program account** (`executable = true`) plus a
**program data account**, using the upgradeable loader. Your wallet pays the
rent and becomes the **upgrade authority**. Check it:

```bash
solana program show <PROGRAM_ID>
```

Upgrading later = rebuild and deploy again (same ID). To delete the program and
get its rent back: `solana program close <PROGRAM_ID>`.

### Step 4: EXECUTE

Now call the program on Devnet. The Rust client in
[`programs/counter/examples/devnet_client.rs`](programs/counter/examples/devnet_client.rs)
builds real transactions, signs them with your wallet and sends them:

```bash
cargo run --example devnet_client
```

The client, in order: creates a new counter account, increments it, runs a
batch loop, decrements, sets a label, toggles pause (and shows that
`increment` is rejected while paused), shows that a stranger is rejected
(`Unauthorized`), runs `inspect` and reads the logs, then closes the account and
prints an explorer link for every transaction.

Each transaction's logs show the program's `msg!` output and compute-unit use,
for example `Program log: Incremented by 5 -> 5`. Open any signature in the
[Explorer](https://explorer.solana.com/?cluster=devnet) to see the same.

You can also inspect accounts directly from the CLI:

```bash
solana account <COUNTER_ACCOUNT_ADDRESS>     # raw bytes (starts with the 8-byte discriminator)
solana logs <PROGRAM_ID>                     # stream live logs of program calls
```

---

## 15. My Devnet deployment record

Everything below really happened on Devnet (September 23, 2026) and can be
looked up in the explorer.

| Item | Value |
|---|---|
| Program ID | [`4Se6gs3Hf6jeDfcNpheFvLLcBnvtoh2qHVx4yxy8gRhh`](https://explorer.solana.com/address/4Se6gs3Hf6jeDfcNpheFvLLcBnvtoh2qHVx4yxy8gRhh?cluster=devnet) |
| Program data account | `Cy7MPU1JCg1DYR22yMSDVYyjdt4Y95mKxNpp9PS3jQDA` |
| Upgrade authority (my wallet) | `5PgQWusbALXk1PcrPC1BVuXV87R25Rhryu9HsmJvipP4` |
| Program size | 198,448 bytes (rent-exempt balance ≈ 1.009 SOL) |
| Loader | `BPFLoaderUpgradeab1e11111111111111111111111` (upgradeable) |
| Deploy transaction | [`2owYbyAM...VvY9MCfVVgbv`](https://explorer.solana.com/tx/2owYbyAMCRAfjWmfEbtYWMutTGA9S6mVVQQTbT6KWLc2pVgYfwyytFmL8ZcfcbVU5qhjWHRfPGVvDvY9MCfVVgbv?cluster=devnet) |

Deployed with:

```bash
solana program deploy target/deploy/counter.so --program-id target/deploy/counter-keypair.json
solana program show 4Se6gs3Hf6jeDfcNpheFvLLcBnvtoh2qHVx4yxy8gRhh
```

### Execution log (`cargo run --example devnet_client`)

The counter account for this run was
`5Fo6if3ojhDUHcVV4Qp9oTUfBRCFHAaVfhMutCeoJ3HG`.

| # | Call | Result | Compute units | Transaction |
|---|---|---|---|---|
| 1 | `initialize(5)` | OK, count = 5 | 7,822 | [`3B4vDnQ4...G1qSCc`](https://explorer.solana.com/tx/3B4vDnQ4prpqyi23zocBpu9ezqVFLtniArwC5uCjH8dKspJgX4b3qsTvdeyqFXZLrfvmPEiutAhQb66ok1G9qSCc?cluster=devnet) |
| 2 | `increment(10)` | OK, count = 15 | 3,727 | [`2Coq6yEU...BjWQnq`](https://explorer.solana.com/tx/2Coq6yEUpTbwtq8dtEq2L6rZSpBpXLYKy7UhzktMqaRsvmoWM6jVUxYbkjwF4w2bBTJiRwa2iynu8LWjQKYBaSnq?cluster=devnet) |
| 3 | `batch_increment(6, 2)` | OK, count = 21 (iterations 0, 2, 4 add 2 each) | 3,633 | [`2oMQ8ZkP...g9DN`](https://explorer.solana.com/tx/2oMQ8ZkPfQ8R2bkNVHyS2iT6v19epSLZs2dZBtvE5HfuMBH9mL4eyFbXxWBJgb6GtG4oF8cFgnxJiH7RQ7eFg9DN?cluster=devnet) |
| 4 | `decrement(1000)` | **Rejected**: `Underflow` (6004) | 3,425 | simulated only |
| 5 | `decrement(1)` | OK, count = 20 | 3,716 | [`8dt61E7h...ZhHV`](https://explorer.solana.com/tx/8dt61E7hH5mHmNTsc6SVdkX7m6Wm2CDq16zbjGh6NPbWZcrahNwXg3Z7b8ZGxgo11qLPwjxWTMuMsMoLw7GZhHV?cluster=devnet) |
| 6 | `set_label("  hello   devnet  ")` | OK, stored as `"hello devnet"` | 4,026 | [`3ccYNNmD...UrML5YW5`](https://explorer.solana.com/tx/3ccYNNmD6487tNGeSNni3YAgrwhHqoYdHkM6woqxoi7PJ6hCnDoPACdxLhKLMYsxHF3QoPrajQtRP3SsUrML5YW5?cluster=devnet) |
| 7 | `inspect` | OK (logs the Rust-feature tour) | 4,583 | [`5NKtmDhg...vpK4`](https://explorer.solana.com/tx/5NKtmDhg1WU3zR5uDHLWMaoF9ybwHCU9REhsc3a5QGy3vd91oSKxfPBTWaQEsifrECgdyAPR2yZ5PGgneYyvpkK4?cluster=devnet) |
| 8 | stranger calls `increment` | **Rejected**: `Unauthorized` (6000) | 3,914 | simulated only |
| 9 | `toggle_pause` | OK, status = Paused | 3,229 | [`5J5Xz9n8...H4Gkq`](https://explorer.solana.com/tx/5J5Xz9n8QPXhrb1BiMTCM2mbtsnW9R7dEYtLzYhGhcB1kahinLwm4765SAVH3r9iC13kfGcGHakXX5483HMH4Gkq?cluster=devnet) |
| 10 | `increment` while paused | **Rejected**: `Paused` (6001) | 3,652 | simulated only |
| 11 | `toggle_pause` | OK, status = Active | 3,231 | [`2DxCFSGQ...p43u`](https://explorer.solana.com/tx/2DxCFSGQYnEB6sWEuTcfYfGD9q1cbCspqSJGHUsskHyfDYmV6LQaLBZjnB9KE7ZvbPxehHaePcXHZzBF27hTp43u?cluster=devnet) |
| 12 | `close_counter` | OK, account deleted, rent refunded | 2,509 | [`n8aA1BPs...epXQ`](https://explorer.solana.com/tx/n8aA1BPs3Sqp755RqT2vgNQWusBxeNUQkvv83k8Q3XhZLjWMCvxmbcfqCmfipN8McG6hmPr9hMFRBPdoK1epzXQ?cluster=devnet) |

Things worth noticing in the real logs:

- **Rejected calls** are not sent for real (the client only *simulates* them),
  but the logs show exactly how Anchor reports errors: the error name, number
  (`6000 + index in CounterError`, hex `0x1770` = 6000) and message.
- The `Unauthorized` log prints **Left** (the authority stored in the account)
  and **Right** (the stranger who tried): that is `has_one` doing its job.
- `Program data: ...` lines are the base64-encoded `CounterChanged` **events**
  (`emit!`).
- Every call used only **2,500 to 7,800** compute units out of the 200,000
  budget. `initialize` costs the most because it makes a CPI to the System
  Program to create the account.
- The `initialize` log confirms the rent maths: `137 bytes need 1346200
  lamports to be rent-exempt`.
- Closing refunded the ~0.00135 SOL rent (balance went up by 1,341,200
  lamports = 1,346,200 refund − 5,000 fee).
- Test results: `cargo test` runs 5 unit tests and 4 LiteSVM integration tests;
  all pass.

---

## 16. Preview: PDAs and ATAs (next topic)

This project deliberately uses a **normal keypair-created account** for the
counter so that the first lesson stays simple. Two ideas come next:

**PDA (Program Derived Address):** an address that is *computed*, not generated,
from `seeds + program_id` (`Pubkey::find_program_address`). It has **no private
key**, so only the owning program can "sign" for it (with `invoke_signed`).
Benefits: predictable addresses (one counter per user derivable from their
wallet), and programs can own and control funds/data safely. In Anchor:
`seeds = [b"counter", user.key().as_ref()], bump`.

**ATA (Associated Token Account):** on Solana a *token* balance is not stored in
your wallet; it lives in a separate **token account**. The ATA is the standard,
deterministic token account for a `(wallet, mint)` pair, computed as a PDA of the
Associated Token Program from `[wallet, token_program, mint]`. That is why it is
recommended to understand PDAs first.

A natural exercise: change `Counter` to be a PDA seeded by the user's wallet, so
each wallet has exactly one counter with a known address.

---

## 17. Troubleshooting

| Symptom | Cause / fix |
|---|---|
| `airdrop request failed ... rate limit` | Public faucet limits. Retry later or use <https://faucet.solana.com>. |
| `not a directory: ...platform-tools.../rust/lib` | Platform-tools download was interrupted and left a half-extracted cache. Delete `~/.cache/solana/v1.52` and run `anchor build` again. |
| First `anchor build` seems frozen | It is downloading ~500 MB; be patient (the output may only appear at the end). |
| `Error: Account ... has insufficient funds` on deploy | Deploy needs a few SOL. Airdrop more. |
| `DeclaredProgramIdMismatch` (error 4100) | `declare_id!` differs from the deployed address. Run `anchor keys sync`, rebuild, redeploy. |
| `AccountNotSigner` / `Unauthorized` (6000) | Wrong wallet used, or the counter belongs to another authority. |
| `Paused` (6001) | Call `toggle_pause` first. |
| `custom program error: 0x1771`-style codes | Hex of custom errors: `6000 = 0x1770` (Unauthorized) and up, in the order of `CounterError`. |
| Slow builds on `/mnt/c` | Use the WSL filesystem, see the tip in the setup section. |
| Error about missing `target/deploy/counter.so` in tests | Run `anchor build` before `cargo test`. |
| Changed the code but the chain behaves the same | You must rebuild **and** redeploy (`anchor deploy` upgrades the program). |

---

## 18. Glossary

- **Account**: the basic storage unit on Solana (lamports + data + owner).
- **Address / Pubkey**: an account's 32-byte identifier (base58 text).
- **Airdrop**: free test SOL from a faucet (Devnet/Testnet only).
- **Anchor**: Rust framework that removes Solana boilerplate.
- **Authority**: the account allowed to perform privileged actions.
- **Base58**: the text encoding used for addresses and signatures.
- **Blockhash**: recent block fingerprint included in a transaction to prove
  freshness (expires after ~1-2 minutes).
- **BPF / sBPF**: the bytecode format Solana programs are compiled to.
- **Commitment**: how final a transaction is (`processed`, `confirmed`, `finalized`).
- **Compute unit (CU)**: measure of computation; each transaction has a budget.
- **CPI**: cross-program invocation (one program calling another).
- **Devnet**: public test network with free SOL.
- **Discriminator**: 8-byte hash prefix identifying an account or instruction type.
- **Epoch**: ~2 days worth of slots.
- **Faucet**: service that hands out test SOL.
- **IDL**: JSON description of a program's interface.
- **Instruction**: one call to one program inside a transaction.
- **Keypair**: private + public key.
- **Lamport**: 1 / 1,000,000,000 of a SOL.
- **Mainnet-beta**: the production network.
- **Mint**: the account that defines a token (supply, decimals).
- **PDA**: program derived address (no private key; controlled by a program).
- **ATA**: associated token account (deterministic token account for wallet + mint).
- **PoH / PoS / PoW**: proof of history / stake / work.
- **Program**: an executable account holding code.
- **RPC**: the API (JSON over HTTPS/WebSocket) used to talk to the network.
- **Rent / rent-exempt**: deposit that lets an account live forever.
- **Signer**: an account whose private key signed the transaction.
- **Slot**: ~400 ms time unit in which a block may be produced.
- **SOL**: Solana's native coin.
- **Sysvar**: special accounts exposing network state (Clock, Rent...).
- **System Program**: built-in program that creates accounts and moves SOL.
- **Transaction**: signed, atomic bundle of instructions.
- **Upgrade authority**: the key allowed to replace a deployed program's code.
- **Validator**: a node that verifies transactions and votes on blocks.
- **WSL**: Windows Subsystem for Linux.

---

## 19. Further reading

- Proof of Work: <https://developer.bitcoin.org/devguide/block_chain.html>
- Proof of Stake: <https://solana.com/staking>
- Proof of History: <https://solana.com/news/proof-of-history>
- PDAs: <https://solana.com/docs/core/pda>
- Token accounts (ATA): <https://solana.com/docs/tokens/basics/create-token-account>
- Solana docs: <https://solana.com/docs>
- Anchor docs: <https://www.anchor-lang.com/docs>
- The Rust Book (free): <https://doc.rust-lang.org/book/>
