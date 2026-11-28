from __future__ import annotations

import json
import os
import re
import subprocess
import sys
import time
import uuid
from pathlib import Path

BUNDLE_ID = "org.ootuniversal.abi-simulator"
RESULT_NAME = "oot-abi-driver-result.txt"
SIMCTL = ["xcrun", "simctl"]
DISCOVERY_TIMEOUT = 240


class SimulatorUnavailable(RuntimeError):
    """Discovery found no runtime and device type suitable for the probe."""


def run(args: list[str], *, timeout: int = 60) -> str:
    command = SIMCTL + args
    print("$ " + " ".join(command), flush=True)
    try:
        result = subprocess.run(command, check=False, capture_output=True, text=True, timeout=timeout)
    except subprocess.TimeoutExpired as error:
        if error.stdout:
            print(error.stdout.decode(errors="replace").rstrip(), flush=True)
        if error.stderr:
            print(error.stderr.decode(errors="replace").rstrip(), file=sys.stderr, flush=True)
        raise RuntimeError(f"command timed out after {timeout}s: {' '.join(command)}") from error
    if result.stdout:
        print(result.stdout.rstrip(), flush=True)
    if result.stderr:
        print(result.stderr.rstrip(), file=sys.stderr, flush=True)
    if result.returncode:
        raise RuntimeError(f"command exited {result.returncode}: {' '.join(command)}")
    return result.stdout


def discover_pair() -> tuple[str, str, str]:
    # The first simctl request may wait for CoreSimulator setup on a fresh runner.
    runtimes_doc = json.loads(run(["list", "runtimes", "--json"], timeout=DISCOVERY_TIMEOUT))
    runtimes = [runtime for runtime in runtimes_doc.get("runtimes", [])
                if runtime.get("isAvailable") is True
                and runtime.get("identifier", "").startswith("com.apple.CoreSimulator.SimRuntime.iOS-")
                and "arm64" in runtime.get("supportedArchitectures", ["arm64"])]
    if not runtimes:
        raise SimulatorUnavailable("simctl listed no available ARM64 iOS runtime.")

    # Prefer the newest available runtime, then the alphabetically first iPhone type.
    def version_key(item: dict) -> tuple[tuple[int, ...], str]:
        parts = tuple(int(part) for part in re.findall(r"\d+", item.get("version", "")))
        return parts, item.get("identifier", "")

    runtimes.sort(key=version_key, reverse=True)
    listed_device_runtimes = None
    for runtime in runtimes:
        runtime_id = runtime["identifier"]
        devices = runtime.get("supportedDeviceTypes")
        if devices is None:
            # Older simctl output can establish compatibility from existing devices.
            if listed_device_runtimes is None:
                devices_doc = json.loads(run(["list", "devices", "available", "--json"], timeout=DISCOVERY_TIMEOUT))
                listed_device_runtimes = devices_doc.get("devices", {})
            devices = [
                {"identifier": device.get("deviceTypeIdentifier", ""), "name": device.get("name", "")}
                for device in listed_device_runtimes.get(runtime_id, [])
                if device.get("isAvailable", True) is True
            ]
        candidates = [device for device in devices
                      if device.get("identifier", "").startswith("com.apple.CoreSimulator.SimDeviceType.iPhone-")]
        if candidates:
            device_type = min(candidates, key=lambda item: (item.get("name", ""), item["identifier"]))
            return runtime_id, device_type["identifier"], device_type.get("name", device_type["identifier"])
    raise SimulatorUnavailable("simctl listed no compatible iPhone type for its available ARM64 iOS runtimes.")


def main() -> None:
    if len(sys.argv) != 2:
        raise RuntimeError("usage: run_ios_simulator.py APP_PATH")
    app = Path(sys.argv[1]).resolve(strict=True)
    if not app.is_dir() or app.suffix != ".app":
        raise RuntimeError(f"expected a packaged .app directory: {app}")

    runtime_id, device_type_id, device_name = discover_pair()
    print(f"Selected available runtime {runtime_id} and iPhone type {device_name} ({device_type_id})")
    created = run(["create", "OOT ABI CI fresh device", device_type_id, runtime_id]).strip()
    if not re.fullmatch(r"[0-9A-Fa-f]{8}-(?:[0-9A-Fa-f]{4}-){3}[0-9A-Fa-f]{12}", created):
        raise RuntimeError(f"simctl create did not return a simulator UDID: {created!r}")
    owned_udid = str(uuid.UUID(created)).upper()
    try:
        run(["boot", owned_udid], timeout=60)
        run(["bootstatus", owned_udid, "-b"], timeout=300)
        run(["install", owned_udid, str(app)], timeout=180)
        container_output = run(["get_app_container", owned_udid, BUNDLE_ID, "data"], timeout=60).strip()
        if not container_output:
            raise RuntimeError("simctl returned an empty app data container path")
        container = Path(container_output)
        if not container.is_absolute() or not container.is_dir():
            raise RuntimeError(f"simctl returned an invalid or missing app data container: {container_output!r}")
        result_file = container / "Documents" / RESULT_NAME
        if result_file.exists():
            raise RuntimeError("found a driver result in the sandbox before launch")
        run(["launch", owned_udid, BUNDLE_ID], timeout=60)
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
            run(["delete", owned_udid], timeout=30)
            print(f"Removed owned simulator {owned_udid}", flush=True)
        except Exception as cleanup_error:
            if not active_error:
                raise
            print(f"ERROR: simulator cleanup also failed: {cleanup_error}", file=sys.stderr, flush=True)


if __name__ == "__main__":
    try:
        main()
    except SimulatorUnavailable as error:
        message = f"iOS Simulator infrastructure unavailable. {error} See the simctl discovery output above."
        print(message, file=sys.stderr, flush=True)
        if summary := os.environ.get("GITHUB_STEP_SUMMARY"):
            with Path(summary).open("a", encoding="utf-8") as stream:
                print(f"### iOS Simulator infrastructure unavailable\n\n{error} See the simctl discovery log.", file=stream)
        sys.exit(2)
    except Exception as error:
        print(f"ERROR: {error}", file=sys.stderr, flush=True)
        sys.exit(1)
