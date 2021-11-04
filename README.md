# Daiquiri

So, you want to query the DAQ?

## Development

---

Set up VSCode's [Remote - Containers](https://marketplace.visualstudio.com/items?itemName=ms-vscode-remote.remote-containers) to use `Dockerfile.dev` as a remote container.

To run:

```shell script
cargo run
```

To compile:

```shell script
cargo build
```

> ⚠️ Note that the development container has been configured to compile into a persistent volume (mounted to `/target`), instead of cargo's normal target directory. The normal target directory is `<repo-root>/target`, and is shared with the host OS by virtue of it being within the workspace's shared volume. If the target directory is shared between a docker container and its host, compilation is extremely slow. If there's any doubt about what's happening, refer to the dockerfile as the source of truth.
