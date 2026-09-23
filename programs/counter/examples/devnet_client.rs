// Devnet client: the EXECUTE step of Write -> Build -> Deploy -> Execute.
//
// It talks to the already-deployed program over JSON-RPC, exactly like a web
// app would. For every call it: builds an instruction, wraps it in a
// transaction, signs it with your wallet, simulates it (to print the program
// logs) and then sends it for real.
//
//     cargo run --example devnet_client
//
// Optional env vars: RPC_URL (default Devnet), WALLET (default ~/.config/solana/id.json)

use {
    anchor_lang::{
        prelude::{AccountMeta, Pubkey},
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    solana_commitment_config::CommitmentConfig,
    solana_keypair::{read_keypair_file, Keypair},
    solana_message::{Message, VersionedMessage},
    solana_rpc_client::rpc_client::RpcClient,
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

struct Ctx {
    rpc: RpcClient,
    wallet: Keypair,
}

impl Ctx {
    fn tx(&self, ix: Instruction, extra: &[&Keypair]) -> VersionedTransaction {
        let mut signers: Vec<&dyn Signer> = vec![&self.wallet];
        signers.extend(extra.iter().map(|k| *k as &dyn Signer));
        let blockhash = self.rpc.get_latest_blockhash().expect("blockhash");
        let msg = Message::new_with_blockhash(&[ix], Some(&self.wallet.pubkey()), &blockhash);
        VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers[..]).unwrap()
    }

    /// Simulate (prints program logs), then send for real.
    fn run(&self, title: &str, ix: Instruction, extra: &[&Keypair]) {
        println!("\n=== {title}");
        let tx = self.tx(ix, extra);
        let sim = self.rpc.simulate_transaction(&tx).expect("simulate");
        for line in sim.value.logs.unwrap_or_default() {
            println!("    {line}");
        }
        let sig = self
            .rpc
            .send_and_confirm_transaction(&tx)
            .unwrap_or_else(|e| panic!("transaction failed: {e}"));
        println!("    OK  https://explorer.solana.com/tx/{sig}?cluster=devnet");
    }

    /// Simulate only: used to show calls that are EXPECTED to be rejected.
    fn expect_rejected(&self, title: &str, ix: Instruction, extra: &[&Keypair]) {
        println!("\n=== {title} (expected to be rejected)");
        let tx = self.tx(ix, extra);
        let sim = self.rpc.simulate_transaction(&tx).expect("simulate");
        for line in sim.value.logs.unwrap_or_default() {
            println!("    {line}");
        }
        assert!(sim.value.err.is_some(), "this call should have failed!");
        println!("    rejected as expected: {:?}", sim.value.err.unwrap());
    }

    fn show(&self, counter: &Pubkey) {
        let data = self.rpc.get_account_data(counter).expect("account data");
        let c = counter::Counter::try_deserialize(&mut &data[..]).expect("decode");
        println!(
            "    state: count={} status={} operations={} history={:?} label={:?}",
            c.count, c.status, c.operations, c.history, c.label
        );
    }
}

fn update(counter: &Pubkey, authority: &Pubkey) -> Vec<AccountMeta> {
    counter::accounts::Update {
        counter: *counter,
        authority: *authority,
    }
    .to_account_metas(None)
}

fn ix(data: Vec<u8>, accounts: Vec<AccountMeta>) -> Instruction {
    Instruction::new_with_bytes(counter::id(), &data, accounts)
}

fn main() {
    let url = std::env::var("RPC_URL").unwrap_or_else(|_| "https://api.devnet.solana.com".into());
    let wallet_path = std::env::var("WALLET").unwrap_or_else(|_| {
        format!("{}/.config/solana/id.json", std::env::var("HOME").expect("HOME"))
    });
    let wallet = read_keypair_file(&wallet_path).expect("cannot read wallet keypair");
    let rpc = RpcClient::new_with_commitment(url.clone(), CommitmentConfig::confirmed());
    let ctx = Ctx { rpc, wallet };
    let me = ctx.wallet.pubkey();

    println!("RPC:     {url}");
    println!("Program: {}", counter::id());
    println!("Wallet:  {me}");
    let balance = ctx.rpc.get_balance(&me).expect("balance");
    println!("Balance: {} SOL", balance as f64 / 1e9);
    assert!(balance > 50_000_000, "need > 0.05 SOL: run `solana airdrop 2`");

    // A brand-new account address for this run (a plain keypair, not a PDA).
    let counter_kp = Keypair::new();
    let counter = counter_kp.pubkey();
    println!("Counter account: {counter}");

    ctx.run(
        "initialize(start = 5)",
        ix(
            counter::instruction::Initialize { start: 5 }.data(),
            counter::accounts::Initialize {
                counter,
                authority: me,
                system_program: system_program::ID,
            }
            .to_account_metas(None),
        ),
        &[&counter_kp], // the new account must sign its own creation
    );
    ctx.show(&counter);

    ctx.run(
        "increment(10)",
        ix(counter::instruction::Increment { amount: 10 }.data(), update(&counter, &me)),
        &[],
    );
    ctx.show(&counter);

    ctx.run(
        "batch_increment(times = 6, step = 2)",
        ix(
            counter::instruction::BatchIncrement { times: 6, step: 2 }.data(),
            update(&counter, &me),
        ),
        &[],
    );
    ctx.show(&counter);

    ctx.expect_rejected(
        "decrement(1000) would go below zero",
        ix(counter::instruction::Decrement { amount: 1_000 }.data(), update(&counter, &me)),
        &[],
    );

    ctx.run(
        "decrement(1)",
        ix(counter::instruction::Decrement { amount: 1 }.data(), update(&counter, &me)),
        &[],
    );

    ctx.run(
        "set_label(\"  hello   devnet  \")",
        ix(
            counter::instruction::SetLabel {
                label: "  hello   devnet  ".to_string(),
            }
            .data(),
            update(&counter, &me),
        ),
        &[],
    );
    ctx.show(&counter);

    ctx.run(
        "inspect (read-only tour of Rust features)",
        ix(
            counter::instruction::Inspect {}.data(),
            counter::accounts::Inspect { counter }.to_account_metas(None),
        ),
        &[],
    );

    let stranger = Keypair::new();
    ctx.expect_rejected(
        "a stranger tries to increment my counter",
        ix(
            counter::instruction::Increment { amount: 1 }.data(),
            update(&counter, &stranger.pubkey()),
        ),
        &[&stranger],
    );

    ctx.run(
        "toggle_pause (Active -> Paused)",
        ix(counter::instruction::TogglePause {}.data(), update(&counter, &me)),
        &[],
    );
    ctx.expect_rejected(
        "increment while paused",
        ix(counter::instruction::Increment { amount: 1 }.data(), update(&counter, &me)),
        &[],
    );
    ctx.run(
        "toggle_pause (Paused -> Active)",
        ix(counter::instruction::TogglePause {}.data(), update(&counter, &me)),
        &[],
    );
    ctx.show(&counter);

    let before = ctx.rpc.get_balance(&me).unwrap();
    ctx.run(
        "close_counter (delete account, refund rent)",
        ix(
            counter::instruction::CloseCounter {}.data(),
            counter::accounts::CloseCounter {
                counter,
                authority: me,
            }
            .to_account_metas(None),
        ),
        &[],
    );
    let after = ctx.rpc.get_balance(&me).unwrap();
    println!(
        "    balance {} -> {} lamports (rent refunded minus the 5000 fee)",
        before, after
    );
    println!("\nDone.");
}
