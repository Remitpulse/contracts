#!/usr/bin/env python3
"""Validate deployment constructor inputs locally before any network fees are spent."""
import json
import os
import sys
from urllib.parse import urlsplit

# NIST P-256: reject malformed/off-curve passkey material before deploying an immutable account.
P = 0xFFFFFFFF00000001000000000000000000000000FFFFFFFFFFFFFFFFFFFFFFFF
B = 0x5AC635D8AA3A93E7B3EBBD55769886BC651D06B0CC53B0F63BCE3C3E27D2604B


def hex_bytes(value, length, label):
    if not isinstance(value, str) or len(value) != 2 * length:
        raise ValueError(f"{label} must contain {length} bytes as hex")
    try:
        decoded = bytes.fromhex(value)
    except ValueError as error:
        raise ValueError(f"{label} must be hex") from error
    if len(decoded) != length:
        raise ValueError(f"{label} must contain {length} bytes as hex")
    return decoded


def account_config(raw):
    try:
        config = json.loads(raw)
    except (TypeError, json.JSONDecodeError) as error:
        raise ValueError(f"ACCOUNT_CONFIG must be valid JSON: {error}") from error
    if not isinstance(config, dict) or set(config) != {"keys", "threshold", "rp_id_hash", "origin"}:
        raise ValueError("ACCOUNT_CONFIG requires keys, threshold, rp_id_hash and origin")
    keys = config["keys"]
    if not isinstance(keys, list) or not 1 <= len(keys) <= 10:
        raise ValueError("ACCOUNT_CONFIG needs 1 to 10 public keys")
    threshold = config["threshold"]
    if type(threshold) is not int or not 1 <= threshold <= len(keys):
        raise ValueError("threshold must be an integer between 1 and the number of keys")
    decoded = []
    for key in keys:
        point = hex_bytes(key, 65, "public key")
        x, y = int.from_bytes(point[1:33], "big"), int.from_bytes(point[33:], "big")
        if point[0] != 4 or x >= P or y >= P or (y * y - (x * x * x - 3 * x + B)) % P:
            raise ValueError("public key must be an uncompressed point on P-256")
        decoded.append(point)
    if len(set(decoded)) != len(decoded):
        raise ValueError("duplicate public keys")
    hex_bytes(config["rp_id_hash"], 32, "rp_id_hash")
    raw_origin = config["origin"]
    if not isinstance(raw_origin, str) or len(raw_origin) > 1024:
        raise ValueError("origin must be UTF-8 hex, at most 512 bytes")
    try:
        origin = bytes.fromhex(raw_origin).decode("utf-8")
    except (ValueError, UnicodeDecodeError) as error:
        raise ValueError("origin must be UTF-8 hex, at most 512 bytes") from error
    url = urlsplit(origin)
    if (url.scheme != "https" or not url.hostname or url.path or url.query or url.fragment
            or url.username is not None or url.password is not None
            or any(c.isspace() or ord(c) < 32 for c in origin) or "\\" in origin):
        raise ValueError("origin must be an HTTPS origin without path, credentials, query or fragment")
    # Also rejects invalid/out-of-range ports.
    try:
        _ = url.port
    except ValueError as error:
        raise ValueError("origin must be an HTTPS origin without path, credentials, query or fragment") from error


def validate(component, env):
    if component not in ("core", "account", "airdrop", "nft"):
        raise ValueError(f"unknown component {component!r}; expected core, account, airdrop or nft")
    if component in ("core", "account"):
        account_config(env["ACCOUNT_CONFIG"])
    if component == "airdrop":
        hex_bytes(env["ROOT_HASH"], 32, "ROOT_HASH")
        try:
            claim_deadline = int(env["CLAIM_DEADLINE"])
        except (TypeError, ValueError) as error:
            raise ValueError("CLAIM_DEADLINE must be a positive integer ledger sequence") from error
        if claim_deadline <= 0:
            raise ValueError("CLAIM_DEADLINE must be a positive integer ledger sequence")
    if component == "nft":
        try:
            supply = int(env["MAX_SUPPLY"])
        except (TypeError, ValueError) as error:
            raise ValueError("MAX_SUPPLY must be a positive u32") from error
        if not 0 < supply <= 0xFFFFFFFF:
            raise ValueError("MAX_SUPPLY must be a positive u32")
        try:
            metadata = json.loads(env["METADATA"])
        except (TypeError, json.JSONDecodeError) as error:
            raise ValueError(f"METADATA must be valid JSON: {error}") from error
        if not isinstance(metadata, dict) or set(metadata) != {"base_uri", "name", "symbol"}:
            raise ValueError("METADATA requires base_uri, name and symbol")
        if not all(isinstance(v, str) and v for v in metadata.values()):
            raise ValueError("metadata values must be nonempty strings")


if __name__ == "__main__":
    try:
        validate(sys.argv[1], os.environ)
    except KeyError as error:
        print(f"Deployment configuration invalid: missing required environment variable {error}", file=sys.stderr)
        sys.exit(2)
    except IndexError:
        print("Deployment configuration invalid: missing component argument", file=sys.stderr)
        sys.exit(2)
    except ValueError as error:
        print(f"Deployment configuration invalid: {error}", file=sys.stderr)
        sys.exit(2)
