Daiquiri
========

So you want to query the DAQ huh?


Development
-----------

#### Remote Debugging (WIP) ####

```shell script
docker build -f Dockerfile.dev -t gdb .
docker run --init -p 3030:3030 -p 5555:5555 -p 1234:1234 --entrypoint bash -ti --privileged gdb:latest
```


#### Cross-Compilation on macOS ####

Add the rust target:

```shell script
rustup target add x86_64-unknown-linux-gnu
```

You'll need to build a toolchain to link against native C libraries. It's easiest to use
crosstool-NG for this. You can get it using Homebrew:

```shell script
brew install crosstool-ng
```

Crosstool-NG requires a case-sensitive file system, so create one using Disk Utility (an APFS
volume is easiest).[^1] You can call this what you want, but I'm going to refer to it as
'Toolchains' from here on.

Copy the crosstool-NG config to the case-sensitive file system and build your toolchain [^2]:
```shell script
cp -r <repo-root>/ct-ng /Volumes/Toolchains/
cd /Volumes/Toolchains/ct-ng
ct-ng menuconfig  # optional (if you want to tweak stuff like where to install etc.)
ct-ng build  # builds & installs the toolchain
```

Download the compiled PowerDNA and ZMQ Linux libraries from (here)[https://ausport.sharepoint.com/:u:/r/sites/ATISoftwareDevTEam/Shared%20Documents/02%20Projects/2020/Wetplate/UEIPAC%20Stuff/syslib.tar.gz?csf=1&web=1&e=UVUDfu],
and extract them in the repository's root:
```shell script
tar xzf syslib.tar.gz
```

Update your `.profile` or `.bashrc` or whatever to include these lines and reload it:
```shell script
export PATH="${PATH}:/Volumes/Toolchains/x-tools/x86_64-unknown-linux-gnu/bin"
export LIBZMQ_PREFIX="<path-to-repo>/syslib/zmq"
```

Put the following in `~/.cargo/config.toml`:
```toml
[target.x86_64-unknown-linux-gnu]
linker = "x86_64-unknown-linux-gnu-gcc"
```

Now this will hopefully just work:
```shell script
cargo build
```

###### Compiling the libraries ######

These aren't complete steps, but there should be enough breadcrumbs here to be able to piece
things together...

If the ZMQ and/or PowerDNA versions need to be bumped, they may need to be recompiled. The
easiest way to do this is to compile them in the target docker container and copy the results
out into the host. Example commands for ZMQ once you've downloaded and extracted the source
tarball:

```shell script
./configure --prefix <some-directory-shared-with-host>
make
make install
```

ZMQ releases can be found (here)[https://github.com/zeromq/libzmq/releases].


[^1]: You can make Macintosh HD case-sensitive if you want, but I don't recommend it - it
tends to break third-party apps.

[^2]: This will install the toolchain to
`/Volumes/Toolchains/x-tools/x86_64-unknown-linux-gnu` by default.
