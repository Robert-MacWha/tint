# Tint circuit workflows. Run `just` to list recipes.

default:
    @just --list

# Run contract formal verification with Halmos.
halmos:
    docker run -v .:/workspace ghcr.io/a16z/halmos:latest halmos --contract TintFormal --forge-build-out packages/contracts/out --loop 4