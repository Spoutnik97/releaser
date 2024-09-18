# Releaser

A tool to create releases for your Node.js projects.
It works like a charm with monorepos!

## Usage

```bash
$ releaser [environment]
```

## Building

For the platform you are working on:

```bash
$ cargo build --release
```

For linux:
you need to install `cross` first:

```bash
$ cargo install cross
```

Then run:

```bash
$ cross build --release --target x86_64-unknown-linux-gnu
```

## License

Releaser is licensed under the [MIT License](LICENSE).
