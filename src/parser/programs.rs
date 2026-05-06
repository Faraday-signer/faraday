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
        "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL" => "AssocToken",
        "Stake11111111111111111111111111111111111111" => "Stake",
        "Vote111111111111111111111111111111111111111" => "Vote",
        "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr" => "Memo",
        "Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo" => "Memo",
        "ComputeBudget111111111111111111111111111111" => "ComputeBudget",
        "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4" => "Jupiter",
        "JUP4Fb2cqiRUcaTHdrPC8h2gNsA2ETXiPDD33WcGuJB" => "Jupiter v4 (legacy)",
        "proVF4pMXVaYqmy4NjniPh4pqKNfMmsihgd4wdkCX3u" => "Jupiter Ultra",
        "61DFfeTKM7trxYcPQCM78bJ794ddZprZpAwAnLiwTpYH" => "Jupiter RFQ",
        "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8" => "Raydium AMM",
        "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK" => "Raydium CLMM",
        "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C" => "Raydium CPMM",
        "DF1ow4tspfHX9JwWJsAb9epbkA8hmpSEAtxXy1V27QBH" => "DFlow",
        "whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc" => "Orca Whirlpools",
        "LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo" => "Meteora DLMM",
        "PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY" => "Phoenix",
        "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P" => "Pump.fun",
        _ => return None,
    };
    Some(KnownProgram { name })
}

#[cfg(test)]
mod tests {
    use super::*;

    fn pubkey_from_b58(s: &str) -> [u8; 32] {
        let bytes = bs58::decode(s).into_vec().unwrap();
        let mut key = [0u8; 32];
        key.copy_from_slice(&bytes);
        key
    }

    #[test]
    fn test_system_program() {
        let id = pubkey_from_b58("11111111111111111111111111111111");
        assert_eq!(identify(&id).unwrap().name, "System");
    }

    #[test]
    fn test_spl_token() {
        let id = pubkey_from_b58("TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA");
        assert_eq!(identify(&id).unwrap().name, "Token");
    }

    #[test]
    fn test_token_2022() {
        let id = pubkey_from_b58("TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb");
        assert_eq!(identify(&id).unwrap().name, "Token-2022");
    }

    #[test]
    fn test_stake_program() {
        let id = pubkey_from_b58("Stake11111111111111111111111111111111111111");
        assert_eq!(identify(&id).unwrap().name, "Stake");
    }

    #[test]
    fn test_vote_program() {
        let id = pubkey_from_b58("Vote111111111111111111111111111111111111111");
        assert_eq!(identify(&id).unwrap().name, "Vote");
    }

    #[test]
    fn test_assoc_token() {
        let id = pubkey_from_b58("ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL");
        assert_eq!(identify(&id).unwrap().name, "AssocToken");
    }

    #[test]
    fn test_dflow() {
        let id = pubkey_from_b58("DF1ow4tspfHX9JwWJsAb9epbkA8hmpSEAtxXy1V27QBH");
        assert_eq!(identify(&id).unwrap().name, "DFlow");
    }

    #[test]
    fn test_orca_whirlpools() {
        let id = pubkey_from_b58("whirLbMiicVdio4qvUfM5KAg6Ct8VwpYzGff3uctyCc");
        assert_eq!(identify(&id).unwrap().name, "Orca Whirlpools");
    }

    #[test]
    fn test_meteora_dlmm() {
        let id = pubkey_from_b58("LBUZKhRxPF3XUpBCjp4YzTKgLccjZhTSDM9YuVaPwxo");
        assert_eq!(identify(&id).unwrap().name, "Meteora DLMM");
    }

    #[test]
    fn test_phoenix() {
        let id = pubkey_from_b58("PhoeNiXZ8ByJGLkxNfZRnkUfjvmuYqLR89jjFHGqdXY");
        assert_eq!(identify(&id).unwrap().name, "Phoenix");
    }

    #[test]
    fn test_pumpfun() {
        let id = pubkey_from_b58("6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P");
        assert_eq!(identify(&id).unwrap().name, "Pump.fun");
    }

    #[test]
    fn test_compute_budget() {
        let id = pubkey_from_b58("ComputeBudget111111111111111111111111111111");
        assert_eq!(identify(&id).unwrap().name, "ComputeBudget");
    }

    #[test]
    fn test_jupiter_v6() {
        let id = pubkey_from_b58("JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4");
        assert_eq!(identify(&id).unwrap().name, "Jupiter");
    }

    #[test]
    fn test_raydium_amm_v4() {
        let id = pubkey_from_b58("675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8");
        assert_eq!(identify(&id).unwrap().name, "Raydium AMM");
    }

    #[test]
    fn test_raydium_clmm() {
        let id = pubkey_from_b58("CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK");
        assert_eq!(identify(&id).unwrap().name, "Raydium CLMM");
    }

    #[test]
    fn test_raydium_cpmm() {
        let id = pubkey_from_b58("CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C");
        assert_eq!(identify(&id).unwrap().name, "Raydium CPMM");
    }

    #[test]
    fn test_unknown_program_returns_none() {
        let id = [0x42u8; 32];
        assert!(identify(&id).is_none());
    }
}
