#!/bin/sh
set -eu

# Build an isolated source copy; never rewrite the host's Cargo.lock or source.
mkdir -p /root/src/src /root/src/assets/generated /root/src/assets/models
cp /project/Cargo.toml /project/Cargo.lock /root/src/
cp -r /project/src/. /root/src/src/
cp /project/assets/generated/*.glb /root/src/assets/generated/
cp /project/assets/models/ball.glb /root/src/assets/models/
cd /root/src
cargo quad-apk build --release

apk=target/android-artifacts/release/apk/game.apk
activity=target/android-artifacts/release/bin/game/sk/fotbaltojehra/game/MainActivity.java

# Check the actual package, signature and JNI loader, as in RicochetRobots.
grep -F 'System.loadLibrary("game")' "$activity"
unzip -Z1 "$apk" | grep -Fx 'lib/arm64-v8a/libgame.so'
"$ANDROID_HOME/build-tools/36.0.0/aapt" dump badging "$apk" > /tmp/game-badging.txt
grep -Fx "targetSdkVersion:'36'" /tmp/game-badging.txt
grep -Fx "sdkVersion:'26'" /tmp/game-badging.txt
"$ANDROID_HOME/build-tools/36.0.0/aapt" dump permissions "$apk" | grep -F 'android.permission.INTERNET'
"$ANDROID_HOME/build-tools/36.0.0/apksigner" verify "$apk"

mkdir -p /output/android-artifacts/release/apk
cp "$apk" /output/android-artifacts/release/apk/game.apk
echo 'Verified ARM64 native loader, API 36 / min API 26, network permission and APK signature.'
