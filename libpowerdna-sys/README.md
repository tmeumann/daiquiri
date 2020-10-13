# powerdna-rust

Raw PowerDNA Rust bindings.

You can run the following to generate a new set of bindings (if we need to bump the PowerDNA version, for example):

```shell script
bindgen -o <output-file.rs> \
        --whitelist-var '^(DQ.*|STS_.*)$' \
        --whitelist-type '^DQ.*$' \
        --whitelist-function '^Dq.*$' \
        --no-debug '^.*$' \
        /pdna/PowerDNA_4.10.1/src/DAQLib/PDNA.h
```

Note that some functions have had the mutability of their arguments massaged (after going over the PowerDNA
documentation). This makes them a bit easier for us to work with in Rust (allowing us to share arguments across
threads without locking, for example). It does mean that you'll have to do a diff if you generate a new bindings
file though, and reapply these changes.