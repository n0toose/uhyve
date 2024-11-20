# Building Uhyve

### Testing Hermit apps using `cargo run`

As mentioned above, the Uhyve repository ships some binaries that can be used for testing purposes.

```sh
cargo run -- -v data/x86_64/rusty_demo
cargo run -- -v data/x86_64/hello_world
cargo run -- -v data/x86_64/hello_c
```

### Debugging Hermit apps

Basic support of (single-core) applications is already integrated into Uhyve.

By specifying variable `HERMIT_GDB_PORT=port`, Uhyve is working as gdbserver and is waiting on port `port` for a connection to a gdb.
For instance, with the following command Uhyve is waiting on port `6677` for a connection.

```bash
HERMIT_GDB_PORT=6677 uhyve /path_to_the_unikernel/hello_world
```

In principle, every gdb-capable IDE should be able to debug Hermit applications. (Eclipse, VSCode, ...)

#### Visual Studio Code / VSCodium

The repository [hermit-rs](https://github.com/hermitcore/hermit-rs) provides [example configuration files](https://github.com/hermitcore/hermit-rs/tree/master/.vscode) to debug a Hermit application with [Visual Studio Code](https://code.visualstudio.com/), [VSCodium](https://vscodium.com/) or derivatives of [Eclipse Theia](https://theia-ide.org/).

![Debugging Hermit apps](img/vs_code.png)

