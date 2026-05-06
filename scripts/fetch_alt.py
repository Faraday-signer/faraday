#!/usr/bin/env python3
"""
Fetch a Solana Address Lookup Table from chain, verify it's FROZEN
(authority = None, immutable), and emit Rust constant code for
`src/parser/lookup_tables.rs`.

Hardware-wallet invariant: we only trust ATLs whose on-chain content
cannot change after our snapshot. That requires the ATL's authority to
be None — at which point the table is permanently fixed and our hash
of its content matches forever.

Usage:
    scripts/fetch_alt.py <ALT_PUBKEY> [--rpc URL]

Default RPC: https://api.mainnet-beta.solana.com
"""

import argparse
import base64
import json
import sys
import urllib.request


SOLANA_RPC = "https://api.mainnet-beta.solana.com"

# AddressLookupTable account layout (from solana-sdk):
#   [0..4]    discriminator (variant tag = 1 for LookupTable)
#   [4..12]   deactivation_slot: u64
#   [12..20]  last_extended_slot: u64
#   [20..21]  last_extended_slot_start_index: u8
#   [21..22]  authority option discriminator (0 = None, 1 = Some)
#   [22..54]  authority pubkey (only present if option = 1)
#   [54..56]  padding (alignment)
#   [56..]    address entries (32 bytes each)
META_SIZE = 56


def b58_encode(data: bytes) -> str:
    alpha = b"123456789ABCDEFGHJKLMNPQRSTUVWXYZabcdefghijkmnopqrstuvwxyz"
    n = int.from_bytes(data, "big")
    out = b""
    while n > 0:
        n, rem = divmod(n, 58)
        out = alpha[rem : rem + 1] + out
    leading = 0
    for byte in data:
        if byte == 0:
            leading += 1
        else:
            break
    return (b"1" * leading + out).decode()


def fetch_account(pubkey: str, rpc: str) -> bytes:
    req = {
        "jsonrpc": "2.0",
        "id": 1,
        "method": "getAccountInfo",
        "params": [pubkey, {"encoding": "base64"}],
    }
    body = json.dumps(req).encode()
    request = urllib.request.Request(
        rpc, data=body, headers={"Content-Type": "application/json"}
    )
    with urllib.request.urlopen(request, timeout=15) as resp:
        result = json.loads(resp.read())
    if "error" in result:
        raise RuntimeError(f"RPC error: {result['error']}")
    value = result["result"]["value"]
    if value is None:
        raise RuntimeError(f"Account {pubkey} not found on chain")
    data_b64, encoding = value["data"]
    if encoding != "base64":
        raise RuntimeError(f"Unexpected encoding: {encoding}")
    return base64.b64decode(data_b64)


def parse_alt(data: bytes) -> tuple[bool, list[str]]:
    if len(data) < META_SIZE:
        raise RuntimeError(f"Account data too short ({len(data)} < {META_SIZE})")
    discriminator = int.from_bytes(data[0:4], "little")
    if discriminator != 1:
        raise RuntimeError(
            f"Not an AddressLookupTable account (discriminator = {discriminator})"
        )
    deactivation_slot = int.from_bytes(data[4:12], "little")
    authority_tag = data[21]
    if authority_tag not in (0, 1):
        raise RuntimeError(f"Invalid authority option tag: {authority_tag}")
    is_frozen = authority_tag == 0
    if not is_frozen:
        authority_pk = b58_encode(data[22:54])
        print(
            f"  ⚠ ATL is NOT frozen — authority = {authority_pk}",
            file=sys.stderr,
        )
    if deactivation_slot != 0xFFFFFFFFFFFFFFFF:
        print(
            f"  ⚠ ATL is deactivated (slot {deactivation_slot})",
            file=sys.stderr,
        )

    addresses_raw = data[META_SIZE:]
    if len(addresses_raw) % 32 != 0:
        raise RuntimeError(
            f"Addresses region length {len(addresses_raw)} not a multiple of 32"
        )
    n = len(addresses_raw) // 32
    addresses = [
        b58_encode(addresses_raw[i * 32 : (i + 1) * 32]) for i in range(n)
    ]
    return is_frozen, addresses


def emit_rust(pubkey: str, addresses: list[str]) -> str:
    name = "TODO_NAME_ME"
    lines = [
        f"const {name}: &[&str] = &[",
    ]
    for i, addr in enumerate(addresses):
        lines.append(f'    "{addr}",  // {i}')
    lines.append("];")
    lines.append("")
    lines.append(
        f'        "{pubkey}" => Some({name}),  // add to find_table()'
    )
    return "\n".join(lines)


def main() -> int:
    parser = argparse.ArgumentParser(description=__doc__)
    parser.add_argument("pubkey", help="ATL account pubkey (base58)")
    parser.add_argument("--rpc", default=SOLANA_RPC, help="Solana RPC URL")
    parser.add_argument(
        "--allow-mutable",
        action="store_true",
        help="Emit Rust code even if ATL is not frozen (dangerous — for testing only)",
    )
    args = parser.parse_args()

    try:
        data = fetch_account(args.pubkey, args.rpc)
        is_frozen, addresses = parse_alt(data)
    except Exception as exc:
        print(f"error: {exc}", file=sys.stderr)
        return 1

    print(
        f"# {args.pubkey}: {len(addresses)} entries, "
        f"frozen={'yes' if is_frozen else 'NO'}",
        file=sys.stderr,
    )

    if not is_frozen and not args.allow_mutable:
        print(
            "refusing to emit code for a mutable ATL — its content can change "
            "on-chain and our hardcoded snapshot would become stale, exposing "
            "users to display-vs-execution mismatches. Pass --allow-mutable "
            "to override (NOT recommended for shipped firmware).",
            file=sys.stderr,
        )
        return 2

    print(emit_rust(args.pubkey, addresses))
    return 0


if __name__ == "__main__":
    sys.exit(main())
