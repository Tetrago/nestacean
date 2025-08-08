alias t := test
alias b := build
alias l := lint

[private]
default:
    @just --list

test tests="":
    @cargo nextest run --features cputest --no-fail-fast {{tests}}

lint:
    @cargo clippy

build:
    @cargo build
