// Integration tests: the COMPILED program (target/deploy/counter.so) is loaded
// into LiteSVM, a tiny in-process Solana runtime. No network, no SOL needed.
// Run `anchor build` first, then `cargo test`.

use {
    anchor_lang::{
        prelude::Pubkey,
        solana_program::{instruction::Instruction, system_program},
        AccountDeserialize, InstructionData, ToAccountMetas,
    },
    litesvm::LiteSVM,
    solana_keypair::Keypair,
    solana_message::{Message, VersionedMessage},
    solana_signer::Signer,
    solana_transaction::versioned::VersionedTransaction,
};

fn setup() -> (LiteSVM, Keypair) {
    let mut svm = LiteSVM::new();
    let bytes = include_bytes!(concat!(
        env!("CARGO_TARGET_TMPDIR"),
        "/../deploy/counter.so"
    ));
    svm.add_program(counter::id(), bytes).unwrap();
    let authority = Keypair::new();
    svm.airdrop(&authority.pubkey(), 10_000_000_000).unwrap();
    (svm, authority)
}

/// Signs and sends one instruction; returns the program logs on failure.
fn send(
    svm: &mut LiteSVM,
    ix: Instruction,
    payer: &Keypair,
    extra_signers: &[&Keypair],
) -> Result<(), String> {
    let mut signers: Vec<&dyn Signer> = vec![payer];
    signers.extend(extra_signers.iter().map(|k| *k as &dyn Signer));

    let blockhash = svm.latest_blockhash();
    let msg = Message::new_with_blockhash(&[ix], Some(&payer.pubkey()), &blockhash);
    let tx = VersionedTransaction::try_new(VersionedMessage::Legacy(msg), &signers[..]).unwrap();
    svm.send_transaction(tx)
        .map(|_| ())
        .map_err(|e| e.meta.logs.join("\n"))
}

fn read_counter(svm: &LiteSVM, address: &Pubkey) -> counter::Counter {
    let account = svm.get_account(address).unwrap();
    let mut data: &[u8] = &account.data;
    counter::Counter::try_deserialize(&mut data).unwrap()
}

fn initialize_ix(counter: &Pubkey, authority: &Pubkey, start: u64) -> Instruction {
    Instruction::new_with_bytes(
        counter::id(),
        &counter::instruction::Initialize { start }.data(),
        counter::accounts::Initialize {
            counter: *counter,
            authority: *authority,
            system_program: system_program::ID,
        }
        .to_account_metas(None),
    )
}

fn update_accounts(counter: &Pubkey, authority: &Pubkey) -> Vec<anchor_lang::prelude::AccountMeta> {
    counter::accounts::Update {
        counter: *counter,
        authority: *authority,
    }
    .to_account_metas(None)
}

#[test]
fn full_counter_flow() {
    let (mut svm, authority) = setup();
    let counter_kp = Keypair::new();
    let counter = counter_kp.pubkey();

    // initialize(5): the new account must sign too, because it is a fresh keypair
    send(
        &mut svm,
        initialize_ix(&counter, &authority.pubkey(), 5),
        &authority,
        &[&counter_kp],
    )
    .expect("initialize should work");
    let state = read_counter(&svm, &counter);
    assert_eq!(state.count, 5);
    assert_eq!(state.authority, authority.pubkey());
    assert_eq!(state.status, counter::Status::Active);

    // increment(10) -> 15
    let ix = Instruction::new_with_bytes(
        counter::id(),
        &counter::instruction::Increment { amount: 10 }.data(),
        update_accounts(&counter, &authority.pubkey()),
    );
    send(&mut svm, ix, &authority, &[]).expect("increment should work");
    let state = read_counter(&svm, &counter);
    assert_eq!(state.count, 15);
    assert_eq!(state.operations, 1);
    assert_eq!(state.history[0], 5); // the previous value was recorded

    // batch_increment(times=6, step=2): only even iterations 0,2,4 add -> +6
    svm.expire_blockhash();
    let ix = Instruction::new_with_bytes(
        counter::id(),
        &counter::instruction::BatchIncrement { times: 6, step: 2 }.data(),
        update_accounts(&counter, &authority.pubkey()),
    );
    send(&mut svm, ix, &authority, &[]).expect("batch should work");
    assert_eq!(read_counter(&svm, &counter).count, 21);

    // decrement below zero must fail with our custom error
    svm.expire_blockhash();
    let ix = Instruction::new_with_bytes(
        counter::id(),
        &counter::instruction::Decrement { amount: 1_000 }.data(),
        update_accounts(&counter, &authority.pubkey()),
    );
    let logs = send(&mut svm, ix, &authority, &[]).unwrap_err();
    assert!(logs.contains("Cannot go below zero"), "logs: {logs}");

    // set_label trims and collapses whitespace
    svm.expire_blockhash();
    let ix = Instruction::new_with_bytes(
        counter::id(),
        &counter::instruction::SetLabel {
            label: "  my   first   counter ".to_string(),
        }
        .data(),
        update_accounts(&counter, &authority.pubkey()),
    );
    send(&mut svm, ix, &authority, &[]).expect("set_label should work");
    assert_eq!(read_counter(&svm, &counter).label, "my first counter");

    // pause -> increment is rejected
    svm.expire_blockhash();
    let ix = Instruction::new_with_bytes(
        counter::id(),
        &counter::instruction::TogglePause {}.data(),
        update_accounts(&counter, &authority.pubkey()),
    );
    send(&mut svm, ix, &authority, &[]).expect("toggle should work");
    assert_eq!(read_counter(&svm, &counter).status, counter::Status::Paused);

    svm.expire_blockhash();
    let ix = Instruction::new_with_bytes(
        counter::id(),
        &counter::instruction::Increment { amount: 1 }.data(),
        update_accounts(&counter, &authority.pubkey()),
    );
    let logs = send(&mut svm, ix, &authority, &[]).unwrap_err();
    assert!(logs.contains("The counter is paused"), "logs: {logs}");
}

#[test]
fn stranger_cannot_modify_counter() {
    let (mut svm, authority) = setup();
    let counter_kp = Keypair::new();
    let counter = counter_kp.pubkey();
    send(
        &mut svm,
        initialize_ix(&counter, &authority.pubkey(), 0),
        &authority,
        &[&counter_kp],
    )
    .unwrap();

    let stranger = Keypair::new();
    svm.airdrop(&stranger.pubkey(), 1_000_000_000).unwrap();
    let ix = Instruction::new_with_bytes(
        counter::id(),
        &counter::instruction::Increment { amount: 1 }.data(),
        update_accounts(&counter, &stranger.pubkey()),
    );
    let logs = send(&mut svm, ix, &stranger, &[]).unwrap_err();
    assert!(logs.contains("Only the counter's authority"), "logs: {logs}");
    assert_eq!(read_counter(&svm, &counter).count, 0);
}

#[test]
fn initialize_rejects_too_large_start() {
    let (mut svm, authority) = setup();
    let counter_kp = Keypair::new();
    let logs = send(
        &mut svm,
        initialize_ix(&counter_kp.pubkey(), &authority.pubkey(), counter::MAX_COUNT + 1),
        &authority,
        &[&counter_kp],
    )
    .unwrap_err();
    assert!(logs.contains("exceeds the maximum"), "logs: {logs}");
}

#[test]
fn close_refunds_rent() {
    let (mut svm, authority) = setup();
    let counter_kp = Keypair::new();
    let counter = counter_kp.pubkey();
    send(
        &mut svm,
        initialize_ix(&counter, &authority.pubkey(), 0),
        &authority,
        &[&counter_kp],
    )
    .unwrap();

    let before = svm.get_balance(&authority.pubkey()).unwrap();
    svm.expire_blockhash();
    let ix = Instruction::new_with_bytes(
        counter::id(),
        &counter::instruction::CloseCounter {}.data(),
        counter::accounts::CloseCounter {
            counter,
            authority: authority.pubkey(),
        }
        .to_account_metas(None),
    );
    send(&mut svm, ix, &authority, &[]).expect("close should work");
    let after = svm.get_balance(&authority.pubkey()).unwrap();

    assert!(after > before, "rent should be refunded ({before} -> {after})");
    assert!(svm.get_account(&counter).is_none(), "account must be gone");
}
