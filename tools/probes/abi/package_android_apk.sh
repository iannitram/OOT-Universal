#!/usr/bin/env bash
set -euo pipefail

# Builds and verifies a synthetic ARM64 NativeActivity APK. This does not run it.
repo_root="$(cd "$(dirname "${BASH_SOURCE[0]}")/../../.." && pwd)"
cd "$repo_root"

if [[ "$(uname -s)-$(uname -m)" != "Linux-x86_64" ]]; then
  echo "Android APK packaging currently requires Linux x86_64." >&2
  exit 2
fi
ndk_root="${ANDROID_NDK_HOME:-${ANDROID_NDK_LATEST_HOME:-}}"
sdk_root="${ANDROID_SDK_ROOT:-${ANDROID_HOME:-}}"
if [[ -z "$ndk_root" || -z "$sdk_root" ]]; then
  echo "Set ANDROID_NDK_HOME and ANDROID_SDK_ROOT to installed Android tools." >&2
  exit 2
fi
toolchain="$ndk_root/toolchains/llvm/prebuilt/linux-x86_64/bin"
clang="$toolchain/aarch64-linux-android24-clang"
readelf="$toolchain/llvm-readelf"
nm="$toolchain/llvm-nm"
[[ -x "$clang" && -x "$readelf" && -x "$nm" ]] || {
  echo "NDK lacks the ARM64 API 24 compiler or LLVM inspection tools." >&2; exit 2;
}
build_tools_dir="$(find "$sdk_root/build-tools" -mindepth 1 -maxdepth 1 -type d -printf '%f\n' | sort -V | tail -n 1)"
platform_dir="$(find "$sdk_root/platforms" -mindepth 1 -maxdepth 1 -type d -name 'android-*' -printf '%f\n' | sort -V | tail -n 1)"
[[ -n "$build_tools_dir" && -n "$platform_dir" ]] || {
  echo "Android SDK needs installed build-tools and platform android.jar." >&2; exit 2;
}
build_tools="$sdk_root/build-tools/$build_tools_dir"
aapt2="$build_tools/aapt2"
zipalign="$build_tools/zipalign"
apksigner="$build_tools/apksigner"
android_jar="$sdk_root/platforms/$platform_dir/android.jar"
for tool in "$aapt2" "$zipalign" "$apksigner" "$android_jar"; do
  [[ -f "$tool" ]] || { echo "Missing Android SDK component: $tool" >&2; exit 2; }
done
printf 'NDK: %s\nSDK platform: %s\nBuild-tools: %s\n' "$ndk_root" "$platform_dir" "$build_tools_dir"

output_dir="$repo_root/target/android-apk"
mkdir -p "$output_dir"
output_apk="$output_dir/OotAbiProbe.apk"
if [[ -e "$output_apk" || -L "$output_apk" ]]; then
  echo "Refusing to replace existing APK: $output_apk" >&2
  exit 1
fi
work_dir="$(mktemp -d "${TMPDIR:-/tmp}/oot-android-apk.XXXXXX")"
trap 'rm -rf "$work_dir"' EXIT

export CARGO_TARGET_AARCH64_LINUX_ANDROID_LINKER="$clang"
cargo +1.100.0 build --locked --manifest-path tools/probes/abi/Cargo.toml \
  --release --target aarch64-linux-android --target-dir "$work_dir/target"

"$clang" -std=c11 -Wall -Wextra -Werror -Dmain=oot_abi_driver_main \
  -Itools/probes/abi -c tools/probes/abi/driver.c -o "$work_dir/abi_driver.o"
"$clang" -std=c11 -Wall -Wextra -Werror -fPIC -shared \
  -Wl,--no-undefined -Wl,-z,relro,-z,now -Wl,-soname,liboot_abi_probe.so \
  tools/probes/abi/android/NativeActivity.c "$work_dir/abi_driver.o" \
  "$work_dir/target/aarch64-linux-android/release/liboot_abi_probe.a" \
  -ldl -llog -lunwind -ldl -lm -lc -o "$work_dir/liboot_abi_probe.so"

header="$("$readelf" -h "$work_dir/liboot_abi_probe.so")"
grep -Eq 'Class:[[:space:]]+ELF64' <<<"$header"
grep -Eq 'Machine:[[:space:]]+AArch64' <<<"$header"
grep -Eq 'Type:[[:space:]]+DYN' <<<"$header"
symbols="$("$nm" -D --defined-only "$work_dir/liboot_abi_probe.so")"
grep -Eq '[[:space:]]ANativeActivity_onCreate$' <<<"$symbols"
grep -Eq '[[:space:]]oot_abi_driver_main$' <<<"$symbols"

mkdir -p "$work_dir/apk/lib/arm64-v8a"
cp "$work_dir/liboot_abi_probe.so" "$work_dir/apk/lib/arm64-v8a/"
"$aapt2" link -o "$work_dir/base.apk" -I "$android_jar" \
  --manifest tools/probes/abi/android/AndroidManifest.xml
cp "$work_dir/base.apk" "$work_dir/unsigned.apk"
(cd "$work_dir/apk" && zip -0 -q "$work_dir/unsigned.apk" lib/arm64-v8a/liboot_abi_probe.so)
"$zipalign" -f -p 4 "$work_dir/unsigned.apk" "$work_dir/aligned.apk"

# Credentials and the synthetic private key exist only in the temp directory.
store_password="$(openssl rand -hex 24)"
key_password="$(openssl rand -hex 24)"
keytool -genkeypair -noprompt -keystore "$work_dir/synthetic.jks" \
  -storetype JKS -storepass "$store_password" -keypass "$key_password" \
  -alias synthetic -keyalg RSA -keysize 2048 -validity 2 \
  -dname 'CN=Synthetic ABI Probe, O=Local Test, C=ZZ' >/dev/null 2>&1
"$apksigner" sign --ks "$work_dir/synthetic.jks" --ks-key-alias synthetic \
  --ks-pass "pass:$store_password" --key-pass "pass:$key_password" \
  --out "$work_dir/signed.apk" "$work_dir/aligned.apk"
"$apksigner" verify --verbose --print-certs "$work_dir/signed.apk"
"$zipalign" -c -p 4 "$work_dir/signed.apk"

package_info="$("$aapt2" dump badging "$work_dir/signed.apk")"
grep -Fq "package: name='dev.ootuniversal.abiprobe'" <<<"$package_info"
grep -Fq "launchable-activity: name='android.app.NativeActivity'" <<<"$package_info"
unzip -l "$work_dir/signed.apk" | grep -F 'lib/arm64-v8a/liboot_abi_probe.so'
manifest_tree="$("$aapt2" dump xmltree "$work_dir/signed.apk" --file AndroidManifest.xml)"
grep -Fq 'android.app.lib_name' <<<"$manifest_tree"
grep -Fq 'oot_abi_probe' <<<"$manifest_tree"
install -m 0644 "$work_dir/signed.apk" "$output_apk"
echo "PASS: manifest, arm64-v8a library, NativeActivity entry and APK signature verified"
echo "Built $output_apk; no device execution performed."
