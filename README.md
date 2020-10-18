Daiquiri
========

So you want to query the DAQ huh?


Development
-----------

#### Cross-Compilation ####

##### macOS #####

You'll need to build a toolchain to cross-compile. It's easiest with crosstool-NG, which you
can get using Homebrew:

```shell script
brew install crosstool-ng
```

Crosstool-NG requires a case-sensitive file system, so create one using Disk Utility (an APFS
volume is easiest).[^1] You can call this what you want, but I'm going to refer to it as
'Toolchains' from here on.

Run the following [^2]:
```shell script
cd <repo-root>/ct-ng
ct-ng menuconfig  # optional (if you want to tweak stuff like where to install etc.)
ct-ng build  # builds & installs the toolchain
```

Update your `PATH` in your `.profile` or `.bashrc` or whatever:
```shell script
export PATH="${PATH}:/Volumes/Toolchains/x-tools/bin"
```

Download the compiled PowerDNA and ZMQ Linux libraries from (here)[https://ausport.sharepoint.com/:u:/r/sites/ATISoftwareDevTEam/Shared%20Documents/02%20Projects/2020/Wetplate/UEIPAC%20Stuff/syslib.tar.gz?csf=1&web=1&e=UVUDfu],
and extract them in the repository's root:
```shell script
tar xzf syslib.tar.gz
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
