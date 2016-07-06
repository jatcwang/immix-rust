cargo build --release --features mt-trace
cp ../target/release/libimmix_rust.dylib .
gcc test.c libimmix_rust.dylib
./a.out
