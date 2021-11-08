# Daiquiri

A REST-based microservice that controls PowerDNA DAQs. Surfaces analogue samples to Kafka.

## Development

---

Download [UEI's PowerDNA software](https://www.ueidaq.com/products/powerdna-linux-drivers-and-examples-software) and place the tar file in your repository's root.

Set up VSCode's [Remote - Containers plugin](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) to use `Dockerfile.dev` as a remote container.

To compile and run:

```shell script
cargo run
```

To compile only:

```shell script
cargo build
```

The output of the build command is placed in `/target`, and isn't shared with the host OS (shared volumes have poor file IO performance).
