//! Hardcoded Address Lookup Tables for common Solana programs.
//!
//! Enables offline resolution of v0 transaction accounts without RPC access.
//!
//! # Security caveat (2026-05)
//!
//! All currently shipped ATLs (`JUPITER_MAIN`, `JUPITER_2`, `RAYDIUM_CLMM`)
//! are **mutable** at chain level — their respective authorities can extend
//! or replace entries. Our snapshot is only correct as of the firmware build
//! time. If an authority is compromised or pushes a malicious update, our
//! display can diverge from on-chain execution: the user sees one mint, the
//! validator resolves another. The signed bytes are unaffected (the
//! signature is over the message, which references ATLs by pubkey, not by
//! resolved content), but the user's *informed consent* to sign is.
//!
//! Going forward, `extract_zoned` refuses to emit a zoned approval screen
//! when the receive token cannot be resolved deterministically — for txs
//! whose dest mint lives in an ATL we don't have or trust, the device falls
//! back to the legacy paginated review (raw amounts, explicit warning).
//!
//! `scripts/fetch_alt.py` will warn if you try to add a non-frozen ATL.
//! Prefer ATLs whose `authority` field is `None` — those are immutable on
//! chain and our hash matches forever. For the existing mutable entries,
//! re-snapshot periodically and audit the authority's behaviour.

const UNRESOLVED: [u8; 32] = [0xFF; 32];

pub fn is_unresolved(key: &[u8; 32]) -> bool {
    *key == UNRESOLVED
}

/// Expands a v0 transaction's static account list with resolved ALT entries.
///
/// Follows the Solana v0 account ordering:
///   [static keys] [all writable from ALTs] [all readonly from ALTs]
pub fn expand_accounts(
    static_accounts: &[[u8; 32]],
    lookups: &[crate::parser::message::AddressTableLookup],
) -> Vec<[u8; 32]> {
    let mut accounts = static_accounts.to_vec();

    // Writable entries from all ALTs (in order)
    for alt in lookups {
        let table = find_table(&alt.account_key);
        for &idx in &alt.writable_indices {
            accounts.push(resolve_entry(table, idx));
        }
    }

    // Readonly entries from all ALTs (in order)
    for alt in lookups {
        let table = find_table(&alt.account_key);
        for &idx in &alt.readonly_indices {
            accounts.push(resolve_entry(table, idx));
        }
    }

    accounts
}

fn resolve_entry(table: Option<&[&str]>, index: u8) -> [u8; 32] {
    table
        .and_then(|t| t.get(index as usize))
        .map(|addr| pubkey_from_b58(addr))
        .unwrap_or(UNRESOLVED)
}

fn find_table(alt_address: &[u8; 32]) -> Option<&'static [&'static str]> {
    let addr = bs58::encode(alt_address).into_string();
    match addr.as_str() {
        "3oy9ojnsDzqmMNi87Gs7Hn5v3MPVqnWjG9k8BmzKR7yW" => Some(JUPITER_MAIN),
        "FBLCh3Mw1cCVyVZKQ9eEfxA1zD4fkHGeP4yt3p6Fy6Eq" => Some(JUPITER_2),
        "AcL1Vo8oy1ULiavEcjSUcwfBSForXMudcZvDZy5nzJkU" => Some(RAYDIUM_CLMM),
        _ => None,
    }
}

fn pubkey_from_b58(s: &str) -> [u8; 32] {
    let bytes = bs58::decode(s)
        .into_vec()
        .expect("invalid base58 in lookup table");
    let mut key = [0u8; 32];
    key.copy_from_slice(&bytes);
    key
}

// ── Jupiter main routing table ──────────────────────────────────────────────
// ALT: 3oy9ojnsDzqmMNi87Gs7Hn5v3MPVqnWjG9k8BmzKR7yW (97 entries)

const JUPITER_MAIN: &[&str] = &[
    "D8cy77BBepLMngZx6ZukaTff5hCt1HrWyKk3Hnd9oitf", // 0
    "jitodontfront11111111111JustUseJupiterU1tra",  // 1
    "jitodontfront1111111111111111JustUseJupiter",  // 2
    "GGztQqQ6pCPaJQnNpXBgELr5cs3WwDakRbh1iEMzjgSJ", // 3
    "2MFoS3MPtvyQ4Wh4M9pdfPjz6UhVoNbFbGJAskCPCj3h", // 4
    "BQ72nSv9f3PRyRKCBnHLVrerrv37CYTHm5h3s9VSGQDV", // 5
    "6U91aKa8pmMxkJwBCfPTmUEfZi6dHe7DcFq2ALvB2tbB", // 6
    "4xDsmeTWPNjgSVSS1VTfzFq3iHZhp77ffPkAmkZkdu71", // 7
    "CapuXNQoDviLvU1PxFiizLgPNQCxrsag1uMeyk6zLVps", // 8
    "9nnLbotNTcUhvbrsA6Mdkx45Sm82G35zo28AqUvjExn8", // 9
    "6LXutJvKUw8Q5ue2gCgKHQdAN4suWW8awzFVC6XCguFx", // 10
    "HFqp6ErWHY6Uzhj8rFyjYuDya2mXUpYEk8VW75K9PSiY", // 11
    "DSN3j1ykL3obAVNv7ZX49VsFCPe4LqzxHnmtLiPwY6xg", // 12
    "69yhtoJR4JYPPABZcSNkzuqbaFbwHsCkja1sP1Q2aVT5", // 13
    "HU23r7UoZbqTUuh3vA7emAGztFtqwTeVips789vqxxBw", // 14
    "3LoAYHuSd7Gh8d7RTFnhvYtiTiefdZ5ByamU42vkzd76", // 15
    "3CgvbiM3op4vjrrjH2zcrQUwsqh5veNVRjFCB9N6sRoD", // 16
    "GP8StUXNYSZjPikyRsvkTbvRV1GBxMErb59cpeCJnDf1", // 17
    "7iWnBRRhBCiNXXPhqiGzvvBkKrvFSWqqmxRyu9VyYBxE", // 18
    "11111111111111111111111111111111",             // 19
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 20
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",  // 21
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL", // 22
    "So11111111111111111111111111111111111111112",  // 23
    "g7dD1FHSemkUQrX1Eak37wzvDjscgBW2pFCENwjLdMX",  // 24
    "H1qQ6Hent1C5wa4Hc3GK2V1sgg4grvDBbmKd5H8dsTmo", // 25
    "8ctcHN52LY21FEipCjr1MVWtoZa1irJQTPyAaTj72h7S", // 26
    "7x4VcEX8aLd3kFsNWULTp1qFgVtDwyWSxpTGQkoMM6XX", // 27
    "6zAcFYmxkaH25qWZW5ek4dk4SyQNpSza3ydSoUxjTudD", // 28
    "91bUbswo6Di8235jAPwim1At4cPZLbG2pkpneyqKg4NQ", // 29
    "A8kEy5wWgdW4FG593fQJ5QPVbqx1wkfXw9c4L9bPo2CN", // 30
    "BuqEDKUwyAotZuK37V4JYEykZVKY8qo1zKbpfU9gkJMo", // 31
    "2p29nqD7DN1PczBMmgrFdtYKTfv6rJ7H3yMut4eu7nYT", // 32
    "EUvpCGh4qiMtq9wKgp28f9Bjv5Xz2WJqrM83XmYAqkEq", // 33
    "qqdJ4z1yu4sTbAitwXZsGNDoGZFgL2HfVKSVwAXWCfq",  // 34
    "EaUghZfmuhgtZEEwaZwX5EBroz4WyH8VM5NSXi4tam5A", // 35
    "6AMWTvaL1pscDo5CAaBhPfv7xnSLjJ3ZXScV9yt5Gr5G", // 36
    "39FWQJxqTmUkFqHKjEs3CawQ1b1rqLcKmHfJ1FWMAWdu", // 37
    "GyY4VgEpJQhiKZRAJJmoM4hv5Q2xC4pvX68MGrGidxyG", // 38
    "2oL6my4QDDCfpgJZX1bZV1NgbmuNptKdgcE8wJm6efgk", // 39
    "EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v", // 40 (USDC)
    "DVCeozFGbe6ew3eWTnZByjHeYqTq1cvbrB7JJhkLxaRJ", // 41
    "JSvtokJbtGsYhneKomFBjnJh4djEQLdHV2kAeS43bBZ",  // 42
    "7u7cD7NxcZEuzRCBaYo8uVpotRdqZwez47vvuwzCov43", // 43
    "59v2cSbCsnyaWymLnsq6TWzE6cEN5KJYNTBNrcP4smRH", // 44
    "m3BrPbv2TFmZZTPpyB9NgsCXqGNujpXvzvGqj8ksars",  // 45
    "Gjmjory7TWKJXD2Jc6hKzAG991wWutFhtbXudzJqgx3p", // 46
    "EXrqY7jLTLp83H38L8Zw3GvGkk1KoQbYTckPGBghwD8X", // 47
    "G4CD7aqqZZ6QKCNHrc1MPdS9Aw8BWmQ5ZkDd54W6mAEG", // 48
    "5SPztfEn1VAaWDBAXjQKwVrGbr6e8g3F6JJnUc9eCuSe", // 49
    "FbruxBVHi463Agw2B3Vy27cBkGnEN5g1f4NcHe3REXfe", // 50
    "E6LAwCLSHLkDCoMXZPtnDtpcvCYWcs3ZZLHLreiFwjUi", // 51
    "DY6pE7aiDafuk35REZF9p9av3vbV2VQrvdZ4YyB1pZ4C", // 52
    "819hAmNKJ4MyEF9fEYc31vzqsnwfxN7NtrM2QpovgvtS", // 53
    "4pCDJv3V4P4WtvK5x4f64DS4wDctkMZDAjz6CxTdqPGj", // 54
    "HoBCz6z9AG92GGozMWEkBPE9UhQWGZ5cXhYcjoGJvwP2", // 55
    "EpdaePzdqRkMtdZJquVPUWgyoJ5YEEpYALki6dv9VBrt", // 56
    "6zQecXhjYTifDGYxbW7vRTQBrBYsi1Uac6BEJ4WzefWS", // 57
    "GgY8theL9n9hQPoz2keQM8y6z8T6G6BH9FPLjBtkF9Hd", // 58
    "G4FUwFD1h4tb4R6jkZXuoyst7YNbYTcJH3MvCUguss6E", // 59
    "F2Xjd4ZJYz6SfszyUVGzLUzAHRRhfU2iJacCfW5GCJHM", // 60
    "3cHRcBKWbJeL2qyjgQ8wdSYxmRYW1ZyC2nVqTakAj57G", // 61
    "6Ugimjtgk7rk5SbZNzcYvZiM3P6ki4Uq3QGtTHWNn8co", // 62
    "4QKRxAfawktf6szGUP456AqBvaKSnmuGy91QnqdBDSke", // 63
    "9kiYqGSb1nbYMc5xxZQHhKvJR57LLAHVyDvSQ3FHjDPK", // 64
    "5xgh5nvdQivmBPCsK3oCLGTQFyQHytQ8BWox7zpXAuye", // 65
    "5eGuZuVysi3Uksa5jRujq9XWWyNiNsosAme4xX8GM216", // 66
    "6NC5sEHoBEyhJTW6mUxzT3CRerMNyjsxK21yjVBMqFY",  // 67
    "J3Up4p7i5LncgWbwytNsH4vrwco3qWoBTVQZnMJYxe7W", // 68
    "75P62FyYr8sHjwUCCcUc2bT1MDrAVcsC6GkwkDyMoVVn", // 69
    "GzSCp13VYUj2kpCBVpHnXaPS7SVE6R5Bq7D7tCbZckxP", // 70
    "75YeSeaeVKiNvskBNZqmZ5EeuktgkfUq8YBqXoF4cTYu", // 71
    "5xgSuPHiMxnvw38P8MxFJJN56WdSYTLBeq8XrUfKWaJE", // 72
    "7hZdUKF3mNFm8NxYfBruNBEUHa8zwHr8SGk2jzP8W3zz", // 73
    "GnqhCoAp1LbSbB5Vti7eHX3sZmFeQVV4y8aPy6sjsMeQ", // 74
    "EDnz8jbGoLNTwxbHyT7cDvDqYLrgZrYzZS19kszebzVt", // 75
    "7Svve3shNti3WaY2MdKtFY4H7eNYRiF9nLCXX6KBhGrq", // 76
    "A2Kj8EzBSif36NNvs2i35GjdYGcs1RAwuXRrLpzfwnFF", // 77
    "Ec6y749opoW5JKbJEip3PuJaZhusQHQtUxtDzWysmTPi", // 78
    "Fja7LBfyWbnZQxrnQnw2UHKzknXyUNFWXXdQVibnrzBk", // 79
    "GBRPRss7LgMQf4rxXyP8YCjHpdpEWM5BDXUCHHt6unEa", // 80
    "EkV7V23YfHztteeNz1tfC4Um3MwWfVwMC9bhotoZtJKv", // 81
    "6EF8rrecthR5Dkzon8Nwu78hRvfCKubJ14M5uBEwF6P",  // 82
    "4wTV1YmiEkRvAtNtsSGPtUrqRYQMe5SKy2uB4Jjaxnjf", // 83
    "Hq2wp8uJ9jCPsYgNHex8RtqdvMPfVGoYwjvF1ATiwn2Y", // 84
    "8Wf5TiAheLUqBrKXeYg2JtAFFMWtKdG2BSFgqUcPVwTt", // 85
    "pfeeUxB6jkeY1Hxd7CsFCAjcbHA9rWtchMGdZ6VojVZ",  // 86
    "Ce6TQqeHC9p8KetsN6JsjHK7UTZk7nasjjnr7XxXp9F1", // 87
    "dbcij3LWUppWqq96dh6gJWwBifmcGfLSB5D4DuSMaqN",  // 88
    "FhVo3mqL8PW5pH5U2CN4XE33DokiyZnUwuGpH2hmHLuM", // 89
    "8Ks12pbrD6PXxfty1hVQiE9sc289zgU1zHkvXhrSdriF", // 90
    "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj",  // 91
    "WLHv2UAZm6z4KyaaELi5pjdbJh6RESMva1Rnn8pJVVh",  // 92
    "2DPAtwB8L12vrMRExbLuyGnC7n2J5LNoZQSejeQGpwkr", // 93
    "LanMV9sAd7wArD4vJFi2qDdfnVhFxYSUg6eADduJ3uj",  // 94
    "WLHv2UAZm6z4KyaaELi5pjdbJh6RESMva1Rnn8pJVVh",  // 95
    "2DPAtwB8L12vrMRExbLuyGnC7n2J5LNoZQSejeQGpwkr", // 96
];

// ── Jupiter routing table 2 ────────────────────────────────────────────────
// ALT: FBLCh3Mw1cCVyVZKQ9eEfxA1zD4fkHGeP4yt3p6Fy6Eq (220 entries)

const JUPITER_2: &[&str] = &[
    "9xWSnkGembR4p2S1SvDuxzp4ENJwggQXJj7Zwy7FS9wx", // 0
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 1
    "DbzLAwxPTFnkBfyQGcEQ1diW2NBhTV75X22wBpfYxKTk", // 2
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 3
    "29hGNnbchKr4GNheqFTV5wz3DWW1mBcdgK82f5M165Vq", // 4
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 5
    "8MNMJd7p15qhrRnEYE7cHmXDNfKyajXKLgadYMBAqpev", // 6
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 7
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 8
    "r7WRtpReP4deeYpcUCcaJWdVCs2PX2iLVoZN42e19Bz",  // 9
    "A1Ek6DubrmpDttZC8ZnKS2DakUHKh39q65we1ynxbk7N", // 10
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 11
    "Anfy6ijnP8o2L7AssKiLrhCPRgFwPzdcjxtaZESdkX1M", // 12
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 13
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 14
    "3AjG5WRyGUPHmcqggTdoApsrNM9RBEYsXHGKAm4ffe32", // 15
    "Aevd3MDbspvNgY3QbPz6AcJ4Kxbh1GSFv3xu6Xh6ApDw", // 16
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 17
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 18
    "FhxqMnwww6DaBsdQbeAqKpExUscJwN1s3cFnUL8Gw8wv", // 19
    "4cdtiM65NTGpNzprRtaQWcCcutuf6LPDQuX2inKUjAkg", // 20
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",  // 21
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",  // 22
    "2Ny1cLHRoDmATQd86o7xo5zUYWKXeAZJoCkDNgcyvKfT", // 23
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 24
    "DsK28ssZWmc2KjuMxhkwf7G7UYwwt2FcJerKaRdpPSB2", // 25
    "So11111111111111111111111111111111111111112",  // 26
    "7JHtXeLsYjVCV6DnUb6YXUwhUKGeYBEvENqDiEqiu9QP", // 27
    "52kDEvxo9ycXK5Ns1ZnEwHMvGiAdiGRE3SK7ouXRC2nY", // 28
    "A2oceCPmqGrdmdhe9zWvaRDy5g2sv97cBTaKJhMv38ud", // 29
    "FgZ73oqSRFEH3cVaF3aZN4ivqFCya9jzJaiunuEiE4Wo", // 30
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 31
    "CRU5M3FJSSeYfZsGJoBPgbUreVqaQDAq3U81XtWayFUj", // 32
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 33
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 34
    "AY1XwKnXz3HKnqvU8Aue1b5LM82qi31NDQ5QJJEayVEh", // 35
    "5vAN4NDHDF1JCDBZAmkXe15spuK3nh7tdWUN8kqEh1LS", // 36
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 37
    "DoUDaM48Kn5D2wGPzHk34CSeTXxzPKh5junMXtF6x3z6", // 38
    "2UNccxrd2dz3yksvMSn9pswMmVykmVpE1ZswG98Z1TGr", // 39
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 40
    "9kDdBqgZtGvbDGDotzkZbynAuZXt11LZjA6FF91igyjr", // 41
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 42
    "HrniNxJ69X6v59d7WRZiw99qaeVTHr8b9nUttAMwf2Dq", // 43
    "Gkz8HEpVsRsFxvsnJWcAkVZWZqT333Hey8onP67ZpLU3", // 44
    "4kZLYKQvMoqh3tDpewugcMXaXX8nt8LDJipUuDKaBVvA", // 45
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 46
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 47
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 48
    "Ea3hu9B5cQdMBHZRswKQ4n12iz7KGwDaM1JvKLfFPUQA", // 49
    "6Wi7uohJDmfFCRVaLETNczo4etD45iW9XBQUbAbnkcG2", // 50
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 51
    "5UJVujmtYaTm66tDzAXKV8owLGFTHUawdtrhF9jHhdDY", // 52
    "8Bgro6892iJVFbMasUJCBU8bkbdA9Qq32w6vEZrLDjnh", // 53
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 54
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 55
    "2NjAupzHSpJ3yc27nuU9htucWoew6kFapPLSkfax8grf", // 56
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 57
    "GASERvWUwaesmP53wsf9J3v8e3Y4JX2iip364C8cWpvw", // 58
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 59
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 60
    "611Bwo8VnUQWpjZqZSjCKzBHXa3BKSHU678YHArhWrPt", // 61
    "So11111111111111111111111111111111111111112",  // 62
    "2bi6ZqxG4qh1PY96bjJGrYchD6SRjDCajSRxgPH5qtNn", // 63
    "FjW5NapbzrYjcgL8SDoLz4nMTfVa9nrEx383vuSSyuRi", // 64
    "89MwtnABHyDUbY9ADWVEEcGi9y3fA6bDa6PRQoDGfnVi", // 65
    "GMzvTK1shUdfGjRcG8qLrSbHnoeDypmSftHmN6v9tYNK", // 66
    "A8n6mnKQH1tCVDcxuT93uS9kp6ret4WQXi8AdX9hsAJ8", // 67
    "So11111111111111111111111111111111111111112",  // 68
    "FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X", // 69
    "6hdVzZRdVfuxMkVsJLSMcpB6LV53tiigrhYVbGTK514U", // 70
    "BECCGKKDYMsUNxQp9CtukWtWFrBkk7pQRLVpZ8JfVEeV", // 71
    "H9gz1rczLYHuz6bDivWzdtd9dRxtU7xbG9t3n6Jf2ZE2", // 72
    "yJHzt6nt8htfSQkBZSnZuYDbAJ3XaHtqPuHWyzMbBtV",  // 73
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 74
    "7emuS9W1atLU6H244z3qpMqzDqL29GS89uvep5J5btZP", // 75
    "7u5EXVirTRNEbUMBG9WcLD2Eq5pMe1N1AHp5kUmoNTJy", // 76
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 77
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",  // 78
    "C8vrifK7QSdP2HWJDrtEqw5e97Tj8qv6hrUcox644xRT", // 79
    "FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X", // 80
    "HcDdejkvXgqTys8PNNAimx2Hk7BWVDb34P8JTuDdiASu", // 81
    "2qEUcP9sBwZ3QTV1my17JvesCDjBqpeAWXfDJoYhxNhM", // 82
    "So11111111111111111111111111111111111111112",  // 83
    "8yM6Aor4mbiCRVsvHfTbaszEC1aNVqdpVamH7wGMAaAy", // 84
    "DHNKfhr9gFbY6pATCMbNT81724NRMqmFVXnoizz2VEKp", // 85
    "GdGxaePTEx1BLEiibofN36jk1W6H27f5T6P7HPxDgy5B", // 86
    "DAivscHyBK2KzyCuqH1rwG41e1foenFZhQK9JA3AaQuY", // 87
    "8bbw2HXJ43ZRzKzH6F9MuEME6wdJYxHwMCQ11JGRmn9x", // 88
    "9wSdspy5DvZxyQcEx5bDsFQHYf6Hu2J78k9TRNk3SsrZ", // 89
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 90
    "6Dy419BP57EC6MDmwffnpsRMsRiYsxMkApbFZ4EUZd3y", // 91
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 92
    "APu6GtTnaTRdJeZ7n67wkx5amMSQTCAYuTBpiweuMtdE", // 93
    "44A9GUw8tdUdfuBFfgLa9ZGnVRbzTbKsfNyNPPckXFzW", // 94
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 95
    "6XQHrt9M4oqFTF88fCJ9GFDQMcxmSQQ4vTbFMQzDUHg8", // 96
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 97
    "EGKTYPNuHr14eBU2DoD4Eo4HZqXWYTsWvc2zj8yKEjka", // 98
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 99
    "GkUtyHLj1jPeuCTUeJf45HaDyhLSqCnjBxMr714ehyCg", // 100
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 101
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 102
    "9ojnTvn89ZhHMzWZNTxpno6hUwCF5K1N47hdJd5Wb1xA", // 103
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 104
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 105
    "CRZrgY7nE4wYUFticLaWfCCRznLyVGcFw4WBZBLrWEeU", // 106
    "67JDrnG2UWsFUEE4EZnptmngwGkmpwwXGn9r2qYonQ6g", // 107
    "Cv8eFQ1LQu1aX9WWxJUo5hAWsJk1bZycdppDHK3QYcXt", // 108
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 109
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",  // 110
    "HVDiRRhuL7yfcvk4WavvvrcjFvVPfBtNL3dVeFJgpf84", // 111
    "5b2L69tU6vXonViNYmx7iRmkESFVvCbkj6WjYnwUQnKw", // 112
    "Czz3NZaR9yYT6Lmt6zYcg6Dzvn1CVA9g9rezRwdTUgAL", // 113
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 114
    "6XKuwNfTGktj3itDoU7WnL77sksWSpyXNm22aatqBwAS", // 115
    "FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X", // 116
    "J59u82DsV2LKvQvmLM7HjLEDZE3y89vvJuCVrjJMgp6E", // 117
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 118
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 119
    "2AthHXX39P63wMQd2882vECCtFF8TrFiZyAFwdxiG4wk", // 120
    "A8j7j9aHsXfhW4AibR5nHvrJen4CgRfSTmGwfyRaf1bt", // 121
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 122
    "GnZXTfQushFmumzKKFU6YZan6uousKbvwtssAonW3RVr", // 123
    "HhkiojNeFMfeHVUez5qAVAZfE8F3gdRencVpvG9eZ85d", // 124
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 125
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 126
    "F3wKPJir3dRuLrsTHwg39W2mufNKkMYLZ3XmJWVBGFoD", // 127
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 128
    "BzqNs1Pg1p3MwegYcVjsxQ9TV7jZBVxbX5FHAB7wczh6", // 129
    "HxCqBGsorZno311kccJex7s8FadYTUzqKKUyFzLoFZJD", // 130
    "8rypDBUqyHewphn4PYiYbySW7ysUBZS26GSqX7EAXRPG", // 131
    "EVoUtvhQvNKVVXXAkaCobHSyuKTq9BzxCnyxRMzRZKsy", // 132
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 133
    "7NDHduJfXJjA8JqPYEJTRXG8Je4qLPxpp7J8nnkVMKLF", // 134
    "BZktFj7D3K2Pt1nmTYMrWeK85WudQkUyY8jGRBQsUg7n", // 135
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 136
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 137
    "8tCx8dKHHuCFvrceNJ9xD86KSRhexFBkhCvDFduZmryB", // 138
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 139
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 140
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 141
    "DocF9aR98Bha5Se9hQiGGQ14tS5xaRQYwNtcYY4gZCzx", // 142
    "3i9c8FbRWERLbd7VoCGKN9NiHTis57crQzWS9QBUo1Eu", // 143
    "9p5o6FGCgsdBuwScdQjEBVitxDMffBzS697vwdiXwEkX", // 144
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 145
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 146
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 147
    "4Fb848uT4Txu8oNsZXfmR5KtHUxvnvMNEkTeyfVVgmYK", // 148
    "CDDr3ivA1Hrx4ZfPFVkgYcmC43RVgxhhhB8mxzY9gLoz", // 149
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 150
    "9JmCaTWvNhkwWw2b45aHYa27b9PHnM1Dep5MJX7C5Xo7", // 151
    "F4TtJRi6872dbsV3DPSpctTJpH7E1ntJ94vteHNfpA5k", // 152
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 153
    "9ijaJxHMD7DePwtzQpbB1dTa6SJraceznUUpf2yvzBc3", // 154
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",  // 155
    "5U5Qgik1AS5JS52rwH7c7E8kukrJvGePo6pG5vXVwxYx", // 156
    "9QRjzLf5QcVREXE8NmqDoHpKWjHu4jBseBp24NE766PF", // 157
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 158
    "7XTGxwS4fHzYpPmsBF1JHSYnZNrXsPqDvSM5GLhsoMDj", // 159
    "FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X", // 160
    "4pefXE9igcmQvQveRvawuqTXHuKSZkoiHiiNcBkWeMPF", // 161
    "So11111111111111111111111111111111111111112",  // 162
    "8WbSe6kyGY5JZCXA389eX186fozWK9mbiHjzJGY5WoMK", // 163
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 164
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 165
    "HD8stw5ux3vAu719igsaB4jfD7Y5cvtzDw3ABbYSmjtz", // 166
    "EgfyNigDJM1TxNJVTZ4fJu2eKsPVcFqX8HLx9amA2NVK", // 167
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 168
    "C3aibQuBT2PVn9jbUvCrbWiTWBbbJ5KMkRAhQNTJc3q6", // 169
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 170
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 171
    "2zG3oyQCvWV99SBaYSeXwiPzgifgNJxtQuJuMQp9W6hi", // 172
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 173
    "DdE6xwK1YQroAMfnLEh92E8dU8TjaXckcXSp6SRnY3N5", // 174
    "2heCthhyRihxoswpZGHPgh2YCWFMvcATR74utABZzA1H", // 175
    "2XH8oqhxqsvnUEBcvPFSiP5vSiLXxgX93vtyVCEnw8eA", // 176
    "BrR7eqMggd7wR5dhmDHPvkL94DmFLdA53ti9FjUMnc9e", // 177
    "8cJdrHw7UroDSA5BPMv8aw1sHfMmUEY8xSGkCaYxT4kC", // 178
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",  // 179
    "FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X", // 180
    "FGbmC95SQoUQ9JyAeDuZgca27ZQc46te18KMNK4CCqU9", // 181
    "So11111111111111111111111111111111111111112",  // 182
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 183
    "U1EBr7pSjpsouAwLYsEjaxkEkSkADbc3dNNwXD86G1G",  // 184
    "FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X", // 185
    "EchtZYW1wBBPV18gRpPQzG4iqdDJHKTeY3VepVZxLLaa", // 186
    "4bAnHbtv1wVZeeVWBzVtf913SDPjCEjLZ3rnHdrTwySB", // 187
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",  // 188
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 189
    "8kkdMEqeR6BTHANvoo2MpuMjAzCiB95TvVdVUUtDbKKP", // 190
    "cvSC32BAcF11u6GcQfyeoVE6Ewpsr9xe4kpkYre2cNp",  // 191
    "CJVkkfLhR9d9RCwxaG1nyxCEVMhhQWWdBzs3i8DWvPSN", // 192
    "So11111111111111111111111111111111111111112",  // 193
    "ELPSEcUr2n3r4DWBn7h1dz8GMnUBymQtFoMJJpnW38mF", // 194
    "4Gtt21DpuiV8MDR5tBDgtg9PSXPaXaPkkgYy4KieFbFU", // 195
    "FLUXubRmkEi2q6K3Y9kBPg9248ggaZVsoSFhtJHSrm1X", // 196
    "GpxrpcXGVT9Amw8S5uvsm284FtSEiEJiDojU9nSP6CeN", // 197
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 198
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 199
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 200
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 201
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 202
    "DwCEGYGZnzUyNGWP65eEeDgpRSGcguja9peJUYsMg2Uc", // 203
    "8cfNhDATZ9mbY8KNngtUmnWWiQ9knruLyFXBJ4ScRxSc", // 204
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8", // 205
    "CfLk6gPcfTreG52aYjHLBrz3xBnzfVgLjCdcW58LyxCy", // 206
    "BZyYYWioam8P2L1srpvcbe7gWdCiGrKWd2LVDZsrY7bB", // 207
    "2jpXQ8riw2dCbnGpNMYHyLvbT97AoaJSffYtgr4XftPx", // 208
    "6s3zG4DLnefP4fhnC9Tw5xfs8Ppr2phMsjunQR8DYybB", // 209
    "HN2dLPM39PEPGZn2H9j9mmRAAVLr8atXvKAhDQd4jB6L", // 210
    "4iWLrEQSotzneHbfAV5CuWcPx9vGZKyKuGuMvYCfDntA", // 211
    "A1BBtTYJd4i3xU8D6Tc2FzU6ZN4oXZWXKZnCxwbHXr8x", // 212
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 213
    "Fjc4XAWJuuykEQseySgR5sgpbaie5xEGn7E5jFph2u3Q", // 214
    "JUP6LkbZbjS1jKKwapdHNy74zcZ3tLUZoi5QNyVTaV4",  // 215
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK", // 216
    "EjAaUnA1GYwBJyjr8FFMMpLNm1kp9ugXi3RbkhbLNVqk", // 217
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1", // 218
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",  // 219
];

// ── Raydium CLMM routing table ─────────────────────────────────────────────
// ALT: AcL1Vo8oy1ULiavEcjSUcwfBSForXMudcZvDZy5nzJkU (80 entries)

const RAYDIUM_CLMM: &[&str] = &[
    "11111111111111111111111111111111",                // 0
    "TokenkegQfeZyiNwAJbNbGKPFXCWuBvf9Ss623VQ5DA",     // 1
    "TokenzQdBNbLqP5VEhdkAS6EPFLC1PHnBqCXEpPxuEb",     // 2
    "Memo1UhkJRfHyvLMcVucJwxXeuD728EqVDDwQDxFMNo",     // 3
    "MemoSq4gqABAXKb96qnH8TysNcWxMyWCqXgDLGmfcHr",     // 4
    "SysvarRent111111111111111111111111111111111",     // 5
    "SysvarC1ock11111111111111111111111111111111",     // 6
    "ATokenGPvbdGVxr1b2hvZbsiqW5xWH25efTNsLJA8knL",    // 7
    "metaqbxxUerdq28cj1RbAWkYQm3ybzjb6a8bt518x1s",     // 8
    "EUqojwWA2rd19FZrzeBncJsm38Jm1hEhE3zsmX3bRc2o",    // 9
    "9xQeWvG816bUx9EPjHmaT23yvVM2ZWbrrpZb9PusVFin",    // 10
    "RVKd61ztZW9GUwhRbbLoYVRE5Xf1B2tVscKqwZqXgEr",     // 11
    "27haf8L6oxUeXrHrgEgsexjSY5hbVUWEmvv9Nyxg8vQv",    // 12
    "675kPX9MHTjS2zt1qfr1NYHuzeLXfQM9H24wFSUt1Mp8",    // 13
    "5quBtoiQqxF9Jv6KYKctB59NT3gtJD2Y65kdnB1Uev3h",    // 14
    "CAMMCzo5YL8w4VFF8KVHrK22GGUsp5VTaW7grrKgrWqK",    // 15
    "CPMMoo8L3F4NbTegBCKVNunggL7H1ZpdTHKxQB5qKP1C",    // 16
    "routeUGWgWzqBWFcrCfv8tritsqukccJPu3q5GPP3xS",     // 17
    "EhhTKczWMGQt46ynNeRX1WfeagwwJd7ufHvCDjRxjo5Q",    // 18
    "CBuCnLe26faBpcBP2fktp4rp8abpcAnTWft6ZrP5Q4T",     // 19
    "9KEPoZmtHUrBbhWN1v1KWLMkkvwY6WLtAVUCPRtRjP4z",    // 20
    "FarmqiPv5eAj3j1GMdMCMUGXqPUvmquZtMy86QH6rzhG",    // 21
    "6FJon3QE27qgPVggARueB22hLvoh22VzJpXv4rBEoSLF",    // 22
    "CC12se5To1CdEuw7fDS27B7Geo5jJyL7t5UK2B44NgiH",    // 23
    "9HzJyW1qZsEiSfMUf6L2jo3CcTKAyBmSyKdwQeYisHrC",    // 24
    "DropEU8AvevN3UrXWXTMuz3rqnMczQVNjq3kcSdW2SQi",    // 25
    "CDSr3ssLcRB6XYPJwAfFt18MZvEZp4LjHcvzBVZ45duo",    // 26
    "5Q544fKrFoe6tsEbD7S8EmxGTJYAKtTVhAW5Q5pge4j1",    // 27
    "3uaZBfHPfmpAHW7dsimC1SnyR61X4bJqQZKWmRSCXJxv",    // 28
    "GpMZbSM2GgvTKHJirzeGfMFoaZ8UR2X7F4v8vHTvxFbL",    // 29
    "LockrWmn6K5twhz3y9w1dQERbmgSaRkfnTeTKbpofwE",     // 30
    "kN1kEznaF5Xbd8LYuqtEFcxzWSBk5Fv6ygX6SqEGJVy",     // 31
    "ComputeBudget111111111111111111111111111111",     // 32
    "3h2e43PunVA5K34vwKCLHWhZF4aZpyaC9RmxvshGAQpL",    // 33
    "3XCQJQryqpDvvZBfGxR7CLAw5dpGJ9aa7kt1jRLdyxuZ",    // 34
    "4BLNHtVe942GSs4teSZqGX24xwKNkqU7bGgNn3iUiUpw",    // 35
    "9EeWRCL8CJnikDFCDzG8rtmBs5KQR1jEYKCR5rRZ2NEi",    // 36
    "9iFER3bpjf1PTTCQCfTRu17EJgvsxo9pVyA9QWwEuX4x",    // 37
    "A1BBtTYJd4i3xU8D6Tc2FzU6ZN4oXZWXKZnCxwbHXr8x",    // 38
    "E64NGkDLLCdQ2yFNPcavaKptrEgmiQaNykUuLC1Qgwyp",    // 39
    "EdPxg8QaeFSrTYqdWJn6Kezwy9McWncTYcT3DcAp949ZwbF", // 40
    "Gex2NJRS3jVLPfbzSFM5d5DRsNoL5ynnwT1TXoDEhanz",    // 41
    "HfERMT5DRA6C1TAqecrJQFpmkf3wsWTMncqnj3RDg5aw",    // 42
    "2fGXL8uhqxJ4tpgtosHZXT4zcQap6j62z3bMDxdkMvy5",    // 43
    "C7Cx2pMLtjybS3mDKSfsBj4zQ3PRZGkKt7RCYTTbCSx2",    // 44
    "D4FPEruKEHrG5TenZ2mpDGEfu1iUvTiqBxvpU8HLBvC2",    // 45
    "G95xxie3XbkCqtE39GgQ9Ggc7xBC8Uceve7HFDEFApkc",    // 46
    "47Nq74YtwjVeTQF6KFKRKU4cY1Vd5AXBHpYRkubkDLZi",    // 47
    "6tBc3ABLaYTTWu94DiRD5PWi92HML34UpAQ8pPTYgudw",    // 48
    "9WjDVMHWCirG9jkchbetHTnSzdXbAPnD9bsoGRcz1xUw",    // 49
    "CDpiwv9eLsRvvuzZEJ8CBtK14wdvkSnkub4vmGtzzdK8",    // 50
    "DQeN7dZyQvXKT7YwmgqyuC7AYFkwMoP7RwtucsDEdfYZ",    // 51
    "DrdecJVzkaRsf1TQu1g7iFncaokikVTHqpzPjenjRySY",    // 52
    "FMrUDGjEe1izXPbn8SZPNjMfB5JvvhVq5ymmpZDebB5R",    // 53
    "J8u7HvA1g1p2CdhBFdsnTxDzGkekRpdw4GrL9MKU2D3U",    // 54
    "RPxHtdN5V7ajwkoG6NnwSBAeaX5k9giY37dpp98xTjD",     // 55
    "Y6YhgJbt9FRk3JVjwdZtsioVCJwCKhy1hum8HMDYyB1",     // 56
    "BhH6HphjBKXu2PkUc2aw3xEMdUvK14NXxE5LbNWZNZAA",    // 57
    "3f7GcQFG397GAaEnv51zR6tsTVihYRydnydDD1cXekxH",    // 58
    "LanMkFSVSncjWqWAM8MUHenZzt9xTcT3DcAp949ZwbF",     // 59
    "495mQpkX8mHAv18yGsfubCXbFQ9Jok1L1BvMrV9KvCHr",    // 60
    "495mQpkX8mHAv18yGsfubCXbFQ9Jok1L1BvMrV9KvCHr",    // 61
    "7YttLkHDoNj9wyDur5pM1ejNaAvT9X4eqaYcHQqtj2G5",    // 62
    "DNXgeM9EiiaAbaWvwjHj9fQQLAX5ZsfHyvmYUNRAdNC8",    // 63
    "So11111111111111111111111111111111111111112",     // 64
    "WLHv2UAZm6z4KyaaELi5pjdbJh6RESMva1Rnn8pJVVh",     // 65
    "6s1xP3hpbAfFoNtUNF8mfHsjr2Bd97JxFJRWLbL6aHuX",    // 66
    "USD1ttGY1N17NEEHLmELoaybftRBUSErhqYiQzvEmuB",     // 67
    "B5u5x9S5pyaJdonf7bXUiEnBfEXsJWhNxXfLGAbRFtg2",    // 68
    "7NrevmQXcRTRGsgruwTCwWkn9qwA29rXcMwfenWunHP",     // 69
    "52MZUkNsRYzP4gn2mCaLTVtJQy2tpoF79hp5K1HhAxxX",    // 70
    "BgxH5ifebqHDuiADWKhLjXGP5hWZeZLoCdmeWJLkRqLP",    // 71
    "ESLj2Rzmvn3RhDo4Z18hY1wYmGyC9xM4ZtRXhvoFkDAi",    // 72
    "EUZHCdd8H7nueb2wLpxUqyuemdbe8TUxfCWcxVfARvgw",    // 73
    "LNmHRmMvk9kmtepfTSr98kqGLThd61kH1DPWf2cVRaC",     // 74
    "Bq9YmSkZ6f13L4rPsx9MxSXuyPfKCBbvrbvarKti1LdG",    // 75
    "CRRS5ieQmBrZjWhcj99JuGrT5tyuWDaGAXLXLFjbAtjQ",    // 76
    "61GwTFRpjM3emvpnNoMT54oKnmjrQF6m1UQxmZRZQFRZ",    // 77
    "9UQpPzgjju8DhHGcdnGDuXck4VGnHzGhRxT2NmEr6aS8",    // 78
    "EPiZbnrThjyLnoQ6QQzkxeFqyL5uyg9RzNHHAudUPxBz",    // 79
];

#[cfg(test)]
mod tests {
    use super::*;

    #[test]
    fn test_resolve_jupiter_main_entry_23_is_wsol() {
        let alt = pubkey_from_b58("3oy9ojnsDzqmMNi87Gs7Hn5v3MPVqnWjG9k8BmzKR7yW");
        let wsol = pubkey_from_b58("So11111111111111111111111111111111111111112");
        let table = find_table(&alt).unwrap();
        assert_eq!(resolve_entry(Some(table), 23), wsol);
    }

    #[test]
    fn test_resolve_jupiter_main_entry_40_is_usdc() {
        let alt = pubkey_from_b58("3oy9ojnsDzqmMNi87Gs7Hn5v3MPVqnWjG9k8BmzKR7yW");
        let usdc = pubkey_from_b58("EPjFWdd5AufqSSqeM2qN1xzybapC8G4wEGGkZwyTDt1v");
        let table = find_table(&alt).unwrap();
        assert_eq!(resolve_entry(Some(table), 40), usdc);
    }

    #[test]
    fn test_resolve_unknown_alt_returns_unresolved() {
        let unknown = [0x42u8; 32];
        let table = find_table(&unknown);
        assert!(table.is_none());
        assert!(is_unresolved(&resolve_entry(table, 0)));
    }

    #[test]
    fn test_resolve_out_of_bounds_returns_unresolved() {
        let alt = pubkey_from_b58("3oy9ojnsDzqmMNi87Gs7Hn5v3MPVqnWjG9k8BmzKR7yW");
        let table = find_table(&alt);
        assert!(is_unresolved(&resolve_entry(table, 255)));
    }

    #[test]
    fn test_expand_accounts_no_lookups() {
        let static_accts = vec![[1u8; 32], [2u8; 32]];
        let result = expand_accounts(&static_accts, &[]);
        assert_eq!(result.len(), 2);
    }

    #[test]
    fn test_expand_accounts_with_known_alt() {
        let static_accts = vec![[1u8; 32]];
        let alt_key = pubkey_from_b58("3oy9ojnsDzqmMNi87Gs7Hn5v3MPVqnWjG9k8BmzKR7yW");
        let wsol = pubkey_from_b58("So11111111111111111111111111111111111111112");

        let lookups = vec![crate::parser::message::AddressTableLookup {
            account_key: alt_key,
            writable_indices: vec![],
            readonly_indices: vec![23], // WSOL
        }];

        let result = expand_accounts(&static_accts, &lookups);
        assert_eq!(result.len(), 2); // 1 static + 1 readonly
        assert_eq!(result[1], wsol);
    }

    #[test]
    fn test_raydium_clmm_table_exists() {
        let alt = pubkey_from_b58("AcL1Vo8oy1ULiavEcjSUcwfBSForXMudcZvDZy5nzJkU");
        assert!(find_table(&alt).is_some());
    }
}
