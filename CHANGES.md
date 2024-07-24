# Changelog

## 0.1.0 - sudo release

- Rename the crate to `sudo`
- Add `polkit` and `doas` functions
- Major code refactor, everything is now in the `Elevate` builder struct
- Add ability to specify a custom wrapper other than `sudo`

## sudo 0.6.0

- Use full path for the `sudo` command while escalating

## sudo 0.5.0

- Add API for keeping environment variables with a certain prefixes
- Make matching RUST_BACKTRACE case-in-sensitive
- Return the previous environment when escalating from SUID
- More documentation improvements

## sudo 0.4.0

- Propagate RUST_BACKTRACE environment variable
- Add example `backtrace.rs`
- Build examples with CI too

## sudo 0.3.1

- Multiple documentation fixes

## sudo 0.3.0

- Handle SUID binaries
- Add example `suid.rs`

## sudo 0.2.1

- Fix documentation

## sudo 0.2.0

- First public release
- Taking over the crate name `sudo` from Vincenzo Tilotta

## sudo 0.1.3 and 0.0.0

- Name squatted by Vincenzo Tilotta
