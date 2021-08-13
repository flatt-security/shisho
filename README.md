# shisho

![shisho](./docs/public/images/header.png)

[![Run tests](https://github.com/flatt-security/shisho/actions/workflows/test.yml/badge.svg?branch=main)](https://github.com/flatt-security/shisho/actions/workflows/test.yml) [![Run lint](https://github.com/flatt-security/shisho/actions/workflows/lint.yml/badge.svg?branch=main)](https://github.com/flatt-security/shisho/actions/workflows/lint.yml)

## How to run shisho locally

You can run shisho program with the following command(s):

```sh
cargo run -- help
```

## How to install shisho

You can install shisho by the following command(s):

```sh
cargo install --locked --path . --force
```

After you have successfully installed shisho, you can see help as follows:

```sh
shisho help
```

You can install shell completions as follows:

```sh
# in bash
eval "$(shisho completion bash)"

# in fish
shisho completion fish | source
```

## How to run tests locally

You can run tests with the following command(s):

```sh
cargo test
```
