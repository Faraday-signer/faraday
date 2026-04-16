//! Known Solana program IDs.

pub struct KnownProgram {
    pub name: &'static str,
}

pub fn identify(program_id: &[u8; 32]) -> Option<KnownProgram> {
    let encoded = bs58::encode(program_id).into_string();
    let name = match encoded.as_str() {
        "11111111111111111111111111111111" => "System",
        "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA" => "Token",
        "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb" => "Token-2022",
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJe1bVU" => "AssocToken",
        "Stake11111111111111111111111111111111111111112" => "Stake",
        "Vote111111111111111111111111111111111111111p8HGB" => "Vote",
        "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr" => "Memo",
        "Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo" => "Memo",
        "ComputeBudget111111111111111111111111111111" => "ComputeBudget",
        _ => return None,
    };
    Some(KnownProgram { name })
}
