# BVE-Reborn

BVE-Reborn is a remake of the train simulator OpenBVE, focusing on modern programming
techniques and visual quality, as well as performance and flexibility.

While progress is strong, there is still a lot of work to do in order to get a working
demo.

BVE uses Unity to provide graphics, main gameplay management, and user input. For all
internal code it uses rust. Rust allows the code to be robust and safe from crashes
but just as fast as C/C++.

## Building from Source

Binaries will be provided when there is a release, but for now, only developers can
make use of BVE-Reborn. If you are a developer the following is how you build from source.

### Rust toolchain

You need to install the 2020-01-16 nightly toolchain of rust:

```
rustup install nightly-2020-01-16
```

Then you may run the main build process:

```
cargo run --bin bve-build --release
```

This will build bve, generate C/C++/C# bindings, and build with Unity.