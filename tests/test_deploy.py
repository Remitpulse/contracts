"""Deployment orchestration tests. The CLI is stubbed; no RPC is contacted."""
import json
import os
from pathlib import Path
import subprocess
import tempfile
import tomllib
import unittest

ROOT = Path(__file__).resolve().parents[1]
PUBLIC_KEY = ("04" + "6b17d1f2e12c4247f8bce6e563a440f277037d812deb33a0f4a13945d898c296"
              + "4fe342e2fe1a7f9b8ee7eb4a7c0f9e162bce33576b315ececbb6406837bf51f5")
CONFIG = {"keys": [PUBLIC_KEY], "threshold": 1, "rp_id_hash": "00" * 32,
          "origin": "https://remitpulse.example".encode().hex()}
CONFIG_JSON = json.dumps(CONFIG)



class DeploymentTests(unittest.TestCase):
    def run_deploy(self, network="testnet", component="core", missing=None, fail=False, toolchain="stable", overrides=None):
        with tempfile.TemporaryDirectory() as directory:
            tmp = Path(directory)
            cli = tmp / "stellar"
            cli.write_text("""#!/usr/bin/env python3
import json, os, sys
with open(os.environ['CLI_LOG'], 'a') as f:
    f.write(json.dumps(sys.argv[1:]) + '\\n')
if sys.argv[1:3] == ['contract', 'build']:
    with open(os.environ['BUILD_TOOLCHAIN_LOG'], 'w') as f:
        f.write(os.environ.get('RUSTUP_TOOLCHAIN', ''))
if os.environ.get('FAIL_BUILD') and sys.argv[1:3] == ['contract', 'build']:
    sys.exit(1)
if sys.argv[1:3] == ['contract', 'deploy']:
    print('C_TEST_CONTRACT_ID')
""")
            cli.chmod(0o755)
            env = {**os.environ, "STELLAR_CLI": str(cli), "CLI_LOG": str(tmp / "calls"),
                   "SOURCE_ACCOUNT": "deployer", "ADMIN": "admin",
                   "BUILD_TOOLCHAIN_LOG": str(tmp / "toolchain"),
                   "RUSTUP_TOOLCHAIN": toolchain,
                   "ACCOUNT_CONFIG": CONFIG_JSON, "ROOT_HASH": "00" * 32,
                   "TOKEN": "asset", "FUNDER": "funder", "OWNER": "owner",
                   "MAX_SUPPLY": "10", "METADATA": '{"base_uri":"ipfs://remitpulse/", "name":"RemitPulse", "symbol":"RP"}'}
            env.update(overrides or {})
            if missing:
                env.pop(missing, None)
            if fail:
                env["FAIL_BUILD"] = "1"
            before = set((ROOT / "deployments").glob("*.env"))
            result = subprocess.run([str(ROOT / "deploy.sh"), network, component],
                                    env=env, text=True, capture_output=True)
            calls = [json.loads(line) for line in (tmp / "calls").read_text().splitlines()] if (tmp / "calls").exists() else []
            if calls and calls[0] == ["contract", "build"]:
                expected = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
                self.assertEqual((tmp / "toolchain").read_text(), expected)
            records = set((ROOT / "deployments").glob("*.env")) - before
            saved = [p.read_text() for p in records]
            for p in records:
                p.unlink()
            return result, calls, saved

    def test_build_entry_points_override_inherited_toolchain(self):
        expected = tomllib.loads((ROOT / "rust-toolchain.toml").read_text())["toolchain"]["channel"]
        with tempfile.TemporaryDirectory() as directory:
            tmp = Path(directory)
            cli = tmp / "stellar"
            cli.write_text("#!/bin/sh\nprintf '%s' \"$RUSTUP_TOOLCHAIN\" > \"$BUILD_TOOLCHAIN_LOG\"\n")
            cli.chmod(0o755)
            for inherited in ("stable", "1.85.0"):
                env = {**os.environ, "STELLAR_CLI": str(cli), "RUSTUP_TOOLCHAIN": inherited,
                       "BUILD_TOOLCHAIN_LOG": str(tmp / "toolchain")}
                for command in ([str(ROOT / "scripts/build.sh")],
                                ["make", "-C", str(ROOT), "build"]):
                    result = subprocess.run(command, cwd=tmp, env=env, text=True, capture_output=True)
                    self.assertEqual(result.returncode, 0, result.stderr)
                    self.assertEqual((tmp / "toolchain").read_text(), expected)

    def test_core_constructor_arguments_and_network(self):
        result, calls, records = self.run_deploy("mainnet")
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertEqual(calls[0], ["contract", "build"])
        self.assertEqual(len(calls), 5)
        for call in calls[1:]:
            self.assertEqual(call[call.index("--network") + 1], "mainnet")
        self.assertEqual(calls[3][-3:], ["--", "--config", CONFIG_JSON])
        self.assertEqual(calls[4][-3:], ["--", "--admin", "admin"])
        self.assertIn("REMITPULSE_ACCOUNT=C_TEST_CONTRACT_ID", records[0])

    def test_preflight_and_build_failures_never_deploy(self):
        result, calls, _ = self.run_deploy(missing="ACCOUNT_CONFIG")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(calls, [])
        result, calls, _ = self.run_deploy(fail=True)
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(calls, [["contract", "build"]])
        result, calls, _ = self.run_deploy(network="unknown")
        self.assertNotEqual(result.returncode, 0)
        self.assertEqual(calls, [])

    def test_invalid_constructor_configuration_never_builds_or_deploys(self):
        for config in ["not json", json.dumps({**CONFIG, "threshold": True}),
                       json.dumps({**CONFIG, "keys": ["04" * 65]}),
                       json.dumps({**CONFIG, "keys": [PUBLIC_KEY, PUBLIC_KEY]}),
                       json.dumps({**CONFIG, "rp_id_hash": "00"}),
                       json.dumps({**CONFIG, "origin": "http://example.com".encode().hex()}),
                       json.dumps({**CONFIG, "origin": "https://example.com/path".encode().hex()})]:
            result, calls, records = self.run_deploy(overrides={"ACCOUNT_CONFIG": config})
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn("Deployment configuration invalid", result.stderr)
            self.assertEqual(calls, [])
            self.assertEqual(records, [])
        for component, overrides in [("airdrop", {"ROOT_HASH": "bad"}),
                                     ("nft", {"MAX_SUPPLY": "-1"}),
                                     ("nft", {"METADATA": "{}"})]:
            result, calls, _ = self.run_deploy(component=component, overrides=overrides)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertEqual(calls, [])

    def test_optional_contract_constructors(self):
        for component, arguments in [("airdrop", ["--root_hash", "--token", "--admin", "--funder"]),
                                     ("nft", ["--owner", "--max_supply", "--metadata"])]:
            result, calls, _ = self.run_deploy(component=component)
            self.assertEqual(result.returncode, 0, result.stderr)
            self.assertEqual(len(calls), 2)
            for arg in arguments:
                self.assertIn(arg, calls[1])

    def test_build_script_honours_output_directory(self):
        with tempfile.TemporaryDirectory() as directory:
            tmp = Path(directory)
            cli = tmp / "stellar"
            cli.write_text("""#!/usr/bin/env python3
import os, sys
with open(os.environ['CLI_LOG'], 'a') as f:
    f.write(' '.join(sys.argv[1:]) + '\\n')
if sys.argv[1:3] == ['contract', 'build']:
    out = sys.argv[sys.argv.index('--out-dir') + 1]
    os.makedirs(out, exist_ok=True)
    for name in ('remitpulse_core', 'remitpulse_airdrop', 'remitpulse_nft'):
        with open(os.path.join(out, name + '.wasm'), 'wb') as f:
            f.write(b'\\0asm')
""")
            cli.chmod(0o755)
            outputs = []
            for name in ("first", "second"):
                out = tmp / name
                env = {**os.environ, "STELLAR_CLI": str(cli), "CLI_LOG": str(tmp / "calls"),
                       "BUILD_OUT_DIR": str(out)}
                result = subprocess.run([str(ROOT / "scripts/build.sh")], cwd=tmp, env=env,
                                        text=True, capture_output=True)
                self.assertEqual(result.returncode, 0, result.stderr)
                outputs.append(out)
            for out in outputs:
                self.assertTrue(out.is_dir())
                self.assertEqual(sorted(p.name for p in out.glob("*.wasm")),
                                 ["remitpulse_airdrop.wasm", "remitpulse_core.wasm",
                                  "remitpulse_nft.wasm"])
            calls = (tmp / "calls").read_text().splitlines()
            self.assertEqual(len(calls), 2)
            for call, out in zip(calls, outputs):
                self.assertIn("--out-dir " + str(out), call)

    def test_deployment_requires_no_secret_values(self):
        # The checklist forbids committing secrets; deployment must not read any
        # private key material from the environment or the repository.
        result, calls, _ = self.run_deploy()
        self.assertEqual(result.returncode, 0, result.stderr)
        self.assertTrue(calls)
        for name in ("SECRET_KEY", "PRIVATE_KEY", "STELLAR_SECRET", "SIGNING_KEY"):
            self.assertNotIn(name, os.environ.get("DEPLOY_REQUIRED_VARS", ""))
        deploy_script = (ROOT / "deploy.sh").read_text()
        for name in ("SECRET_KEY", "PRIVATE_KEY", "STELLAR_SECRET"):
            self.assertNotIn(name, deploy_script)
        for path in (ROOT / "deployments").glob("*.env"):
            self.assertNotIn("SECRET", path.read_text().upper())

    def test_unknown_component_names_are_rejected(self):
        for component in ["bogus", "Core", "", "core ", "nft2"]:
            result, calls, records = self.run_deploy(component=component)
            self.assertEqual(result.returncode, 2, result.stderr)
            self.assertIn("Unknown component", result.stderr)
            self.assertEqual(calls, [])
            self.assertEqual(records, [])

    def test_missing_required_environment_values_are_reported(self):
        cases = [
            ("core", "SOURCE_ACCOUNT"),
            ("core", "ADMIN"),
            ("core", "ACCOUNT_CONFIG"),
            ("account", "ACCOUNT_CONFIG"),
            ("subsidy", "SOURCE_ACCOUNT"),
            ("subsidy", "ADMIN"),
            ("airdrop", "SOURCE_ACCOUNT"),
            ("airdrop", "ADMIN"),
            ("airdrop", "ROOT_HASH"),
            ("airdrop", "TOKEN"),
            ("airdrop", "FUNDER"),
            ("nft", "SOURCE_ACCOUNT"),
            ("nft", "OWNER"),
            ("nft", "MAX_SUPPLY"),
            ("nft", "METADATA"),
        ]
        for component, variable in cases:
            with self.subTest(component=component, variable=variable):
                result, calls, records = self.run_deploy(component=component, missing=variable)
                self.assertNotEqual(result.returncode, 0, result.stderr)
                self.assertIn(variable, result.stderr)
                self.assertEqual(calls, [])
                self.assertEqual(records, [])

    def test_malformed_metadata_is_rejected(self):
        for metadata in ["not json", "[]", "null", '"string"',
                         "{}",
                         '{"base_uri":"ipfs://remitpulse/", "name":"RemitPulse"}',
                         '{"base_uri":"", "name":"RemitPulse", "symbol":"RP"}',
                         '{"base_uri":"ipfs://remitpulse/", "name":"", "symbol":"RP"}',
                         '{"base_uri":"ipfs://remitpulse/", "name":"RemitPulse", "symbol":""}',
                         '{"base_uri":1, "name":"RemitPulse", "symbol":"RP"}',
                         '{"base_uri":"ipfs://remitpulse/", "name":2, "symbol":"RP"}',
                         '{"base_uri":"ipfs://remitpulse/", "name":"RemitPulse", "symbol":3}',
                         '{"base_uri":"ipfs://remitpulse/", "name":"RemitPulse", "symbol":"RP", "extra":"x"}']:
            with self.subTest(metadata=metadata):
                result, calls, records = self.run_deploy(component="nft",
                                                         overrides={"METADATA": metadata})
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn("Deployment configuration invalid", result.stderr)
                self.assertEqual(calls, [])
                self.assertEqual(records, [])

    def test_rejected_network_selection(self):
        for network in ["devnet", "mainnet ", "", "Mainnet", "futurenet"]:
            with self.subTest(network=network):
                result, calls, records = self.run_deploy(network=network)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn("Network must be testnet or mainnet", result.stderr)
                self.assertEqual(calls, [])
                self.assertEqual(records, [])

    def test_validate_deployment_script_reports_invalid_configuration(self):
        cases = [
            {"ACCOUNT_CONFIG": "not json"},
            {"ACCOUNT_CONFIG": json.dumps({**CONFIG, "threshold": 0})},
            {"ROOT_HASH": "bad"},
            {"MAX_SUPPLY": "-1"},
            {"MAX_SUPPLY": "not-a-number"},
            {"METADATA": "{}"},
            {"METADATA": "not json"},
        ]
        for overrides in cases:
            with self.subTest(overrides=overrides):
                env = {**os.environ, "ACCOUNT_CONFIG": CONFIG_JSON, "ROOT_HASH": "00" * 32,
                       "MAX_SUPPLY": "10",
                       "METADATA": '{"base_uri":"ipfs://remitpulse/", "name":"RemitPulse", "symbol":"RP"}'}
                env.update(overrides)
                result = subprocess.run([str(ROOT / "scripts/validate_deployment.py")],
                                        env=env, text=True, capture_output=True)
                self.assertEqual(result.returncode, 2, result.stderr)
                self.assertIn("Deployment configuration invalid", result.stderr)

    def test_invalid_addresses_never_build_or_deploy(self):
        # Malformed strkeys and identities must be rejected before any CLI call.
        bad_addresses = [
            "",
            " ",
            " deployer",
            "deployer ",
            "G" + "A" * 55,          # wrong length
            "G" + "A" * 57,          # wrong length
            "G" + "0" * 55,          # invalid base32 characters
            "G" + "A" * 54 + "1",    # bad checksum
            "C" + "A" * 55,          # wrong length contract strkey
            "S" + "A" * 55,          # secret key form is not accepted
            "not a valid identity!",
        ]
        for variable in ("SOURCE_ACCOUNT", "ADMIN", "TOKEN", "FUNDER", "OWNER"):
            for value in bad_addresses:
                with self.subTest(variable=variable, value=value):
                    result, calls, records = self.run_deploy(overrides={variable: value})
                    self.assertEqual(result.returncode, 2, result.stderr)
                    self.assertIn("Deployment configuration invalid", result.stderr)
                    self.assertEqual(calls, [])
                    self.assertEqual(records, [])

    def test_valid_addresses_are_accepted(self):
        # Well-formed G/C strkeys and plain identity names must pass validation.
        valid_addresses = [
            "deployer",
            "admin",
            "funder",
            "owner",
            "G" + "A" * 55,
            "C" + "A" * 55,
        ]
        for variable in ("SOURCE_ACCOUNT", "ADMIN", "TOKEN", "FUNDER", "OWNER"):
            for value in valid_addresses:
                with self.subTest(variable=variable, value=value):
                    env = {**os.environ, "ACCOUNT_CONFIG": CONFIG_JSON, "ROOT_HASH": "00" * 32,
                           "MAX_SUPPLY": "10",
                           "METADATA": '{"base_uri":"ipfs://remitpulse/", "name":"RemitPulse", "symbol":"RP"}'}
                    env.update({"SOURCE_ACCOUNT": "deployer", "ADMIN": "admin",
                                "TOKEN": "asset", "FUNDER": "funder", "OWNER": "owner"})
                    env[variable] = value
                    result = subprocess.run([str(ROOT / "scripts/validate_deployment.py")],
                                            env=env, text=True, capture_output=True)
                    self.assertEqual(result.returncode, 0, result.stderr)


if __name__ == "__main__":
    unittest.main()
