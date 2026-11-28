from __future__ import annotations

import json
import re
import subprocess
import sys
import time
import uuid
from pathlib import Path

BUNDLE_ID = "org.ootuniversal.abi-simulator"
RESULT_NAME = "oot-abi-driver-result.txt"
SIMCTL = ["xcrun", "simctl"]
owned_udid: str | None = None


def run(args: list[str], *, timeout: int = 60, check: bool = True) -> str:
    command = SIMCTL + args
    print("$ " + " ".join(command), flush=True)
    try:
        result = subprocess.run(command, check=False, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        raise RuntimeError(f"command timed out after {timeout}s: {' '.join(command)}") from error
    if result.stdout:
        print(result.stdout.rstrip(), flush=True)
    if result.stderr:
        print(result.stderr.rstrip(), file=sys.stderr, flush=True)
    if check and result.returncode:
        raise RuntimeError(f"command exited {result.returncode}: {' '.join(command)}")
    return result.stdout


def discover_pair() -> tuple[str, str, str]:
    runtimes_doc = json.loads(run(["list", "runtimes", "--json"]))
    devices_doc = json.loads(run(["list", "devices", "available", "--json"]))
    types_doc = json.loads(run(["list", "devicetypes", "--json"]))
    runtimes = [runtime for runtime in runtimes_doc.get("runtimes", [])
                if runtime.get("isAvailable") is True
                and runtime.get("identifier", "").startswith("com.apple.CoreSimulator.SimRuntime.iOS-")]
    if not runtimes:
        observed = ", ".join(f"{r.get('identifier', '?')} (available={r.get('isAvailable')})"
                              for r in runtimes_doc.get("runtimes", [])) or "none"
        raise RuntimeError(f"No available iOS Simulator runtime is installed; discovered: {observed}")
    device_types = {device.get("identifier"): device for device in types_doc.get("devicetypes", [])
                    if device.get("productFamily") == "iPhone"
                    or device.get("name", "").startswith("iPhone")}
    if not device_types:
        raise RuntimeError("Available iOS runtimes were found, but simctl reported no compatible iPhone device types")

    # Prefer the newest available runtime, then the alphabetically first iPhone type.
    def version_key(item: dict) -> tuple[tuple[int, ...], str]:
        parts = tuple(int(part) for part in re.findall(r"\d+", item.get("version", "")))
        return parts, item.get("identifier", "")

    runtimes.sort(key=version_key, reverse=True)
    listed_device_runtimes = devices_doc.get("devices", {})
    # Reuse the type/runtime pairing that simctl itself lists as available, then create
    # a separate simulator of that exact type so this run owns its entire lifecycle.
    for runtime in runtimes:
        listed = listed_device_runtimes.get(runtime["identifier"], [])
        candidates = []
        for device in listed:
            type_id = device.get("deviceTypeIdentifier")
            device_type = device_types.get(type_id)
            if device_type and device.get("isAvailable", True) is True:
                candidates.append(device_type)
        candidates.sort(key=lambda item: (item.get("name", ""), item.get("identifier", "")))
        if candidates:
            device_type = candidates[0]
            return runtime["identifier"], device_type["identifier"], device_type.get("name", device_type["identifier"])
    raise RuntimeError("Available iOS runtimes were found, but none has an available compatible iPhone type")


def main() -> None:
    global owned_udid
    if len(sys.argv) != 2:
        raise RuntimeError("usage: run_ios_simulator.py APP_PATH")
    app = Path(sys.argv[1]).resolve(strict=True)
    if not app.is_dir() or app.suffix != ".app":
        raise RuntimeError(f"expected a packaged .app directory: {app}")

    runtime_id, device_type_id, device_name = discover_pair()
    print(f"Selected available runtime {runtime_id} and iPhone type {device_name} ({device_type_id})")
    created = run(["create", "OOT ABI CI fresh device", device_type_id, runtime_id], timeout=60).strip()
    if not re.fullmatch(r"[0-9A-Fa-f]{8}-(?:[0-9A-Fa-f]{4}-){3}[0-9A-Fa-f]{12}", created):
        raise RuntimeError(f"simctl create did not return a simulator UDID: {created!r}")
    try:
        owned_udid = str(uuid.UUID(created))
    except ValueError as error:
        raise RuntimeError(f"simctl create returned an invalid simulator UUID: {created!r}") from error
    try:
        run(["boot", owned_udid], timeout=60)
        run(["bootstatus", owned_udid, "-b"], timeout=180)
        run(["install", owned_udid, str(app)], timeout=120)
        run(["launch", owned_udid, BUNDLE_ID], timeout=60)
        container_output = run(["get_app_container", owned_udid, BUNDLE_ID, "data"], timeout=60).strip()
        if not container_output:
            raise RuntimeError("simctl returned an empty app data container path")
        container = Path(container_output.splitlines()[-1])
        if not container.is_absolute() or not container.is_dir():
            raise RuntimeError(f"simctl returned an invalid or missing app data container: {container_output!r}")
        result_file = container / "Documents" / RESULT_NAME
        deadline = time.monotonic() + 120
        while time.monotonic() < deadline:
            try:
                contents = result_file.read_bytes()
            except FileNotFoundError:
                time.sleep(1)
                continue
            result_match = re.fullmatch(rb"OOT_ABI_DRIVER_RESULT=(-?\d+)\n", contents)
            if not result_match:
                raise RuntimeError(f"driver result file has unexpected contents: {contents!r}")
            if contents == b"OOT_ABI_DRIVER_RESULT=0\n":
                print("PASS: UIKit app launched and the C/Rust ABI driver returned 0", flush=True)
                return
            driver_result = int(result_match.group(1))
            if driver_result != 0:
                raise RuntimeError(f"iOS simulator app reported nonzero ABI driver result {driver_result}")
            raise RuntimeError(f"driver result file did not match the exact success marker: {contents!r}")
        raise RuntimeError(f"timed out waiting 120s for {RESULT_NAME} from the launched app")
    finally:
        # The UUID was created by this run; delete only that simulator.
        active_error = sys.exc_info()[0] is not None
        try:
            cleanup = subprocess.run(SIMCTL + ["delete", owned_udid], check=False,
                                     capture_output=True, text=True, timeout=30)
            if cleanup.stdout:
                print(cleanup.stdout.rstrip(), flush=True)
            if cleanup.stderr:
                print(cleanup.stderr.rstrip(), file=sys.stderr, flush=True)
            if cleanup.returncode:
                raise RuntimeError(f"failed to delete owned simulator {owned_udid}: {cleanup.stderr.strip()}")
            print(f"Removed owned simulator {owned_udid}", flush=True)
        except Exception as cleanup_error:
            if not active_error:
                raise
            print(f"ERROR: simulator cleanup also failed: {cleanup_error}", file=sys.stderr, flush=True)
        finally:
            owned_udid = None


if __name__ == "__main__":
    try:
        main()
    except Exception as error:
        print(f"ERROR: {error}", file=sys.stderr, flush=True)
        sys.exit(1)
