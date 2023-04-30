#!/bin/sh
## name: release_nanocl.sh

# variables
pkg_name="nanocl"
arch=$(dpkg --print-architecture)
version=$(cat ./bin/nanocl/Cargo.toml | grep -m 1 "version = \"" | sed 's/[^0-9.]*\([0-9.]*\).*/\1/')
release_path="./target/${pkg_name}_${version}_${arch}"

# clear directory
rm -fr "${release_path}"
# create directories structure for package
mkdir -p "${release_path}"
mkdir -p "${release_path}"/DEBIAN
mkdir -p "${release_path}"/usr/local/bin
mkdir -p "${release_path}"/usr/local/man/man1

# Build binary

export RUSTFLAGS="-C target-feature=+crt-static"
cargo build --release --target=x86_64-unknown-linux-musl --features release --bin nanocl

# Generate man pages
for file in ./bin/nanocl/target/man/*; do
  file_name=$(basename "${file}")
  gzip < "$file" > "${release_path}"/usr/local/man/man1/"$file_name".gz
  pandoc --from man --to markdown < "$file" > ./doc/man/"${file_name%.1}".md
done
# Compress binary
upx ./target/x86_64-unknown-linux-musl/release/${pkg_name}
# Copy binary
cp ./target/x86_64-unknown-linux-musl/release/${pkg_name} "${release_path}"/usr/local/bin
# Generate DEBIAN controll
cat > "${release_path}"/DEBIAN/control <<- EOM
Package: ${pkg_name}
Version: ${version}
Architecture: ${arch}
Maintainer: next-hat team@next-hat.com
Description: A self-sufficient vms and containers orchestrator
EOM

mkdir -p ./target/debian
dpkg-deb --build --root-owner-group "${release_path}" ./target/debian/${pkg_name}_"${version}"_"${arch}".deb
