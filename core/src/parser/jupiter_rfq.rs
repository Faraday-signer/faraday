//! Jupiter Order Engine / RFQ (`61DFfeTKM7trxYcPQCM78bJ794ddZprZpAwAnLiwTpYH`).
//!
//! Two-signer RFQ flow: user signs intent, market-maker co-signs to fill.
//! No public IDL as of writing — `fill` discriminator and amount layout
//! reverse-engineered from real txs. We surface the raw u64s without
//! denominating: without the IDL we cannot reliably tell which u64 is the
//! input vs output amount, and a hardware wallet must not guess.

use crate::parser::anchor;
use crate::parser::bytes::{read_disc8, read_u64_le};
use crate::parser::{ParsedInstruction, ReviewItem};

pub fn parse(data: &[u8], _account_indices: &[u8], _all_accounts: &[[u8; 32]]) -> ParsedInstruction {
    let disc = match read_disc8(data, 0) {
        Ok(d) => d,
        Err(_) => return error_ix("Jupiter RFQ data too short for discriminator"),
    };

    if disc == anchor::discriminator("fill") {
        return parse_fill(data);
    }

    ParsedInstruction {
        program: "Jupiter RFQ".into(),
        items: vec![
            ReviewItem::Header(format!(
                "Jupiter RFQ: unknown action ({:02x}{:02x}{:02x}{:02x}…)",
                disc[0], disc[1], disc[2], disc[3]
            )),
            ReviewItem::Warning(
                "Decoder doesn't recognize this Jupiter RFQ instruction yet.".into(),
            ),
        ],
    }
}

fn parse_fill(data: &[u8]) -> ParsedInstruction {
    let mut items: Vec<ReviewItem> = Vec::new();
    items.push(ReviewItem::Header("Jupiter RFQ fill".into()));

    if let (Ok(a), Ok(b)) = (read_u64_le(data, 8), read_u64_le(data, 16)) {
        items.push(ReviewItem::Field {
            label: "Amount 1 (raw)".into(),
            value: a.to_string(),
        });
        items.push(ReviewItem::Field {
            label: "Amount 2 (raw)".into(),
            value: b.to_string(),
        });
    }
    items.push(ReviewItem::Warning(
        "RFQ fill — market maker is completing your order. Decimals/symbols not decoded; verify amounts on dApp."
            .into(),
    ));
    ParsedInstruction {
        program: "Jupiter RFQ".into(),
        items,
    }
}

fn error_ix(msg: &'static str) -> ParsedInstruction {
    ParsedInstruction {
        program: "Jupiter RFQ".into(),
        items: vec![ReviewItem::Warning(msg.into())],
    }
}
